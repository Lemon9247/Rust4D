# Long-Term Plan: Advanced Rendering

**Status:** FUTURE/DRAFT - Not for immediate implementation
**Created:** 2026-01-27
**Estimated Effort:** 8-10 sessions
**Priority:** P6 (Long-term consideration)

---

## Overview

This plan outlines the path from Rust4D's current basic rendering to a feature-rich, visually sophisticated rendering system. The current implementation provides functional 4D-to-3D cross-section rendering with basic lighting and W-depth visualization. Advanced rendering features will significantly improve visual quality, enable more expressive 4D visualization techniques, and provide artists/developers with greater creative control.

### Why This Matters

4D rendering presents unique challenges and opportunities. While 3D engines can rely on established techniques, 4D visualization requires novel approaches to help users comprehend higher-dimensional geometry. Advanced rendering features will:

- **Improve spatial comprehension** through better depth cues and visual hierarchy
- **Enable artistic expression** via custom shaders and material systems
- **Support scientific visualization** with advanced cross-section effects
- **Maintain performance** through efficient multi-pass architecture
- **Differentiate Rust4D** as a serious 4D visualization tool

---

## Current State Analysis

### Existing Architecture

The current rendering system (`rust4d_render` crate) uses a two-stage wgpu pipeline:

```
4D Geometry → Compute Shader (Slicing) → 3D Triangles → Render Pipeline → Display
```

**Components:**
- `RenderContext`: wgpu device, queue, surface management
- `Camera4D`: 4D camera with position and rotation (transforms to 3D view)
- `SlicePipeline`: Compute shader for 4D→3D hyperplane slicing
- `RenderPipeline`: 3D rendering with basic lighting and depth
- `RenderableGeometry`: Converts World/Entity data to GPU buffers

### Current Features

**Strengths:**
- ✓ Efficient GPU-based 4D slicing via compute shaders
- ✓ Indirect rendering for dynamic triangle counts
- ✓ W-depth color gradient (red/blue visualization of 4th dimension)
- ✓ Basic diffuse lighting (Lambert shading)
- ✓ Depth buffer for proper occlusion
- ✓ Alternative fragment shaders (wireframe, normals, W-depth-only)
- ✓ Clean separation of slicing and rendering stages

**Limitations:**
- ✗ Single-pass rendering only
- ✗ No post-processing effects
- ✗ Fixed shader pipeline (no custom shaders)
- ✗ No shadow mapping
- ✗ Basic material system (color + lighting params only)
- ✗ No PBR (Physically Based Rendering)
- ✗ Limited 4D visualization techniques
- ✗ No render-to-texture support
- ✗ No advanced blending modes
- ✗ No GPU-driven particle effects

### Technical Debt & Opportunities

**Pipeline Architecture:**
- Current: Compute → Single Render Pass → Present
- Needed: Compute → Multi-Pass Render → Post-Process → Present

**Shader Organization:**
- Current: 3 hardcoded WGSL files embedded in source
- Needed: Modular shader library with runtime selection

**Material System:**
- Current: Per-vertex color + global lighting params
- Needed: Material abstraction with texture support

---

## Feature Roadmap

### Phase 1: Pipeline Refactoring (2 sessions)

**Goal:** Restructure rendering to support multiple passes and render targets.

#### Tasks

1. **Render Graph Architecture**
   - Introduce `RenderGraph` abstraction for multi-pass rendering
   - Define `RenderPass` trait for pluggable passes
   - Implement pass dependency tracking
   - Add render target management (attachments, formats)

2. **Intermediate Render Targets**
   - Create `RenderTarget` abstraction (color + depth + stencil)
   - Support multiple color attachments (MRT - Multiple Render Targets)
   - Implement render target pooling/reuse for performance
   - Add render target resizing on window resize

3. **Pass System**
   - Refactor current rendering into `GeometryPass`
   - Create `PassBuilder` API for declarative pass construction
   - Support pass inputs/outputs (textures, buffers)
   - Add debug visualization mode (show intermediate targets)

**Deliverables:**
- `render_graph` module with `RenderGraph`, `RenderPass` types
- `GeometryPass` wrapping existing render pipeline
- Example showing multi-pass rendering structure
- Tests for render graph execution order

**Blockers:** None (foundation work)

---

### Phase 2: Post-Processing Effects (2-3 sessions)

**Goal:** Add essential post-processing effects to enhance visual quality.

#### Effects to Implement

1. **Bloom (Glow Effect)**
   - Extract bright pixels to separate texture
   - Gaussian blur (separable convolution for efficiency)
   - Composite bloom with scene color
   - Adjustable intensity/threshold parameters

2. **SSAO (Screen-Space Ambient Occlusion)**
   - Generate random sample kernel
   - Sample depth buffer in screen space
   - Calculate occlusion factor
   - Blur to reduce noise
   - Apply to ambient lighting

3. **Tone Mapping & Color Grading**
   - HDR to LDR conversion (Reinhard, ACES, Uncharted 2)
   - Exposure adjustment
   - Gamma correction (sRGB)
   - Optional color grading LUT (Look-Up Table)

4. **Anti-Aliasing**
   - FXAA (Fast Approximate Anti-Aliasing) as baseline
   - Optional: TAA (Temporal Anti-Aliasing) for higher quality
   - Configurable quality settings

5. **Depth of Field** (Optional)
   - Focus plane calculation
   - Bokeh blur for out-of-focus areas
   - Useful for directing attention in complex 4D scenes

#### Architecture

```rust
pub trait PostProcessPass: RenderPass {
    fn setup(&mut self, device: &wgpu::Device, input_format: wgpu::TextureFormat);
    fn process(&self, encoder: &mut wgpu::CommandEncoder, input: &wgpu::TextureView, output: &wgpu::TextureView);
    fn resize(&mut self, device: &wgpu::Device, width: u32, height: u32);
}

pub struct PostProcessChain {
    passes: Vec<Box<dyn PostProcessPass>>,
    intermediate_targets: Vec<RenderTarget>,
}
```

**Deliverables:**
- `post_process` module with effect passes
- `PostProcessChain` orchestrator
- Configuration for enabling/disabling effects
- Performance benchmarks (before/after)

**Blockers:** Requires Phase 1 (multi-pass support)

---

### Phase 3: Material & Shader System (2 sessions)

**Goal:** Enable custom materials and shader authoring.

#### Material System

1. **Material Abstraction**
   ```rust
   pub struct Material {
       pub name: String,
       pub shader: ShaderRef,
       pub properties: MaterialProperties,
       pub textures: HashMap<String, TextureRef>,
       pub blend_mode: BlendMode,
       pub double_sided: bool,
   }

   pub enum MaterialProperties {
       Unlit { color: Vec4 },
       Lit { albedo: Vec4, metallic: f32, roughness: f32 },
       Custom { uniforms: HashMap<String, UniformValue> },
   }
   ```

2. **Shader Registry**
   - Register shaders by name at runtime
   - Hot-reload support (detect file changes, recompile)
   - Shader validation and error reporting
   - Preprocessor for shader variants (#ifdef FEATURE_X)

3. **Texture System**
   - Load images (PNG, JPG) via `image` crate
   - Generate mipmaps automatically
   - Support for texture atlases
   - Streaming for large textures

#### Shader Organization

**Directory Structure:**
```
shaders/
├── include/          # Shared functions/structs
│   ├── common.wgsl
│   ├── lighting.wgsl
│   └── noise.wgsl
├── geometry/         # Geometry stage shaders
│   ├── standard.wgsl
│   ├── wireframe.wgsl
│   └── w_depth.wgsl
├── post/             # Post-processing shaders
│   ├── bloom.wgsl
│   ├── ssao.wgsl
│   └── tonemap.wgsl
└── custom/           # User-defined shaders
```

**Shader Include System:**
- Preprocess `#include` directives before compilation
- Cache processed shaders
- Track dependencies for hot-reload

**Deliverables:**
- `Material` and `Shader` types in core
- `ShaderRegistry` with hot-reload
- Texture loading and management
- Examples: custom material, procedural texture shader
- Migration guide for existing code

**Blockers:** None (can run parallel with Phase 2)

---

### Phase 4: Shadow Mapping (1-2 sessions)

**Goal:** Add shadows for depth cues and visual realism.

#### Implementation

1. **Shadow Pass**
   - Render scene from light's perspective to depth texture
   - Support directional lights (orthographic projection)
   - Optional: Point lights (cubemap shadows)
   - Configurable shadow map resolution

2. **Shadow Sampling**
   - PCF (Percentage Closer Filtering) for soft shadows
   - Shadow bias to reduce acne
   - Fade shadows based on distance

3. **4D Shadow Challenges**
   - Shadows in 4D are complex (4D light → 3D cross-section)
   - Initial approach: treat sliced geometry as 3D for shadows
   - Future: Research proper 4D shadow projection

**Deliverables:**
- `ShadowPass` render pass
- Shadow map texture management
- PCF sampling in fragment shader
- Example scene with shadows

**Blockers:** Requires Phase 1 (multi-pass support)

---

### Phase 5: PBR Materials (1-2 sessions)

**Goal:** Implement Physically Based Rendering for realistic materials.

#### PBR Pipeline

1. **Material Properties**
   - Albedo (base color)
   - Metallic (0 = dielectric, 1 = metal)
   - Roughness (0 = smooth, 1 = rough)
   - Normal maps (tangent space)
   - Ambient occlusion maps
   - Emissive color

2. **Lighting Model**
   - Cook-Torrance BRDF (Bidirectional Reflectance Distribution Function)
   - Fresnel (Schlick approximation)
   - GGX normal distribution
   - Smith geometry function
   - Image-Based Lighting (IBL) with environment maps

3. **Environment Maps**
   - Load HDR environment maps (`.hdr` format)
   - Generate irradiance map (diffuse IBL)
   - Prefilter environment map (specular IBL)
   - BRDF integration LUT

**Deliverables:**
- `PbrMaterial` type
- PBR shader implementation
- Environment map loading/processing
- Material editor example
- PBR asset pipeline guide

**Blockers:** Requires Phase 3 (material system)

---

### Phase 6: 4D-Specific Visualizations (1 session)

**Goal:** Unique rendering techniques for 4D understanding.

#### Visualization Techniques

1. **Enhanced W-Axis Coloring**
   - Multiple color schemes (cool/warm, rainbow, custom gradients)
   - Per-object color mapping settings
   - W-depth fog (fade distant 4D geometry)

2. **Cross-Section Effects**
   - Wireframe overlay on cross-section plane
   - Highlight newly-sliced edges
   - Fade geometry based on distance from slice plane
   - Onion-skin mode (show multiple nearby slices)

3. **4D Motion Blur**
   - Blur based on 4D velocity (especially W motion)
   - Help users track rotation in 4D space

4. **Distance Fields**
   - Signed distance field visualization
   - Show "thickness" in W dimension
   - Useful for understanding 4D shapes

**Deliverables:**
- Color scheme configurations
- Cross-section effect shaders
- Interactive examples for each technique
- User guide on when to use each visualization

**Blockers:** Requires Phase 3 (shader system)

---

## Technical Considerations

### wgpu Pipeline Architecture

**Design Principles:**
- Minimize state changes (batch by material/shader)
- Use indirect rendering where possible (dynamic geometry)
- Leverage compute shaders for pre-processing (culling, LOD)
- Profile GPU performance regularly

**Resource Management:**
- Pool buffers/textures to reduce allocations
- Use staging buffers for large uploads
- Implement GPU memory budgets
- Add resource usage visualization (debug mode)

**Optimization Opportunities:**
- Early-Z testing (depth pre-pass)
- Frustum culling (CPU and GPU)
- Occlusion culling (hierarchical Z-buffer)
- LOD system for distant 4D geometry

### Shader Organization Strategy

**Module System:**
```
// lighting.wgsl - Shared lighting functions
fn lambert_diffuse(N: vec3<f32>, L: vec3<f32>) -> f32 { ... }
fn blinn_phong_specular(...) -> f32 { ... }
fn pbr_brdf(...) -> vec3<f32> { ... }

// Include in other shaders:
#include "lighting.wgsl"
```

**Shader Variants:**
- Preprocessor-based feature toggles
- Generate multiple pipeline states at runtime
- Cache compiled variants

**Error Handling:**
- Graceful fallback for compilation errors
- Detailed error messages with line numbers
- Shader validation before load

### Performance Considerations

**Target Performance:**
- 60 FPS at 1920x1080 with moderate scene complexity
- < 5ms budget for post-processing
- < 2ms budget for shadow mapping

**Profiling Strategy:**
- Integrate GPU timestamps (wgpu::TimestampWrites)
- Track per-pass timings
- Expose profiling UI (Dear ImGui integration)

**Scalability:**
- Quality presets (Low, Medium, High, Ultra)
- Dynamic resolution scaling
- Adaptive LOD based on frame time

**Memory Budget:**
- < 500 MB VRAM for rendering resources
- Texture streaming for large assets
- Purge unused resources after N frames

### Platform Compatibility

**wgpu Backends:**
- Vulkan (primary, Linux/Windows)
- Metal (macOS)
- DX12 (Windows fallback)
- WebGPU (future: browser support)

**Feature Detection:**
- Query device limits at startup
- Gracefully disable unsupported features
- Provide fallbacks (e.g., no SSAO on low-end GPUs)

**Testing:**
- CI tests on multiple backend/OS combinations
- Performance regression tracking

---

## Phased Implementation Plan

### Session Breakdown

| Session | Focus | Deliverables |
|---------|-------|--------------|
| 1 | Render Graph Core | `RenderGraph`, `RenderPass` trait, `RenderTarget` abstraction |
| 2 | Geometry Pass Refactor | Migrate existing rendering to `GeometryPass`, multi-pass example |
| 3 | Post-Process Foundation | Bloom + tone mapping, `PostProcessChain` |
| 4 | SSAO + Anti-Aliasing | SSAO pass, FXAA pass, quality settings |
| 5 | Material System | `Material`, `Shader`, texture loading |
| 6 | Shader Registry | Hot-reload, preprocessor, shader includes |
| 7 | Shadow Mapping | `ShadowPass`, PCF sampling |
| 8 | PBR Implementation | Cook-Torrance BRDF, environment maps |
| 9 | 4D Visualizations | Enhanced W-coloring, cross-section effects |
| 10 | Polish & Optimization | Profiling, quality presets, documentation |

### Wave Structure

**Wave 1 (Sessions 1-2): Foundation**
- Sequential: Render graph must be solid before building on it
- Agent A: Render graph architecture + refactoring

**Wave 2 (Sessions 3-4): Post-Processing**
- Sequential: Effects build on each other
- Agent A: Bloom/tone mapping, SSAO/FXAA

**Wave 3 (Sessions 5-6): Materials**
- Parallel: Material system is independent
- Agent A: Material abstraction, texture loading
- Agent B: Shader registry, hot-reload system

**Wave 4 (Sessions 7-8): Advanced Lighting**
- Parallel: Shadows and PBR are independent
- Agent A: Shadow mapping
- Agent B: PBR materials

**Wave 5 (Session 9): 4D Visuals**
- Sequential: Uses shader system from Wave 3
- Agent A: 4D-specific visualizations

**Wave 6 (Session 10): Polish**
- Sequential: Wraps up all previous work
- Agent A: Performance tuning, documentation, examples

### Testing Strategy

**Unit Tests:**
- Render graph execution order
- Pass dependency resolution
- Shader preprocessing correctness
- Material property serialization

**Integration Tests:**
- Multi-pass rendering produces correct output
- Post-process chain applies effects in order
- Shadow maps render correctly
- PBR materials match reference images

**Performance Tests:**
- Benchmark each post-process effect
- Compare frame times before/after optimizations
- Validate 60 FPS target on reference hardware

**Visual Tests:**
- Render reference scenes, compare to baseline images
- Detect regressions in visual quality
- Use perceptual diff tools (e.g., `difftest`)

---

## Risk Assessment

### High Risk

**Shader Complexity**
- *Risk:* WGSL shader debugging is difficult; complex effects may have subtle bugs
- *Mitigation:* Write shader unit tests, use RenderDoc for GPU debugging, invest in tooling

**Performance Degradation**
- *Risk:* Multiple passes and post-processing could tank frame rate
- *Mitigation:* Profile early and often, establish performance budgets, optimize hot paths

**API Instability**
- *Risk:* wgpu is still evolving; breaking changes possible
- *Mitigation:* Pin wgpu version, test on stable releases, monitor changelog

### Medium Risk

**Platform Fragmentation**
- *Risk:* Effects may not work identically across Vulkan/Metal/DX12
- *Mitigation:* Test on all platforms, use wgpu abstractions correctly, avoid vendor-specific features

**Shader Compilation Times**
- *Risk:* Hot-reload may be slow with many shader variants
- *Mitigation:* Cache compiled shaders, use incremental compilation, optimize preprocessor

**Resource Management**
- *Risk:* Memory leaks or excessive VRAM usage
- *Mitigation:* Implement resource tracking, use valgrind/GPU profilers, add memory budgets

### Low Risk

**User Adoption**
- *Risk:* Users may not understand how to use advanced features
- *Mitigation:* Provide excellent documentation, interactive examples, sensible defaults

**Maintenance Burden**
- *Risk:* Rendering code becomes hard to maintain
- *Mitigation:* Modular architecture, comprehensive tests, code reviews

---

## Success Criteria

### Must Have

- [ ] Multi-pass rendering with render graph system
- [ ] At least 3 post-processing effects (bloom, SSAO, tone mapping)
- [ ] Custom shader support with hot-reload
- [ ] Material system with texture support
- [ ] Shadow mapping for directional lights
- [ ] Documentation and examples for all features
- [ ] 60 FPS target maintained on reference hardware

### Should Have

- [ ] PBR material pipeline with IBL
- [ ] Multiple 4D visualization techniques
- [ ] Quality presets for scalability
- [ ] Profiling tools for performance monitoring
- [ ] CI tests for rendering correctness

### Nice to Have

- [ ] TAA (Temporal Anti-Aliasing)
- [ ] Depth of field effect
- [ ] Point light shadows (cubemaps)
- [ ] Advanced 4D motion blur
- [ ] Shader hot-reload UI

### Success Metrics

- **Visual Quality:** Rendering looks polished, no obvious artifacts
- **Performance:** 60 FPS at 1080p with all effects enabled (medium-high settings)
- **Flexibility:** Users can create custom materials without engine changes
- **Stability:** No crashes or GPU errors across platforms
- **Maintainability:** Code is modular, well-tested, and documented

---

## Trigger Conditions

**When to start this work:**

1. **Foundation Complete:**
   - Phase 1-4 of near-term roadmap are done
   - Core engine is stable and well-documented
   - Physics system is integrated

2. **User Demand:**
   - Users request better visuals or specific effects
   - Current rendering becomes a blocker for demos/projects

3. **Technical Readiness:**
   - wgpu stabilizes (v1.0 or equivalent)
   - Team is comfortable with GPU programming
   - Performance profiling infrastructure exists

4. **Project Maturity:**
   - Rust4D has users beyond Willow
   - Visual quality becomes important for adoption
   - Examples need polish for showcasing

**Do NOT start if:**
- Core engine has fundamental stability issues
- Documentation is incomplete (users can't use current features)
- Physics or ECS migration is in progress (avoid parallel large refactors)
- Team capacity is limited (prioritize near-term work)

---

## Dependencies

**Depends On:**
- Phase 1-3 of near-term roadmap (foundation + docs)
- Stable wgpu API (currently on v23)
- GPU profiling tools (RenderDoc, Nsight)

**Enables:**
- Advanced 4D demos and visualizations
- Game development with Rust4D (better visuals)
- Scientific visualization applications
- Marketing materials (screenshots, videos)

---

## Open Questions

1. **Shader Language:** Stick with WGSL or support SPIR-V cross-compilation?
   - *Lean:* WGSL is wgpu's native format; stick with it unless cross-compilation becomes necessary

2. **Material Serialization:** RON, JSON, or binary format?
   - *Lean:* RON for human-editable, TOML for simple configs, binary for performance-critical assets

3. **PBR or Stylized Rendering?** Should we prioritize realism or artistic freedom?
   - *Lean:* Both; PBR as default, but allow unlit/custom materials for stylization

4. **4D Shadow Model:** How should shadows work in 4D?
   - *Research needed:* Survey academic papers, start with 3D approximation

5. **WebGPU Support:** When to target browsers?
   - *Defer:* Wait for WebGPU to stabilize, focus on native first

---

## References

### Technical Resources

- **wgpu Documentation:** https://wgpu.rs/
- **WebGPU Spec:** https://www.w3.org/TR/webgpu/
- **Learn wgpu:** https://sotrh.github.io/learn-wgpu/
- **PBR Theory:** "Real Shading in Unreal Engine 4" (Karis, Epic Games)
- **SSAO:** "Screen Space Ambient Occlusion" (Crytek)
- **GPU Gems:** Classic rendering techniques

### Inspiration

- **Unity URP/HDRP:** Modern multi-pass rendering architecture
- **Bevy Engine:** Rust render graph implementation
- **three.js:** Post-processing effect library
- **Shadertoy:** Community shader experiments

### Tools

- **RenderDoc:** GPU debugger (essential)
- **Nsight Graphics:** NVIDIA profiler
- **PIX:** DirectX profiler (Windows)
- **Shader Playground:** WGSL/SPIR-V exploration

---

## Document Metadata

- **Author:** Claude Code (Session 2026-01-27)
- **Last Updated:** 2026-01-27
- **Next Review:** After Phase 3 near-term roadmap completion
- **Status:** Draft - Open for feedback and revisions

**Related Documents:**
- `/scratchpad/plans/engine-roadmap-2026/00-index.md` (roadmap overview)
- `/scratchpad/plans/engine-roadmap-2026/phase-1-foundation.md` (near-term work)
- `/crates/rust4d_render/README.md` (current rendering docs - to be written)

---

## Appendix: Example API Usage

### Multi-Pass Rendering

```rust
use rust4d_render::{RenderGraph, GeometryPass, BloomPass, ToneMappingPass};

let mut graph = RenderGraph::new();

// Define passes
let geometry = GeometryPass::new(&device, surface_format);
let bloom = BloomPass::new(&device, width, height);
let tonemap = ToneMappingPass::new(&device, surface_format);

// Connect passes
graph.add_pass("geometry", geometry);
graph.add_pass("bloom", bloom)
    .with_input("color", "geometry.color_output");
graph.add_pass("tonemap", tonemap)
    .with_input("hdr_color", "bloom.output")
    .with_output("final", surface_texture);

// Execute
graph.execute(&device, &queue);
```

### Custom Material

```rust
use rust4d_core::Material;

let custom_mat = Material::builder()
    .name("Glowing Grid")
    .shader("custom/grid.wgsl")
    .property("grid_size", 10.0)
    .property("glow_color", Vec4::new(0.0, 1.0, 0.5, 1.0))
    .texture("noise", "assets/noise.png")
    .blend_mode(BlendMode::Additive)
    .build();

entity.set_material(custom_mat);
```

### 4D Visualization Config

```rust
use rust4d_render::W_DepthVisualization;

let w_viz = W_DepthVisualization::builder()
    .color_scheme(ColorScheme::Rainbow)
    .fog_distance(5.0)
    .highlight_slice_plane(true)
    .onion_skin_layers(3)
    .build();

renderer.set_w_visualization(w_viz);
```

---

**End of Document**
