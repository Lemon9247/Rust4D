//! Headless smoke test for the Wave 5 Lua demo game (`games/demo`).
//!
//! Loads the real demo scene + scripts and drives the Lua lifecycle
//! (`on_init` / `on_update` / `on_fixed_update`) against the live `hecs::World`
//! the same way `ScriptSystem` does — but without a window, GPU, or audio
//! device. This proves the demo's Lua loads, the scene entities are found by
//! name, the gallery rotators mutate transforms (geometry dirty), and the
//! trigger zone fires on enter.
//!
//! Run with:
//!   cargo run --example demo_smoke
//!   cargo run --example demo_smoke -- --frames 600
//!
//! Audio is unavailable headless, so `audio.get_listener_position()` reads as
//! the zero vector — the player is therefore "inside" the central zone (which
//! is centered on the origin), and the trigger-enter path fires on the first
//! fixed update. That is the expected headless behaviour; see the demo-game
//! report's "Engine limitations" section.

use std::env;

use rust4d_core::{ActiveScene, Scene, Transform4D};
use rust4d_scripting::{InputSnapshot, ScriptConfig, ScriptEngine};

/// Fixed timestep used by `ScriptSystem` (1/60 s).
const FIXED_DT: f32 = 1.0 / 60.0;

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .format_timestamp(None)
        .init();

    let args: Vec<String> = env::args().collect();
    let frames: u32 = args
        .iter()
        .position(|a| a == "--frames")
        .and_then(|idx| args.get(idx + 1))
        .and_then(|v| v.parse::<u32>().ok())
        .unwrap_or(240);

    // --- Load the demo scene into a live World (no physics player body; the
    //     app creates that separately and the demo scripts don't need it). ---
    let scene = Scene::load("games/demo/scenes/demo.ron")
        .expect("failed to load games/demo/scenes/demo.ron — run from the repo root");
    let active = ActiveScene::from_template(&scene, None);
    let entity_count = active.world.entity_count();
    let scene_name = active.name.clone();
    let mut world = active.world; // take ownership; World is not Clone
    log::info!(
        "[smoke] demo scene '{}' loaded with {} entities",
        scene_name,
        entity_count
    );

    // --- Load the demo game scripts. ---
    let config = ScriptConfig {
        scripts_dir: "games/demo".to_string(),
        hot_reload: false,
        ..Default::default()
    };
    let mut engine = ScriptEngine::new(config).expect("ScriptEngine init failed");
    engine.load_game().expect("demo load_game failed");

    // --- on_init ---
    engine.set_world(&mut world);
    engine.call_init().expect("demo on_init errored");
    let init_dirty = engine.clear_world();
    log::info!("[smoke] on_init geometry_dirty={}", init_dirty);

    // Sanity: the scripts should have found the gallery + trigger entities.
    let rotator_count: i64 = engine
        .lua()
        .load("return gallery_count")
        .eval()
        .expect("read gallery_count");
    assert!(
        rotator_count == 6,
        "expected 6 gallery rotators, got {}",
        rotator_count
    );
    log::info!("[smoke] rotators created: {}", rotator_count);

    // --- Drive the loop: update + fixed-update per frame. ---
    let input = InputSnapshot::new();
    let mut any_dirty = false;
    for f in 0..frames {
        engine.set_world(&mut world);
        engine.set_input(&input);

        engine
            .call_update(FIXED_DT)
            .expect("demo on_update errored");
        engine
            .call_fixed_update(FIXED_DT)
            .expect("demo on_fixed_update errored");

        let dirty = engine.clear_world();
        engine.clear_input();
        any_dirty = any_dirty || dirty;

        // Log a heartbeat every ~2 seconds of simulated time.
        if f > 0 && f % 120 == 0 {
            let enters: i64 = engine
                .lua()
                .load("return trigger_enters")
                .eval()
                .expect("read trigger_enters");
            log::info!(
                "[smoke] frame {} entity_count={} trigger_enters={}",
                f,
                world.entity_count(),
                enters
            );
        }
    }

    // --- Assertions: the demo actually did something. ---
    let enters: i64 = engine
        .lua()
        .load("return trigger_enters")
        .eval()
        .expect("read trigger_enters");
    assert!(
        enters >= 1,
        "trigger should have entered at least once headless (listener at origin = inside zone); got {}",
        enters
    );

    assert!(
        any_dirty,
        "scripts should have flagged geometry_dirty by setting transforms"
    );

    // A gallery entity's rotation should have advanced away from identity.
    let tesseract = world
        .get_by_name("tesseract")
        .expect("tesseract entity should exist");
    let tx = *world
        .ecs()
        .get::<&Transform4D>(tesseract)
        .expect("tesseract should have a transform");
    let rotor = tx.rotation;
    let identity = rotor.s == 1.0
        && rotor.b_xw == 0.0
        && rotor.b_xy == 0.0
        && rotor.b_xz == 0.0
        && rotor.b_yz == 0.0
        && rotor.b_yw == 0.0
        && rotor.b_zw == 0.0
        && rotor.p == 0.0;
    assert!(
        !identity,
        "tesseract rotation should have advanced past identity after {} frames",
        frames
    );
    log::info!(
        "[smoke] tesseract rotor after run: s={:.4} b_xw={:.4}",
        rotor.s,
        rotor.b_xw
    );

    // --- on_shutdown ---
    engine.call_shutdown().expect("demo on_shutdown errored");

    log::info!(
        "[smoke] PASS: {} frames, trigger_enters={}, geometry_dirty={}, rotation advanced",
        frames,
        enters,
        any_dirty
    );
}
