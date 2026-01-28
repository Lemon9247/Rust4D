# Wave 1: Config Connections & Test Fix

**Effort**: 1 session
**Priority**: HIGH
**Dependencies**: None

---

## Overview

Connect 4 config values that are loaded but never used, and fix the flaky test. These are quick wins that improve user experience (config actually works) and developer experience (reliable tests).

---

## Task 1: Fix Flaky test_env_override

**Priority**: HIGH
**Effort**: 15 minutes
**Files**: `src/main.rs`, `Cargo.toml`

### Problem
`test_env_override` and `test_user_config_loading` both manipulate the `R4D_WINDOW__TITLE` environment variable. When run in parallel, they race and the test fails non-deterministically.

### Solution
Use the `serial_test` crate to ensure env var tests run sequentially.

### Steps

1. Add dependency to `Cargo.toml`:
```toml
[dev-dependencies]
serial_test = "3.0"
```

2. Update `src/main.rs` tests:
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    #[test]
    #[serial]
    fn test_env_override() {
        std::env::set_var("R4D_WINDOW__TITLE", "Test From Env");
        let config = AppConfig::load().unwrap();
        assert_eq!(config.window.title, "Test From Env");
        std::env::remove_var("R4D_WINDOW__TITLE");
    }

    #[test]
    #[serial]
    fn test_user_config_loading() {
        std::env::remove_var("R4D_WINDOW__TITLE");
        // ... rest of test
    }
}
```

### Verification
```bash
# Run multiple times to verify no flakiness
for i in {1..10}; do cargo test --package rust4d test_env; done
```

---

## Task 2: Connect camera.pitch_limit

**Priority**: HIGH
**Effort**: 15 minutes
**Files**: `crates/rust4d_render/src/camera4d.rs`, `src/main.rs`

### Problem
`config/default.toml` has `camera.pitch_limit = 89.0` but `Camera4D` uses hardcoded:
```rust
const PITCH_LIMIT: f32 = 1.553; // ~89 degrees in radians
```

### Solution
Make pitch limit configurable via constructor parameter.

### Steps

1. Update `Camera4D` in `crates/rust4d_render/src/camera4d.rs`:

```rust
pub struct Camera4D {
    // ... existing fields ...
    pitch_limit: f32,  // Add new field
}

impl Camera4D {
    /// Creates a new Camera4D with default pitch limit (89 degrees)
    pub fn new() -> Self {
        Self::with_pitch_limit(89.0_f32.to_radians())
    }

    /// Creates a new Camera4D with custom pitch limit (in radians)
    pub fn with_pitch_limit(pitch_limit: f32) -> Self {
        Self {
            position: Vec4::new(0.0, 0.0, 5.0, 0.0),
            yaw: 0.0,
            pitch: 0.0,
            roll: 0.0,
            rotation_4d: Rotor4::identity(),
            pitch_limit,  // Use parameter instead of const
        }
    }

    // Update clamp_pitch to use self.pitch_limit instead of PITCH_LIMIT constant
    fn clamp_pitch(&mut self) {
        self.pitch = self.pitch.clamp(-self.pitch_limit, self.pitch_limit);
    }
}
```

2. Update `src/main.rs` camera creation (~line 192):

```rust
let camera = Camera4D::with_pitch_limit(
    self.config.camera.pitch_limit.to_radians()
);
```

### Verification
```bash
# Change pitch_limit in config/default.toml to 45.0
# Run app, verify camera can only look up/down ~45 degrees
cargo run
```

---

## Task 3: Connect window.fullscreen

**Priority**: HIGH
**Effort**: 10 minutes
**Files**: `src/main.rs`

### Problem
`window.fullscreen = false` is in config but only the F key toggle works at runtime. The initial window state ignores the config.

### Solution
Apply fullscreen setting when creating the window.

### Steps

1. Update window creation in `src/main.rs` `resumed()` method (~line 180):

```rust
let window_attributes = Window::default_attributes()
    .with_title(&self.config.window.title)
    .with_inner_size(PhysicalSize::new(
        self.config.window.width,
        self.config.window.height,
    ));

// Apply fullscreen from config
let window_attributes = if self.config.window.fullscreen {
    window_attributes.with_fullscreen(Some(winit::window::Fullscreen::Borderless(None)))
} else {
    window_attributes
};

let window = event_loop.create_window(window_attributes).unwrap();
```

### Verification
```bash
# Set fullscreen = true in config/default.toml
cargo run
# App should start in fullscreen mode
```

---

## Task 4: Connect window.vsync

**Priority**: HIGH
**Effort**: 10 minutes
**Files**: `src/main.rs`

### Problem
`window.vsync = true` is in config but the wgpu present mode is not configured from it.

### Solution
Set `present_mode` based on config when creating the surface configuration.

### Steps

1. Update surface configuration in `src/main.rs` `resumed()` method (~line 200):

```rust
let present_mode = if self.config.window.vsync {
    wgpu::PresentMode::AutoVsync
} else {
    wgpu::PresentMode::AutoNoVsync
};

let surface_config = wgpu::SurfaceConfiguration {
    usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
    format: surface_format,
    width: size.width,
    height: size.height,
    present_mode,  // Use config-based value
    alpha_mode: wgpu::CompositeAlphaMode::Auto,
    view_formats: vec![],
    desired_maximum_frame_latency: 2,
};
```

### Verification
```bash
# Set vsync = false, run app, check for screen tearing or high FPS
# Set vsync = true, run app, FPS should be capped to monitor refresh rate
```

---

## Task 5: Add with_w_rotation_sensitivity() Builder

**Priority**: HIGH
**Effort**: 15 minutes
**Files**: `crates/rust4d_input/src/camera_controller.rs`, `src/main.rs`

### Problem
`input.w_rotation_sensitivity = 0.005` is in config but `CameraController` has no builder method for it. The controller uses its own hardcoded default.

### Solution
Add builder method and connect in main.rs.

### Steps

1. Add field and builder to `crates/rust4d_input/src/camera_controller.rs`:

```rust
pub struct CameraController {
    // ... existing fields ...
    w_rotation_sensitivity: f32,  // Add if not present
}

impl CameraController {
    pub fn new() -> Self {
        Self {
            // ... existing defaults ...
            w_rotation_sensitivity: 0.005,  // Default
        }
    }

    /// Set W-axis rotation sensitivity
    pub fn with_w_rotation_sensitivity(mut self, sensitivity: f32) -> Self {
        self.w_rotation_sensitivity = sensitivity;
        self
    }

    // Ensure the field is used in rotation calculations
    // Find where W rotation is applied and use self.w_rotation_sensitivity
}
```

2. Update `src/main.rs` controller creation (~line 97-102):

```rust
let controller = CameraController::new()
    .with_move_speed(self.config.input.move_speed)
    .with_w_move_speed(self.config.input.w_move_speed)
    .with_mouse_sensitivity(self.config.input.mouse_sensitivity)
    .with_w_rotation_sensitivity(self.config.input.w_rotation_sensitivity)  // Add this
    .with_smoothing_half_life(self.config.input.smoothing_half_life)
    .with_smoothing(self.config.input.smoothing_enabled);
```

### Verification
```bash
# Change w_rotation_sensitivity to 0.02 in config (4x default)
# Run app, W rotation should feel faster
```

---

## Checklist

- [ ] Add `serial_test` dependency
- [ ] Mark env var tests with `#[serial]`
- [ ] Verify tests pass 10+ times in a row
- [ ] Add `pitch_limit` field to Camera4D
- [ ] Add `with_pitch_limit()` constructor
- [ ] Connect pitch_limit from config
- [ ] Apply fullscreen from config on window creation
- [ ] Set present_mode from vsync config
- [ ] Add `with_w_rotation_sensitivity()` builder
- [ ] Connect w_rotation_sensitivity from config
- [ ] Run full test suite: `cargo test --workspace`
- [ ] Manual verification of all config changes

---

## Commits

Make one commit per task for easy review/revert:

1. `Fix test_env_override flaky test with serial_test crate`
2. `Connect camera.pitch_limit config to Camera4D`
3. `Apply window.fullscreen config on startup`
4. `Connect window.vsync to wgpu present mode`
5. `Add w_rotation_sensitivity config connection`

---

## Notes

### Config Values NOT Being Connected (Intentional)

The following config values are being left unconnected for now:
- `debug.show_overlay` - Feature not implemented (would need egui or similar)
- `debug.show_colliders` - Feature not implemented (would need debug rendering)
- `debug.log_level` - env_logger conventionally uses RUST_LOG env var

These could be removed from config or implemented in a future wave focused on debug tooling.
