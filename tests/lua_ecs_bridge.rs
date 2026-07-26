//! Lua ↔ hecs ECS bridge regression tests (Wave 5 Phase 2)
//!
//! Drives the real `ScriptEngine` + per-call `WorldRef` integration seam (the
//! same path `ScriptSystem` uses) to prove that Lua `world.spawn` / `set` /
//! `despawn` operate on the live `rust4d_core::World` and that Rust can read
//! the results back.

use std::fs;
use std::path::Path;

use rust4d_core::{Material, Name, Transform4D, World};
use rust4d_math::Vec4;
use rust4d_scripting::{InputSnapshot, ScriptConfig, ScriptEngine};
use tempfile::TempDir;

/// Write a `main.lua` into a temp game directory and return a `ScriptConfig`
/// pointing at it.
fn make_game(main_lua: &str) -> (TempDir, ScriptConfig) {
    let dir = TempDir::new().unwrap();
    write_main(dir.path(), main_lua);
    let config = ScriptConfig {
        scripts_dir: dir.path().to_string_lossy().to_string(),
        hot_reload: false,
        ..Default::default()
    };
    (dir, config)
}

fn write_main(dir: &Path, main_lua: &str) {
    fs::write(dir.join("main.lua"), main_lua).unwrap();
}

fn make_engine(main_lua: &str) -> ScriptEngine {
    let (_dir, config) = make_game(main_lua);
    let mut engine = ScriptEngine::new(config).unwrap();
    engine.load_game().unwrap();
    engine
}

/// Dispatch `on_init` with a live world registered via app_data.
fn call_init_with_world(engine: &ScriptEngine, world: &mut World) -> bool {
    engine.set_world(world);
    let res = engine.call_init();
    let dirty = engine.clear_world();
    res.unwrap();
    dirty
}

/// Dispatch `on_update(dt)` with a live world + input snapshot.
fn call_update_with_world(engine: &ScriptEngine, world: &mut World, dt: f32) -> bool {
    engine.set_world(world);
    engine.set_input(&InputSnapshot::new());
    let res = engine.call_update(dt);
    let dirty = engine.clear_world();
    engine.clear_input();
    res.unwrap();
    dirty
}

#[test]
fn lua_spawn_then_rust_queries_back() {
    let engine = make_game_engine(
        r#"
        spawned = false
        function on_init()
            e = world.spawn({
                name = "from_lua",
                transform = { x = 1, y = 2, z = 3, w = 4 },
                material = { 0.2, 0.4, 0.6, 1.0 },
            })
            spawned = true
        end
    "#,
    );

    let mut world = World::new();
    call_init_with_world(&engine, &mut world);

    assert_eq!(world.entity_count(), 1);
    let entity = world
        .get_by_name("from_lua")
        .expect("Lua-spawned entity should be findable by name from Rust");

    let name = world.ecs().get::<&Name>(entity).unwrap();
    assert_eq!(name.0, "from_lua");

    let transform = *world.ecs().get::<&Transform4D>(entity).unwrap();
    assert_eq!(transform.position, Vec4::new(1.0, 2.0, 3.0, 4.0));

    let material = *world.ecs().get::<&Material>(entity).unwrap();
    assert_eq!(material.base_color, [0.2, 0.4, 0.6, 1.0]);

    // Rust-side query agrees with Lua spawn.
    let transform_count = world.ecs().query::<&Transform4D>().iter().count();
    assert_eq!(transform_count, 1);
}

#[test]
fn lua_set_transform_then_rust_reads() {
    let engine = make_game_engine(
        r#"
        function on_init()
            e = world.spawn({ transform = { x = 0, y = 0, z = 0, w = 0 } })
        end
        function on_update(dt)
            e:set("transform", { x = 10, y = 0, z = 0, w = 5 })
        end
    "#,
    );

    let mut world = World::new();
    call_init_with_world(&engine, &mut world);
    let entity = world
        .root_entities()
        .pop()
        .expect("entity should exist after init");

    // Before update, transform is at origin.
    let t0 = *world.ecs().get::<&Transform4D>(entity).unwrap();
    assert_eq!(t0.position.x, 0.0);

    let dirty = call_update_with_world(&engine, &mut world, 0.016);
    assert!(dirty, "setting a transform should flag geometry dirty");

    let t1 = *world.ecs().get::<&Transform4D>(entity).unwrap();
    assert_eq!(t1.position.x, 10.0);
    assert_eq!(t1.position.w, 5.0);
}

#[test]
fn lua_despawn_removes_entity() {
    let engine = make_game_engine(
        r#"
        function on_init()
            e = world.spawn({ name = "gone" })
        end
        function on_update(dt)
            if e:is_alive() then
                world.despawn(e)
            end
        end
    "#,
    );

    let mut world = World::new();
    call_init_with_world(&engine, &mut world);
    assert_eq!(world.entity_count(), 1);
    assert!(world.get_by_name("gone").is_some());

    let dirty = call_update_with_world(&engine, &mut world, 0.016);
    assert!(dirty, "despawning should flag geometry dirty");

    assert_eq!(world.entity_count(), 0);
    assert!(world.get_by_name("gone").is_none());
}

#[test]
fn lua_query_counts_spawned_entities() {
    let engine = make_game_engine(
        r#"
        function on_init()
            world.spawn({ transform = { x = 1, y = 0, z = 0, w = 0 } })
            world.spawn({ transform = { x = 2, y = 0, z = 0, w = 0 } })
            world.spawn({ name = "marker" })
            count = 0
            for _ in world.query("transform") do
                count = count + 1
            end
            total = world.entity_count()
        end
    "#,
    );

    let mut world = World::new();
    call_init_with_world(&engine, &mut world);

    // Lua-reported counts (read back through the engine).
    let count: i64 = engine.lua().load("return count").eval().unwrap();
    assert_eq!(count, 2, "two entities have transforms");
    let total: i64 = engine.lua().load("return total").eval().unwrap();
    assert_eq!(total, 3, "three entities total");
    assert_eq!(world.entity_count(), 3);
}

/// Helper: build a `ScriptEngine` from inline Lua source.
fn make_game_engine(main_lua: &str) -> ScriptEngine {
    make_engine(main_lua)
}
