# Configuration and Architecture Reviewer Report

## Summary

The Rust4D codebase has a well-structured configuration system using figment for layered config loading. However, several configuration values are defined but not actually used, and some architectural concerns exist in main.rs organization. The crate structure is clean with minimal dependency issues, though rust4d_render depends on rust4d_input which creates an unusual coupling for a render crate.

---

## Unused Config Values

| Config Key | Location | Status | Notes |
|------------|----------|--------|-------|
| `camera.pitch_limit` | `config.rs:137`, `default.toml:16` | **NOT CONNECTED** | Hardcoded at 89 degrees in `Camera4D::PITCH_LIMIT` (camera4d.rs:49). Config value is loaded but never used. |
| `window.fullscreen` | `config.rs:108`, `default.toml:8` | **NOT CONNECTED** | Loaded but never read on startup. Fullscreen only toggles via F key at runtime. |
| `window.vsync` | `config.rs:110`, `default.toml:9` | **NOT CONNECTED** | Loaded but never used. VSync/present mode not configured from TOML. |
| `input.w_rotation_sensitivity` | `config.rs:162`, `default.toml:22` | **NOT CONNECTED** | Loaded but never passed to `CameraController`. Controller uses its own default (0.005). No `with_w_rotation_sensitivity()` builder method exists or is called. |
| `debug.show_overlay` | `config.rs:247`, `default.toml:40` | **NOT IMPLEMENTED** | Loaded but feature not implemented. No debug overlay exists. |
| `debug.log_level` | `config.rs:249`, `default.toml:41` | **NOT CONNECTED** | Loaded but never used. `env_logger::init()` uses RUST_LOG env var, ignoring config. |
| `debug.show_colliders` | `config.rs:251`, `default.toml:42` | **NOT IMPLEMENTED** | Loaded but feature not implemented. No collider visualization exists. |

### Config Values That ARE Connected (for reference)
- `window.title` - Used in window creation and title updates
- `window.width/height` - Used in window creation
- `camera.start_position` - Fallback if scene has no player_spawn
- `camera.fov/near/far` - Used in perspective matrix
- `input.move_speed/w_move_speed/mouse_sensitivity/smoothing_*` - Passed to CameraController
- `physics.gravity/jump_velocity` - Converted via `to_physics_config()`
- `rendering.*` - All used (max_triangles, background_color, light_dir, ambient/diffuse)
- `scene.path/player_radius` - Used in scene loading

---

## Architecture Issues

| Issue | Location | Severity | Description |
|-------|----------|----------|-------------|
| Dead code: `thickness` field | `crates/rust4d_math/src/hyperplane.rs:33` | Low | Field is declared but never read. Compiler warning present. |
| Test flakiness | `src/main.rs:555-560` | Medium | `test_env_override` fails when run with other tests due to env var pollution. Tests share environment state. |
| Render depends on Input | `crates/rust4d_render/Cargo.toml:9` | Low | `rust4d_render` depends on `rust4d_input` for `CameraControl` trait. Unusual coupling - render crate shouldn't need input crate. |
| Config duplication | `src/config.rs` + `default.toml` | Low | Default values duplicated in both Rust code and TOML. If TOML is missing, Rust defaults are used. Could diverge. |
| No config validation | `src/config.rs:62-95` | Low | Config values accepted without validation (e.g., negative max_triangles, invalid fov). |

---

## main.rs Analysis

| Concern | Lines | Description |
|---------|-------|-------------|
| God object: `App` struct | 29-44 | `App` holds all state: window, render context, pipelines, scene, geometry, camera, controller, timing. Could be split. |
| Mixed responsibilities | 173-223 | `resumed()` handles window creation, render context, pipelines, and geometry upload. Single method doing too much. |
| Massive event handler | 226-521 | `window_event()` is ~300 lines handling close, resize, keyboard, mouse wheel, and full game loop in `RedrawRequested`. |
| Game loop in redraw | 304-518 | The entire physics update, geometry rebuild, camera sync, and render is in `RedrawRequested`. Should be separate methods. |
| Magic numbers | 309, 452-453 | `1.0 / 30.0` for max dt, `0.5`/`2.0` for w_color_strength/w_range not configurable. |
| Hardcoded slice_offset | 301 | `scroll * 0.1` sensitivity hardcoded, not configurable. |

### Positive Aspects
- Clean use of the ApplicationHandler trait
- Good separation of physics step and camera sync
- Dirty flag system for geometry rebuild is efficient
- Configuration loading is well-structured with fallbacks

---

## Crate Structure Review

### Current Dependencies (from Cargo.toml files)
```
rust4d (main app)
  -> rust4d_math
  -> rust4d_core -> rust4d_math, rust4d_physics
  -> rust4d_render -> rust4d_math, rust4d_core, rust4d_input (!)
  -> rust4d_input -> rust4d_math
  -> rust4d_physics -> rust4d_math
```

### Observations
1. **rust4d_render depends on rust4d_input** - This is unusual. The dependency exists only for the `CameraControl` trait (used by Camera4D). Consider moving the trait to rust4d_core or rust4d_math.

2. **rust4d_core depends on rust4d_physics** - This creates a tight coupling between core types and physics. Could be optional via feature flag.

3. **Re-exports are good** - Each crate re-exports commonly used types from dependencies, making imports cleaner.

### ARCHITECTURE.md Accuracy
The documentation diagram shows:
```
rust4d_render --> rust4d_math
```
But actual code has:
```
rust4d_render --> rust4d_math, rust4d_core, rust4d_input
```
**The ARCHITECTURE.md is incomplete** - it doesn't show render's dependency on input.

---

## Recommendations

### High Priority
1. **Connect `camera.pitch_limit`** - Add a way to pass pitch_limit to Camera4D instead of hardcoding.
2. **Connect `window.fullscreen`** - Apply fullscreen setting on startup, not just toggle.
3. **Connect `window.vsync`** - Configure wgpu present mode based on config.
4. **Connect `input.w_rotation_sensitivity`** - Add builder method and pass from config.

### Medium Priority
5. **Fix test_env_override flakiness** - Use `serial_test` crate or isolate env var tests.
6. **Split main.rs game loop** - Extract update/render into separate methods.
7. **Connect `debug.log_level`** - Initialize logger with config value.
8. **Move CameraControl trait** - Relocate from rust4d_input to rust4d_math to break render->input dependency.

### Low Priority
9. **Remove or use `thickness` field** - Either use it in hyperplane slicing or remove it.
10. **Add config validation** - Validate numeric ranges, clamp or error on invalid values.
11. **Update ARCHITECTURE.md** - Add missing render->input dependency to diagram.
12. **Consider physics as optional** - Make rust4d_core's physics dependency a feature flag.

---

## Cross-Cutting Issues for Hive Mind

1. **Hyperplane `thickness` dead code** - Math Reviewer should verify if this was intended for thick slicing or should be removed.
2. **CameraControl trait location** - Input Reviewer should consider if the trait belongs in a different crate.
3. **Debug visualization features** - Render Reviewer should note that `show_overlay` and `show_colliders` are config but not implemented.

---

## Files Modified/Examined
- `/home/lemoneater/Projects/Rust4D/src/main.rs` - Main application, game loop
- `/home/lemoneater/Projects/Rust4D/src/config.rs` - Configuration structs
- `/home/lemoneater/Projects/Rust4D/config/default.toml` - Default config values
- `/home/lemoneater/Projects/Rust4D/Cargo.toml` - Workspace and main package
- `/home/lemoneater/Projects/Rust4D/ARCHITECTURE.md` - Architecture documentation
- `/home/lemoneater/Projects/Rust4D/crates/rust4d_render/src/camera4d.rs` - Camera implementation
- `/home/lemoneater/Projects/Rust4D/crates/rust4d_input/src/camera_controller.rs` - Input handling
- `/home/lemoneater/Projects/Rust4D/crates/*/Cargo.toml` - Crate dependencies
- `/home/lemoneater/Projects/Rust4D/crates/*/src/lib.rs` - Crate entry points
