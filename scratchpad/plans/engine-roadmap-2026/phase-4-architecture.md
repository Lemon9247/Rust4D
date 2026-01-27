# Phase 4: Architecture Refactoring

**Status:** Ready
**Priority:** P3 (depends on Phase 1B Configuration)
**Estimated Effort:** 4 sessions
**Target:** Reduce main.rs from 511 lines to ~150 lines

---

## Overview

Phase 4 extracts main.rs responsibilities into well-defined systems, reducing complexity and improving testability. The current monolithic structure (511 lines, 42% in one event handler) makes the application difficult to test and maintain. This phase systematically decomposes main.rs into modular systems while preserving all functionality.

## Goals

1. **Reduce main.rs complexity** from 511 lines to ~150 lines
2. **Extract four systems**: WindowSystem (~60 lines), RenderSystem (~120 lines), SimulationSystem (~80 lines), InputMapper (~40 lines)
3. **Improve testability** by isolating concerns into testable units
4. **Maintain functionality** - zero behavioral changes, all features work identically
5. **Establish error handling** - replace panics with Result types and proper error propagation

## Dependencies

### Hard Dependency: Phase 1B (Configuration System)

Phase 4 **requires** Phase 1B to be completed first because:

1. **Extracted systems need configuration**: WindowSystem, RenderSystem, SimulationSystem all consume config values currently hardcoded in main.rs
2. **System initialization depends on config**: Systems should be constructed from `AppConfig` rather than hardcoded constants
3. **Testing requires configurable systems**: Unit tests need to inject different configs without modifying code

**What Phase 1B provides:**
- `AppConfig` struct with window, camera, input, physics, rendering sections
- Figment-based hierarchical config loading (default → user → env vars)
- Config structs in `src/config.rs`
- `config/default.toml` with all engine parameters

**Integration points:**
```rust
// Systems will accept config during construction
WindowSystem::new(config.window)
RenderSystem::new(window, config.rendering)
SimulationSystem::new(config.physics)
InputMapper::new(config.input)
```

### Soft Dependency: Phase 1A (Scene Serialization)

Not strictly required, but if Phase 1A is complete, the Scene system integration will be cleaner.

---

## Phase Structure

### Phase 4A: System Extraction (3 sessions)

Extract four independent systems from main.rs, each with clear responsibilities and APIs.

### Phase 4B: Error Handling (1 session)

Replace panics with proper error types and Result propagation.

---

## Phase 4A: System Extraction

### Session 1: WindowSystem (1 session)

**Risk:** Low - well-isolated functionality
**Impact:** Reduces main.rs by ~60 lines

#### Current State Analysis

**Lines to extract from main.rs:**
- `ApplicationHandler::resumed` (lines 141-186): Window creation and GPU initialization - 45 lines
- `capture_cursor()` (lines 113-128): Cursor capture logic - 15 lines
- `release_cursor()` (lines 130-138): Cursor release logic - 8 lines
- Fullscreen toggle in KeyboardInput handler (lines 227-236): ~10 lines
- Window title updates in RedrawRequested (lines 348-362): ~15 lines

**Total extraction:** ~60 lines

#### Target API

```rust
// src/systems/window.rs

use std::sync::Arc;
use winit::{window::Window, event_loop::ActiveEventLoop};
use crate::config::WindowConfig;

pub struct WindowSystem {
    window: Arc<Window>,
    cursor_captured: bool,
}

impl WindowSystem {
    /// Create window from config
    pub fn create(event_loop: &ActiveEventLoop, config: &WindowConfig) -> Self {
        // Window creation logic from resumed()
    }

    /// Get window reference
    pub fn window(&self) -> &Arc<Window> {
        &self.window
    }

    /// Check if cursor is captured
    pub fn is_cursor_captured(&self) -> bool {
        self.cursor_captured
    }

    /// Capture cursor for FPS-style controls
    pub fn capture_cursor(&mut self) {
        // Logic from App::capture_cursor()
    }

    /// Release cursor
    pub fn release_cursor(&mut self) {
        // Logic from App::release_cursor()
    }

    /// Toggle fullscreen mode
    pub fn toggle_fullscreen(&mut self) {
        // Logic from KeyboardInput handler
    }

    /// Update window title with debug info
    pub fn update_title(&self, camera_pos: Vec4, slice_w: f32) {
        // Format title based on cursor capture state
    }
}
```

#### Implementation Strategy

1. **Create module structure**:
   ```
   src/systems/
   ├── mod.rs       # Public exports
   └── window.rs    # WindowSystem implementation
   ```

2. **Extract cursor management**: Move `capture_cursor()` and `release_cursor()` methods into WindowSystem

3. **Extract window creation**: Move `resumed()` window creation logic into `WindowSystem::create()`

4. **Extract fullscreen toggle**: Move fullscreen logic from keyboard handler

5. **Extract title updates**: Move window title formatting logic

6. **Update main.rs**: Replace direct window operations with WindowSystem calls

#### Success Criteria

- [ ] WindowSystem compiles and exports clean API
- [ ] main.rs creates WindowSystem in `resumed()`
- [ ] Cursor capture/release works identically
- [ ] Fullscreen toggle works identically
- [ ] Window title updates work identically
- [ ] main.rs reduced by ~60 lines

#### Test Requirements

```rust
// src/systems/window.rs tests
#[cfg(test)]
mod tests {
    #[test]
    fn test_cursor_state_management() {
        // Test cursor capture/release state transitions
    }

    #[test]
    fn test_title_formatting() {
        // Test window title format matches expected patterns
    }
}
```

---

### Session 2: RenderSystem (1 session)

**Risk:** Medium - touches GPU code, but well-isolated
**Impact:** Reduces main.rs by ~120 lines

#### Current State Analysis

**Lines to extract from main.rs:**
- GPU initialization in `resumed()` (lines 154-184): ~30 lines
- Geometry upload logic (lines 171-179, 311-318): ~15 lines
- Render frame logic in RedrawRequested (lines 364-475): ~110 lines

**Total extraction:** ~120 lines

#### Target API

```rust
// src/systems/render.rs

use std::sync::Arc;
use winit::window::Window;
use rust4d_render::{RenderContext, camera4d::Camera4D, RenderableGeometry};
use crate::config::RenderingConfig;

pub struct RenderSystem {
    context: RenderContext,
    slice_pipeline: SlicePipeline,
    render_pipeline: RenderPipeline,
}

pub enum RenderError {
    SurfaceLost,
    OutOfMemory,
    Other(String),
}

impl RenderSystem {
    /// Create render system from window and config
    pub async fn new(window: Arc<Window>, config: RenderingConfig) -> Self {
        // GPU initialization logic from resumed()
    }

    /// Handle window resize
    pub fn resize(&mut self, width: u32, height: u32) {
        // Resize logic from window_event
    }

    /// Upload geometry to GPU
    pub fn upload_geometry(&mut self, geometry: &RenderableGeometry) {
        // Geometry upload logic
    }

    /// Render a single frame
    pub fn render_frame(
        &self,
        camera: &Camera4D,
        geometry: &RenderableGeometry,
    ) -> Result<(), RenderError> {
        // Complete render pipeline from RedrawRequested
        // Returns Err on surface errors requiring handling
    }

    /// Get rendering context
    pub fn context(&self) -> &RenderContext {
        &self.context
    }
}
```

#### Implementation Strategy

1. **Create RenderSystem module**: `src/systems/render.rs`

2. **Extract GPU initialization**:
   - Move RenderContext, pipeline creation from `resumed()`
   - Accept `RenderingConfig` for max_output_triangles, lighting params

3. **Extract geometry upload**:
   - Move slice_pipeline.upload_tetrahedra() into `upload_geometry()`
   - Handle both initial upload and dirty updates

4. **Extract render frame**:
   - Move entire render logic (lines 364-475) into `render_frame()`
   - Create SliceParams from camera
   - Handle surface acquisition and errors
   - Return Result instead of panicking

5. **Error handling**:
   - Convert surface errors to RenderError enum
   - Propagate to main.rs for proper handling
   - Lost surface triggers resize, OutOfMemory exits gracefully

6. **Update main.rs**: Replace render logic with single `render_system.render_frame()` call

#### Success Criteria

- [ ] RenderSystem compiles with clean API
- [ ] GPU initialization works identically
- [ ] Geometry upload works identically
- [ ] Rendering produces identical output
- [ ] Surface errors handled gracefully
- [ ] Window resize works correctly
- [ ] main.rs reduced by ~120 lines

#### Test Requirements

```rust
// Integration tests - requires GPU access
#[cfg(test)]
mod tests {
    #[test]
    fn test_render_system_creation() {
        // Test RenderSystem can be created with valid config
    }

    #[test]
    fn test_geometry_upload() {
        // Test geometry can be uploaded and re-uploaded
    }

    #[test]
    fn test_resize_handling() {
        // Test resize updates depth texture correctly
    }
}
```

**Note:** Full render pipeline tests require GPU access and are best done as integration tests, not unit tests.

---

### Session 3: SimulationSystem (1 session)

**Risk:** Medium - core game logic, needs careful extraction
**Impact:** Reduces main.rs by ~80 lines

#### Current State Analysis

**Lines to extract from main.rs:**
- Delta time calculation (lines 267-270): ~4 lines
- Input processing (lines 274-294): ~20 lines
- Physics integration (lines 296-304): ~8 lines
- Camera synchronization (lines 321-345): ~25 lines
- Dirty tracking and geometry rebuild (lines 306-319): ~15 lines
- Last frame tracking (line 42 in App struct, line 82 in new(), line 270 in update): ~3 lines

**Total extraction:** ~80 lines

#### Target API

```rust
// src/systems/simulation.rs

use std::time::Instant;
use rust4d_core::World;
use rust4d_render::camera4d::Camera4D;
use rust4d_input::CameraController;
use crate::config::PhysicsConfig;

pub struct SimulationSystem {
    last_frame: Instant,
    config: PhysicsConfig,
}

pub struct SimulationUpdate {
    /// Whether geometry needs to be rebuilt
    pub geometry_dirty: bool,
    /// Delta time used for this update
    pub delta_time: f32,
}

impl SimulationSystem {
    /// Create simulation system with config
    pub fn new(config: PhysicsConfig) -> Self {
        Self {
            last_frame: Instant::now(),
            config,
        }
    }

    /// Run one simulation step
    pub fn update(
        &mut self,
        world: &mut World,
        camera: &mut Camera4D,
        controller: &CameraController,
        cursor_captured: bool,
    ) -> SimulationUpdate {
        // 1. Calculate delta time
        // 2. Process controller input
        // 3. Apply player movement to physics
        // 4. Step world physics
        // 5. Sync camera to player position
        // 6. Apply W-axis movement (4D navigation)
        // 7. Apply camera rotation from controller
        // 8. Check for dirty entities
        // 9. Return SimulationUpdate with dirty flag
    }
}
```

#### Implementation Strategy

1. **Create SimulationSystem module**: `src/systems/simulation.rs`

2. **Extract delta time tracking**:
   - Move `last_frame: Instant` into SimulationSystem
   - Calculate dt at start of `update()`

3. **Extract input processing**:
   - Move controller input queries (forward, right, W-axis)
   - Move camera forward/right projection to XZ plane
   - Move movement direction calculation

4. **Extract physics integration**:
   - Move player movement application
   - Move jump handling
   - Move world physics step
   - Move dirty entity checking

5. **Extract camera synchronization**:
   - Move camera XYZ sync to player position
   - Move W-axis navigation (non-physics movement)
   - Move controller.update() call for rotation
   - Move camera re-sync after rotation

6. **Return simulation results**:
   - Create SimulationUpdate struct
   - Set `geometry_dirty` flag from `world.has_dirty_entities()`
   - Return delta_time for potential use in title/debug

7. **Update main.rs**: Replace entire simulation block with single `simulation_system.update()` call

#### Success Criteria

- [ ] SimulationSystem compiles with clean API
- [ ] Delta time calculation works identically
- [ ] Player movement works identically
- [ ] Physics integration works identically
- [ ] Camera sync works identically
- [ ] Jump mechanic works identically
- [ ] Dirty tracking reports correctly
- [ ] main.rs reduced by ~80 lines

#### Test Requirements

```rust
#[cfg(test)]
mod tests {
    #[test]
    fn test_delta_time_calculation() {
        // Test delta time is calculated correctly between updates
    }

    #[test]
    fn test_simulation_reports_dirty_correctly() {
        // Test dirty flag is set when entities change
    }

    #[test]
    fn test_camera_sync_preserves_w_axis() {
        // Test camera W position is preserved during XYZ sync
    }
}
```

---

### Session 4: InputMapper (0.5 sessions)

**Risk:** Low - simple key mapping logic
**Impact:** Reduces main.rs by ~40 lines

#### Current State Analysis

**Lines to extract from main.rs:**
- Special key handling in KeyboardInput (lines 209-246): ~35 lines
- Mouse input handling (lines 249-255): ~6 lines

**Total extraction:** ~40 lines

#### Target API

```rust
// src/input/input_mapper.rs

use winit::keyboard::KeyCode;
use winit::event::{ElementState, MouseButton};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputAction {
    /// Toggle cursor capture/release
    ToggleCursor,
    /// Exit application (when cursor not captured)
    Exit,
    /// Reset camera to starting position
    ResetCamera,
    /// Toggle fullscreen mode
    ToggleFullscreen,
    /// Toggle input smoothing
    ToggleSmoothing,
}

pub struct InputMapper;

impl InputMapper {
    /// Map keyboard input to action
    /// Returns Some(action) if this key triggers a special action
    /// Returns None if key should be passed to controller
    pub fn map_keyboard(
        key: KeyCode,
        state: ElementState,
        cursor_captured: bool,
    ) -> Option<InputAction> {
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
            _ => None,
        }
    }

    /// Map mouse input to action
    /// Returns Some(action) if this mouse input triggers an action
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
```

#### Implementation Strategy

1. **Create input mapper module**: `src/input/input_mapper.rs`

2. **Define InputAction enum**: Represent all special actions (not movement)

3. **Extract keyboard mapping**:
   - Move Escape, R, F, G key handling
   - Return InputAction enum instead of executing directly
   - Handle context (cursor_captured) in mapping logic

4. **Extract mouse mapping**:
   - Move left-click to capture logic
   - Return InputAction enum

5. **Update main.rs**:
   - Call `InputMapper::map_keyboard()` in WindowEvent::KeyboardInput
   - Match on returned InputAction to execute corresponding system calls
   - Pass unmapped keys to controller

6. **Decouple input from execution**:
   - Mapping returns *what* to do, main.rs decides *how* to do it
   - Enables future features: rebindable keys, input recording, tutorials

#### Success Criteria

- [ ] InputMapper compiles with clean API
- [ ] All special keys work identically (Escape, R, F, G)
- [ ] Click to capture works identically
- [ ] Escape context-sensitive behavior preserved
- [ ] Movement keys still passed to controller
- [ ] main.rs reduced by ~40 lines

#### Test Requirements

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_escape_when_captured() {
        let action = InputMapper::map_keyboard(
            KeyCode::Escape,
            ElementState::Pressed,
            true
        );
        assert_eq!(action, Some(InputAction::ToggleCursor));
    }

    #[test]
    fn test_escape_when_released() {
        let action = InputMapper::map_keyboard(
            KeyCode::Escape,
            ElementState::Pressed,
            false
        );
        assert_eq!(action, Some(InputAction::Exit));
    }

    #[test]
    fn test_movement_keys_not_mapped() {
        let action = InputMapper::map_keyboard(
            KeyCode::KeyW,
            ElementState::Pressed,
            true
        );
        assert_eq!(action, None);
    }

    #[test]
    fn test_click_to_capture() {
        let action = InputMapper::map_mouse_button(
            MouseButton::Left,
            ElementState::Pressed,
            false
        );
        assert_eq!(action, Some(InputAction::ToggleCursor));
    }
}
```

---

## Target main.rs Structure

After Phase 4A extraction, main.rs should be ~150 lines with this structure:

```rust
// src/main.rs (~150 lines)

mod config;
mod scene;
mod systems;
mod input;

use systems::{WindowSystem, RenderSystem, SimulationSystem};
use input::{InputMapper, InputAction};

struct App {
    // System components
    window_system: Option<WindowSystem>,
    render_system: Option<RenderSystem>,
    simulation_system: SimulationSystem,

    // Domain state
    world: World,
    geometry: RenderableGeometry,
    camera: Camera4D,
    controller: CameraController,

    // Configuration
    config: AppConfig,
}

impl App {
    fn new() -> Self {
        // Load configuration
        let config = AppConfig::load().expect("Failed to load config");

        // Build initial scene
        let world = SceneBuilder::from_config(&config.scene).build();
        let geometry = Self::build_geometry(&world);
        let camera = Camera4D::from_config(&config.camera);
        let controller = CameraController::from_config(&config.input);
        let simulation_system = SimulationSystem::new(config.physics.clone());

        Self {
            window_system: None,
            render_system: None,
            simulation_system,
            world,
            geometry,
            camera,
            controller,
            config,
        }
    }

    fn build_geometry(world: &World) -> RenderableGeometry {
        // Keep geometry building in main.rs (scene-specific logic)
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window_system.is_none() {
            // Create window and render systems
            let window_system = WindowSystem::create(event_loop, &self.config.window);
            let render_system = pollster::block_on(
                RenderSystem::new(window_system.window().clone(), self.config.rendering.clone())
            );

            // Upload initial geometry
            render_system.upload_geometry(&self.geometry);

            self.window_system = Some(window_system);
            self.render_system = Some(render_system);
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }

            WindowEvent::Resized(size) => {
                if let Some(render_system) = &mut self.render_system {
                    render_system.resize(size.width, size.height);
                }
            }

            WindowEvent::KeyboardInput { event, .. } => {
                if let PhysicalKey::Code(key) = event.physical_key {
                    let window_system = self.window_system.as_mut().unwrap();

                    // Map special keys to actions
                    if let Some(action) = InputMapper::map_keyboard(
                        key,
                        event.state,
                        window_system.is_cursor_captured()
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
                            InputAction::ResetCamera => self.camera.reset(),
                            InputAction::ToggleFullscreen => window_system.toggle_fullscreen(),
                            InputAction::ToggleSmoothing => {
                                self.controller.toggle_smoothing();
                            }
                        }
                    } else {
                        // Pass to controller for movement
                        self.controller.process_keyboard(key, event.state);
                    }
                }
            }

            WindowEvent::MouseInput { state, button, .. } => {
                let window_system = self.window_system.as_mut().unwrap();

                if let Some(InputAction::ToggleCursor) = InputMapper::map_mouse_button(
                    button,
                    state,
                    window_system.is_cursor_captured()
                ) {
                    window_system.capture_cursor();
                }

                self.controller.process_mouse_button(button, state);
            }

            WindowEvent::MouseWheel { delta, .. } => {
                let scroll = match delta {
                    MouseScrollDelta::LineDelta(_, y) => y,
                    MouseScrollDelta::PixelDelta(pos) => pos.y as f32 / 100.0,
                };
                self.camera.adjust_slice_offset(scroll * 0.1);
            }

            WindowEvent::RedrawRequested => {
                let window_system = self.window_system.as_ref().unwrap();
                let render_system = self.render_system.as_ref().unwrap();

                // Run simulation
                let update = self.simulation_system.update(
                    &mut self.world,
                    &mut self.camera,
                    &self.controller,
                    window_system.is_cursor_captured(),
                );

                // Rebuild geometry if needed
                if update.geometry_dirty {
                    self.geometry = Self::build_geometry(&self.world);
                    render_system.upload_geometry(&self.geometry);
                    self.world.clear_all_dirty();
                }

                // Update window title
                window_system.update_title(self.camera.position, self.camera.get_slice_w());

                // Render frame
                match render_system.render_frame(&self.camera, &self.geometry) {
                    Ok(()) => {}
                    Err(RenderError::SurfaceLost) => {
                        render_system.resize(
                            render_system.context().size.width,
                            render_system.context().size.height
                        );
                    }
                    Err(RenderError::OutOfMemory) => {
                        event_loop.exit();
                    }
                    Err(e) => {
                        log::warn!("Render error: {:?}", e);
                    }
                }

                // Request next frame
                window_system.window().request_redraw();
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

**Line count breakdown:**
- Imports and module declarations: ~15 lines
- App struct: ~15 lines
- App::new(): ~25 lines
- App::build_geometry(): ~20 lines
- ApplicationHandler::resumed: ~15 lines
- window_event(): ~60 lines (largest section, but clean dispatch)
- device_event(): ~5 lines
- main(): ~10 lines

**Total:** ~150 lines

---

## Phase 4B: Error Handling (1 session)

**Goal:** Replace panics with proper error types and Result propagation throughout extracted systems.

### Current Panic Points in main.rs

1. **Window creation** (line 151): `.expect("Failed to create window")`
2. **Event loop creation** (line 505): `.expect("Failed to create event loop")`
3. **Event loop run** (line 510): `.expect("Event loop error")`

### Error Types to Create

```rust
// src/error.rs

use std::fmt;

#[derive(Debug)]
pub enum AppError {
    Window(WindowError),
    Render(RenderError),
    Config(figment::Error),
    Io(std::io::Error),
}

#[derive(Debug)]
pub enum WindowError {
    CreationFailed(String),
    EventLoopFailed(String),
}

#[derive(Debug)]
pub enum RenderError {
    SurfaceLost,
    OutOfMemory,
    DeviceCreationFailed(String),
    Other(String),
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            AppError::Window(e) => write!(f, "Window error: {:?}", e),
            AppError::Render(e) => write!(f, "Render error: {:?}", e),
            AppError::Config(e) => write!(f, "Config error: {}", e),
            AppError::Io(e) => write!(f, "I/O error: {}", e),
        }
    }
}

impl std::error::Error for AppError {}

impl From<WindowError> for AppError {
    fn from(e: WindowError) -> Self {
        AppError::Window(e)
    }
}

impl From<RenderError> for AppError {
    fn from(e: RenderError) -> Self {
        AppError::Render(e)
    }
}

impl From<figment::Error> for AppError {
    fn from(e: figment::Error) -> Self {
        AppError::Config(e)
    }
}
```

### System Updates

#### WindowSystem Error Handling

```rust
impl WindowSystem {
    pub fn create(
        event_loop: &ActiveEventLoop,
        config: &WindowConfig
    ) -> Result<Self, WindowError> {
        let window = event_loop
            .create_window(window_attributes)
            .map_err(|e| WindowError::CreationFailed(e.to_string()))?;

        // ... rest of creation logic
        Ok(Self { window, cursor_captured: false })
    }
}
```

#### RenderSystem Error Handling

```rust
impl RenderSystem {
    pub async fn new(
        window: Arc<Window>,
        config: RenderingConfig
    ) -> Result<Self, RenderError> {
        let context = RenderContext::new(window)
            .await
            .map_err(|e| RenderError::DeviceCreationFailed(e.to_string()))?;

        // ... rest of creation logic
        Ok(Self { context, slice_pipeline, render_pipeline })
    }

    pub fn render_frame(
        &self,
        camera: &Camera4D,
        geometry: &RenderableGeometry,
    ) -> Result<(), RenderError> {
        let output = match self.context.surface.get_current_texture() {
            Ok(output) => output,
            Err(wgpu::SurfaceError::Lost) => return Err(RenderError::SurfaceLost),
            Err(wgpu::SurfaceError::OutOfMemory) => return Err(RenderError::OutOfMemory),
            Err(e) => return Err(RenderError::Other(format!("{:?}", e))),
        };

        // ... render logic
        Ok(())
    }
}
```

#### App::new() Error Handling

```rust
impl App {
    fn new() -> Result<Self, AppError> {
        let config = AppConfig::load()?;

        let world = SceneBuilder::from_config(&config.scene).build();
        let geometry = Self::build_geometry(&world);
        let camera = Camera4D::from_config(&config.camera);
        let controller = CameraController::from_config(&config.input);
        let simulation_system = SimulationSystem::new(config.physics.clone());

        Ok(Self {
            window_system: None,
            render_system: None,
            simulation_system,
            world,
            geometry,
            camera,
            controller,
            config,
        })
    }
}
```

#### main() Error Handling

```rust
fn main() -> Result<(), AppError> {
    env_logger::init();
    log::info!("Starting Rust4D");

    let event_loop = EventLoop::new()
        .map_err(|e| WindowError::EventLoopFailed(e.to_string()))?;
    event_loop.set_control_flow(ControlFlow::Poll);

    let mut app = App::new()?;

    event_loop.run_app(&mut app)
        .map_err(|e| WindowError::EventLoopFailed(e.to_string()))?;

    Ok(())
}
```

### Error Handling Strategy

1. **Fail fast at startup**: Config errors, window creation errors should exit with clear messages
2. **Graceful recovery during runtime**: Surface lost → resize, out of memory → exit cleanly
3. **Informative error messages**: Include context (what failed, why, what to do)
4. **No silent failures**: Every error should be logged or propagated
5. **User-friendly**: Non-technical users should understand what went wrong

### Success Criteria

- [ ] No `.unwrap()` or `.expect()` in main.rs or systems
- [ ] All system creation returns Result
- [ ] Render errors handled gracefully
- [ ] Config errors display helpful messages
- [ ] Application exits cleanly on fatal errors
- [ ] Surface errors trigger appropriate recovery
- [ ] Error types implement Display and Error traits

---

## Files to Create

### New Files

```
src/
├── error.rs              # Error types (AppError, WindowError, RenderError)
├── systems/
│   ├── mod.rs           # Public exports: WindowSystem, RenderSystem, SimulationSystem
│   ├── window.rs        # WindowSystem (~80 lines with tests)
│   ├── render.rs        # RenderSystem (~140 lines with error handling)
│   └── simulation.rs    # SimulationSystem (~90 lines)
└── input/
    ├── mod.rs           # Public exports: InputMapper, InputAction
    └── input_mapper.rs  # InputMapper (~50 lines with tests)
```

### Modified Files

```
src/main.rs              # Reduced from 511 to ~150 lines
```

### Module Tree

```
rust4d (binary)
├── error          # Error types
├── config         # From Phase 1B
├── scene          # SceneBuilder (existing)
├── systems        # NEW
│   ├── window     # WindowSystem
│   ├── render     # RenderSystem
│   └── simulation # SimulationSystem
└── input          # NEW
    └── input_mapper # InputMapper
```

---

## Parallelization Strategy

Phase 4A extractions have minimal dependencies and can be partially parallelized:

### Sequential Approach (Single Agent)

```
Session 1: WindowSystem      → 1 session
Session 2: RenderSystem      → 1 session
Session 3: SimulationSystem  → 1 session
Session 4: InputMapper       → 0.5 sessions
Session 5: Error Handling    → 1 session

Total: 4.5 sessions
```

### Parallel Approach (2 Agents)

```
Wave 1 (Sessions 1-2):
├── Agent A: WindowSystem + InputMapper (1.5 sessions)
└── Agent B: RenderSystem (1 session, then wait)

Wave 2 (Sessions 3-4):
├── Agent A: SimulationSystem (1 session)
└── Agent B: Error Handling (1 session)

Total: 2 waves, 4 sessions wall-clock time
```

**Recommendation:** Single agent sequential approach is safer. Parallel approach saves minimal time and risks merge conflicts in main.rs.

---

## Testing Strategy

### Unit Tests

Each system should have unit tests for isolated logic:

- **WindowSystem**: Cursor state management, title formatting
- **SimulationSystem**: Delta time calculation, dirty tracking
- **InputMapper**: Key mapping logic, context-sensitive behavior

### Integration Tests

Create integration tests for system interactions:

```rust
// tests/systems_integration.rs

#[test]
fn test_full_frame_cycle() {
    // Create all systems
    // Run one frame cycle
    // Verify state transitions
}

#[test]
fn test_geometry_dirty_pipeline() {
    // Modify entity
    // Run simulation
    // Verify geometry_dirty flag
    // Verify geometry rebuild triggered
}
```

### Regression Tests

Before extraction, capture baseline behavior:

1. **Screenshot test**: Capture initial render, verify identical after refactor
2. **Physics test**: Record entity positions over 100 frames, verify deterministic
3. **Input test**: Record input sequence, verify identical behavior

### Manual Testing Checklist

After each system extraction:

- [ ] Application launches without errors
- [ ] Window creation works (resolution, title)
- [ ] Cursor capture/release works (Escape, click)
- [ ] Fullscreen toggle works (F key)
- [ ] Camera movement works (WASD)
- [ ] Camera rotation works (mouse)
- [ ] Jump works (Space)
- [ ] Physics simulation works (gravity, collisions)
- [ ] 4D movement works (RF keys)
- [ ] Geometry updates when entities move
- [ ] Window title updates correctly
- [ ] Application exits cleanly (Escape twice)

---

## Risks and Mitigations

### Risk 1: Breaking Working Code

**Likelihood:** Medium
**Impact:** High

**Mitigation:**
1. **Extract one system at a time**: Compile and test after each extraction
2. **Preserve all behavior**: Zero functional changes during refactoring
3. **Regression tests**: Capture baseline behavior before starting
4. **Small commits**: Commit after each working system extraction
5. **Feature branch**: Work in feature/architecture-refactor, merge when stable

### Risk 2: Performance Regression

**Likelihood:** Low
**Impact:** Medium

**Mitigation:**
1. **Avoid unnecessary allocations**: Systems should reuse buffers where possible
2. **Profile before and after**: Ensure frame time doesn't increase
3. **Keep hot paths inline**: Rendering and simulation should be zero-cost abstractions

### Risk 3: Ownership and Lifetime Issues

**Likelihood:** Medium (Rust borrow checker challenges)
**Impact:** Medium (implementation time)

**Mitigation:**
1. **Clear ownership model**: Systems own their state, App owns domain state
2. **Avoid shared mutable state**: Pass references explicitly, no Arc<Mutex<T>> unless required
3. **Use builder pattern**: Complex system creation should use builders
4. **Prototype first**: Test ownership patterns in small test files before full extraction

### Risk 4: Integration with Future Configuration

**Likelihood:** Low (if Phase 1B completed first)
**Impact:** Medium

**Mitigation:**
1. **Hard dependency on Phase 1B**: Don't start Phase 4 until config system exists
2. **Design systems for config**: All systems accept config structs in constructors
3. **Default config values**: Systems should work with default config if needed

### Risk 5: Over-Engineering

**Likelihood:** Medium
**Impact:** Low

**Mitigation:**
1. **YAGNI principle**: Extract only what's needed, avoid premature abstraction
2. **Keep it simple**: Systems are thin wrappers, not frameworks
3. **No ECS conversion**: Phase 4 is refactoring, not architecture redesign
4. **Preserve existing patterns**: Use same patterns as SceneBuilder (proven to work)

---

## Success Criteria

### Quantitative Metrics

- [ ] main.rs reduced from 511 lines to ≤150 lines
- [ ] WindowSystem ~80 lines (code + tests)
- [ ] RenderSystem ~140 lines (code + error handling)
- [ ] SimulationSystem ~90 lines (code + tests)
- [ ] InputMapper ~50 lines (code + tests)
- [ ] Zero behavioral changes (all tests pass)
- [ ] All new systems have ≥80% test coverage

### Qualitative Metrics

- [ ] Code is more readable and maintainable
- [ ] Systems have clear, documented APIs
- [ ] main.rs is primarily event dispatch and integration
- [ ] Systems can be tested independently
- [ ] Error messages are helpful and actionable
- [ ] Future features can be added without modifying main.rs

### Integration Criteria

- [ ] Application launches and runs identically
- [ ] All input controls work correctly
- [ ] Rendering produces identical output
- [ ] Physics simulation is deterministic
- [ ] Performance is equivalent (±5% frame time)
- [ ] No new compiler warnings
- [ ] All existing tests pass
- [ ] New tests pass

---

## Post-Phase 4 Benefits

### Immediate Benefits

1. **Testability**: Each system can be tested independently
2. **Readability**: main.rs is now a clear orchestration layer
3. **Maintainability**: Changes to rendering don't require touching simulation
4. **Error handling**: Graceful failures instead of panics

### Long-Term Benefits

1. **Extensibility**: New features plug into existing systems
2. **Modularity**: Systems can be replaced or upgraded independently
3. **Documentation**: System APIs provide clear contracts
4. **Onboarding**: New contributors understand architecture quickly

### Enablers for Future Phases

- **Phase 5 (Advanced Features)**: Scene serialization can integrate cleanly
- **UI System**: Can be added as new system without modifying main.rs
- **Hot Reload**: Systems can support reload without full restart
- **Multiplayer**: Clear separation enables client/server split
- **Editor**: Systems provide API for external tooling

---

## Next Steps After Phase 4

1. **Code review**: Review extracted code for quality and consistency
2. **Documentation**: Update ARCHITECTURE.md with system diagrams
3. **Performance validation**: Profile and compare with pre-refactor baseline
4. **Merge to main**: Merge feature branch after all tests pass
5. **Begin Phase 5**: Advanced features can now build on clean architecture

---

## References

- Architecture Review: `scratchpad/reports/2026-01-27-engine-review-swarm/architecture-review.md`
- Config Recommendations: `scratchpad/reports/2026-01-27-engine-review-swarm/config-recommendations.md`
- Current main.rs: `src/main.rs` (511 lines, feature/physics branch)
- SceneBuilder pattern: `src/scene/scene_builder.rs` (model for system extraction)

---

**Phase 4 Plan Complete**

This plan provides actionable steps for an agent to systematically extract main.rs into modular systems while preserving all functionality. The plan prioritizes safety (one system at a time), testability (unit tests for each system), and maintainability (clear APIs and error handling).

Start with Session 1 (WindowSystem) after Phase 1B (Configuration) is complete.
