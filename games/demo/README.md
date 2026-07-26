# Rust4D — Lua Demo Game (Wave 5, Phase 3)

A scripted 4D demo shipped as a **pure game directory** — no compiled game
code. The engine binary loads `games/demo/` via `--game`, runs `main.lua`, and
the demo drives the scene with the Lua scripting surface Wave 5 integrated into
the app loop.

It is a *vertical slice* proving the engine/game split: scripting, ECS bridge,
4D math, input, audio, HUD, trigger zones, and tweens — all from Lua, on top of
a polished renderer (fragment-space checkerboard floor, wgpu 25).

## Run

From the repo root:

```sh
R4D_SCENE__PATH=games/demo/scenes/demo.ron \
  nix develop --command cargo run -- --game games/demo
```

* `--game games/demo` points the engine at this game directory (sets
  `GameConfig.game_dir`; scripts resolve from `games/demo/`).
* `R4D_SCENE__PATH` selects the demo scene. The engine loads the scene from
  `[scene].path` (default `scenes/default.ron`), which is independent of
  `--game`, so the demo ships its scene at `games/demo/scenes/demo.ron` and
  points the scene path at it via the env var. `config/default.toml` is left
  unchanged so `cargo run` still boots the default scene.

### Headless smoke test (no window/GPU/audio)

```sh
nix develop --command cargo run --example demo_smoke
nix develop --command cargo run --example demo_smoke -- --frames 600
```

This loads the real scene + scripts, drives `on_init` / `on_update` /
`on_fixed_update` against the live `hecs::World`, and asserts the gallery
rotators advance, geometry is marked dirty, and the trigger zone fires. Audio is
unavailable headless, so the audio listener reads as the origin (inside the
central zone) and the trigger-enter path fires on the first fixed update — the
expected headless behaviour.

## What it demonstrates

A floor, six 4D primitives in a ring, and a central **trigger zone**:

* **Scripted 4D rotation** — each gallery primitive is found by name and rotated
  every frame in a W-involving bivector plane (`XW` / `YW` / `ZW`) via
  `Rotor4.from_plane`, so the 3D slice morphs as the shape spins. Rotation is
  written back with `entity:set("transform", …)`; `geometry_dirty` propagates
  so the render cache rebuilds and motion is visible.
* **Trigger zone** — `on_fixed_update` runs a 4D AABB containment check
  (player vs the zone center). On **enter** it fires a spatial audio cue, a HUD
  flash, and a scale pulse on the zone entity; on **exit** it plays a low tone
  and resets the scale.
* **HUD** — `on_update` draws a readout: player 4D position, W-slide indicator,
  entity count, trigger state + status bar, and controls. (See *Limitations*
  below re. on-screen rendering.)
* **Audio** — two tiny synthesized WAV cues (`assets/blip.wav`,
  `assets/tone.wav`) played spatially at the zone position. Synthesized from
  pure math with the Python stdlib — see `assets/generate_cues.py`. Degrades to
  silent + logged when no audio device is available.
* **Tween** — the zone's enter pulse is a manual sine
  (`scale = base * (1 + 0.3*sin(t))`). The engine's `TweenManager` is not
  exposed to Lua, so the pulse is implemented by hand in
  `scripts/trigger_zone.lua`.
* **Input** — `input.is_key_pressed("Q"/"E")` drives the W-slide HUD indicator,
  showing the live input binding works end-to-end.

## Controls

| Input | Action |
|-------|--------|
| `W` `A` `S` `D` | Move (3D, handled by the engine) |
| `Q` / `E` | Slide along the W axis |
| Mouse | Look (capture toggles on click) |
| Scroll | Adjust the slice hyperplane offset |
| `Esc` | Release / recapture cursor |
| `R` | Reset camera |
| Walk to the ring center | Trigger the zone |

## Files

```
games/demo/
├── main.lua                 # entry point: on_init/update/fixed_update/shutdown
├── scripts/
│   ├── rotator.lua          # per-entity 4D spin (W-involving planes)
│   ├── trigger_zone.lua     # 4D AABB detection + scale pulse
│   └── hud.lua              # per-frame HUD readout
├── scenes/
│   └── demo.ron             # floor + 6-primitive ring + central trigger zone
├── assets/
│   ├── blip.wav             # synthesized enter cue (880 Hz, 0.35 s, 15 KB)
│   ├── tone.wav             # synthesized exit cue (220 Hz, 0.45 s, 20 KB)
│   └── generate_cues.py     # regenerates the WAVs (stdlib only)
├── config.toml              # reference config (see header comment)
└── README.md                # this file
```

## Engine features exercised

| Feature | Binding used | Where |
|--------|--------------|-------|
| ECS bridge | `world.find_by_name`, `entity:get/set("transform")`, `world.entity_count` | `main.lua`, `rotator.lua`, `trigger_zone.lua` |
| 4D math | `Rotor4.from_plane`, `Rotor4.identity` | `rotator.lua`, `trigger_zone.lua` |
| Input | `input.is_key_pressed` | `main.lua` |
| Audio | `audio.load_sound`, `audio.play_oneshot_spatial`, `audio.get_listener_position` | `main.lua` |
| HUD | `hud.text`, `hud.progress_bar`, `hud.flash`, `hud.screen_size` | `hud.lua` |
| Logging | `log.info` | all scripts |

## Engine limitations hit (documented, not worked around)

1. **No player/camera-position binding.** Lua has no direct way to read the
   camera or player body position. The demo uses `audio.get_listener_position()`
   as the player-position proxy — `ScriptSystem` updates the audio listener to
   the camera position each frame. It is one frame stale, which is negligible
   for a trigger. When audio is unavailable (headless), the listener reads as
   the zero vector.
2. **HUD bindings are render-stubs.** `hud.*` calls are API-complete but only
   log at `trace` level; they are not wired into the render `HudContext`, so the
   HUD is not visibly drawn yet. The calls exercise the full API so a future
   wave that plugs a `HudContext` into `ScriptSystem::update` makes them visible
   with no script changes.
3. **`TweenManager` is not Lua-bound.** The pulse is a manual sine.
4. **No slice-offset binding.** The camera's slice offset (`get_slice_w()`) is
   not exposed to Lua, so the HUD shows the player W-position instead.
5. **Spawn-from-Lua does not support `shape` components.** Visible geometry is
   placed in the RON scene; Lua only rotates/recolors it. (Per the Wave 5
   engine-integration report.)
6. **Physics raycast/query bindings are stubs.** Trigger detection uses
   `world.query`-style AABB math in Lua instead. (Per the report.)

See `scratchpad/reports/2026-07-26-wave-5-demo-game.md` for the full report.

## Related

* Wave 5 plan: `scratchpad/plans/2026-07-26-wave-5-polish-demos.md` (Phase 3)
* Engine integration report: `scratchpad/reports/2026-07-26-wave-5-engine-integration.md`
* Render polish report: `scratchpad/reports/2026-07-26-wave-5-render-polish.md`