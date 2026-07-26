# Changelog

All notable changes to Rust4D are documented here. Format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the project uses
Conventional Commits.

## Unreleased — Engine Expansion

### Added

- Fragment-space floor checkerboard: the render pipeline now passes the
  sliced world-space XYZ through to the fragment shader and computes the
  checker pattern per-pixel from world XZ (`mod(floor(x/cell) +
  floor(z/cell), 2)`), producing crisp, resolution-independent cell
  boundaries instead of the smeared per-vertex gradient on large hyperplane
  faces. Floor fragments are tagged via a sentinel vertex alpha written by
  `CheckerboardGeometry`; the two floor colors and cell size are carried in
  new `RenderUniforms` fields (`floor_color_a` / `floor_color_b`).
- `Vertex3D` gains a `world_position` field (struct grows 48 → 64 bytes) and
  `FLOOR_ALPHA_SENTINEL` is re-exported from `rust4d_render::pipeline`.
- General `Mesh4D` tetrahedral mesh type with merge, transform, weld,
  validation, Gram-determinant cell volumes, and watertightness checks.
- Full primitive catalog:
  - tesseract (fixed boundary-only tetrahedralization),
  - hypersphere,
  - regular 5-cell, 16-cell, 24-cell, 600-cell,
  - spherinder,
  - cubinder,
  - duocylinder.
- RON `ShapeTemplate` variants for all primitives, including defaulted
  resolution fields.
- Shape-aware physics collider hints for scene instantiation.
- `scenes/gallery.ron` with all primitive exhibits.
- `examples/shape_showcase.rs`, an offscreen visual verification harness for
  the full primitive catalog.
- Two-sided Blinn-Phong lighting with specular highlights, point lights, and
  distance fog.
- `rust4d_input::ActionMap` and `CameraAction` for semantic camera bindings.
- Lua ECS entity handle bit round-tripping via `world.entity_from_bits(bits)`
  and `entity:equals(other)`.
- GitHub Actions CI: formatting, clippy with `-D warnings`, rustdoc with
  `-D warnings`, and workspace tests.
- Project skills for 4D geometry, headless visual verification, and production
  readiness.
- Shape catalog documentation (`docs/shapes.md`).
- **Wave 5 — scripting + audio integration:** `ScriptSystem` (`src/systems/script.rs`) owns a `ScriptEngine` and an optional `AudioEngine4D`, drives `on_init` / `on_update` / `on_fixed_update` / `on_shutdown` from the app loop, and runs a fixed-step accumulator. `update` runs before `SimulationSystem` so scripts set velocities/transforms before physics. `AppConfig` gains `GameConfig` + `AudioConfig`; `--game <dir>` overrides the game directory. Audio and scripting both degrade gracefully (silent / disabled) when no device / no game dir is configured.
- **Wave 5 — real Lua ECS bridge:** `world.spawn` / `query` / `find_by_name` / `get` / `set` / `despawn` / `entity_count` operate on the live `hecs::World` via a per-call `WorldRef` registered into `app_data` for the duration of each callback. Component access dispatches through an explicit name registry (`name` / `tags` / `transform` / `material` / `dirty` / `shape` / `physics_body` / `parent` / `children`). Input bindings read a per-frame `InputSnapshot`; audio bindings call the live `AudioEngine4D` via an `AudioRef`. `tests/lua_ecs_bridge.rs` guards the bridge.
- **Wave 5 — Lua demo game (`games/demo/`):** a shipped game directory with no compiled game code, exercising the full scripting surface — scripted 4D rotation in W-involving planes, a 4D-AABB trigger zone with a spatial audio cue + HUD flash + manual sine scale-pulse, a HUD readout, and synthesized WAV cues. Run with `R4D_SCENE__PATH=games/demo/scenes/demo.ron cargo run -- --game games/demo`; headless smoke via `cargo run --example demo_smoke`.

### Changed

- `CameraController` now processes semantic actions from an `ActionMap` while
  preserving the legacy keyboard defaults.
- Workspace is now `rustfmt` clean.
- Rendering disables back-face culling because slice-generated triangle winding
  is not stable across all marching-tetrahedra cases.
- Bumped `wgpu` 24 -> 25 (and `egui` / `egui-wgpu` / `egui-winit` 0.31 ->
  0.32, since egui-wgpu 0.31 pins wgpu 24). API fixes: `request_device` now
  takes a single `DeviceDescriptor` argument (the trace path moved into the
  descriptor's new `trace: Trace` field), and `device.poll(Maintain::Wait)` is
  now `device.poll(PollType::Wait)` (poll returns `Result<PollStatus,
  PollError>`). All examples and the render context updated.

### Fixed

- Tesseract geometry now emits only the 48 boundary tetrahedra. The previous
  84-tetrahedron Kuhn-derived surface included 36 internal membranes, wasting
  GPU slice work and producing spurious interior walls when viewed from inside.

## PR #15 — 4D Rendering Debug Fix

### Added

- `tests/slice_invariant.rs`, an end-to-end invariant suite for camera,
  physics, controller, and simulation movement.
- `examples/headless_protocol.rs`, an offscreen GPU visual verification harness
  for slice-plane drift and projection issues.
- `flake.nix` dev shell with Rust, Vulkan wiring, lavapipe, and image tools.
- `docs/4d-math.md`, documenting rotors, SkipY, slicing, projection, movement
  invariants, and matrix conventions.
- Minimal `AGENTS.md` plus progressive-disclosure skills.

### Fixed

- Long-standing 4D movement bug: WASD movement after 4D rotation drifted across
  the slice plane because world axes were scaled anisotropically. Speeds now
  scale semantic movement inputs instead.
- Perspective matrix depth range now matches wgpu `[0, 1]` rather than OpenGL
  `[-1, 1]`.
- `rotate_w` and `rotate_xw` now operate in their documented 4D planes after
  SkipY remapping.
- Removed dead `camera_eye` from `SliceParams`.

### Quality

- Workspace clippy-clean and rustdoc-clean at merge time.
- Windowed and headless visual verification performed.
