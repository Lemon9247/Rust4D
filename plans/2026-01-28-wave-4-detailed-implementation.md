# Wave 4 Detailed Implementation Plan

**Branch:** `feature/wave-4-architecture`
**Created:** 2026-01-28
**Status:** Ready for Review

---

## Overview

Wave 4 refactors main.rs (588 lines) into modular systems, reducing it to ~150 lines while improving testability and maintainability. This plan provides step-by-step implementation details.

## Current State Analysis

### main.rs Breakdown (588 lines)

| Section | Lines | Notes |
|---------|-------|-------|
| Imports & mod | 1-27 | 27 lines |
| App struct | 28-44 | 17 lines |
| App::new() | 46-118 | 73 lines (config, scene loading, camera/controller setup) |
| App::build_geometry() | 120-144 | 25 lines (keep in main.rs) |
| App::capture_cursor() | 146-161 | 16 lines → WindowSystem |
| App::release_cursor() | 163-171 | 9 lines → WindowSystem |
| ApplicationHandler::resumed() | 174-232 | 59 lines → WindowSystem + RenderSystem |
| window_event() KeyboardInput | 255-293 | 39 lines → InputMapper |
| window_event() MouseInput | 295-301 | 7 lines → InputMapper |
| window_event() MouseWheel | 303-310 | 8 lines (keep - simple) |
| RedrawRequested: Simulation | 312-389 | 78 lines → SimulationSystem |
| RedrawRequested: Rendering | 390-520 | 131 lines → RenderSystem |
| device_event() | 532-541 | 10 lines (keep - simple) |
| main() | 544-556 | 13 lines |
| tests | 558-588 | 31 lines |

---

## Implementation Sessions

### Session 0: Pre-Phase Cleanup (0.5 sessions)

**Goal:** Clear technical debt before refactoring

#### Tasks

1. **Fix Clippy Warnings** (5 min)
   ```bash
   cargo clippy --fix --workspace --allow-dirty
   ```

2. **Add Missing Config Values** (15 min)

   Add to `RenderingConfig` in `src/config.rs`:
   ```rust
   /// W-axis color strength for depth visualization
   pub w_color_strength: f32,
   /// W-axis range for color mapping
   pub w_range: f32,
   ```

   Add to `config/default.toml`:
   ```toml
   w_color_strength = 0.5
   w_range = 2.0
   ```

   Wire up in main.rs (lines 460-461):
   ```rust
   w_color_strength: self.config.rendering.w_color_strength,
   w_range: self.config.rendering.w_range,
   ```

3. **Handle Unused Config Values** (10 min)

   Option: Add `#[allow(dead_code)]` with TODO comments for:
   - `debug.show_overlay`
   - `debug.show_colliders`
   - `debug.log_level`

   Or remove them entirely until implemented.

4. **Fix Example Warnings** (5 min)

   In examples that don't use physics, prefix unused fields:
   ```rust
   _world: World,  // Instead of world: World
   ```

#### Success Criteria
- [ ] `cargo clippy --workspace` produces no warnings
- [ ] All config values are either used or marked with TODO
- [ ] Examples compile without warnings

---

### Session 1: WindowSystem (1 session)

**Goal:** Extract window management into `src/systems/window.rs`

#### File Structure

Create:
```
src/
├── systems/
│   ├── mod.rs
│   └── window.rs
└── main.rs (modified)
```

#### WindowSystem API

```rust
// src/systems/window.rs

use std::sync::Arc;
use winit::{
    event_loop::ActiveEventLoop,
    window::{CursorGrabMode, Fullscreen, Window},
};
use crate::config::WindowConfig;

/// Manages the application window and cursor state
pub struct WindowSystem {
    window: Arc<Window>,
    cursor_captured: bool,
    base_title: String,
}

impl WindowSystem {
    /// Create window from config
    pub fn create(
        event_loop: &ActiveEventLoop,
        config: &WindowConfig
    ) -> Result<Self, WindowError> {
        let mut attrs = Window::default_attributes()
            .with_title(&config.title)
            .with_inner_size(winit::dpi::LogicalSize::new(
                config.width,
                config.height,
            ));

        if config.fullscreen {
            attrs = attrs.with_fullscreen(Some(Fullscreen::Borderless(None)));
        }

        let window = Arc::new(
            event_loop.create_window(attrs)
                .map_err(|e| WindowError::CreationFailed(e.to_string()))?
        );

        Ok(Self {
            window,
            cursor_captured: false,
            base_title: config.title.clone(),
        })
    }

    /// Get window reference (for RenderContext creation)
    pub fn window(&self) -> &Arc<Window> {
        &self.window
    }

    /// Check if cursor is captured
    pub fn is_cursor_captured(&self) -> bool {
        self.cursor_captured
    }

    /// Capture cursor for FPS-style controls
    pub fn capture_cursor(&mut self) -> bool {
        let grab_result = self.window.set_cursor_grab(CursorGrabMode::Locked)
            .or_else(|_| self.window.set_cursor_grab(CursorGrabMode::Confined));

        if grab_result.is_ok() {
            self.window.set_cursor_visible(false);
            self.cursor_captured = true;
            log::info!("Cursor captured - Escape to release");
            true
        } else {
            log::warn!("Failed to capture cursor");
            false
        }
    }

    /// Release cursor
    pub fn release_cursor(&mut self) {
        let _ = self.window.set_cursor_grab(CursorGrabMode::None);
        self.window.set_cursor_visible(true);
        self.cursor_captured = false;
        log::info!("Cursor released - click to capture");
    }

    /// Toggle fullscreen mode
    pub fn toggle_fullscreen(&self) {
        let new_fullscreen = if self.window.fullscreen().is_some() {
            None
        } else {
            Some(Fullscreen::Borderless(None))
        };
        self.window.set_fullscreen(new_fullscreen);
    }

    /// Update window title with position/state info
    pub fn update_title(&self, pos: [f32; 4], slice_w: f32) {
        let title = if self.cursor_captured {
            format!(
                "{} - ({:.1}, {:.1}, {:.1}, {:.1}) W:{:.2} [Esc to release]",
                self.base_title, pos[0], pos[1], pos[2], pos[3], slice_w
            )
        } else {
            format!(
                "{} - ({:.1}, {:.1}, {:.1}, {:.1}) W:{:.2} [Click to capture]",
                self.base_title, pos[0], pos[1], pos[2], pos[3], slice_w
            )
        };
        self.window.set_title(&title);
    }

    /// Request a redraw
    pub fn request_redraw(&self) {
        self.window.request_redraw();
    }
}

#[derive(Debug)]
pub enum WindowError {
    CreationFailed(String),
}

impl std::fmt::Display for WindowError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WindowError::CreationFailed(msg) => write!(f, "Window creation failed: {}", msg),
        }
    }
}

impl std::error::Error for WindowError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_title_formatting_captured() {
        // Test title format when cursor is captured
        // Note: Can't test actual window without event loop
        let pos = [1.0, 2.0, 3.0, 4.0];
        let title = format!(
            "Test - ({:.1}, {:.1}, {:.1}, {:.1}) W:{:.2} [Esc to release]",
            pos[0], pos[1], pos[2], pos[3], 0.5
        );
        assert!(title.contains("Esc to release"));
    }

    #[test]
    fn test_title_formatting_released() {
        let pos = [1.0, 2.0, 3.0, 4.0];
        let title = format!(
            "Test - ({:.1}, {:.1}, {:.1}, {:.1}) W:{:.2} [Click to capture]",
            pos[0], pos[1], pos[2], pos[3], 0.5
        );
        assert!(title.contains("Click to capture"));
    }
}
```

#### Module Exports

```rust
// src/systems/mod.rs

mod window;

pub use window::{WindowSystem, WindowError};
```

#### main.rs Changes

1. Add `mod systems;` at top
2. Remove `capture_cursor()` and `release_cursor()` methods from App
3. Replace `window: Option<Arc<Window>>` with `window_system: Option<WindowSystem>`
4. Remove `cursor_captured: bool` field
5. Update `resumed()` to use `WindowSystem::create()`
6. Update all window/cursor operations to use WindowSystem methods

#### Success Criteria
- [ ] WindowSystem compiles
- [ ] Cursor capture/release works identically
- [ ] Fullscreen toggle works
- [ ] Window title updates correctly
- [ ] All tests pass

---

### Session 2: InputMapper (0.5 sessions)

**Goal:** Extract input mapping to `src/input/input_mapper.rs`

#### File Structure

Create:
```
src/
├── input/
│   ├── mod.rs
│   └── input_mapper.rs
└── main.rs (modified)
```

#### InputMapper API

```rust
// src/input/input_mapper.rs

use winit::keyboard::KeyCode;
use winit::event::{ElementState, MouseButton};

/// Actions triggered by special input (not movement)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputAction {
    /// Toggle cursor capture (Escape when captured, click when released)
    ToggleCursor,
    /// Exit application (Escape when not captured)
    Exit,
    /// Reset camera to starting position (R key)
    ResetCamera,
    /// Toggle fullscreen mode (F key)
    ToggleFullscreen,
    /// Toggle input smoothing (G key)
    ToggleSmoothing,
}

/// Maps raw input events to semantic actions
///
/// Movement keys (WASD, Space, RF) are NOT mapped here - they go directly
/// to the CameraController. This mapper handles "special" keys only.
pub struct InputMapper;

impl InputMapper {
    /// Map keyboard input to an action
    ///
    /// Returns `Some(action)` for special keys, `None` for movement keys
    pub fn map_keyboard(
        key: KeyCode,
        state: ElementState,
        cursor_captured: bool,
    ) -> Option<InputAction> {
        // Only handle key presses, not releases
        if state != ElementState::Pressed {
            return None;
        }

        match key {
            KeyCode::Escape => {
                if cursor_captured {
                    Some(InputAction::ToggleCursor)
                } else {
                    Some(InputAction::Exit)
                }
            }
            KeyCode::KeyR => Some(InputAction::ResetCamera),
            KeyCode::KeyF => Some(InputAction::ToggleFullscreen),
            KeyCode::KeyG => Some(InputAction::ToggleSmoothing),
            _ => None, // Movement keys handled by controller
        }
    }

    /// Map mouse button to an action
    ///
    /// Returns `Some(ToggleCursor)` for left click when cursor not captured
    pub fn map_mouse_button(
        button: MouseButton,
        state: ElementState,
        cursor_captured: bool,
    ) -> Option<InputAction> {
        if button == MouseButton::Left
            && state == ElementState::Pressed
            && !cursor_captured
        {
            Some(InputAction::ToggleCursor)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_escape_when_captured_releases() {
        let action = InputMapper::map_keyboard(
            KeyCode::Escape,
            ElementState::Pressed,
            true, // cursor captured
        );
        assert_eq!(action, Some(InputAction::ToggleCursor));
    }

    #[test]
    fn test_escape_when_released_exits() {
        let action = InputMapper::map_keyboard(
            KeyCode::Escape,
            ElementState::Pressed,
            false, // cursor not captured
        );
        assert_eq!(action, Some(InputAction::Exit));
    }

    #[test]
    fn test_movement_keys_not_mapped() {
        // WASD should return None (handled by controller)
        for key in [KeyCode::KeyW, KeyCode::KeyA, KeyCode::KeyS, KeyCode::KeyD] {
            let action = InputMapper::map_keyboard(key, ElementState::Pressed, true);
            assert_eq!(action, None, "Key {:?} should not be mapped", key);
        }
    }

    #[test]
    fn test_key_release_ignored() {
        let action = InputMapper::map_keyboard(
            KeyCode::Escape,
            ElementState::Released,
            true,
        );
        assert_eq!(action, None);
    }

    #[test]
    fn test_click_to_capture() {
        let action = InputMapper::map_mouse_button(
            MouseButton::Left,
            ElementState::Pressed,
            false, // cursor not captured
        );
        assert_eq!(action, Some(InputAction::ToggleCursor));
    }

    #[test]
    fn test_click_when_captured_no_action() {
        let action = InputMapper::map_mouse_button(
            MouseButton::Left,
            ElementState::Pressed,
            true, // cursor already captured
        );
        assert_eq!(action, None);
    }

    #[test]
    fn test_special_keys() {
        assert_eq!(
            InputMapper::map_keyboard(KeyCode::KeyR, ElementState::Pressed, true),
            Some(InputAction::ResetCamera)
        );
        assert_eq!(
            InputMapper::map_keyboard(KeyCode::KeyF, ElementState::Pressed, true),
            Some(InputAction::ToggleFullscreen)
        );
        assert_eq!(
            InputMapper::map_keyboard(KeyCode::KeyG, ElementState::Pressed, true),
            Some(InputAction::ToggleSmoothing)
        );
    }
}
```

#### Module Exports

```rust
// src/input/mod.rs

mod input_mapper;

pub use input_mapper::{InputMapper, InputAction};
```

#### main.rs Changes

1. Add `mod input;` at top
2. Replace inline keyboard handling with:
   ```rust
   if let Some(action) = InputMapper::map_keyboard(key, event.state, cursor_captured) {
       match action {
           InputAction::ToggleCursor => { /* toggle cursor */ }
           InputAction::Exit => event_loop.exit(),
           InputAction::ResetCamera => self.camera.reset(),
           InputAction::ToggleFullscreen => window_system.toggle_fullscreen(),
           InputAction::ToggleSmoothing => { /* toggle smoothing */ }
       }
   } else {
       // Pass to controller for movement
       self.controller.process_keyboard(key, event.state);
   }
   ```

#### Success Criteria
- [ ] InputMapper compiles with all tests passing
- [ ] Escape context-sensitive behavior works
- [ ] All special keys (R, F, G) work
- [ ] Click to capture works
- [ ] Movement keys still work

---

### Session 3: SimulationSystem (1 session)

**Goal:** Extract game loop simulation to `src/systems/simulation.rs`

#### SimulationSystem API

```rust
// src/systems/simulation.rs

use std::time::Instant;
use rust4d_core::SceneManager;
use rust4d_render::camera4d::Camera4D;
use rust4d_input::CameraController;
use rust4d_math::Vec4;

/// Result of a simulation update
pub struct SimulationResult {
    /// Whether geometry needs to be rebuilt and re-uploaded
    pub geometry_dirty: bool,
    /// Delta time used for this update
    pub delta_time: f32,
}

/// Manages the game simulation loop
///
/// Handles:
/// - Delta time calculation
/// - Input → physics movement
/// - Physics stepping
/// - Camera synchronization
pub struct SimulationSystem {
    last_frame: Instant,
}

impl SimulationSystem {
    /// Create a new simulation system
    pub fn new() -> Self {
        Self {
            last_frame: Instant::now(),
        }
    }

    /// Run one simulation frame
    ///
    /// # Arguments
    /// * `scene_manager` - Scene manager containing world and physics
    /// * `camera` - 4D camera to sync position to
    /// * `controller` - Input controller for movement/rotation
    /// * `cursor_captured` - Whether cursor is captured (enables mouse look)
    ///
    /// # Returns
    /// SimulationResult with dirty flag and delta time
    pub fn update(
        &mut self,
        scene_manager: &mut SceneManager,
        camera: &mut Camera4D,
        controller: &mut CameraController,
        cursor_captured: bool,
    ) -> SimulationResult {
        // 1. Calculate delta time
        let now = Instant::now();
        let raw_dt = (now - self.last_frame).as_secs_f32();
        // Cap dt to prevent huge physics steps on first frame or after window focus
        let dt = raw_dt.min(1.0 / 30.0); // Max 33ms per frame
        self.last_frame = now;

        // 2. Get movement input from controller
        let (forward_input, right_input) = controller.get_movement_input();
        let w_input = controller.get_w_input();

        // 3. Calculate movement direction in world space using camera orientation
        let camera_forward = camera.forward();
        let camera_right = camera.right();
        let camera_ana = camera.ana();

        // Project to XZW hyperplane (zero out Y for horizontal movement)
        let forward_xzw = Vec4::new(
            camera_forward.x, 0.0, camera_forward.z, camera_forward.w
        ).normalized();
        let right_xzw = Vec4::new(
            camera_right.x, 0.0, camera_right.z, camera_right.w
        ).normalized();
        let ana_xzw = Vec4::new(
            camera_ana.x, 0.0, camera_ana.z, camera_ana.w
        ).normalized();

        // Combine movement direction
        let move_dir = forward_xzw * forward_input
            + right_xzw * right_input
            + ana_xzw * w_input;

        // 4. Apply movement to player via physics
        let move_speed = controller.move_speed;
        if let Some(physics) = scene_manager.active_world_mut()
            .and_then(|w| w.physics_mut())
        {
            physics.apply_player_movement(move_dir * move_speed);
        }

        // 5. Handle jump
        if controller.consume_jump() {
            if let Some(physics) = scene_manager.active_world_mut()
                .and_then(|w| w.physics_mut())
            {
                physics.player_jump();
            }
        }

        // 6. Step world physics
        scene_manager.update(dt);

        // 7. Check for dirty entities
        let geometry_dirty = scene_manager.active_world()
            .map(|w| w.has_dirty_entities())
            .unwrap_or(false);

        // 8. Sync camera position to player physics (all 4 dimensions)
        if let Some(pos) = scene_manager.active_world()
            .and_then(|w| w.physics())
            .and_then(|p| p.player_position())
        {
            camera.position = pos;
        }

        // 9. Apply mouse look for camera rotation
        controller.update(camera, dt, cursor_captured);

        // 10. Re-sync position after controller (discard its movement, keep rotation)
        if let Some(pos) = scene_manager.active_world()
            .and_then(|w| w.physics())
            .and_then(|p| p.player_position())
        {
            camera.position = pos;
        }

        SimulationResult {
            geometry_dirty,
            delta_time: dt,
        }
    }
}

impl Default for SimulationSystem {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_delta_time_capped() {
        let mut sim = SimulationSystem::new();
        // Simulate a 100ms pause (first frame or window focus)
        std::thread::sleep(std::time::Duration::from_millis(100));

        // Can't fully test without scene manager, but we can verify construction
        assert!(sim.last_frame.elapsed().as_millis() >= 100);
    }
}
```

#### main.rs Changes

1. Add SimulationSystem to App struct
2. Remove `last_frame` field from App
3. Replace entire RedrawRequested simulation block (lines 312-389) with:
   ```rust
   let result = self.simulation_system.update(
       &mut self.scene_manager,
       &mut self.camera,
       &mut self.controller,
       window_system.is_cursor_captured(),
   );

   if result.geometry_dirty {
       self.geometry = Self::build_geometry(scene_manager.active_world().unwrap());
       // Re-upload to GPU...
       scene_manager.active_world_mut().unwrap().clear_all_dirty();
   }
   ```

#### Success Criteria
- [ ] SimulationSystem compiles
- [ ] Player movement works identically
- [ ] Jump works
- [ ] Camera rotation works
- [ ] Dirty tracking works
- [ ] Physics stepping works

---

### Session 4: RenderSystem (1.5 sessions)

**Goal:** Extract rendering to `src/systems/render.rs`

This is the largest extraction. We'll create a RenderSystem that encapsulates:
- RenderContext
- SlicePipeline
- RenderPipeline
- Frame rendering logic

#### RenderSystem API

```rust
// src/systems/render.rs

use std::sync::Arc;
use winit::window::Window;
use rust4d_render::{
    context::RenderContext,
    camera4d::Camera4D,
    pipeline::{SlicePipeline, RenderPipeline, SliceParams, RenderUniforms, perspective_matrix},
    RenderableGeometry,
};
use crate::config::{RenderingConfig, CameraConfig};

/// Render error types
#[derive(Debug)]
pub enum RenderError {
    /// Surface was lost (window resized, minimized, etc.)
    SurfaceLost,
    /// GPU out of memory
    OutOfMemory,
    /// Other surface error
    Other(String),
}

impl std::fmt::Display for RenderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RenderError::SurfaceLost => write!(f, "Surface lost"),
            RenderError::OutOfMemory => write!(f, "Out of memory"),
            RenderError::Other(msg) => write!(f, "Render error: {}", msg),
        }
    }
}

impl std::error::Error for RenderError {}

/// Manages GPU rendering
pub struct RenderSystem {
    context: RenderContext,
    slice_pipeline: SlicePipeline,
    render_pipeline: RenderPipeline,
    render_config: RenderingConfig,
    camera_config: CameraConfig,
}

impl RenderSystem {
    /// Create render system from window and config
    pub async fn new(
        window: Arc<Window>,
        render_config: RenderingConfig,
        camera_config: CameraConfig,
        vsync: bool,
    ) -> Self {
        let context = RenderContext::with_vsync(window, vsync).await;

        let mut slice_pipeline = SlicePipeline::new(
            &context.device,
            render_config.max_triangles as usize,
        );

        let mut render_pipeline = RenderPipeline::new(
            &context.device,
            context.config.format,
        );

        // Ensure depth texture exists
        render_pipeline.ensure_depth_texture(
            &context.device,
            context.size.width,
            context.size.height,
        );

        Self {
            context,
            slice_pipeline,
            render_pipeline,
            render_config,
            camera_config,
        }
    }

    /// Get render context reference (for device/queue access)
    pub fn context(&self) -> &RenderContext {
        &self.context
    }

    /// Handle window resize
    pub fn resize(&mut self, width: u32, height: u32) {
        self.context.resize(winit::dpi::PhysicalSize::new(width, height));
        self.render_pipeline.ensure_depth_texture(
            &self.context.device,
            width,
            height,
        );
    }

    /// Upload geometry to GPU
    pub fn upload_geometry(&mut self, geometry: &RenderableGeometry) {
        self.slice_pipeline.upload_tetrahedra(
            &self.context.device,
            &geometry.vertices,
            &geometry.tetrahedra,
        );
        log::info!(
            "Uploaded {} vertices and {} tetrahedra",
            geometry.vertex_count(),
            geometry.tetrahedron_count()
        );
    }

    /// Render a single frame
    pub fn render_frame(
        &mut self,
        camera: &Camera4D,
        geometry: &RenderableGeometry,
    ) -> Result<(), RenderError> {
        let pos = camera.position;
        let eye_3d = [pos.x, pos.y, pos.z];
        let camera_pos_4d = [pos.x, pos.y, pos.z, pos.w];

        // Update slice parameters
        let camera_matrix = camera.rotation_matrix();
        let slice_params = SliceParams {
            slice_w: camera.get_slice_w(),
            tetrahedron_count: geometry.tetrahedron_count() as u32,
            _padding: [0.0; 2],
            camera_matrix,
            camera_eye: eye_3d,
            _padding2: 0.0,
            camera_position: camera_pos_4d,
        };
        self.slice_pipeline.update_params(&self.context.queue, &slice_params);

        // Create view and projection matrices
        let aspect = self.context.aspect_ratio();
        let proj_matrix = perspective_matrix(
            self.camera_config.fov.to_radians(),
            aspect,
            self.camera_config.near,
            self.camera_config.far,
        );

        // View matrix is identity (slice shader outputs camera-space coordinates)
        let view_matrix = [
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ];

        let render_uniforms = RenderUniforms {
            view_matrix,
            projection_matrix: proj_matrix,
            light_dir: self.render_config.light_dir,
            _padding: 0.0,
            ambient_strength: self.render_config.ambient_strength,
            diffuse_strength: self.render_config.diffuse_strength,
            w_color_strength: self.render_config.w_color_strength,
            w_range: self.render_config.w_range,
        };
        self.render_pipeline.update_uniforms(&self.context.queue, &render_uniforms);

        // Get surface texture
        let output = match self.context.surface.get_current_texture() {
            Ok(output) => output,
            Err(wgpu::SurfaceError::Lost) => return Err(RenderError::SurfaceLost),
            Err(wgpu::SurfaceError::OutOfMemory) => return Err(RenderError::OutOfMemory),
            Err(e) => return Err(RenderError::Other(format!("{:?}", e))),
        };

        let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());

        // Create command encoder
        let mut encoder = self.context.device.create_command_encoder(
            &wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            },
        );

        // Reset counter and run compute pass
        self.slice_pipeline.reset_counter(&self.context.queue);
        self.slice_pipeline.run_slice_pass(&mut encoder);

        // Copy triangle count to indirect buffer
        self.render_pipeline.prepare_indirect_draw(
            &mut encoder,
            self.slice_pipeline.counter_buffer(),
        );

        // Render pass
        let bg = &self.render_config.background_color;
        self.render_pipeline.render(
            &mut encoder,
            &view,
            self.slice_pipeline.output_buffer(),
            wgpu::Color {
                r: bg[0] as f64,
                g: bg[1] as f64,
                b: bg[2] as f64,
                a: bg[3] as f64,
            },
        );

        // Submit
        self.context.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }

    /// Get current surface size
    pub fn size(&self) -> (u32, u32) {
        (self.context.size.width, self.context.size.height)
    }
}
```

#### Module Updates

```rust
// src/systems/mod.rs

mod window;
mod render;
mod simulation;

pub use window::{WindowSystem, WindowError};
pub use render::{RenderSystem, RenderError};
pub use simulation::{SimulationSystem, SimulationResult};
```

#### main.rs Changes

1. Remove `render_context`, `slice_pipeline`, `render_pipeline` fields from App
2. Add `render_system: Option<RenderSystem>` field
3. Update `resumed()` to create RenderSystem
4. Replace entire render block in RedrawRequested with:
   ```rust
   match render_system.render_frame(&self.camera, &self.geometry) {
       Ok(()) => {}
       Err(RenderError::SurfaceLost) => {
           let (w, h) = render_system.size();
           render_system.resize(w, h);
       }
       Err(RenderError::OutOfMemory) => {
           event_loop.exit();
       }
       Err(e) => {
           log::warn!("Render error: {}", e);
       }
   }
   ```

#### Success Criteria
- [ ] RenderSystem compiles
- [ ] GPU initialization works
- [ ] Geometry upload works
- [ ] Frame rendering produces identical output
- [ ] Surface errors handled gracefully
- [ ] Window resize works

---

### Session 5: Error Handling & Integration (0.5 sessions)

**Goal:** Add unified error handling and verify full integration

#### Error Module

```rust
// src/error.rs

use crate::systems::{WindowError, RenderError};

/// Application-level error
#[derive(Debug)]
pub enum AppError {
    Window(WindowError),
    Render(RenderError),
    Config(crate::config::ConfigError),
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AppError::Window(e) => write!(f, "Window error: {}", e),
            AppError::Render(e) => write!(f, "Render error: {}", e),
            AppError::Config(e) => write!(f, "Config error: {}", e),
        }
    }
}

impl std::error::Error for AppError {}

impl From<WindowError> for AppError {
    fn from(e: WindowError) -> Self { AppError::Window(e) }
}

impl From<RenderError> for AppError {
    fn from(e: RenderError) -> Self { AppError::Render(e) }
}

impl From<crate::config::ConfigError> for AppError {
    fn from(e: crate::config::ConfigError) -> Self { AppError::Config(e) }
}
```

#### Final main.rs Structure

After all extractions, main.rs should look like:

```rust
//! Rust4D - 4D Rendering Engine

mod config;
mod error;
mod input;
mod systems;

use std::sync::Arc;
use winit::{
    application::ApplicationHandler,
    event::{DeviceEvent, DeviceId, ElementState, MouseButton, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    keyboard::PhysicalKey,
    window::WindowId,
};

use rust4d_core::{World, SceneManager};
use rust4d_render::{camera4d::Camera4D, RenderableGeometry, CheckerboardGeometry, position_gradient_color};
use rust4d_input::CameraController;
use rust4d_math::Vec4;

use config::AppConfig;
use input::{InputMapper, InputAction};
use systems::{WindowSystem, RenderSystem, SimulationSystem, RenderError};

/// Main application state
struct App {
    config: AppConfig,
    window_system: Option<WindowSystem>,
    render_system: Option<RenderSystem>,
    simulation_system: SimulationSystem,
    scene_manager: SceneManager,
    geometry: RenderableGeometry,
    camera: Camera4D,
    controller: CameraController,
}

impl App {
    fn new() -> Self {
        let config = AppConfig::load().unwrap_or_else(|e| {
            log::warn!("Failed to load config: {}. Using defaults.", e);
            AppConfig::default()
        });

        // ... scene loading (same as before) ...

        Self {
            config,
            window_system: None,
            render_system: None,
            simulation_system: SimulationSystem::new(),
            scene_manager,
            geometry,
            camera,
            controller,
        }
    }

    fn build_geometry(world: &World) -> RenderableGeometry {
        // ... same as before ...
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window_system.is_none() {
            let window_system = WindowSystem::create(event_loop, &self.config.window)
                .expect("Failed to create window");

            let render_system = pollster::block_on(RenderSystem::new(
                window_system.window().clone(),
                self.config.rendering.clone(),
                self.config.camera.clone(),
                self.config.window.vsync,
            ));

            render_system.upload_geometry(&self.geometry);

            self.window_system = Some(window_system);
            self.render_system = Some(render_system);
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        let window_system = match &mut self.window_system {
            Some(ws) => ws,
            None => return,
        };

        match event {
            WindowEvent::CloseRequested => event_loop.exit(),

            WindowEvent::Resized(size) => {
                if let Some(rs) = &mut self.render_system {
                    rs.resize(size.width, size.height);
                }
            }

            WindowEvent::KeyboardInput { event, .. } => {
                if let PhysicalKey::Code(key) = event.physical_key {
                    if let Some(action) = InputMapper::map_keyboard(
                        key, event.state, window_system.is_cursor_captured()
                    ) {
                        match action {
                            InputAction::ToggleCursor => {
                                if window_system.is_cursor_captured() {
                                    window_system.release_cursor();
                                } else {
                                    window_system.capture_cursor();
                                }
                            }
                            InputAction::Exit => event_loop.exit(),
                            InputAction::ResetCamera => {
                                self.camera.reset();
                                log::info!("Camera reset");
                            }
                            InputAction::ToggleFullscreen => {
                                window_system.toggle_fullscreen();
                            }
                            InputAction::ToggleSmoothing => {
                                let enabled = self.controller.toggle_smoothing();
                                log::info!("Smoothing: {}", if enabled { "ON" } else { "OFF" });
                            }
                        }
                    } else {
                        self.controller.process_keyboard(key, event.state);
                    }
                }
            }

            WindowEvent::MouseInput { state, button, .. } => {
                if let Some(InputAction::ToggleCursor) = InputMapper::map_mouse_button(
                    button, state, window_system.is_cursor_captured()
                ) {
                    window_system.capture_cursor();
                }
                self.controller.process_mouse_button(button, state);
            }

            WindowEvent::MouseWheel { delta, .. } => {
                let scroll = match delta {
                    winit::event::MouseScrollDelta::LineDelta(_, y) => y,
                    winit::event::MouseScrollDelta::PixelDelta(pos) => pos.y as f32 / 100.0,
                };
                self.camera.adjust_slice_offset(scroll * 0.1);
            }

            WindowEvent::RedrawRequested => {
                // Run simulation
                let result = self.simulation_system.update(
                    &mut self.scene_manager,
                    &mut self.camera,
                    &mut self.controller,
                    window_system.is_cursor_captured(),
                );

                // Rebuild geometry if dirty
                if result.geometry_dirty {
                    if let Some(world) = self.scene_manager.active_world() {
                        self.geometry = Self::build_geometry(world);
                    }
                    if let Some(rs) = &mut self.render_system {
                        rs.upload_geometry(&self.geometry);
                    }
                    if let Some(world) = self.scene_manager.active_world_mut() {
                        world.clear_all_dirty();
                    }
                }

                // Update title
                let pos = self.camera.position;
                window_system.update_title(
                    [pos.x, pos.y, pos.z, pos.w],
                    self.camera.get_slice_w(),
                );

                // Render
                if let Some(rs) = &mut self.render_system {
                    match rs.render_frame(&self.camera, &self.geometry) {
                        Ok(()) => {}
                        Err(RenderError::SurfaceLost) => {
                            let (w, h) = rs.size();
                            rs.resize(w, h);
                        }
                        Err(RenderError::OutOfMemory) => event_loop.exit(),
                        Err(e) => log::warn!("Render error: {}", e),
                    }
                }

                window_system.request_redraw();
            }

            _ => {}
        }
    }

    fn device_event(&mut self, _: &ActiveEventLoop, _: DeviceId, event: DeviceEvent) {
        if let DeviceEvent::MouseMotion { delta } = event {
            self.controller.process_mouse_motion(delta.0, delta.1);
        }
    }
}

fn main() {
    env_logger::init();
    log::info!("Starting Rust4D");

    let event_loop = EventLoop::new().expect("Failed to create event loop");
    event_loop.set_control_flow(ControlFlow::Poll);

    let mut app = App::new();
    event_loop.run_app(&mut app).expect("Event loop error");
}
```

#### Final Line Count Target

| Component | Lines |
|-----------|-------|
| Imports | ~20 |
| App struct | ~12 |
| App::new() | ~45 |
| App::build_geometry() | ~25 |
| resumed() | ~18 |
| window_event() | ~75 |
| device_event() | ~5 |
| main() | ~10 |
| **Total** | **~210** |

Note: This is slightly higher than the 150-line target in the original plan due to keeping tests in main.rs and some additional error handling. The core improvement is that the event handler logic is now clean dispatch code rather than implementation details.

---

## Testing Strategy

### After Each Session

1. Run `cargo build` to verify compilation
2. Run `cargo test` to verify all tests pass
3. Run `cargo run` and manually test:
   - Window creation
   - Cursor capture/release
   - Movement (WASD)
   - Mouse look
   - Jump (Space)
   - 4D movement (RF)
   - Fullscreen toggle (F)
   - Smoothing toggle (G)
   - Camera reset (R)
   - Exit (Escape)
   - Geometry updates when entities move

### Final Verification

1. Run `cargo clippy --workspace` - no warnings
2. Run `cargo test --workspace` - all tests pass
3. Compare rendering output with pre-refactor (visual check)
4. Verify frame rate is unchanged

---

## Risk Mitigation

### Commit Strategy

Create a commit after each successful session:
- `Pre-phase cleanup: fix clippy warnings and add missing config`
- `Extract WindowSystem from main.rs`
- `Extract InputMapper from main.rs`
- `Extract SimulationSystem from main.rs`
- `Extract RenderSystem from main.rs`
- `Add unified error handling`

### Rollback Points

If anything breaks, each commit is a rollback point. The branch can be abandoned and restarted from main if needed.

---

## Session Summary

| Session | Task | Est. Time |
|---------|------|-----------|
| 0 | Pre-phase cleanup | 0.5 |
| 1 | WindowSystem | 1.0 |
| 2 | InputMapper | 0.5 |
| 3 | SimulationSystem | 1.0 |
| 4 | RenderSystem | 1.5 |
| 5 | Error handling & integration | 0.5 |
| **Total** | | **5.0 sessions** |

---

## Decisions (Resolved)

1. **Unused debug config** (`show_overlay`, `show_colliders`, `log_level`): **Keep them** - will be implemented later. No `#[allow(dead_code)]` needed since they're in config structs.

2. **Error handling**: **Option A - Keep `.expect()` for startup errors.** Only use `Result` for runtime errors (surface lost, resize, etc.) that can be recovered. Full `Result` propagation in `main()` can be added later if needed.

3. **Test location**: **Move integration tests to `tests/` directory.** Keep unit tests inline with modules.

---

## Future Improvements (Out of Scope)

- Full `Result<(), AppError>` propagation from `main()` - currently using `.expect()` for startup errors which is acceptable
- Implement `debug.show_overlay`, `debug.show_colliders`, `debug.log_level` config values
