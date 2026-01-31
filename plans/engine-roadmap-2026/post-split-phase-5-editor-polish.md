# Post-Split Phase 5: Editor & Polish

**Status**: Planning Complete
**Source**: Agent P5 (Editor & Polish) report from 2026-01-30 engine roadmap swarm
**Revised Estimate**: 10.85-13.75 sessions total (minimal Lua scoping), up to 11.5-15 sessions (full-featured Lua editor tools). Critical path: 8-10 sessions.
**Prerequisite**: Engine/game split complete, ECS migration done, Phases 1-4 complete, `rust4d_scripting` crate operational

> **Updated 2026-01-31**: Integrated Lua scripting amendments. This document now includes all Lua binding work, editor script/console panels, and engine-vs-game boundary shifts from the Lua migration. Original Rust implementation details are fully preserved; Lua additions are clearly marked. This document is self-contained and supersedes the separate `lua-phase-amendments.md` for Phase 5.

---

## Overview

Phase 5 is the largest and most complex post-split phase in the Rust4D engine roadmap. It encompasses four major sub-systems: texture support, point lights with shadows, input rebinding, and a full egui-based editor framework. The editor has the deepest dependency chain of any feature in the engine -- it is the integration point where ECS, serialization, all shape types, and the working renderer converge into a development tool.

With the Lua migration, Phase 5 also becomes the point where the editor gains **script-aware development tools**: a script error panel, a Lua console for runtime inspection, and Lua bindings for textures, lighting, and input rebinding. These additions are modest in scope but critical for the Lua-driven game workflow.

Agent P5's deep analysis of the rendering pipeline, input system, and 4D slicing architecture revealed several findings that revised the original cross-swarm synthesis estimate upward from 6-10 sessions to 10-12.5 sessions (8-10 on the critical path):

1. **Texture support in 4D is genuinely hard.** The compute shader produces interpolated `Vertex3D` from tetrahedra slicing, and there is no UV coordinate path through the pipeline. This requires careful design.
2. **The editor's dependency chain is the deepest of any feature.** It needs ECS, serialization, all shape types (P4), a working renderer, and egui (front-loaded from P2 HUD work).
3. **W-slice navigation thumbnails require off-screen rendering**, adding non-trivial complexity.
4. **Undo/redo for property editing** adds complexity to the property inspector.

The Lua amendments add +0.85-1.25 sessions (minimal scoping) to +1.35-2.85 sessions (full-featured), primarily from two new editor panels (script error display and Lua console) and thin Lua bindings for input, textures, and lighting.

Despite the deep external dependency chain, the four sub-features of Phase 5 are largely independent of each other internally, enabling significant parallelism.

---

## Engine vs Game Boundary

The editor is **100% engine**. It is a development tool for creating and inspecting 4D scenes, not gameplay UI. The game repo never depends on the editor crate.

| Component | Where | Why |
|-----------|-------|-----|
| egui integration + panels | `rust4d_editor` (new engine crate) | Generic scene editing tool |
| Entity list, property inspector | `rust4d_editor` | Works with any ECS components |
| W-slice navigation widget | `rust4d_editor` | 4D-specific visualization tool |
| 3D viewport | `rust4d_editor` (reuses `rust4d_render`) | Renders via existing pipeline |
| Scene save/load | `rust4d_editor` (uses `rust4d_core` scene API) | Generic engine capability |
| Script error panel + Lua console | `rust4d_editor` | Script-aware development tools (Lua) |
| Lua input/texture/lighting bindings | `rust4d_scripting` | Expose engine APIs to Lua scripts |
| Pause menu | **Game repo (Lua scripts)** | Game-specific UI, built with HUD API |
| Gameplay-specific editors (weapon tuning, enemy config) | **Not needed** -- game config is Lua data files, editable as text | Replaced by script editing + Lua console |

**Integration model**: The editor is opt-in via the `EditorHost` trait. Games toggle the editor overlay with F12. Production builds can simply not depend on `rust4d_editor`. The editor never takes over the game's event loop -- it provides a composable overlay rendered on top of the game viewport.

**Input rebinding**: 80% is already planned by the split plan's action/axis abstraction (`InputAction` enum, `InputMap` struct, `default_input_map()`). Phase 5 adds persistence (`to_toml()` / `from_toml()`) and runtime modification (`rebind()`, `unbind()`, `conflicts()`). With Lua, the game calls these through `input:bind()`, `input:save()`, etc. -- the engine API is the same, just Lua-accessible.

### Lua Boundary Shift: What Was Game-Side Rust Now Needs Lua Bindings or Editor Features

| Was (Rust game code) | Becomes (Engine Lua binding / Editor feature) |
|---------------------|----------------------------------------------|
| `InputMap::rebind()` called from Rust settings screen | `input:rebind(action, key)` from Lua settings script |
| `InputMap::to_toml()` / `from_toml()` called from Rust | `input:save(path)` / `input:load(path)` from Lua |
| `InputMap::conflicts()` checked in Rust | `input:conflicts()` returns table of conflicts to Lua |
| Game-specific editor panels in Rust | **Not needed** -- game config is Lua data files, editable as text |
| No script editing needed | **Editor needs script error panel** (new feature) |
| No runtime console needed | **Editor needs Lua console** (new feature) |
| Game sets material textures in Rust | `entity:set_material({ texture="stone_wall", ... })` from Lua |
| Game creates lights as ECS entities in Rust | `entity:add_light({ color={1,0.9,0.7}, intensity=2.0, ... })` from Lua |

### What Gets Simpler with Lua

- **Input rebinding UI**: In the Rust approach, the game needed to build a full settings screen in Rust (complex UI code). With Lua + the HUD API, a simple Lua script can build a rebinding screen. Even simpler: the game could use a TOML config file for bindings and not have an in-game UI at all (common for indie games).

- **Game-specific editor panels are unnecessary**: The Rust approach planned for games to extend the editor with custom panels (weapon tuning, AI config). With Lua, game configuration is data files (Lua tables or TOML). The editor's script editing panel + Lua console replaces the need for bespoke property editors. Tweaking enemy health is just editing a Lua file, not building a custom editor panel.

- **Pause menu**: Was planned as game-side Rust using egui. With Lua + HUD API, it is a simple Lua script.

### What Gets Removed or Reduced

- **Game-side editor extension API importance reduced**: The `EditorHost` trait's purpose of letting games add custom panels becomes less important. Games customize through Lua scripts, not Rust editor extensions. The `EditorHost` trait still exists for engine-level editor integration, but game-specific panels are unnecessary.

- **Complex input rebinding UI infrastructure**: The engine just provides the API (`input:rebind`, `input:save`, etc.). The game builds whatever UI it wants in Lua. No need for the engine to provide a polished rebinding widget.

---

## Sub-Phase A: Texture Support (1.5-2.5 sessions + 0.1 session Lua bindings)

### The Problem: No UV Path in the Pipeline

This is the most technically challenging item in Phase 5. The current pipeline:

```
4D vertices (Vertex4D: position + color)
  -> Compute shader: slice tetrahedra at W=slice_w
  -> 3D vertices (Vertex3D: position + normal + color + w_depth)
  -> Render shader: Lambert lighting + W-depth coloring
```

`Vertex4D` has `position: [f32; 4]` and `color: [f32; 4]`. There are **no UV coordinates** anywhere in the pipeline. Adding textures requires UV coordinates on 4D vertices, UV interpolation during tetrahedra slicing (in the compute shader), and texture sampling in the fragment shader.

### The UV Problem in 4D

Three approaches were analyzed:

**Option A: 4D UV coordinates (UVW mapping)**
Each 4D vertex has a 3D texture coordinate (u, v, s). When the tetrahedron is sliced, the 3D texture coordinate is interpolated. Requires 3D textures. Complex.

**Option B: 2D UV on 4D vertices (projected mapping)**
Each 4D vertex has a 2D UV coordinate. Slicing interpolates UVs. Simpler but UV mapping on 4D geometry is non-intuitive.

**Option C: Triplanar mapping on the 3D slice (runtime projection)**
Ignore UVs entirely. After slicing, compute UVs in the fragment shader based on the 3D world position. No pipeline changes needed.

### Recommended: Triplanar Mapping (Option C) First

For the boomer shooter target, triplanar mapping is the pragmatic choice:

1. **No pipeline changes for UVs.** The compute shader is untouched.
2. **No UV authoring problem.** No need to UV-unwrap 4D tetrahedra (an unsolved UX problem).
3. **Works immediately with all shapes.** Tesseracts, hyperplanes, any future shape.
4. **Looks good for architectural geometry.** Walls, floors, ceilings get natural-looking textures.
5. **Proven technique.** Used by many voxel and BSP-style games.

### Shader Implementation

Add to `render.wgsl`:

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

### Texture Loading System

```rust
// In rust4d_render or rust4d_core
pub struct TextureHandle(u32);  // Index into texture array

pub struct TextureManager {
    textures: Vec<wgpu::Texture>,
    views: Vec<wgpu::TextureView>,
    // Uses the existing AssetCache for actual loading
}
```

Dependency: The `image` crate for loading PNG/JPG (standard Rust image loading, minimal dependency weight).

### Per-Material Texture Assignment

The `Material` component gains a `texture: Option<TextureHandle>` field. Materials without textures continue to use vertex color. The render pipeline supports per-draw-call texture binding.

### Lua Texture/Material API (+0.1 session)

Thin Lua wrappers over the existing `TextureManager` and `Material` system:

- `textures:load(name, path)` -- load a texture by name (string key for Lua-friendly access)
- `entity:set_material({ texture="stone_wall", color={0.8, 0.8, 0.8, 1.0} })` -- set material on entity
- `entity:get_material()` -- returns material table with current texture name and color

This is minimal work because the `TextureManager` already handles loading and GPU resource management. The Lua layer is a thin name-to-handle mapping plus ECS component setters.

### Task Breakdown

| Task | Sessions | Notes |
|------|----------|-------|
| 5.3: Triplanar mapping + texture loading | 1 | Shader changes + TextureManager + image loading |
| 5.4: Per-material texture + UV path (deferrable) | 0.5-1.5 | Modify Vertex4D (32->40 bytes), Vertex3D, compute shader UV interpolation |
| 5.3L: Lua texture/material API | 0.1 | `textures:load()`, `entity:set_material()` wrappers |
| **Sub-Phase A Total** | **1.6-2.6** | UV path deferred if triplanar is sufficient |

### Risk: Texture Quality

Triplanar mapping produces visible seams at certain angles and can look blurry on surfaces not aligned with major axes. For a boomer shooter aesthetic (Doom used stretched textures), this is likely acceptable. If insufficient, Task 5.4 (UV path through pipeline) is the fallback. Sprite billboards (enemy rendering from P3) use straightforward UVs since sprites are flat quads.

---

## Sub-Phase B: Lighting System (2 sessions + 0.1 session Lua bindings)

### Current Lighting State

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

### Point Light Design

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

**GPU light data** (in `rust4d_render::pipeline::types`):

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

### 4D-Specific Lighting: W-Distance Attenuation

Lights exist in 4D space, but the rendered scene is a 3D cross-section. A light at W=5.0 when the slice is at W=0.0 is "behind" the slice in the 4th dimension.

**Approach**: W-distance attenuation creates the intuition that lights "bleed through" nearby W-slices, getting dimmer with W-distance:

1. Each light has a 4D position (`Transform4D`).
2. At render time, compute the W-distance from the light to the current slice plane.
3. If `|light_w - slice_w| > w_range`, the light is invisible (culled).
4. Otherwise, attenuate by W-distance: `w_factor = 1.0 - (|light_w - slice_w| / w_range)`.
5. The light's 3D position is its XYZ coordinates (projected into the slice).
6. Standard 3D point light attenuation applies, multiplied by `w_factor`.

A torch at W=0.2 when viewing W=0.0 would appear slightly dimmed, while a torch at W=3.0 would be invisible.

### Shader Changes for Point Lights

New bind group (group 1) for lights in `render.wgsl`:

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

Fragment shader changes from:

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

### Pipeline Changes Required

1. **New bind group (group 1)**: Add a `LightUniforms` buffer and bind group to `RenderPipeline`.
2. **Update bind group layout**: The render pipeline currently has one bind group (group 0 for `RenderUniforms`). Add group 1 for lights.
3. **Light collection system**: Before rendering, query ECS for `(Transform4D, PointLight4D)` entities, compute their 3D positions (XYZ projection + W-attenuation), fill `GpuPointLight` array.
4. **Upload light buffer**: Each frame, update the light uniform buffer.

This is a contained change to `render_pipeline.rs` and `render.wgsl`. It does not affect the compute pipeline (`slice_tetra.wgsl`) at all. The slice pipeline produces geometry; the render pipeline lights it.

### Basic Shadows (Directional Only)

**Approach**: Single directional shadow map (traditional).

1. Create a shadow pass render pipeline (depth-only, from light's perspective).
2. Render the sliced geometry (reuse the same vertex buffer from compute pass) from the directional light's perspective.
3. Store the depth map in a texture.
4. In the main render pass, sample the shadow map to determine if each fragment is in shadow.

**Critical 4D insight**: Shadows operate on the already-sliced 3D geometry. Since all rendered geometry is already in 3D (the compute shader sliced it), shadow mapping works identically to standard 3D shadow mapping. **No 4D shadow math needed** -- the slicing already happened. An object not in your W-slice does not cast a shadow you can see.

**Implementation details**:
- Add a `ShadowPipeline` struct (depth-only render pipeline variant)
- Create shadow map texture (2048x2048 depth texture)
- Add shadow map sampling to `render.wgsl`
- Add shadow bias to prevent shadow acne
- PCF (percentage closer filtering) for soft shadows
- The existing `ensure_depth_texture` pattern in `render_pipeline.rs` shows exactly how to manage additional render targets
- Only for the directional light (point light shadows are much harder and not needed for the boomer shooter)

### Lua Lighting API (+0.1 session)

Thin Lua wrappers over the `PointLight4D` ECS component:

- `entity:add_light({ color={1,0.9,0.7}, intensity=2.0, range=15, w_range=3 })` -- add point light component to an entity
- `entity:set_light({ intensity=3.0 })` -- modify light properties on an existing entity
- `entity:remove_light()` -- remove point light component

Since `PointLight4D` is already an ECS component, the Lua layer just needs to add/modify/remove it. This is standard ECS component manipulation that the `rust4d_scripting` crate's entity API likely already supports generically; the lighting API provides typed convenience wrappers.

### Task Breakdown

| Task | Sessions | Notes |
|------|----------|-------|
| 5.1: Point lights | 1 | Component, GPU types, shader, light collection, W-attenuation |
| 5.2: Basic shadows | 1 | ShadowPipeline, depth texture, shadow sampling, PCF |
| 5.1L: Lua lighting API | 0.1 | `entity:add_light()`, `entity:set_light()` wrappers |
| **Sub-Phase B Total** | **2.1** | |

### Risk: Point Light Performance

With 16-32 point lights computed per fragment in a forward renderer, performance could degrade in scenes with many lights. Mitigations:
- Cap at 16 lights initially (plenty for a boomer shooter)
- Sort lights by distance, only process the closest N
- W-distance culling naturally reduces visible light count
- If needed later, upgrade to clustered forward or deferred (but this is post-Phase 5 territory)

---

## Sub-Phase C: Input Rebinding (0.5 sessions + 0.15 session Lua bindings)

### Foundation Already Planned

The engine/game split plan (Phase 2) already specifies:
- `InputAction` enum for abstract actions (`MoveForward`, `MoveRight`, `Jump`, `Look`, `RotateW`, etc.)
- `InputMap` struct mapping physical inputs to actions
- `default_input_map()` convenience function
- `CameraController` refactored to work with abstract actions

This is the foundation. Input rebinding builds ON TOP of this.

### Engine-Side API Additions

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

### Game-Side Responsibilities (Now Lua)

With Lua scripting, the game repo provides these as **Lua scripts** rather than compiled Rust:
- **Rebinding UI**: A Lua script using the HUD API to build a settings screen where the player clicks an action, then presses the desired key. Much simpler than building this in Rust.
- **Persistence**: Lua calls `input:save()` / `input:load()` which internally use `to_toml()` / `from_toml()`. The game decides when to save/load.
- **Pause menu**: A Lua script using the HUD API -- trivially simple compared to the Rust approach.

### Lua Input Rebinding API (+0.15 session)

- `input:bind(action_name, key_name)` -- bind an action to a physical input
- `input:unbind(action_name)` -- remove a binding
- `input:conflicts()` -- returns table of conflicting bindings
- `input:save(path?)` -- save input map to TOML file
- `input:load(path?)` -- load input map from TOML file
- `input:reset()` -- reset to defaults
- `input:get_binding(action_name)` -- get current binding for an action
- `input:define_action("custom_action")` -- useful for game-specific actions beyond the engine defaults

These are thin wrappers over the existing `InputMap` methods. Action and key names are strings in Lua, mapped to the Rust enums internally.

### Task Breakdown

| Task | Sessions | Notes |
|------|----------|-------|
| 5.5: Input rebinding API + TOML persistence | 0.5 | `rebind()`, `unbind()`, `conflicts()`, `to_toml()`, `from_toml()`, `reset_defaults()` + tests |
| 5.5L: Lua input rebinding API | 0.15 | `input:bind()`, `input:save()`, `input:load()`, etc. |
| **Sub-Phase C Total** | **0.65** | |

---

## Sub-Phase D: Editor Framework (6-8 sessions)

### Crate Structure

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
      script_panel.rs   # Script error display (Lua integration)
      lua_console.rs    # Lua REPL console (Lua integration)
    state.rs            # Editor state (selected entity, gizmo mode, etc.)
    integration.rs      # EditorIntegration trait -- how the editor plugs into a game's main loop
```

### Dependencies

```toml
[dependencies]
egui = "0.31"                   # Core egui library
egui-wgpu = "0.31"              # wgpu rendering backend
egui-winit = "0.31"             # winit event integration
egui_dock = "0.14"              # Dockable panel layout
rust4d_core.workspace = true
rust4d_render.workspace = true
rust4d_math.workspace = true
rust4d_scripting.workspace = true  # For Lua console and script error reporting
```

These are well-maintained, widely-used crates. egui-wgpu works directly with the wgpu version Rust4D already uses (wgpu 24).

**Note**: The egui dependency is front-loaded by Phase 2's HUD system, which adds an `OverlayRenderer` in `rust4d_render` using egui-wgpu. Phase 5 builds on that existing integration.

### EditorHost Trait Design

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

    /// Provide the Lua runtime for console and script error reporting (Lua integration)
    fn lua_runtime(&self) -> Option<&LuaRuntime>;
    fn lua_runtime_mut(&mut self) -> Option<&mut LuaRuntime>;
}
```

Note: The `lua_runtime()` methods return `Option` so hosts without Lua (pure engine tests, minimal examples) can return `None` and the editor gracefully hides the script/console panels.

### EditorApp API

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
    pub fn render(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        output: &egui::FullOutput,
    );

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

### Render Pass Ordering (Cross-Phase Coordination)

Confirmed ordering with P2 (HUD) and P3 (sprites):

1. 4D slice compute pass (geometry)
2. 3D cross-section render pass (main scene)
3. Sprites/billboards (P3)
4. Particles (P2/P3)
5. HUD overlay (P2 egui overlay)
6. **egui editor overlay (P5 -- rendered last)**

The editor is always the final render pass so it draws on top of everything.

### Minimum Viable Editor (MVE) Features

Based on the boomer shooter's needs:

**Wave 1: Core Framework (2 sessions)**
1. egui integration with wgpu (egui-wgpu + egui-winit setup)
2. `EditorHost` trait and integration
3. Toggle on/off with F12
4. Basic panel layout with `egui_dock`

**Wave 2: Entity Editing (2 sessions)**
5. Entity list panel (query all entities with `Name` component, show tree via `Parent`/`Children`)
6. Property inspector (`Transform4D`: position, rotation display; `Material`: color picker; `ShapeRef`: shape type display)
7. Entity selection (click in list, highlight in viewport -- requires a selection component or side-state)
8. Add/remove entity (spawn with default components)

**Wave 3: W-Slice Navigation (1-2 sessions)**
9. W-slice slider widget (drag to change `slice_w`)
10. W-position indicator showing camera W position and slice offset
11. **Multi-slice thumbnail strip** -- the killer feature for 4D level design:
    - Render the scene at W-2, W-1, W, W+1, W+2 as small previews
    - Shows what's "nearby" in W
    - Implementation: render to off-screen textures at different `slice_w` values, display as egui images
    - Click a thumbnail to jump to that W-slice

**Wave 4: Scene Operations (1 session)**
12. Save scene to RON
13. Load scene from RON
14. New scene
15. Undo/redo (basic command pattern -- at least for transform changes)

**Wave 5: Lua Development Tools (0.5-2.5 sessions, see below)**
16. Script error panel
17. Lua console panel

### Editor: Script Error / Editing Panel (0.25-1.5 sessions)

This is a new editor panel driven by the Lua migration. There are two scoping options:

**Minimal approach (recommended for MVE): Error log panel (~0.25 session)**
- Displays Lua script errors with file name and line number
- Errors reported by the `rust4d_scripting` crate's error handler
- Clickable errors (could copy file:line to clipboard for external editor)
- Clears on successful hot-reload
- Relies on external editors (VS Code with Lua plugin) for actual script editing

**Full-featured approach (Phase 6 territory): Script editing panel (~1.0-1.5 sessions)**
- Text editor panel in the editor for viewing and editing Lua scripts
- Syntax highlighting for Lua (egui text editor with highlighting, or integrate a simple highlighter)
- File tree showing the game's script directory
- Save button (writes to disk)
- Error display: when a Lua script has an error, show the error message with line number in the panel
- Hot-reload button: trigger script reload from the editor (or automatic on save)

**Recommendation**: Start with the error log panel for MVE. The full script editor is a nice-to-have that can come later -- most developers will use VS Code or their preferred editor for Lua scripts anyway.

### Editor: Lua Console Panel (0.25-1.0 session)

REPL-style console in the editor for runtime Lua evaluation. This is extremely valuable for debugging and iteration -- modify game state at runtime without editing files.

**Minimal approach (recommended for MVE): Basic console (~0.25 session)**
- Text input field + scrollable output log
- `eval(lua_string)` execution against the running Lua state
- Print results and errors to the output log
- Command history (up/down arrow)

**Full-featured approach (Phase 6 territory): Rich console (~0.5-1.0 session)**
- Auto-complete for common API names (e.g., `world:`, `audio:`, `input:`)
- Syntax highlighting in the input field
- Persistent command history across sessions
- Helper commands: `:entities` (list all), `:inspect <id>` (show components), etc.

**Recommendation**: Start with the basic console. Even without auto-complete, a Lua REPL in the editor is immensely useful for debugging. Type `world:query_sphere(player_pos, 10)` and immediately see nearby entities.

### RON Preview Tool as Foundation

Agent P4 (Level Design Pipeline) noted that the RON preview tool (`examples/ron_preview.rs`, their Wave 4) could serve as the foundation for the editor's viewport. Sharing camera/render code avoids duplication. The editor viewport wraps the existing render pipeline.

### What the Game Builds On Top

With Lua scripting, the game repo:
- Uses the editor during development by enabling `rust4d_editor` as a dev-dependency
- Edits game configuration (enemy stats, weapon balance, etc.) directly as Lua data files -- no custom editor panels needed
- Uses the Lua console for runtime debugging and iteration
- Ships without the editor entirely (no egui in release builds)

The game repo owns (as Lua scripts):
- Pause menu (using the HUD API -- simple Lua script)
- In-game HUD (using the HUD API from Phase 2)
- Game logic (AI, weapons, pickups, doors -- all Lua)

### Task Breakdown

| Task | Sessions | Notes |
|------|----------|-------|
| 5.6: Editor framework | 2 | Crate setup, egui integration, EditorHost trait, toggle, panel layout |
| 5.7: Entity list + property inspector | 2 | Entity list, hierarchy, selection, Transform4D/Material/ShapeRef editing, add/delete, undo/redo |
| 5.8: W-slice navigation | 1-2 | Slider, W-position display, multi-slice thumbnails (off-screen rendering) |
| 5.9: Scene operations | 1 | RON save/load, new scene, scene tree panel, file dialog |
| 5.10: Script error panel (minimal) | 0.25 | Error log display with file:line, clear on reload |
| 5.11: Lua console panel (minimal) | 0.25 | Text input + output log, eval execution, command history |
| **Sub-Phase D Total** | **6.5-8.5** | Minimal Lua editor tools; full-featured adds +1.0-2.0 |

---

## Complete Session Estimates

| Task | Sessions | Parallel Group | Description |
|------|----------|----------------|-------------|
| 5.1 Point lights | 1 | Wave 1 | PointLight4D component, GPU types, shader, W-attenuation |
| 5.2 Basic shadows | 1 | Wave 1 (after 5.1) | ShadowPipeline, depth texture, PCF |
| 5.3 Triplanar textures | 1 | Wave 1 | TextureManager, triplanar shader, image loading |
| 5.4 UV path (deferrable) | 0.5-1 | Wave 1 (after 5.3) | Vertex4D/Vertex3D UV fields, compute shader changes |
| 5.5 Input rebinding API | 0.5 | Wave 1 | rebind(), TOML persistence, conflict detection |
| 5.1L Lua lighting API | 0.1 | Wave 1 (after 5.1) | `entity:add_light()`, `entity:set_light()` |
| 5.3L Lua texture/material API | 0.1 | Wave 1 (after 5.3) | `textures:load()`, `entity:set_material()` |
| 5.5L Lua input rebinding API | 0.15 | Wave 1 (after 5.5) | `input:bind()`, `input:save()`, `input:load()` |
| 5.6 Editor framework | 2 | Wave 2 | Crate, egui integration, EditorHost, panel layout |
| 5.7 Entity list + inspector | 2 | Wave 2 (after 5.6) | Entity editing, hierarchy, selection, undo/redo |
| 5.8 W-slice navigation | 1-2 | Wave 2 (after 5.6) | Slider, thumbnails with off-screen rendering |
| 5.9 Scene operations | 1 | Wave 2 (after 5.6) | RON save/load, new scene, scene tree |
| 5.10 Script error panel | 0.25 | Wave 2 (after 5.6) | Error log with file:line display |
| 5.11 Lua console panel | 0.25 | Wave 2 (after 5.6) | Basic REPL: text input + output log |
| **Total (minimal Lua)** | **10.85-13.35** | | |
| **Total (full Lua editor)** | **11.85-15.35** | | +1.0-2.0 for full script editor + rich console |

### Critical Path Analysis

With maximum parallelism (Wave 1 and Wave 2 overlapping where possible):
- **Wave 1 critical path**: 2.5-3.1 sessions (lights + shadows + textures + Lua bindings, all in parallel with input rebinding)
- **Wave 2 critical path**: 6.5-7.5 sessions (editor framework, then entity editing + W-nav + scene ops + Lua panels)
- **Total critical path**: 8.5-10.5 sessions
- **If Wave 1 and Wave 2 run in parallel agents**: ~7-8.5 sessions wall time

### Comparison to Original Estimates

| Version | Total Sessions | Critical Path |
|---------|---------------|---------------|
| Original cross-swarm synthesis | 6-10 | -- |
| Pre-Lua detailed breakdown | 10-12.5 | 8-10 |
| **Post-Lua (minimal scoping)** | **10.85-13.75** | **8.5-10.5** |
| Post-Lua (full-featured) | 11.85-15.35 | 9-11 |

The Lua additions (minimal) add +0.85-1.25 sessions over the pre-Lua detailed breakdown. The increase comes from:
1. Three thin Lua binding tasks (+0.35 total): lighting, textures, input
2. Script error panel (+0.25): display Lua errors in the editor
3. Lua console panel (+0.25): basic REPL for runtime debugging

If Task 5.4 (UV path) is deferred, Task 5.8 (W-slice nav) is simplified (without thumbnails), and Lua editor panels are kept minimal, the total fits in 9-10 sessions.

---

## Dependencies on Previous Phases

Phase 5 has the deepest dependency chain of any phase:

```
Split Plan (ECS + rust4d_game + Input refactor + Scene pluggable)
  -> Foundation (serialization, fixed timestep)
    -> Scripting Phase (rust4d_scripting crate with mlua, hot-reload)
      -> Phase 1 (raycasting, events)
        -> Phase 2 (audio, HUD, particles)
          -> Phase 3 (sprites, AI)
            -> Phase 4 (level tools, shapes)
              -> Phase 5 (EDITOR, lights, textures, rebinding)  <-- HERE
```

### Hard Dependencies (must be complete)

| Dependency | Phase | What P5 Needs It For |
|------------|-------|---------------------|
| ECS migration (hecs) | Split Plan Phase 1 | Editor queries ECS world, lights are components |
| Serialization (Rotor4 Serialize/Deserialize) | Foundation | Editor save/load, scene round-tripping |
| rust4d_game crate exists | Split Plan Phase 2 | Editor is separate from game framework |
| Input action/axis abstraction | Split Plan Phase 2 | Input rebinding builds on InputMap |
| `rust4d_scripting` crate operational | Scripting Phase | Lua console, script error reporting, all Lua bindings |
| All shape types (P4) | Phase 4 | Editor must display/create all shapes |
| Event system (P1) | Phase 1 | Editor undo/redo uses events |
| Scene save/load works with ECS | Split Plan Phase 3 | Editor save/load |
| egui-wgpu overlay (P2 HUD) | Phase 2 | Front-loads the egui dependency editor needs |

### Soft Dependencies (helpful but not blocking)

| Dependency | Phase | Why Helpful |
|------------|-------|-------------|
| Audio system (P2) | Phase 2 | Editor could preview audio, but not MVE |
| Sprite billboard rendering (P3) | Phase 3 | Editor could show sprite previews, but not MVE |
| Raycasting (P1) | Phase 1 | Viewport entity picking (click to select). Can use bounding box checks as fallback. |
| HUD system (P2) | Phase 2 | Editor overlays use egui, not HUD system. Separate. |
| P1-P4 Lua bindings | Phases 1-4 | Lua console is more useful when all APIs are bound, but console works with whatever bindings exist |

### Cross-Phase Coordination Notes

From the hive-mind file:
- **Agent F (Foundation)**: Rotor4 serialization fix changes RON format from `[f32; 8]` arrays to struct fields `{ s: 1.0, b_xy: 0.0, ... }`. Existing scene RON files will need re-export. Compatible with P5's timeline.
- **Agent P4 (Level Design)**: The RON preview tool could serve as the foundation for the editor's viewport.
- **Agent P2 (Weapons & Feedback)**: Point lights add bind group 1 to the main render pipeline. HUD/sprite passes use separate pipelines, no conflict.
- **Render pass ordering confirmed** across P2, P3, P5: geometry -> sprites -> particles -> HUD -> egui editor (last).
- **Scripting Phase**: The `rust4d_scripting` crate must be operational before P5 Lua binding work and editor Lua panels. Bindings from P1-P4 are registered into the same Lua state.

---

## Parallelization Strategy

### Internal Parallelism (Within Phase 5)

```
Phase 5 Wave 1 (Parallel - can all start simultaneously)
  Agent A: Point lights + shadows + Lua lighting API (render pipeline changes)
  Agent B: Texture support + Lua texture API (shader + texture loading)
  Agent C: Input rebinding API + Lua input API (rust4d_input changes)

Phase 5 Wave 2 (Sequential - needs Wave 1 render changes)
  Agent D: Editor framework + entity editing + W-slice nav + scene operations
           + script error panel + Lua console
           (needs lights working for viewport to look decent)
```

The editor (Wave 2) benefits from lights and textures being available, but could technically start in parallel and integrate them later. The three rendering/input features (Wave 1) have no dependencies on each other.

### Wave 1 Internal Dependencies

```
5.1 Point lights ──> 5.2 Basic shadows (needs light bind group)
                 ──> 5.1L Lua lighting API (needs PointLight4D component)
5.3 Triplanar textures ──> 5.4 UV path (if needed)
                       ──> 5.3L Lua texture/material API (needs TextureManager)
5.5 Input rebinding ──> 5.5L Lua input API (needs InputMap methods)
```

### Wave 2 Internal Dependencies

```
5.6 Editor framework ──> 5.7 Entity list + inspector
                     ──> 5.8 W-slice navigation
                     ──> 5.9 Scene operations
                     ──> 5.10 Script error panel
                     ──> 5.11 Lua console panel
(5.7, 5.8, 5.9, 5.10, 5.11 can run in parallel after 5.6)
```

---

## Verification Criteria

### Sub-Phase A: Textures
- [ ] Triplanar mapping produces correct UVs on all shape types (tesseract, hyperplane, hyperprism, hypersphere)
- [ ] Texture loading from PNG/JPG via `image` crate works
- [ ] `TextureManager` creates GPU textures and views correctly
- [ ] Per-material texture assignment works (materials with and without textures render correctly)
- [ ] Triplanar sampling in shader produces visually acceptable results on architectural geometry

**Lua integration tests:**
- [ ] Lua script calls `textures:load("stone", "assets/stone.png")` and texture is available
- [ ] Lua script calls `entity:set_material({ texture="stone" })` and entity renders with texture
- [ ] Invalid texture name in Lua produces error (not crash)

### Sub-Phase B: Lighting
- [ ] `PointLight4D` component can be added to ECS entities
- [ ] Light collection query correctly gathers all `(Transform4D, PointLight4D)` entities
- [ ] W-distance attenuation correctly culls lights beyond `w_range`
- [ ] W-distance attenuation correctly dims lights based on W-distance from slice
- [ ] Point light loop in shader produces correct Lambert diffuse per light
- [ ] Distance attenuation (inverse square with range clamp) works correctly
- [ ] Shadow map renders depth-only from directional light perspective
- [ ] Shadow sampling in main pass correctly occludes fragments
- [ ] Shadow bias prevents shadow acne
- [ ] PCF produces soft shadow edges
- [ ] Objects not in current W-slice do not cast visible shadows (correct by construction)

**Lua integration tests:**
- [ ] Lua script calls `entity:add_light({ color={1,0.9,0.7}, intensity=2.0, range=15, w_range=3 })` and light appears
- [ ] Lua script calls `entity:set_light({ intensity=3.0 })` and light intensity changes
- [ ] Lua script removes light component and light disappears

### Sub-Phase C: Input Rebinding
- [ ] `InputMap::rebind()` correctly reassigns actions to new physical inputs
- [ ] `InputMap::unbind()` removes bindings
- [ ] `InputMap::conflicts()` detects when two actions share the same physical input
- [ ] `InputMap::to_toml()` serializes the full input map
- [ ] `InputMap::from_toml()` deserializes and round-trips correctly
- [ ] `InputMap::reset_defaults()` restores the default input map

**Lua integration tests:**
- [ ] Lua script calls `input:bind("jump", "space")` and binding takes effect
- [ ] Lua script calls `input:save()` and TOML file is written
- [ ] Lua script calls `input:load()` and bindings are restored
- [ ] `input:conflicts()` returns correct conflict table to Lua
- [ ] `input:get_binding("jump")` returns current binding string

### Sub-Phase D: Editor
- [ ] `rust4d_editor` crate compiles with egui + egui-wgpu + egui-winit
- [ ] `EditorHost` trait can be implemented by any application
- [ ] F12 toggles editor overlay on/off
- [ ] Entity list panel shows all entities with `Name` component
- [ ] Entity hierarchy displays via `Parent`/`Children` components
- [ ] Clicking an entity in the list selects it (EditorState tracks selection)
- [ ] Property inspector displays `Transform4D` position and rotation
- [ ] Property inspector provides `Material` color picker
- [ ] Add/delete entity operations work
- [ ] W-slice slider changes `slice_w` in real time
- [ ] Multi-slice thumbnails render the scene at multiple W values
- [ ] Save scene to RON produces valid scene files
- [ ] Load scene from RON correctly populates ECS world
- [ ] New scene clears the world
- [ ] Undo/redo works for at least transform property changes

**Lua editor panel tests:**
- [ ] Script error in Lua displays in editor error panel with file name and line number
- [ ] Error panel clears on successful hot-reload
- [ ] Lua console evaluates expressions and returns results
- [ ] Lua console can inspect entity properties at runtime (e.g., `world:query_sphere(...)`)
- [ ] Lua console error messages are displayed (not swallowed)
- [ ] Editor gracefully hides Lua panels when `EditorHost::lua_runtime()` returns `None`
- [ ] Hot-reload of Lua scripts reflects in editor immediately

---

## Risk Assessment

### High Risk: Editor Scope Creep
The editor is the single largest feature. The temptation to add "just one more panel" is enormous. **With Lua, the risk increases** -- the script editor and console are genuinely useful features that could expand endlessly.
**Mitigation**: Define MVE (Minimum Viable Editor) clearly and stick to it. Ship entity list + property inspector + W-slice slider + error log + basic console first. Full syntax-highlighted script editor and auto-completing REPL are Phase 6 territory. No spatial gizmos (transform handles in viewport) in MVE -- too complex for the benefit.

### Medium Risk: egui + wgpu Integration
egui-wgpu is well-maintained but version mismatches between egui, egui-wgpu, and wgpu can cause frustrating compile errors.
**Mitigation**: Pin exact versions of egui ecosystem crates. Test integration early (Task 5.6) before building features on top. If egui-wgpu integration proves problematic, consider egui-glow as a fallback (OpenGL backend, less optimal but more stable). The egui-winit version must be compatible with workspace `winit = "0.30"` (flagged by P2 as a risk to verify).

### Medium Risk: Texture UV Quality
Triplanar mapping produces visible seams at certain angles and can look blurry on surfaces not aligned with major axes.
**Mitigation**: Start with triplanar, evaluate quality. If insufficient, Task 5.4 (UV path) is the fallback. For sprite billboards (enemy rendering from P3), UVs are straightforward since sprites are flat quads.

### Low Risk: Lua Console Security
The Lua console gives full access to engine APIs at runtime. This is a development tool, not shipping in production builds.
**Mitigation**: The editor (and console) only exist in dev builds. Production builds do not depend on `rust4d_editor`. The console executes in the same sandbox as regular game scripts -- no additional attack surface.

### Low Risk: Point Light Performance
With 16-32 point lights computed per fragment in a forward renderer, performance could degrade in scenes with many lights.
**Mitigation**: Cap at 16 lights initially (plenty for a boomer shooter). Sort lights by distance, only process the closest N. W-distance culling naturally reduces visible light count. Upgrade to clustered forward or deferred only if needed (post-Phase 5).

### Low Risk: Shadow Map Quality
Single directional shadow map is basic but sufficient for a boomer shooter. 4D-specific risk: objects at different W-slices might cast shadows incorrectly if not handled.
**Mitigation**: Shadows are computed on the already-sliced 3D geometry, so only visible objects cast shadows. This is correct behavior -- an object not in your W-slice should not cast a shadow you can see.

---

## What This Phase Provides to the Overall Project

### Engine APIs Delivered

| Feature | Crate | API |
|---------|-------|-----|
| Point lights with W-attenuation | `rust4d_core` + `rust4d_render` | `PointLight4D` component, automatic light collection |
| Directional shadow mapping | `rust4d_render` | Automatic, configurable bias + PCF |
| Triplanar texture mapping | `rust4d_render` | `TextureManager`, `Material.texture` field |
| Input rebinding API | `rust4d_input` | `InputMap::rebind()`, TOML persistence |
| Scene editor with entity editing | `rust4d_editor` | `EditorApp`, `EditorHost` trait |
| W-slice navigation widget | `rust4d_editor` | Integrated in editor UI |
| Script error panel | `rust4d_editor` | Lua error display with file:line |
| Lua console panel | `rust4d_editor` | Runtime REPL for debugging |
| Lua lighting API | `rust4d_scripting` | `entity:add_light()`, `entity:set_light()` |
| Lua texture/material API | `rust4d_scripting` | `textures:load()`, `entity:set_material()` |
| Lua input rebinding API | `rust4d_scripting` | `input:bind()`, `input:save()`, `input:load()` |

### What the Game Builds Using These APIs (via Lua)

| Feature | Implementation |
|---------|---------------|
| Pause menu | Lua script using HUD API |
| Settings screen with key rebinding | Lua script calling `input:bind()`, `input:save()` |
| Game-specific textures | Loaded via `textures:load()` in Lua, assigned to materials |
| Level-specific lighting | Placed as entities with `entity:add_light()` in Lua or in RON scene files |
| Game configuration tweaking | Edit Lua data files directly -- no custom editor panels needed |
| Runtime debugging | Lua console in the editor |

### Files Modified/Created

**New files**:
- `crates/rust4d_editor/Cargo.toml`
- `crates/rust4d_editor/src/lib.rs`
- `crates/rust4d_editor/src/state.rs`
- `crates/rust4d_editor/src/integration.rs`
- `crates/rust4d_editor/src/panels/mod.rs`
- `crates/rust4d_editor/src/panels/entity_list.rs`
- `crates/rust4d_editor/src/panels/property_inspector.rs`
- `crates/rust4d_editor/src/panels/w_slice_nav.rs`
- `crates/rust4d_editor/src/panels/viewport.rs`
- `crates/rust4d_editor/src/panels/scene_panel.rs`
- `crates/rust4d_editor/src/panels/script_panel.rs` (Lua error display)
- `crates/rust4d_editor/src/panels/lua_console.rs` (Lua REPL)

**Modified files**:
- `crates/rust4d_render/src/pipeline/render_pipeline.rs` -- bind group 1 for lights, shadow pipeline
- `crates/rust4d_render/src/pipeline/types.rs` -- `GpuPointLight`, `LightUniforms`
- `crates/rust4d_render/src/shaders/render.wgsl` -- point light loop, triplanar mapping, shadow sampling
- `crates/rust4d_core/src/components.rs` (or equivalent) -- `PointLight4D` component
- `crates/rust4d_input/src/lib.rs` (or `input_actions.rs`) -- `rebind()`, `to_toml()`, `from_toml()`
- `crates/rust4d_scripting/src/bindings/` -- Lua binding modules for input, textures, lighting
- `Cargo.toml` -- add `rust4d_editor` to workspace members

---

## Summary

Phase 5 is where the Rust4D engine goes from a rendering/physics foundation to a complete development toolkit. The editor is the crown jewel -- a 4D-aware scene editor with the unique W-slice navigation feature that no other tool can provide. The lighting, texture, and input rebinding systems round out the engine's capabilities for shipping a game.

With the Lua migration, Phase 5 gains three additional dimensions: thin Lua bindings for textures, lighting, and input rebinding (+0.35 sessions), plus two new editor panels -- a script error display and Lua console (+0.5 sessions minimal). These additions are modest but critical for the Lua-driven development workflow: developers need to see script errors in the editor and have a REPL for runtime debugging.

The phase is large (10.85-13.75 sessions total with minimal Lua scoping) but highly parallelizable internally. With three agents on Wave 1 and one agent on Wave 2, the critical path drops to 7-8.5 sessions. The deepest risk is scope creep on the editor; the MVE definition exists specifically to guard against this. The Lua editor panels (full script editor, rich console) are explicitly deferred to Phase 6 if the minimal versions prove sufficient.

After Phase 5, the Rust4D engine provides everything a 4D game needs: math, physics, rendering (with lighting and textures), input (with rebinding), ECS, serialization, Lua scripting with full API access, and a development editor with script-aware tooling. The game repo builds game-specific systems on top of these APIs entirely in Lua.
