# Agent P5: Editor & Polish -- Engine Implementation Plan

**Agent**: P5 - Editor & Polish
**Date**: 2026-01-30
**Scope**: Phase 5 items from cross-swarm synthesis: egui editor, point lights + shadows, texture support, input rebinding
**Assumption**: Engine/game split is complete. ECS migration (hecs) is done. rust4d_game exists. Game repo exists.

---

## Executive Summary

Phase 5 is the largest single feature phase in the roadmap, dominated by the egui editor (6-8 sessions out of 6-10 total). After deep analysis of the rendering pipeline, input system, and 4D slicing architecture, I have refined the plan with several key findings:

1. **The editor is an engine tool** and belongs in a new `rust4d_editor` crate, but it must be designed as an *optional* crate that games can choose not to depend on.
2. **Point lights are straightforward** in the current pipeline -- the shader already has a lighting model, just needs expansion from one directional light to N point lights via a storage buffer.
3. **Texture support in 4D is genuinely hard** -- the compute shader produces interpolated Vertex3D from tetrahedra slicing, and there is no UV coordinate path through the pipeline. This needs careful design.
4. **Input rebinding is already 80% planned** by the engine/game split plan's action/axis abstraction. The remaining work is persistence and runtime modification.
5. **The editor's dependency chain is the deepest of any feature** -- it needs ECS, serialization, all shape types, and a working renderer. This makes it naturally the last major engine feature.

**Revised session estimate: 8-12 sessions** (up from 6-10 in the synthesis, because texture support is harder than estimated).

---

## 1. Editor Architecture

### 1.1 Engine vs Game Boundary

The editor is **100% engine**. It is a development tool for creating and inspecting 4D scenes, not gameplay UI. The game repo never depends on the editor crate.

| Component | Where | Why |
|-----------|-------|-----|
| egui integration + panels | `rust4d_editor` (new engine crate) | Generic scene editing tool |
| Entity list, property inspector | `rust4d_editor` | Works with any ECS components |
| W-slice navigation widget | `rust4d_editor` | 4D-specific visualization tool |
| 3D viewport | `rust4d_editor` (reuses `rust4d_render`) | Renders via existing pipeline |
| Scene save/load | `rust4d_editor` (uses `rust4d_core` scene API) | Generic engine capability |
| Pause menu | **Game repo** | Game-specific UI |
| Gameplay-specific editors (weapon tuning, enemy config) | **Game repo** | Game-specific tools |

### 1.2 Crate Structure

```
crates/rust4d_editor/
  Cargo.toml            # depends on: egui, egui-wgpu, egui-winit, rust4d_core, rust4d_render, rust4d_math
  src/
    lib.rs              # EditorApp struct, integration trait
    panels/
      mod.rs
      entity_list.rs    # Entity list panel (ECS query for Name components)
      property_inspector.rs  # Component display/editing (Transform4D, Material, ShapeRef, custom)
      w_slice_nav.rs    # W-slice slider + multi-slice preview thumbnails
      viewport.rs       # 3D viewport (wraps existing render pipeline)
      scene_panel.rs    # Scene save/load, scene tree
    state.rs            # Editor state (selected entity, gizmo mode, etc.)
    integration.rs      # EditorIntegration trait -- how the editor plugs into a game's main loop
```

### 1.3 Integration Design

The editor must not take over the game's event loop. Instead, it provides a composable integration:

```rust
/// Trait that the application implements to integrate the editor
pub trait EditorHost {
    /// Provide access to the ECS world for querying/editing
    fn world(&self) -> &hecs::World;
    fn world_mut(&mut self) -> &mut hecs::World;

    /// Provide the render context for the viewport
    fn render_context(&self) -> &RenderContext;

    /// Provide the current camera for the viewport
    fn camera(&self) -> &Camera4D;
    fn camera_mut(&mut self) -> &mut Camera4D;

    /// Provide the current slice_w for W-slice navigation
    fn slice_w(&self) -> f32;
    fn set_slice_w(&mut self, w: f32);

    /// Scene file operations
    fn save_scene(&self, path: &std::path::Path) -> Result<(), Box<dyn std::error::Error>>;
    fn load_scene(&mut self, path: &std::path::Path) -> Result<(), Box<dyn std::error::Error>>;
}
```

The editor then provides:

```rust
pub struct EditorApp {
    ctx: egui::Context,
    state: EditorState,
    panels: PanelManager,
}

impl EditorApp {
    /// Process winit events for egui
    pub fn handle_event(&mut self, event: &winit::event::WindowEvent) -> bool;

    /// Build the UI for this frame. Returns egui paint jobs.
    pub fn update(&mut self, host: &mut dyn EditorHost) -> egui::FullOutput;

    /// Render the egui overlay onto the existing frame
    pub fn render(&self, encoder: &mut wgpu::CommandEncoder, view: &wgpu::TextureView, output: &egui::FullOutput);

    /// Is the editor currently visible/active?
    pub fn is_active(&self) -> bool;

    /// Toggle editor visibility (e.g., F12)
    pub fn toggle(&mut self);
}
```

This design means:
- The game's main loop stays in control
- The editor is an overlay rendered on top of the game viewport
- Games opt-in to the editor by implementing `EditorHost` and calling `EditorApp::update()`
- Production builds can simply not depend on `rust4d_editor`

### 1.4 Minimum Viable Editor (MVE) Features

Based on B2's recommendation and the boomer shooter's needs:

**Wave 1: Core Framework (2 sessions)**
1. egui integration with wgpu (egui-wgpu + egui-winit setup)
2. EditorHost trait and integration
3. Toggle on/off with F12
4. Basic panel layout with egui_dock

**Wave 2: Entity Editing (2 sessions)**
5. Entity list panel (query all entities with Name component, show tree via Parent/Children)
6. Property inspector (Transform4D: position, rotation display; Material: color picker; ShapeRef: shape type display)
7. Entity selection (click in list, highlight in viewport -- requires a selection component or side-state)
8. Add/remove entity (spawn with default components)

**Wave 3: W-Slice Navigation (1-2 sessions)**
9. W-slice slider widget (drag to change slice_w)
10. W-position indicator showing camera W position and slice offset
11. Multi-slice thumbnail strip (render the scene at W-2, W-1, W, W+1, W+2 as small previews)
    - This is the killer feature for 4D level design. It shows what's "nearby" in W.
    - Implementation: render to off-screen textures at different slice_w values, display as egui images.

**Wave 4: Scene Operations (1 session)**
12. Save scene to RON
13. Load scene from RON
14. New scene
15. Undo/redo (basic command pattern -- at least for transform changes)

**Total: 6-8 sessions** (matches synthesis estimate)

### 1.5 What the Game Builds On Top

The game repo can:
- Use the editor during development by enabling `rust4d_editor` as a dev-dependency
- Build a custom editor tool that extends the engine editor (add panels for weapon config, enemy AI, etc.)
- Ship without the editor entirely (no egui in release builds)

The game repo owns:
- Pause menu (using egui or a separate UI system -- this is game UI, not editor UI)
- In-game HUD (already covered by Phase 2 of the roadmap)
- Game-specific property inspectors (Health, Weapon, AIState components)

### 1.6 egui Dependencies

```toml
[dependencies]
egui = "0.31"                   # Core egui library
egui-wgpu = "0.31"              # wgpu rendering backend
egui-winit = "0.31"             # winit event integration
egui_dock = "0.14"              # Dockable panel layout
rust4d_core.workspace = true
rust4d_render.workspace = true
rust4d_math.workspace = true
```

These are well-maintained, widely-used crates. egui-wgpu works directly with the wgpu version Rust4D already uses (wgpu 24).

---

## 2. Rendering Additions: Point Lights & Shadows

### 2.1 Current Lighting State

The current pipeline has a single directional light defined in `RenderUniforms`:

```rust
// types.rs (RenderUniforms)
pub light_dir: [f32; 3],        // Normalized direction
pub ambient_strength: f32,       // 0.3 default
pub diffuse_strength: f32,       // 0.7 default
```

The fragment shader (`render.wgsl`) computes simple Lambert diffuse:
```wgsl
let n_dot_l = max(dot(normal, light_dir), 0.0);
let diffuse = n_dot_l * uniforms.diffuse_strength;
let light = uniforms.ambient_strength + diffuse;
```

There is no per-light data, no attenuation, no specular, no shadows.

### 2.2 Point Light Design

**Engine component** (in `rust4d_core`):
```rust
/// A point light source in 4D space
pub struct PointLight4D {
    pub color: [f32; 3],
    pub intensity: f32,
    pub range: f32,          // Attenuation radius
    pub w_range: f32,        // How far in W the light reaches (4D-specific)
}
```

Entities with `(Transform4D, PointLight4D)` components are lights. The renderer queries for these.

**GPU light data** (in `rust4d_render`):
```rust
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct GpuPointLight {
    pub position: [f32; 3],  // 3D position after slicing (derived from 4D position)
    pub range: f32,
    pub color: [f32; 3],
    pub intensity: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct LightUniforms {
    pub point_light_count: u32,
    pub _pad: [u32; 3],
    pub point_lights: [GpuPointLight; MAX_POINT_LIGHTS],  // MAX_POINT_LIGHTS = 16 or 32
}
```

### 2.3 4D-Specific Lighting Considerations

This is where it gets interesting. Lights exist in 4D space, but the rendered scene is a 3D cross-section. A light at W=5.0 when the slice is at W=0.0 is "behind" the slice in the 4th dimension. How should this work?

**Approach: W-distance attenuation**

1. Each light has a 4D position (Transform4D).
2. At render time, compute the W-distance from the light to the current slice plane.
3. If |light_w - slice_w| > w_range, the light is invisible (culled).
4. Otherwise, attenuate by W-distance: `w_factor = 1.0 - (|light_w - slice_w| / w_range)`.
5. The light's 3D position is its XYZ coordinates (projected into the slice).
6. Standard 3D point light attenuation applies, multiplied by w_factor.

This creates the intuition that lights "bleed through" nearby W-slices, getting dimmer with W-distance. A torch at W=0.2 when viewing W=0.0 would appear slightly dimmed, while a torch at W=3.0 would be invisible.

### 2.4 Shader Changes

The render shader needs a new bind group for lights:

```wgsl
struct PointLight {
    position: vec3<f32>,
    range: f32,
    color: vec3<f32>,
    intensity: f32,
}

struct LightUniforms {
    point_light_count: u32,
    _pad: vec3<u32>,
    point_lights: array<PointLight, 32>,  // Fixed-size array
}

@group(1) @binding(0) var<uniform> lights: LightUniforms;
```

The fragment shader changes from:
```wgsl
let light = uniforms.ambient_strength + diffuse;
```
To:
```wgsl
var total_light = uniforms.ambient_strength;

// Directional light (existing)
let dir_n_dot_l = max(dot(normal, light_dir), 0.0);
total_light += dir_n_dot_l * uniforms.diffuse_strength;

// Point lights
for (var i = 0u; i < lights.point_light_count; i++) {
    let light = lights.point_lights[i];
    let to_light = light.position - input.world_position;
    let distance = length(to_light);
    let direction = to_light / max(distance, 0.001);

    // Distance attenuation (inverse square with range clamp)
    let attenuation = max(1.0 - (distance / light.range), 0.0);
    let att_sq = attenuation * attenuation;

    // Diffuse contribution
    let n_dot_l = max(dot(normal, direction), 0.0);
    total_light += n_dot_l * light.intensity * att_sq * light.color;
}
```

### 2.5 Pipeline Changes Required

1. **New bind group (group 1)**: Add a `LightUniforms` buffer and bind group to `RenderPipeline`.
2. **Update bind group layout**: The render pipeline currently has one bind group (group 0 for RenderUniforms). Add group 1 for lights.
3. **Light collection system**: Before rendering, query ECS for `(Transform4D, PointLight4D)` entities, compute their 3D positions (XYZ projection + W-attenuation), fill `GpuPointLight` array.
4. **Upload light buffer**: Each frame, update the light uniform buffer.

**This is a contained change to `render_pipeline.rs` and `render.wgsl`.** It does not affect the compute pipeline (slice_tetra.wgsl) at all. The slice pipeline produces geometry; the render pipeline lights it.

### 2.6 Basic Shadows

Shadows for a boomer shooter in 4D are optional polish. The B1 report estimated 1-2 sessions. Here is the design:

**Approach: Single directional shadow map (traditional)**

1. Create a shadow pass render pipeline (depth-only, from light's perspective).
2. Render the sliced geometry (reuse the same vertex buffer from compute pass) from the directional light's perspective.
3. Store the depth map in a texture.
4. In the main render pass, sample the shadow map to determine if each fragment is in shadow.

**4D complication: The shadow map is for the 3D slice.** Since all rendered geometry is already in 3D (the compute shader sliced it), shadow mapping works identically to standard 3D shadow mapping. No 4D adaptation needed for shadows -- the slicing already happened.

**Implementation:**
- Add a `ShadowPipeline` struct (depth-only render pipeline variant).
- Add shadow map texture to `RenderPipeline`.
- Add shadow sampling to `render.wgsl`.
- The existing `ensure_depth_texture` pattern in `render_pipeline.rs` shows exactly how to manage additional render targets.

**Session estimate: 1-2 sessions** (matches synthesis). The key insight is that shadows operate on the already-sliced 3D geometry, so no 4D-specific shadow math is needed.

---

## 3. Texture Support

### 3.1 Why Textures in 4D Are Hard

This is the most technically challenging item in Phase 5. Here is the problem:

**The current pipeline:**
```
4D vertices (Vertex4D: position + color)
  -> Compute shader: slice tetrahedra at W=slice_w
  -> 3D vertices (Vertex3D: position + normal + color + w_depth)
  -> Render shader: Lambert lighting + W-depth coloring
```

Vertex4D has `position: [f32; 4]` and `color: [f32; 4]`. There are **no UV coordinates** anywhere in the pipeline. Adding textures requires:

1. UV coordinates on the 4D vertices
2. UV interpolation during tetrahedra slicing (in the compute shader)
3. Texture sampling in the fragment shader

### 3.2 The UV Problem in 4D

In 3D, a triangle has 2D UV coordinates (u, v) per vertex. In 4D, a tetrahedron has vertices in 4D space. What are the "UV coordinates" of a 4D vertex?

**Option A: 4D UV coordinates (UVW mapping)**
Each 4D vertex has a 3D texture coordinate (u, v, s). When the tetrahedron is sliced, the 3D texture coordinate is interpolated just like position and color. The resulting 3D slice vertex has a 2D texture coordinate (from the 3D UVW interpolation). This requires 3D textures.

**Option B: 2D UV on 4D vertices (projected mapping)**
Each 4D vertex has a 2D UV coordinate. When slicing interpolates between vertices, it also interpolates UVs. The resulting 3D vertices have 2D UVs that can be used with standard 2D textures. This is simpler but the UV mapping on the 4D geometry may be non-intuitive.

**Option C: Triplanar mapping on the 3D slice (runtime projection)**
Ignore UVs entirely. After slicing, compute UVs in the fragment shader based on the 3D world position (triplanar projection). This is how many voxel games handle texturing and would work naturally with the slicing pipeline.

### 3.3 Recommended Approach: Triplanar Mapping (Option C) First

For a boomer shooter, triplanar mapping is the pragmatic choice:

1. **No pipeline changes for UVs.** The compute shader is untouched.
2. **No UV authoring problem.** You do not need to UV-unwrap 4D tetrahedra (an unsolved UX problem).
3. **Works immediately with all shapes.** Tesseracts, hyperplanes, any future shape.
4. **Looks good for architectural geometry.** Walls, floors, ceilings all get natural-looking textures.
5. **Used by many games.** Triplanar is proven for procedural and BSP-style geometry.

**Implementation in render.wgsl:**
```wgsl
@group(1) @binding(1) var texture_sampler: sampler;
@group(1) @binding(2) var base_texture: texture_2d<f32>;

fn triplanar_sample(world_pos: vec3<f32>, normal: vec3<f32>, scale: f32) -> vec4<f32> {
    let blend = abs(normal);
    let blend_norm = blend / (blend.x + blend.y + blend.z);

    let uv_x = world_pos.yz * scale;
    let uv_y = world_pos.xz * scale;
    let uv_z = world_pos.xy * scale;

    let col_x = textureSample(base_texture, texture_sampler, uv_x);
    let col_y = textureSample(base_texture, texture_sampler, uv_y);
    let col_z = textureSample(base_texture, texture_sampler, uv_z);

    return col_x * blend_norm.x + col_y * blend_norm.y + col_z * blend_norm.z;
}
```

**Phase 2 (future): Add UV path through pipeline**
If specific UV-mapped textures are needed later (e.g., for sprite billboards or specific surface decoration), add a `uv: [f32; 2]` field to Vertex4D and Vertex3D, and interpolate in the compute shader. This is a larger change (modifies both shaders and all GPU types) but can be deferred.

### 3.4 Texture Loading

The engine needs a texture loading system:

```rust
// In rust4d_render or rust4d_core
pub struct TextureHandle(u32);  // Index into texture array

pub struct TextureManager {
    textures: Vec<wgpu::Texture>,
    views: Vec<wgpu::TextureView>,
    // Use the existing AssetCache for actual loading
}
```

For a boomer shooter, the texture requirements are modest:
- Wall textures (tileable)
- Floor/ceiling textures
- Sprite textures (for enemies -- already billboarded, separate from triplanar)
- UI textures (HUD elements)

**Dependency**: The `image` crate for loading PNG/JPG. This is the standard Rust image loading crate and adds minimal dependency weight.

### 3.5 Session Estimate for Textures

The original synthesis estimated 1 session. This is too low for the full picture:

- **Triplanar mapping + texture loading: 1 session** (shader changes + TextureManager + image loading)
- **Per-material texture assignment: 0.5 session** (Material component gets a TextureHandle, pipeline supports per-draw-call texture binding)
- **UV path through pipeline (optional, deferred): 1-2 sessions** (modify Vertex4D, Vertex3D, compute shader, render shader)

**Revised: 1.5 sessions minimum, 2.5 if UV path is included.**

---

## 4. Input Rebinding

### 4.1 What the Split Plan Already Covers

The engine/game split plan (Phase 2) already specifies:
- `InputAction` enum for abstract actions (MoveForward, MoveRight, Jump, Look, RotateW, etc.)
- `InputMap` struct mapping physical inputs to actions
- `default_input_map()` convenience function
- `CameraController` refactored to work with abstract actions

This is the foundation. Input rebinding is building ON TOP of this.

### 4.2 What Remains for Rebinding (Engine Side)

The engine needs to provide:

```rust
// In rust4d_input

impl InputMap {
    /// Get the current binding for an action
    pub fn binding_for(&self, action: InputAction) -> Option<&PhysicalInput>;

    /// Set a new binding for an action
    pub fn rebind(&mut self, action: InputAction, input: PhysicalInput);

    /// Remove a binding
    pub fn unbind(&mut self, action: InputAction);

    /// Check for binding conflicts (two actions on same key)
    pub fn conflicts(&self) -> Vec<(InputAction, InputAction, PhysicalInput)>;

    /// Serialize to TOML for persistence
    pub fn to_toml(&self) -> String;

    /// Deserialize from TOML
    pub fn from_toml(s: &str) -> Result<Self, Error>;

    /// Reset to defaults
    pub fn reset_defaults(&mut self);
}

/// Physical input that can be bound to an action
pub enum PhysicalInput {
    Key(KeyCode),
    MouseButton(MouseButton),
    MouseAxis(MouseAxisType),  // DeltaX, DeltaY, ScrollY
    // GamepadButton, GamepadAxis (future)
}
```

### 4.3 What the Game Builds

The game repo provides:
- **Rebinding UI**: A settings screen where the player clicks an action, then presses the key they want. This is game UI, not engine.
- **Persistence**: Save/load the InputMap to a config file. The engine provides `to_toml()`/`from_toml()`, the game decides where the file goes.
- **Pause menu**: Game-owned overlay that shows when Escape is pressed. Uses the engine's scene stack or a simple state flag.

### 4.4 Session Estimate

The synthesis estimated 1 session for "input rebinding + pause menu." Breaking that down:

- **InputMap rebinding API + TOML persistence: 0.5 session** (engine)
- **Pause menu + rebinding UI: 0.5 session** (game)

This is accurate. The heavy lifting (action/axis abstraction, CameraController refactor) is already done in the split plan.

---

## 5. Dependencies on Previous Phases

Phase 5 has the deepest dependency chain of any phase. Here is the complete picture:

### 5.1 Hard Dependencies (must be complete)

| Dependency | Phase | What P5 Needs It For |
|------------|-------|---------------------|
| ECS migration (hecs) | Split Plan Phase 1 | Editor queries ECS world, lights are components |
| Serialization (Rotor4 Serialize/Deserialize) | Foundation | Editor save/load, scene round-tripping |
| rust4d_game crate exists | Split Plan Phase 2 | Editor is separate from game framework |
| Input action/axis abstraction | Split Plan Phase 2 | Input rebinding builds on InputMap |
| All shape types (P4) | Phase 4 | Editor must display/create all shapes |
| Event system (P1) | Phase 1 | Editor undo/redo uses events |
| Scene save/load works with ECS | Split Plan Phase 3 | Editor save/load |

### 5.2 Soft Dependencies (helpful but not blocking)

| Dependency | Phase | Why Helpful |
|------------|-------|-------------|
| Audio system (P2) | Phase 2 | Editor could preview audio, but not MVP |
| Sprite billboard rendering (P3) | Phase 3 | Editor could show sprite previews, but not MVP |
| Raycasting (P1) | Phase 1 | Viewport entity picking (click to select). Can use bounding box checks as fallback. |
| HUD system (P2) | Phase 2 | Editor overlays use egui, not HUD system. Separate. |

### 5.3 Dependency Graph

```
Split Plan (ECS + rust4d_game + Input refactor + Scene pluggable)
  -> Foundation (serialization, fixed timestep)
    -> Phase 1 (raycasting, events)
      -> Phase 2 (audio, HUD, particles)
        -> Phase 3 (sprites, AI)
          -> Phase 4 (level tools, shapes)
            -> Phase 5 (EDITOR, lights, textures, rebinding)  <-- HERE
```

**Critical path**: Every previous phase is a predecessor. This is why the synthesis correctly placed the editor at P5. It is the integration point for the entire engine.

### 5.4 What Can Be Parallelized Within Phase 5

Despite the deep external dependency chain, the four sub-features of Phase 5 are largely independent of each other:

```
Phase 5 Wave 1 (Parallel - can all start simultaneously)
  Agent A: Point lights + shadows (render pipeline changes)
  Agent B: Texture support (shader + texture loading)
  Agent C: Input rebinding API (rust4d_input changes)

Phase 5 Wave 2 (Sequential - needs Wave 1 render changes)
  Agent D: Editor framework + entity editing + W-slice nav + scene operations
           (needs lights working for viewport to look decent)
```

The editor (Wave 2) benefits from lights and textures being available, but could technically start in parallel and integrate them later. The three rendering/input features (Wave 1) have no dependencies on each other.

---

## 6. Complete Implementation Plan

### Task Breakdown

#### Task 5.1: Point Lights (1 session)
- Add `PointLight4D` component to `rust4d_core`
- Add `GpuPointLight` and `LightUniforms` to `rust4d_render::pipeline::types`
- Add light buffer and bind group to `RenderPipeline`
- Implement W-distance attenuation for light collection
- Update `render.wgsl` fragment shader with point light loop
- Add light collection query (ECS: Transform4D + PointLight4D)
- Tests: light attenuation math, W-culling, light buffer upload
- Update examples to demonstrate point lights

#### Task 5.2: Basic Shadows (1 session)
- Create `ShadowPipeline` (depth-only render from light perspective)
- Create shadow map texture (2048x2048 depth texture)
- Add shadow map sampling to `render.wgsl`
- Add shadow bias to prevent shadow acne
- PCF (percentage closer filtering) for soft shadows
- Tests: shadow map creation, bias values
- Only for the directional light (point light shadows are much harder and not needed for boomer shooter)

#### Task 5.3: Texture Support -- Triplanar (1 session)
- Add `image` dependency to `rust4d_render`
- Create `TextureManager` for loading and managing GPU textures
- Add texture sampler and base_texture to render bind group
- Implement triplanar mapping in `render.wgsl`
- Add `texture: Option<TextureHandle>` to Material component
- Per-material texture selection (materials without textures use vertex color)
- Tests: texture loading, triplanar UV computation

#### Task 5.4: Texture Support -- Per-Material + UV Path (0.5-1 session, deferrable)
- Add `uv: [f32; 2]` to Vertex4D (32 -> 40 bytes) and Vertex3D
- Update compute shader to interpolate UVs during slicing
- Support standard UV-mapped textures alongside triplanar
- This is only needed if triplanar proves insufficient for the game's needs

#### Task 5.5: Input Rebinding API (0.5 session)
- Add `rebind()`, `unbind()`, `conflicts()` to InputMap
- Add `to_toml()` / `from_toml()` for persistence
- Add `reset_defaults()`
- Tests: rebinding, conflict detection, serialization round-trip
- Documentation for game repo usage

#### Task 5.6: Editor Framework (2 sessions)
- Add `rust4d_editor` crate to workspace
- Set up egui + egui-wgpu + egui-winit integration
- Define `EditorHost` trait
- Create `EditorApp` with toggle on/off
- Set up dockable panel layout with egui_dock
- Implement basic panel framework (Panel trait, PanelManager)
- Tests: editor state management, panel registration

#### Task 5.7: Entity List + Property Inspector (2 sessions)
- Entity list panel: query ECS for all entities with Name component
- Show entity hierarchy via Parent/Children components
- Click to select entity (EditorState tracks selected entity)
- Property inspector: display and edit Transform4D (position Vec4, rotation Rotor4 as Euler-like)
- Property inspector: Material color picker (egui color edit)
- Property inspector: ShapeRef display (shape type, dimensions)
- Add/delete entity operations
- Undo/redo for property changes (basic command pattern)

#### Task 5.8: W-Slice Navigation (1-2 sessions)
- W-slice slider widget (egui slider bound to camera's slice_w)
- Current W-position display
- Multi-slice thumbnail strip:
  - Render the scene at 5 different W values to small off-screen textures
  - Display as egui image widgets in a horizontal strip
  - Click a thumbnail to jump to that W-slice
- This is the most 4D-specific editor feature and the most important for level design

#### Task 5.9: Scene Operations (1 session)
- Save current ECS world to RON scene file
- Load RON scene file into ECS world
- New scene (clear world)
- Scene tree panel (list loaded scenes, active scene)
- File dialog integration (rfd crate or simple text input)

### Session Summary

| Task | Sessions | Parallel Group |
|------|----------|----------------|
| 5.1 Point lights | 1 | Wave 1 |
| 5.2 Basic shadows | 1 | Wave 1 (after 5.1) |
| 5.3 Triplanar textures | 1 | Wave 1 |
| 5.4 UV path (deferrable) | 0.5-1 | Wave 1 (after 5.3) |
| 5.5 Input rebinding API | 0.5 | Wave 1 |
| 5.6 Editor framework | 2 | Wave 2 |
| 5.7 Entity list + inspector | 2 | Wave 2 (after 5.6) |
| 5.8 W-slice navigation | 1-2 | Wave 2 (after 5.6) |
| 5.9 Scene operations | 1 | Wave 2 (after 5.6) |
| **Total** | **10-12.5** | |

With maximum parallelism (Wave 1 and Wave 2 overlapping where possible):
- **Critical path: 8-10 sessions** (Wave 1: 2.5-3 sessions for lights+shadows+textures, Wave 2: 6-7 sessions for editor)
- **If Wave 1 and Wave 2 run in parallel agents: ~7-8 sessions wall time**

### Revised Estimate vs Synthesis

The synthesis estimated 6-10 sessions. My detailed breakdown yields 10-12.5 sessions total (8-10 critical path). The increase comes from:
1. Texture support being harder than "1 session" when you account for the 4D UV problem
2. W-slice navigation thumbnails requiring off-screen rendering (non-trivial)
3. Undo/redo adding complexity to the property inspector

However, if we defer Task 5.4 (UV path) and simplify 5.8 (W-slice nav without thumbnails), it fits in 8-9 sessions, closer to the upper bound of the synthesis estimate.

---

## 7. Risk Assessment

### 7.1 High Risk: Editor Scope Creep

The editor is the single largest feature. The temptation to add "just one more panel" is enormous. Mitigation:
- Define MVE (Minimum Viable Editor) clearly and stick to it
- Ship entity list + property inspector + W-slice slider first
- Multi-slice thumbnails and gizmos are Phase 2 of the editor, not Phase 1
- No spatial gizmos (transform handles in viewport) in MVE -- too complex for the benefit

### 7.2 Medium Risk: egui + wgpu Integration

egui-wgpu is well-maintained but version mismatches between egui, egui-wgpu, and wgpu can cause frustrating compile errors. Mitigation:
- Pin exact versions of egui ecosystem crates
- Test integration early (Task 5.6) before building features on top
- If egui-wgpu integration proves problematic, consider egui-glow as a fallback (OpenGL backend, less optimal but more stable)

### 7.3 Medium Risk: Texture UV Quality

Triplanar mapping produces visible seams at certain angles and can look blurry on surfaces not aligned with major axes. For a boomer shooter, this may be acceptable (Doom used stretched textures). Mitigation:
- Start with triplanar, evaluate quality
- If insufficient, Task 5.4 (UV path) is the fallback
- For sprite billboards (enemy rendering from P3), UVs are straightforward since sprites are flat quads

### 7.4 Low Risk: Point Light Performance

With 16-32 point lights computed per fragment in a forward renderer, performance could degrade in scenes with many lights. Mitigation:
- Cap at 16 lights initially (plenty for a boomer shooter)
- Sort lights by distance, only process the closest N
- W-distance culling naturally reduces visible light count
- If needed later, upgrade to clustered forward or deferred (but this is Phase 6 territory)

### 7.5 Low Risk: Shadow Map Quality

Single directional shadow map is basic but sufficient for a boomer shooter. 4D-specific risk: objects at different W-slices might cast shadows incorrectly if not handled. Mitigation:
- Shadows are computed on the already-sliced 3D geometry, so only visible objects cast shadows
- This is actually correct behavior -- an object not in your W-slice should not cast a shadow you can see

---

## 8. Summary: What the Engine Provides vs What the Game Provides

### Engine Provides (Phase 5)

| Feature | Crate | API |
|---------|-------|-----|
| Point lights with W-attenuation | `rust4d_core` + `rust4d_render` | `PointLight4D` component, automatic light collection |
| Directional shadow mapping | `rust4d_render` | Automatic, configurable bias + PCF |
| Triplanar texture mapping | `rust4d_render` | `TextureManager`, `Material.texture` field |
| Input rebinding API | `rust4d_input` | `InputMap::rebind()`, TOML persistence |
| Scene editor with entity editing | `rust4d_editor` | `EditorApp`, `EditorHost` trait |
| W-slice navigation widget | `rust4d_editor` | Integrated in editor UI |

### Game Provides (using engine APIs)

| Feature | Implementation |
|---------|---------------|
| Pause menu | Game UI using egui or custom system |
| Settings screen with key rebinding | Game UI calling `InputMap::rebind()` |
| Custom editor panels (weapon tuning, AI config) | Game-side panels implementing editor extension API |
| Game-specific textures | Loaded via `TextureManager`, assigned to Materials |
| Level-specific lighting | Placed as entities with `PointLight4D` components in scenes |

---

*Report completed by Agent P5.*
*Files referenced:*
- `/home/lemoneater/Projects/Rust4D/crates/rust4d_render/src/pipeline/render_pipeline.rs`
- `/home/lemoneater/Projects/Rust4D/crates/rust4d_render/src/pipeline/types.rs`
- `/home/lemoneater/Projects/Rust4D/crates/rust4d_render/src/shaders/render.wgsl`
- `/home/lemoneater/Projects/Rust4D/crates/rust4d_render/src/shaders/slice_tetra.wgsl`
- `/home/lemoneater/Projects/Rust4D/crates/rust4d_render/src/renderable.rs`
- `/home/lemoneater/Projects/Rust4D/crates/rust4d_render/src/pipeline/slice_pipeline.rs`
- `/home/lemoneater/Projects/Rust4D/crates/rust4d_render/src/context.rs`
- `/home/lemoneater/Projects/Rust4D/crates/rust4d_render/src/camera4d.rs`
- `/home/lemoneater/Projects/Rust4D/crates/rust4d_input/src/camera_controller.rs`
- `/home/lemoneater/Projects/Rust4D/crates/rust4d_input/src/lib.rs`
- `/home/lemoneater/Projects/Rust4D/Cargo.toml`
- `/home/lemoneater/Projects/Rust4D/scratchpad/plans/2026-01-30-engine-game-split.md`
- `/home/lemoneater/Projects/Rust4D/scratchpad/reports/2026-01-30-multi-engine-review/cross-swarm-synthesis.md`
- `/home/lemoneater/Projects/Rust4D/scratchpad/reports/2026-01-30-multi-engine-review/swarm-b-roadmap/b2-scripting-editor-networking.md`
- `/home/lemoneater/Projects/Rust4D/scratchpad/reports/2026-01-30-multi-engine-review/swarm-b-roadmap/b1-ecs-rendering.md`
- `/home/lemoneater/Projects/Rust4D/scratchpad/reports/2026-01-30-multi-engine-review/swarm-c-features/c1-engine-features.md`
