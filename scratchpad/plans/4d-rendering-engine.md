# Plan: 4D Rendering Engine with 4D Golf-Style Camera

## Overview

Build a 4D rendering engine in Rust using wgpu that displays 3D cross-sections of a 4D world. Camera movement replicates 4D Golf's intuitive controls.

---

## Project Structure

### Cargo Workspace

```toml
# Cargo.toml (workspace root)
[workspace]
resolver = "2"
members = [
    "crates/rust4d_math",
    "crates/rust4d_render",
    "crates/rust4d_input",
]

[workspace.dependencies]
wgpu = "24"
winit = "0.30"
bytemuck = { version = "1.14", features = ["derive"] }
pollster = "0.4"
env_logger = "0.11"
log = "0.4"
```

### Directory Layout

```
rust4d/
├── Cargo.toml                     # Workspace definition
├── crates/
│   ├── rust4d_math/               # 4D mathematics
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── vec4.rs            # 4D vector type
│   │       ├── rotor4.rs          # 4D rotor (8 components)
│   │       └── transform4d.rs     # Combined transform struct
│   │
│   ├── rust4d_render/             # Rendering pipeline
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── context.rs         # wgpu device/queue/surface
│   │       ├── camera4d.rs        # 4D camera with 6-DoF
│   │       ├── geometry/
│   │       │   ├── mod.rs
│   │       │   ├── tesseract.rs   # Hypercube geometry
│   │       │   └── simplex4d.rs   # 4D simplex (5-cell)
│   │       ├── pipeline/
│   │       │   ├── mod.rs
│   │       │   ├── slice_compute.rs
│   │       │   ├── render_pipeline.rs
│   │       │   └── lookup_texture.rs
│   │       └── shaders/
│   │           ├── slice.wgsl
│   │           └── render.wgsl
│   │
│   └── rust4d_input/              # Input handling
│       └── src/
│           ├── lib.rs
│           └── camera_controller.rs
│
└── src/main.rs                    # Application entry
```

---

## 1. 4D Math Types (`rust4d_math`)

### Vec4

```rust
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Pod, Zeroable)]
pub struct Vec4 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,  // 4th spatial dimension (ana/kata)
}

impl Vec4 {
    pub const ZERO: Self = Self { x: 0.0, y: 0.0, z: 0.0, w: 0.0 };
    pub const X: Self = Self { x: 1.0, y: 0.0, z: 0.0, w: 0.0 };
    pub const Y: Self = Self { x: 0.0, y: 1.0, z: 0.0, w: 0.0 };
    pub const Z: Self = Self { x: 0.0, y: 0.0, z: 1.0, w: 0.0 };
    pub const W: Self = Self { x: 0.0, y: 0.0, z: 0.0, w: 1.0 };

    pub fn new(x: f32, y: f32, z: f32, w: f32) -> Self;
    pub fn dot(self, other: Self) -> f32;
    pub fn length(self) -> f32;
    pub fn normalized(self) -> Self;
    pub fn xyz(&self) -> [f32; 3];  // Extract for 3D rendering
}
```

### Rotor4 (4D Rotation)

4D rotation uses 6 planes instead of 3 axes. A rotor has 8 components:

```rust
/// The 6 rotation planes in 4D
pub enum RotationPlane {
    XY,  // Standard yaw
    XZ,  // Standard pitch
    YZ,  // Standard roll
    XW,  // Ana-kata rotation (X)
    YW,  // Ana-kata rotation (Y)
    ZW,  // Ana-kata rotation (Z) - "W-roll" in 4D Golf
}

/// Rotor = scalar + 6 bivectors + pseudoscalar
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct Rotor4 {
    pub s: f32,       // Scalar
    pub b_xy: f32,    // Bivector components (6 planes)
    pub b_xz: f32,
    pub b_xw: f32,
    pub b_yz: f32,
    pub b_yw: f32,
    pub b_zw: f32,
    pub p: f32,       // Pseudoscalar
}

impl Rotor4 {
    pub const IDENTITY: Self;

    /// Create rotation in single plane
    pub fn from_plane_angle(plane: RotationPlane, angle: f32) -> Self {
        let half = angle * 0.5;
        let (cos_h, sin_h) = (half.cos(), half.sin());
        // Set scalar = cos(θ/2), bivector[plane] = -sin(θ/2)
    }

    /// Apply rotation: v' = R * v * R†
    pub fn rotate(&self, v: Vec4) -> Vec4;

    /// Compose rotations: R_combined = R_other * R_self
    pub fn compose(&self, other: &Self) -> Self;

    /// Conjugate (negate bivectors)
    pub fn reverse(&self) -> Self;

    /// Convert to 4x4 matrix for GPU
    pub fn to_matrix(&self) -> [[f32; 4]; 4];
}
```

---

## 2. Camera System (`rust4d_render/camera4d.rs`)

### Camera4D Structure

```rust
pub struct Camera4D {
    pub position: Vec4,          // 4D position (x, y, z, w)
    pub orientation: Rotor4,     // 6-DoF rotation
    pub slice_offset: f32,       // Cross-section offset

    // Euler-like angles for incremental control
    pitch: f32,      // XZ plane
    yaw: f32,        // XY plane
    roll_w: f32,     // ZW plane (4D Golf's W-rotation)
    pitch_w: f32,    // XW plane
    yaw_w: f32,      // YW plane
}

impl Camera4D {
    pub fn new() -> Self {
        Self {
            position: Vec4::new(0.0, 0.0, 5.0, 0.0),
            orientation: Rotor4::IDENTITY,
            slice_offset: 0.0,
            pitch: 0.0, yaw: 0.0,
            roll_w: 0.0, pitch_w: 0.0, yaw_w: 0.0,
        }
    }

    /// Standard 3D mouse look
    pub fn rotate_3d(&mut self, delta_yaw: f32, delta_pitch: f32) {
        self.yaw += delta_yaw;
        self.pitch = (self.pitch + delta_pitch).clamp(-1.5, 1.5);
        self.rebuild_orientation();
    }

    /// 4D W-rotation (click+hold in 4D Golf)
    pub fn rotate_w(&mut self, delta: f32) {
        self.roll_w += delta;
        self.rebuild_orientation();
    }

    /// WASD movement in camera-local XZ plane
    pub fn move_local_xz(&mut self, forward: f32, right: f32) {
        let fwd = self.orientation.rotate(Vec4::new(0.0, 0.0, -1.0, 0.0));
        let rgt = self.orientation.rotate(Vec4::new(1.0, 0.0, 0.0, 0.0));
        self.position.x += fwd.x * forward + rgt.x * right;
        self.position.y += fwd.y * forward + rgt.y * right;
        self.position.z += fwd.z * forward + rgt.z * right;
    }

    /// Q/E movement along W axis
    pub fn move_w(&mut self, delta: f32) {
        self.position.w += delta;
    }

    /// Get W-coordinate for cross-section
    pub fn get_slice_w(&self) -> f32 {
        self.position.w + self.slice_offset
    }

    fn rebuild_orientation(&mut self) {
        // Compose: yaw * pitch * roll_w * pitch_w * yaw_w
        let r_yaw = Rotor4::from_plane_angle(RotationPlane::XY, self.yaw);
        let r_pitch = Rotor4::from_plane_angle(RotationPlane::XZ, self.pitch);
        let r_roll_w = Rotor4::from_plane_angle(RotationPlane::ZW, self.roll_w);
        self.orientation = r_yaw.compose(&r_pitch).compose(&r_roll_w).normalize();
    }
}
```

---

## 3. Input Controls (`rust4d_input`)

Replicates 4D Golf controls:

| Input | Action |
|-------|--------|
| W/S | Forward/backward (Z) |
| A/D | Left/right strafe (X) |
| Q/E | Ana/kata movement (W) |
| Mouse drag | 3D camera rotation |
| Right-click + drag | W-axis rotation |

```rust
pub struct CameraController {
    forward: bool, backward: bool,
    left: bool, right: bool,
    ana: bool, kata: bool,          // Q/E for W-axis
    mouse_pressed: bool,
    w_rotation_mode: bool,          // Right-click held
    pending_yaw: f32,
    pending_pitch: f32,

    move_speed: f32,                // 5.0
    w_move_speed: f32,              // 3.0
    mouse_sensitivity: f32,         // 0.003
    w_rotation_sensitivity: f32,    // 0.005
}

impl CameraController {
    pub fn update(&mut self, camera: &mut Camera4D, dt: f32) {
        // Movement
        let fwd = (self.forward as i32 - self.backward as i32) as f32;
        let rgt = (self.right as i32 - self.left as i32) as f32;
        let w = (self.ana as i32 - self.kata as i32) as f32;

        camera.move_local_xz(fwd * self.move_speed * dt, rgt * self.move_speed * dt);
        camera.move_w(w * self.w_move_speed * dt);

        // Rotation
        if self.mouse_pressed {
            if self.w_rotation_mode {
                camera.rotate_w(self.pending_yaw * self.w_rotation_sensitivity);
            } else {
                camera.rotate_3d(
                    -self.pending_yaw * self.mouse_sensitivity,
                    -self.pending_pitch * self.mouse_sensitivity,
                );
            }
        }
        self.pending_yaw = 0.0;
        self.pending_pitch = 0.0;
    }
}
```

---

## 4. Cross-Section Pipeline

### Lookup Texture Approach (per user's note)

A 5-cell (4D simplex) has 5 vertices. Each can be above or below the hyperplane → 2^5 = 32 cases.

```rust
// Edge table: which of 10 edges are crossed (bitmask)
pub const EDGE_TABLE: [u16; 32] = [
    0b0000000000, // Case 0: all below
    0b0000001111, // Case 1: v0 above → edges 0,1,2,3
    // ... 32 cases
];

// Triangle table: edge indices to form triangles (-1 = end)
pub const TRI_TABLE: [[i8; 12]; 32] = [
    [-1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1], // Case 0
    [0, 1, 2, 0, 2, 3, -1, -1, -1, -1, -1, -1],       // Case 1: quad
    // ... 32 cases, up to 4 triangles each
];
```

### Compute Shader (`slice.wgsl`)

```wgsl
struct Simplex4D { v0: Vertex4D, v1: Vertex4D, v2: Vertex4D, v3: Vertex4D, v4: Vertex4D }
struct Triangle3D { v0: Vertex3D, v1: Vertex3D, v2: Vertex3D }

@group(0) @binding(0) var<storage, read> simplices: array<Simplex4D>;
@group(0) @binding(1) var<storage, read_write> output_triangles: array<Triangle3D>;
@group(0) @binding(2) var<storage, read_write> triangle_count: atomic<u32>;
@group(0) @binding(3) var<uniform> params: SliceParams;

@group(1) @binding(0) var edge_table: texture_1d<u32>;
@group(1) @binding(1) var tri_table: texture_2d<i32>;

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let idx = id.x;
    if (idx >= arrayLength(&simplices)) { return; }

    var simplex = simplices[idx];

    // Transform to camera space
    for each vertex: pos = camera_rotation * (pos - camera_position);

    // Compute case index (which vertices above slice_w)
    var case_idx: u32 = 0u;
    if (simplex.v0.position.w > params.slice_w) { case_idx |= 1u; }
    if (simplex.v1.position.w > params.slice_w) { case_idx |= 2u; }
    // ... v2, v3, v4

    if (case_idx == 0u || case_idx == 31u) { return; } // No intersection

    // Lookup which edges are crossed
    let edge_mask = textureLoad(edge_table, i32(case_idx), 0).r;

    // Compute intersection points
    var intersections: array<Vertex3D, 10>;
    var count: u32 = 0u;
    for (var e: u32 = 0u; e < 10u; e++) {
        if ((edge_mask & (1u << e)) != 0u) {
            intersections[count] = compute_edge_intersection(simplex, e, params.slice_w);
            count++;
        }
    }

    // Output triangles from lookup table
    for (var t: u32 = 0u; t < 4u; t++) {
        let i0 = textureLoad(tri_table, vec2(t*3+0, case_idx), 0).r;
        let i1 = textureLoad(tri_table, vec2(t*3+1, case_idx), 0).r;
        let i2 = textureLoad(tri_table, vec2(t*3+2, case_idx), 0).r;
        if (i0 < 0) { break; }

        let out_idx = atomicAdd(&triangle_count, 1u);
        output_triangles[out_idx] = Triangle3D(
            intersections[i0], intersections[i1], intersections[i2]
        );
    }
}
```

### Render Shader (`render.wgsl`)

```wgsl
@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Lighting
    let light_dir = normalize(vec3(0.5, 1.0, 0.3));
    let diffuse = max(dot(in.normal, light_dir), 0.0) * 0.7 + 0.3;

    // W-depth color coding (4D Golf style)
    let w_norm = clamp(in.w_depth * w_depth_scale, -1.0, 1.0);
    var w_color: vec3<f32>;
    if (w_norm > 0.0) {
        w_color = mix(vec3(1.0), vec3(1.0, 0.3, 0.3), w_norm);  // Red for +W
    } else {
        w_color = mix(vec3(1.0), vec3(0.3, 0.3, 1.0), -w_norm); // Blue for -W
    }

    return vec4(in.color.rgb * w_color * diffuse, in.color.a);
}
```

---

## 5. Tesseract Geometry

A tesseract (4D hypercube) has 16 vertices, decomposed into 24 5-cells for slicing:

```rust
pub struct Tesseract {
    pub vertices: [Vec4; 16],      // All ±h combinations
    pub simplices: Vec<[usize; 5]>, // 24 5-cells
}

impl Tesseract {
    pub fn new(size: f32) -> Self {
        let h = size * 0.5;
        let vertices = [
            Vec4::new(-h, -h, -h, -h), // 0
            Vec4::new( h, -h, -h, -h), // 1
            // ... 16 vertices total
            Vec4::new( h,  h,  h,  h), // 15
        ];
        // Decompose into 24 5-cells
        let simplices = compute_simplex_decomposition();
        Self { vertices, simplices }
    }
}
```

---

## 6. Buffer Management

Variable output requires indirect drawing:

```rust
const MAX_OUTPUT_TRIANGLES: usize = 100_000;

struct SliceBuffers {
    triangle_buffer: wgpu::Buffer,  // Triangle3D array
    count_buffer: wgpu::Buffer,     // Atomic counter
    indirect_buffer: wgpu::Buffer,  // DrawIndirect command
}

// Frame flow:
// 1. Reset counter to 0
// 2. Run compute pass (fills triangles, increments counter)
// 3. Copy count to indirect buffer
// 4. draw_indirect() renders variable triangle count
```

---

## Implementation Sessions

### Parallelization Strategy

Each session should use `/swarm` to parallelize independent work:

| Session | Parallel Agents |
|---------|-----------------|
| 1 | Math Agent (Vec4, Rotor4) + Render Agent (wgpu setup) |
| 2 | Math Agent (geometric product) + Camera Agent (Camera4D) + Input Agent |
| 3 | Geometry Agent (tesseract decomposition) + Shader Agent (slice.wgsl) + Lookup Agent (tables) |
| 4 | Render Agent (pipeline) + Shader Agent (render.wgsl) |
| 5 | Polish Agent (UI) + Optimization Agent (profiling) |

### Session 1: Foundation

**Swarm**: 2 parallel agents
- **Math Agent**: Implement `Vec4` and basic `Rotor4`
- **Render Agent**: Set up wgpu window, device, surface

Tasks:
- [ ] Create Cargo workspace with all crates
- [ ] Implement `Vec4` (new, dot, length, normalize, add/sub/mul)
- [ ] Implement `Rotor4::from_plane_angle()` for single-plane rotation
- [ ] Create tesseract vertex positions (16 vertices)
- [ ] Set up wgpu window with clear color

**Deliverable**: Window clears to color, `Vec4` math works

### Session 2: Rotor + Camera

**Swarm**: 3 parallel agents
- **Math Agent**: Complete `Rotor4` (geometric product, sandwich product, to_matrix)
- **Camera Agent**: Implement `Camera4D` structure and methods
- **Input Agent**: Implement `CameraController` with WASD+QE+mouse

Tasks:
- [ ] Implement `Rotor4::rotate()` (sandwich product)
- [ ] Implement `Rotor4::compose()` (geometric product)
- [ ] Implement `Rotor4::to_matrix()`
- [ ] Create `Camera4D` with position/orientation
- [ ] Add WASD + QE input handling
- [ ] Add mouse look (3D rotation)

**Deliverable**: Camera moves through 4D space (debug position output)

### Session 3: Cross-Section Compute

**Swarm**: 3 parallel agents
- **Geometry Agent**: Tesseract simplex decomposition (24 5-cells)
- **Lookup Agent**: Build EDGE_TABLE and TRI_TABLE (32 cases), create GPU textures
- **Shader Agent**: Implement `slice.wgsl` compute shader

Tasks:
- [ ] Compute tesseract simplex decomposition (24 5-cells)
- [ ] Build EDGE_TABLE and TRI_TABLE (32 cases)
- [ ] Create lookup textures on GPU
- [ ] Implement `slice.wgsl` compute shader
- [ ] Create triple-buffered output management
- [ ] Verify via buffer readback

**Deliverable**: Triangles generated (logged count)

### Session 4: Rendering

**Swarm**: 2 parallel agents
- **Render Agent**: Set up render pipeline, indirect draw, view matrices
- **Shader Agent**: Implement `render.wgsl` with W-depth coloring and lighting

Tasks:
- [ ] Implement `render.wgsl` vertex/fragment shaders
- [ ] Set up indirect draw from compute output
- [ ] Add W-depth color coding (red/blue gradient)
- [ ] Add basic diffuse lighting
- [ ] Create 3D view/projection matrices

**Deliverable**: Visible tesseract cross-section

### Session 5: Polish

**Swarm**: 2 parallel agents
- **UI Agent**: Right-click mode, scroll wheel, debug text display
- **Optimization Agent**: Profile, tune speeds, optimize hot paths

Tasks:
- [ ] Add right-click W-rotation mode
- [ ] Tune movement/rotation speeds
- [ ] Add scroll wheel for slice_offset
- [ ] Add debug UI (W-position text)
- [ ] Profile and optimize

**Deliverable**: Full 4D Golf-style controls

---

## Critical Files

| File | Purpose |
|------|---------|
| `rust4d_math/src/rotor4.rs` | 4D rotation (geometric product, sandwich product) |
| `rust4d_render/src/camera4d.rs` | 6-DoF camera with 4D Golf controls |
| `rust4d_render/src/shaders/slice.wgsl` | Cross-section compute shader |
| `rust4d_render/src/pipeline/lookup_texture.rs` | 32-case triangulation tables |
| `rust4d_input/src/camera_controller.rs` | WASD+QE + mouse input |

---

## Performance Targets

- Compute pass: <4ms for 10K simplices
- Render pass: <4ms for 50K triangles
- Total: 60 FPS with headroom

---

## Verification

After implementation:
1. `cargo run` opens window with tesseract cross-section
2. WASD moves in XZ plane, Q/E moves through W (objects appear/disappear)
3. Mouse rotates view, right-click+drag rotates through W
4. Objects tinted red (+W) or blue (-W) based on W-depth
5. Moving through W shows tesseract morphing (cube→octahedron→cube)
