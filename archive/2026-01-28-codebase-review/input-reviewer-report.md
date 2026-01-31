# Input Reviewer Report

## Summary
The input handling system in Rust4D is reasonably clean and functional, centered around the `rust4d_input` crate's `CameraController` struct. The main issues found relate to **config values that are loaded but never connected to the code** - specifically `w_rotation_sensitivity` and `pitch_limit`. Several debug config options are also unused. No dead code warnings were found in the input crate itself, though some public API methods (`is_moving`, `is_smoothing_enabled`) are only used in documentation, not production code.

## Dead Code / Unused Config

| Item | Location | Type | Notes |
|------|----------|------|-------|
| `input.w_rotation_sensitivity` | `config/default.toml:22`, `src/config.rs:162` | Unused config | Value loaded but never passed to CameraController. No `with_w_rotation_sensitivity()` builder method exists. Controller uses hardcoded default `0.005`. |
| `camera.pitch_limit` | `config/default.toml:16`, `src/config.rs:137` | Unused config | Value loaded but Camera4D uses hardcoded `PITCH_LIMIT: f32 = 1.553` constant (~89 degrees). |
| `debug.show_overlay` | `src/config.rs:247` | Unused config | Value loaded but never checked in main.rs |
| `debug.show_colliders` | `src/config.rs:251` | Unused config | Value loaded but never checked in main.rs |
| `debug.log_level` | `src/config.rs:249` | Unused config | Value loaded but never used to configure env_logger |
| `is_moving()` | `crates/rust4d_input/src/camera_controller.rs:189` | Potentially unused | Public method only documented, not called in production code |
| `is_smoothing_enabled()` | `crates/rust4d_input/src/camera_controller.rs:204` | Potentially unused | Public method not called anywhere |

## Implementation Gaps

| Feature | Expected | Actual | Notes |
|---------|----------|--------|-------|
| W rotation sensitivity config | `config.input.w_rotation_sensitivity` passed to controller | Hardcoded to `0.005` | Missing builder method `with_w_rotation_sensitivity()` in CameraController |
| Pitch limit config | `config.camera.pitch_limit` passed to Camera4D | Hardcoded `PITCH_LIMIT` constant | Camera4D would need modification to accept configurable pitch limit |
| Gamepad/controller support | (Roadmap mentions this) | Not implemented | No gamepad input handling code exists |
| Debug overlay | `debug.show_overlay` should toggle FPS/position overlay | Not implemented | Config value exists but no overlay rendering |
| Collider visualization | `debug.show_colliders` should show physics bounds | Not implemented | Config value exists but no collider rendering |
| Log level from config | `debug.log_level` should configure logging | Not implemented | env_logger uses RUST_LOG env var, not config |

## Code Quality Issues

| Issue | Location | Severity | Description |
|-------|----------|----------|-------------|
| Inconsistent config connection | `src/main.rs:97-102` | Medium | Most input config values are connected via builder pattern, but `w_rotation_sensitivity` is omitted |
| Public API unused | `camera_controller.rs:189,204` | Low | `is_moving()` and `is_smoothing_enabled()` are public but unused - either use them or mark internal |
| Hardcoded pitch limit | `camera4d.rs:49` | Low | `PITCH_LIMIT` should be configurable via config, not const |

## Config Connection Status

### Connected (Working)
- `input.move_speed` -> `CameraController::with_move_speed()`
- `input.w_move_speed` -> `CameraController::with_w_move_speed()`
- `input.mouse_sensitivity` -> `CameraController::with_mouse_sensitivity()`
- `input.smoothing_half_life` -> `CameraController::with_smoothing_half_life()`
- `input.smoothing_enabled` -> `CameraController::with_smoothing()`

### Not Connected (Issues)
- `input.w_rotation_sensitivity` - no builder method exists
- `camera.pitch_limit` - Camera4D uses hardcoded constant

## Input Handling Architecture

The input system is well-designed:
1. `CameraController` handles keyboard/mouse state tracking
2. `CameraControl` trait abstracts camera implementation
3. `Camera4D` implements `CameraControl` for 4D camera operations
4. main.rs orchestrates input -> physics -> camera sync loop

The physics-based movement (lines 312-380 in main.rs) correctly:
- Gets movement input from controller
- Transforms to world space using camera orientation
- Applies via physics system for collision handling
- Re-syncs camera position from physics

## Recommendations

1. **Add `with_w_rotation_sensitivity()` builder method** to CameraController and connect in main.rs
   - Priority: High (config value is documented but non-functional)
   - Effort: ~10 minutes

2. **Make Camera4D pitch limit configurable**
   - Priority: Medium (hardcoded value works, but config exists)
   - Effort: ~15 minutes (add constructor parameter or builder)

3. **Implement debug overlay** or remove debug config options
   - Priority: Low (nice to have for development)
   - Effort: 1-2 sessions for basic implementation

4. **Connect log_level config** to env_logger initialization
   - Priority: Low (env var works fine)
   - Effort: ~10 minutes

5. **Remove or document unused public methods**
   - Priority: Low (API cleanliness)
   - Either use `is_moving()` / `is_smoothing_enabled()` or mark `pub(crate)`

## Files Reviewed

- `/home/lemoneater/Projects/Rust4D/src/main.rs` (input handling sections)
- `/home/lemoneater/Projects/Rust4D/src/config.rs` (InputConfig, CameraConfig, DebugConfig)
- `/home/lemoneater/Projects/Rust4D/config/default.toml` ([input], [camera], [debug] sections)
- `/home/lemoneater/Projects/Rust4D/crates/rust4d_input/src/lib.rs`
- `/home/lemoneater/Projects/Rust4D/crates/rust4d_input/src/camera_controller.rs`
- `/home/lemoneater/Projects/Rust4D/crates/rust4d_render/src/camera4d.rs` (CameraControl impl)
