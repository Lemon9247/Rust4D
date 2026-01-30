# Agent A3: Render, Input & Main Binary Review
**Date**: 2026-01-30
**Scope**: `rust4d_render`, `rust4d_input`, `src/` (main binary + systems)

---

## 1. Rendering Pipeline (`rust4d_render`)

### 1.1 Architecture Overview

The rendering pipeline is a **two-stage GPU pipeline** that converts 4D tetrahedral geometry into 3D triangles and renders them:

1. **Stage 1 - Compute Shader (Slice Pipeline)**: A WGSL compute shader (`slice_tetra.wgsl`) takes 4D tetrahedra and slices them with a W-hyperplane, producing 3D triangles. This is the core 4D-to-3D projection mechanism.

2. **Stage 2 - Render Pipeline**: A standard vertex+fragment shader pair (`render.wgsl`) renders the 3D triangles produced by the compute shader, with lighting and W-depth coloring.

**Data flow**:
```
World entities
  -> RenderableGeometry (CPU: vertices + tetrahedra indices)
    -> SlicePipeline (GPU compute: 4D tetra -> 3D triangles)
      -> RenderPipeline (GPU render: 3D triangles -> screen)
```

The architecture is clean and well-separated. Each stage has its own pipeline struct, buffers, and shader.

### 1.2 Crate Structure

| File | Lines | Purpose |
|------|-------|---------|
| `lib.rs` | 31 | Re-exports from sub-modules |
| `context.rs` | 149 | wgpu device/queue/surface management |
| `camera4d.rs` | 609 | 4D camera with Engine4D-style architecture |
| `renderable.rs` | 317 | CPU-side geometry conversion (Entity -> GPU buffers) |
| `pipeline/mod.rs` | 26 | Re-exports |
| `pipeline/types.rs` | 239 | GPU data types (Vertex4D, Vertex3D, SliceParams, etc.) |
| `pipeline/lookup_tables.rs` | 281 | Compile-time tetrahedron slicing tables |
| `pipeline/slice_pipeline.rs` | 304 | Compute pipeline for 4D slicing |
| `pipeline/render_pipeline.rs` | 375 | Render pipeline with indirect drawing |
| `shaders/slice_tetra.wgsl` | 322 | Compute shader for tetrahedron slicing |
| `shaders/render.wgsl` | 131 | Vertex + fragment shader |

**Dependencies**: `rust4d_math`, `rust4d_core`, `rust4d_input`, `wgpu`, `winit`, `bytemuck`, `log`

### 1.3 Camera4D (`camera4d.rs`)

The camera uses an **Engine4D-style architecture** that cleanly separates pitch from 4D rotation:

- **Pitch** (`f32`): Stored separately, clamped to +/-89 degrees. Only affects YZ plane.
- **rotation_4d** (`Rotor4`): Operates in XZW hyperplane via `SkipY` transform, preserving the Y axis.
- **Movement**: Transformed by the full camera matrix (`skip_y(rotation_4d) * pitch_rotation`).

Key design properties (lines 6-10):
- Pitch is stored separately from 4D rotation
- 4D rotations operate in XZW hyperplane only (via SkipY)
- Movement is transformed by the full camera matrix
- Y axis always remains aligned with gravity/world up

This is a well-thought-out design. The separation ensures:
- Walking forward stays horizontal regardless of 4D rotation state
- 4D rotations never tilt the world
- Pitch naturally affects vertical movement

**Methods**:
- `rotate_3d(yaw, pitch)` - Standard mouse look; yaw goes to `rotation_4d` as XY plane rotation (becomes XZ after SkipY), pitch updates the separate pitch field
- `rotate_w(delta)` - ZW rotation via XZ rotor
- `rotate_xw(delta)` - XW rotation via YZ rotor
- `move_local_xz(forward, right)` - Horizontal movement transformed by camera matrix
- `move_y(delta)` - World-space Y movement (always vertical)
- `move_w(delta)` - Camera-relative W movement
- `forward()`, `right()`, `up()`, `ana()` - Direction vectors from camera matrix

The camera implements the `CameraControl` trait from `rust4d_input`, enabling the controller to work with it.

### 1.4 Compute Shader: Slice Pipeline (`slice_tetra.wgsl`)

**Algorithm** (lines 213-321 of shader):
1. Each invocation processes one tetrahedron (workgroup size 64)
2. Transform all 4 vertices from world space to camera space via `transform_to_camera_space()` (translate by -camera_pos, then rotate by transpose of camera_matrix)
3. Compute case index (0-15) based on which vertices have `w > slice_w`
4. Skip if case is 0 or 15 (no intersection)
5. Use lookup tables (`TETRA_EDGE_TABLE`, `TETRA_TRI_TABLE`) to determine crossed edges
6. Interpolate intersection points along crossed edges using `edge_intersection()`
7. Compute triangle normals, orient toward camera
8. Atomically allocate output slots and write triangles

**Key details**:
- Counter increments by 3 (vertex count) for direct use in `DrawIndirect` (line 315)
- Normal orientation ensures consistent face-toward-camera rendering (lines 297-307)
- Degenerate triangle fallback uses (0, 1, 0) normal (line 204)
- W-depth is stored per vertex for depth-based coloring

**Lookup Tables** (`lookup_tables.rs`):
- Computed at compile time via `const fn` (lines 52-79, 88-161)
- 16 cases for a tetrahedron (2^4 vertex configurations)
- 6 edges per tetrahedron: [0,1], [0,2], [0,3], [1,2], [1,3], [2,3]
- Edge table: bitmask of crossed edges per case
- Triangle table: up to 2 triangles per case (3 edges = 1 triangle, 4 edges = 2 triangles)
- Quad cases use cyclic order (0,1,3) and (0,3,2) for proper fan triangulation (WGSL shader line 120)

**Note**: The Rust-side lookup tables and the WGSL-side tables are maintained separately. The Rust tables use `[i8; 6]` with -1 sentinel while the WGSL tables use `array<i32, 6>`. The quad triangulation differs slightly: Rust uses `(0,1,2),(0,2,3)` vs WGSL uses `(0,1,3),(0,3,2)`. The WGSL version appears more correct for cyclic quad ordering.

### 1.5 Render Shader (`render.wgsl`)

**Vertex shader** (lines 54-72):
- Transforms positions through view * projection matrices
- Passes world_position, world_normal, vertex_color, w_depth to fragment

**Fragment shader** (lines 104-130):
- Lambert diffuse lighting with configurable ambient/diffuse strengths
- W-depth color gradient: blue (-W) -> neutral gray (0) -> red (+W)
- Blends vertex color with W-depth color using `w_color_strength`
- Preserves original vertex alpha

**Supported effects**:
- Basic diffuse + ambient lighting
- W-depth coloring (4D depth visualization)
- Per-vertex color blending
- Alpha blending enabled in pipeline state (line 88 of render_pipeline.rs)

### 1.6 Pipeline Stages & Connection

**SlicePipeline** (`slice_pipeline.rs`):
- Creates compute pipeline, bind group layout (5 bindings: vertices, tetrahedra, output, counter, params)
- Output buffer sized by `max_triangles * 3 * sizeof(Vertex3D)`, clamped to GPU's `max_storage_buffer_binding_size`
- Counter buffer doubles as `INDIRECT` buffer for draw call
- `run_slice_pass()` dispatches `ceil(tetra_count / 64)` workgroups

**RenderPipeline** (`render_pipeline.rs`):
- Creates render pipeline with vertex buffer layout matching `Vertex3D` (48 bytes stride)
- Depth buffer: `Depth32Float`, less-than comparison
- Back-face culling disabled (line 97: "Disabled for debugging")
- Alpha blending enabled
- `prepare_indirect_draw()` copies counter to indirect buffer
- `render()` uses `draw_indirect()` for GPU-driven vertex count

**Connection flow in `RenderSystem::render_frame()`** (`src/systems/render.rs` lines 104-205):
1. Update slice params (camera matrix, position, slice_w, tetra count)
2. Update render uniforms (view=identity, projection, lighting params)
3. Reset counter to 0
4. Run compute slice pass
5. Copy counter to indirect draw buffer
6. Run render pass with indirect draw

The view matrix is identity because the compute shader already outputs camera-space coordinates. The projection matrix is applied by the vertex shader.

### 1.7 RenderableGeometry (`renderable.rs`)

Bridges `Entity`/`World` (from `rust4d_core`) to GPU buffers:
- `from_entity()` / `from_world()` - Convert entities to `Vertex4D` + `GpuTetrahedron` arrays
- Transforms vertex positions by entity's `Transform4D`
- Supports custom color functions (material-based, position-gradient, checkerboard)
- Pre-allocates capacity when building from World (lines 61-67)
- Properly offsets tetrahedra indices when adding multiple entities (line 100-106)

**Limitation**: The entire world is re-uploaded as a single flat buffer. No per-entity GPU buffers, no instancing, no incremental updates.

### 1.8 GPU Performance Characteristics

**Strengths**:
- Indirect rendering: triangle count determined on GPU, no CPU readback
- Compute shader parallelism: 64 invocations per workgroup, one tetrahedron each
- Atomic counter for lock-free triangle allocation
- Configurable max_triangles with GPU limit clamping

**Weaknesses**:
- Single monolithic vertex/tetrahedra upload - entire world re-uploaded on any change
- No frustum culling (all tetrahedra processed even if off-screen)
- No LOD system
- No multi-sample anti-aliasing (MSAA) - sample count is 1 (line 110 of render_pipeline.rs)
- Back-face culling disabled (wastes fragment shader invocations)
- No texture support at all

### 1.9 Testing

**Well-tested**:
- `camera4d.rs`: 16 tests covering Y-axis preservation, pitch clamping, orthogonality, movement directions, reset, 4D rotation combinations, slice stability
- `renderable.rs`: 9 tests for entity/world conversion, capacity, clearing, color functions, transform application, index offsetting
- `pipeline/types.rs`: 7 tests for struct sizes and alignment (critical for GPU compatibility)
- `pipeline/lookup_tables.rs`: 6 tests for edge table correctness, triangle table coverage, symmetry, index validity
- `pipeline/render_pipeline.rs`: 3 tests for vertex layout stride, perspective matrix, indirect args size
- `pipeline/slice_pipeline.rs`: 1 test for output buffer size calculation

**Cannot be unit-tested** (requires GPU):
- Actual compute shader execution
- Render pipeline execution
- Buffer upload/download
- Indirect draw flow

**Missing tests**:
- `context.rs`: Zero tests (requires window + GPU)
- Shader correctness (would need integration tests or CPU reference implementation)
- Cross-validation between Rust and WGSL lookup tables

---

## 2. Input System (`rust4d_input`)

### 2.1 Architecture

The input crate is minimal and focused:

| File | Lines | Purpose |
|------|-------|---------|
| `lib.rs` | 9 | Re-exports |
| `camera_controller.rs` | 934 | Camera controller + CameraControl trait |

**Dependencies**: `rust4d_math` (for Vec4), `winit` (for key/mouse types)

### 2.2 CameraController (`camera_controller.rs`)

**Movement state** (boolean flags): forward, backward, left, right, up, down, ana (Q), kata (E), jump_pressed

**Mouse state**: mouse_pressed (left), w_rotation_mode (right), pending_yaw, pending_pitch, smooth_yaw, smooth_pitch

**Configuration** (all `pub` fields with builder pattern):
- `move_speed` (default: 3.0)
- `w_move_speed` (default: 2.0)
- `mouse_sensitivity` (default: 0.002)
- `w_rotation_sensitivity` (default: 0.005)
- `smoothing_half_life` (default: 0.05s)
- `smoothing_enabled` (default: false)

**Input processing**:
- `process_keyboard(KeyCode, ElementState)` - Sets boolean flags for WASD/QE/Space/Shift, returns `true` if handled
- `process_mouse_button(MouseButton, ElementState)` - Tracks left click and right click
- `process_mouse_motion(dx, dy)` - Accumulates pending yaw/pitch

**Update method** (`update()`, lines 135-186):
1. Calculate movement deltas from boolean flags
2. Apply movement: `move_local_xz(fwd * speed * dt, rgt * speed * dt)`, `move_y(up_down * speed * dt)`, `move_w(w * w_speed * dt)`
3. Optionally smooth mouse input using exponential smoothing: `factor = 2^(-dt / half_life)`
4. Apply rotation: if w_rotation_mode (right-click), apply `rotate_w` and `rotate_xw`; otherwise if can_look, apply `rotate_3d`
5. Clear pending mouse movement
6. Return camera position

**Free look modes**:
- Cursor captured: free look always active
- Cursor released: left-click + drag for 3D rotation, right-click + drag for 4D rotation

### 2.3 CameraControl Trait

```rust
pub trait CameraControl {
    fn move_local_xz(&mut self, forward: f32, right: f32);
    fn move_y(&mut self, delta: f32);
    fn move_w(&mut self, delta: f32);
    fn rotate_3d(&mut self, delta_yaw: f32, delta_pitch: f32);
    fn rotate_w(&mut self, delta: f32);
    fn rotate_xw(&mut self, delta: f32);
    fn position(&self) -> Vec4;
}
```

Clean abstraction that allows the controller to work with different camera implementations. `Camera4D` implements this trait.

### 2.4 Testing

**Excellent test coverage** - 37 tests covering:
- Builder pattern (7 tests)
- Key state tracking (10 tests)
- Movement direction calculation (9 tests)
- Jump consume pattern (4 tests)
- Mouse input (5 tests)
- Smoothing toggle (2 tests)
- Full update loop integration with MockCamera (9 tests)

The MockCamera pattern is well-designed - records all calls for verification.

### 2.5 Input Architecture Gaps

- **No input remapping**: Key bindings are hardcoded in `process_keyboard()`. No config-driven binding system.
- **No gamepad support**: Only keyboard + mouse.
- **No action/axis abstraction**: Movement keys directly set boolean flags. No intermediate "action" layer that could support multiple input sources.
- **Diagonal movement not normalized**: Pressing W+D gives sqrt(2) speed. The update method applies forward and right independently.

---

## 3. Main Binary & Systems (`src/`)

### 3.1 Application Structure (`main.rs`)

The application uses winit's `ApplicationHandler` trait:

```
main() -> EventLoop::run_app(App)
  App::resumed() -> creates WindowSystem + RenderSystem
  App::window_event() -> handles input, triggers render
  App::device_event() -> raw mouse motion
```

**App struct fields**: config, window_system, render_system, scene_manager, geometry (cached), camera, controller, simulation

**Event flow**:
1. `KeyboardInput` -> `InputMapper::map_keyboard()` for special keys (Escape, R, F, G), then `controller.process_keyboard()` for movement
2. `MouseInput` -> `InputMapper::map_mouse_button()` for cursor capture, then `controller.process_mouse_button()`
3. `MouseWheel` -> `camera.adjust_slice_offset()`
4. `DeviceEvent::MouseMotion` -> `controller.process_mouse_motion()`
5. `RedrawRequested` -> `simulation.update()` -> conditional geometry rebuild -> `render_system.render_frame()`

### 3.2 System Modules

**WindowSystem** (`systems/window.rs`, 154 lines):
- Window creation from config
- Cursor capture/release (locked mode, fallback to confined)
- Fullscreen toggle (borderless)
- Title bar debug display (position + slice_w)

**RenderSystem** (`systems/render.rs`, 226 lines):
- Wraps `RenderContext`, `SlicePipeline`, `RenderPipeline`
- `upload_geometry()` - sends vertex/tetra data to GPU
- `render_frame()` - orchestrates compute + render passes
- View matrix is identity (compute shader outputs camera-space)

**SimulationSystem** (`systems/simulation.rs`, 162 lines):
- Delta time calculation with cap (max 33ms = 30fps minimum)
- Movement direction computation using camera orientation vectors, projected to XZW hyperplane
- Physics integration: movement -> jump -> world update -> camera sync
- Double camera position sync: before and after controller update (lines 112-130) to ensure physics position wins over controller movement
- Returns `SimulationResult { geometry_dirty }` for conditional re-upload

### 3.3 InputMapper (`input/input_mapper.rs`)

Separates "special" key actions from movement:
- `Escape` (captured) -> ToggleCursor
- `Escape` (released) -> Exit
- `R` -> ResetCamera
- `F` -> ToggleFullscreen
- `G` -> ToggleSmoothing
- Left-click (not captured) -> ToggleCursor

Movement keys (WASD, etc.) are NOT mapped here - they pass through to `CameraController`.

### 3.4 Configuration (`config.rs`)

Uses `figment` for layered config loading:
1. `config/default.toml` (version controlled)
2. `config/user.toml` (gitignored)
3. Environment variables (`R4D_SECTION__KEY`)

Config sections: `WindowConfig`, `CameraConfig`, `InputConfig`, `PhysicsConfigToml`, `RenderingConfig`, `DebugConfig`, `SceneConfig`

All config values flow into the application:
- Camera pitch limit, FOV, near/far
- Input speeds and sensitivities
- Rendering: max_triangles, background color, lighting parameters, W-depth effects
- Physics: gravity, jump velocity
- Scene: path, player_radius

### 3.5 Scene Loading

The application loads scenes via `SceneManager`:
1. `load_scene(path)` - loads a `.ron` scene file
2. `instantiate(name)` - creates world from scene definition
3. `push_scene(name)` - activates the scene
4. Player spawn position from scene file, with config fallback

### 3.6 Geometry Building

`App::build_geometry()` (lines 120-143) applies different coloring strategies per entity tag:
- `"dynamic"` tagged entities -> position gradient coloring
- Everything else -> checkerboard pattern (2.0 cell size, dark/light gray)

This is hardcoded in the binary, not configurable.

---

## 4. Boomer Shooter Gaps

### 4.1 Rendering Gaps

| Gap | Impact | Difficulty |
|-----|--------|------------|
| **No multi-pass rendering** | Cannot do shadow maps, post-processing | Major architecture change |
| **No shadow system** | Flat lighting kills atmosphere | Major feature |
| **No particle system** | No muzzle flash, explosions, blood | Major feature |
| **No HUD/UI overlay** | No health bar, ammo counter, crosshair | Needs separate 2D pipeline |
| **No transparency/sorting** | No windows, forcefields, energy effects | Needs alpha sorting or OIT |
| **No sprites/billboards** | No pickups, enemy sprites, decals | New render path |
| **No weapon viewmodel** | No gun on screen | Separate camera space rendering |
| **No texture support** | Everything is flat-colored | Major pipeline change |
| **No skybox/environment** | Solid color background only | Minor feature |
| **No MSAA** | Jagged edges on geometry | Config change (sample_count) |
| **No frustum culling** | All geometry processed every frame | Performance critical for large levels |
| **No per-entity GPU state** | Full world re-upload on any change | Performance critical for dynamic entities |
| **Back-face culling disabled** | Wasted GPU work | Simple fix (re-enable) |
| **No screen-space effects** | No bloom, no chromatic aberration | Post-processing pipeline needed |

### 4.2 Input Gaps

| Gap | Impact | Difficulty |
|-----|--------|------------|
| **No weapon switching** | Cannot change weapons (1-9 keys) | Simple addition |
| **No shoot/fire action** | No primary fire (left click when captured) | Medium (needs action system) |
| **No interaction key** | Cannot use doors, switches (E key) | Simple addition (but E = kata) |
| **No gamepad support** | No controller play | Medium feature |
| **No rebindable controls** | Hardcoded bindings | Config-driven binding system |
| **No diagonal normalization** | Moving diagonally is faster | Simple fix |
| **No sprint/crouch** | Missing FPS staples | Simple addition |

**Key conflict**: E is currently bound to "kata" (4D movement), but E is traditionally "use/interact" in FPS games. Q is "ana". This will need rethinking for a shooter.

### 4.3 System Gaps

| Gap | Impact | Difficulty |
|-----|--------|------------|
| **No fixed-timestep update** | Physics tied to frame rate | Architecture change (SimulationSystem) |
| **No audio system** | No sound at all | Major feature |
| **No AI/enemy system** | No enemies, no pathfinding | Major feature |
| **No spawn/respawn system** | No player death/respawn | Medium feature |
| **No weapon/projectile system** | No bullets, rockets | Major feature |
| **No health/damage system** | No combat | Medium feature |
| **No level transition** | Cannot change maps | SceneManager partially supports this |
| **No networking** | No multiplayer | Very major feature |
| **No frame rate limiter** | `ControlFlow::Poll` runs as fast as possible | Simple fix |

### 4.4 Architecture Gaps

- **No ECS**: Uses simple Entity struct, no component system. Adding weapons, health, AI would require ad-hoc fields or a proper ECS migration.
- **No event/message bus**: Systems communicate through direct struct access. No way to broadcast "player shot", "enemy died" events.
- **Single-threaded**: Everything runs on the main thread. No async systems, no job system.

---

## 5. Overall Assessment

### Ratings (1-5)

| Criterion | `rust4d_render` | `rust4d_input` | `src/` (main) |
|-----------|----------------|----------------|----------------|
| Feature Completeness | 3/5 | 2/5 | 2/5 |
| Code Quality | 4/5 | 4/5 | 4/5 |
| Test Coverage | 4/5 | 5/5 | 2/5 |
| FPS Readiness | 1/5 | 2/5 | 2/5 |

### Top 3 Strengths

**`rust4d_render`**:
1. **Elegant 4D->3D pipeline**: The two-stage compute+render approach with lookup tables is well-engineered and mathematically sound
2. **Camera4D architecture**: Engine4D-style pitch/rotation separation is exactly right for 4D FPS movement
3. **GPU-driven rendering**: Indirect draw with atomic counters avoids CPU-GPU readback

**`rust4d_input`**:
1. **Excellent test coverage**: 37 unit tests including MockCamera-based integration tests
2. **Builder pattern**: Clean configuration with method chaining
3. **CameraControl trait**: Good abstraction for camera-agnostic input handling

**`src/` (main binary)**:
1. **Clean system separation**: WindowSystem, RenderSystem, SimulationSystem are well-modularized
2. **Layered configuration**: figment-based config with defaults/user/env layering
3. **Physics-driven movement**: SimulationSystem properly integrates physics with camera

### Top 3 Gaps

**`rust4d_render`**:
1. **Single-pass only**: No multi-pass rendering, no post-processing, no shadows
2. **No texture support**: Only flat vertex colors
3. **No frustum culling or instancing**: Cannot scale to complex scenes

**`rust4d_input`**:
1. **No action/axis abstraction**: Hardcoded key-to-movement mapping
2. **No game actions**: Only camera movement, no shoot/interact/weapon switch
3. **No gamepad support**: Keyboard+mouse only

**`src/` (main binary)**:
1. **No game loop separation**: No fixed-timestep physics, simulation tied to frame rate
2. **No game systems**: No health, damage, weapons, AI, audio
3. **Hardcoded geometry building**: Color strategies and scene construction are not data-driven

---

## 6. Notable Code Quality Observations

1. **Documentation**: Every public type and function has doc comments. Module-level `//!` docs explain purpose. Camera4D comments explain the Engine4D architectural reasoning.

2. **Consistent error handling**: `RenderContext::new()` panics on adapter/device failure (acceptable for engine init), while `render_frame()` returns proper error types.

3. **Memory safety**: All GPU types use `bytemuck::Pod + Zeroable` for safe buffer operations. Struct sizes and alignments are tested.

4. **The double-sync pattern** in `SimulationSystem::update()` (lines 112-130): Physics position is synced to camera before AND after `controller.update()`. This discards the controller's own movement (which would double-count with physics movement) while keeping the controller's rotation. Clever but fragile -- a comment explains the intent but it's a non-obvious interaction.

5. **Lookup table divergence**: The Rust-side `TETRA_TRI_TABLE` uses `(0,1,2),(0,2,3)` quad triangulation while the WGSL shader uses `(0,1,3),(0,3,2)`. The Rust tables are not used at runtime (only in tests), so this isn't a bug, but it's a maintenance risk.
