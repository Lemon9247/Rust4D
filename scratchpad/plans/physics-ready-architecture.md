# Refactoring Plan: Physics-Ready Architecture

## Current State Analysis

The codebase is currently a **rendering-focused** engine with these characteristics:

| Aspect | Current State | Issue for Physics |
|--------|---------------|-------------------|
| Geometry | Tesseract/Hyperplane produce GPU vertices directly | Shapes coupled to rendering (colors baked in) |
| Objects | Raw arrays of vertices/tetrahedra | No entity abstraction, no transforms |
| Movement | Only camera moves | No concept of moving bodies |
| Scene | Everything global in main.rs | No scene/world container |

## Goals

1. **Decouple geometry from rendering** - Pure shapes without color/material
2. **Create entity system** - Objects with transforms, shapes, and optional physics
3. **Prepare for physics crate** - Clean interfaces for collision/dynamics
4. **Maintain rendering quality** - No visual regression

## Proposed Architecture

```
┌─────────────────────────────────────────────────────────────────────┐
│                          Application Layer                          │
│  main.rs: event loop, window, user input                           │
└─────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────────┐
│                      rust4d_core (NEW CRATE)                        │
│  World, Entity, Transform4D, Material                               │
│  - Owns entities                                                    │
│  - Steps physics (future)                                           │
│  - Collects renderables                                             │
└─────────────────────────────────────────────────────────────────────┘
                                    │
           ┌────────────────────────┼────────────────────────┐
           ▼                        ▼                        ▼
┌──────────────────┐    ┌──────────────────┐    ┌──────────────────┐
│     Entity       │    │     Entity       │    │     Camera4D     │
│  - Transform4D   │    │  - Transform4D   │    │  (in render)     │
│  - Shape (Arc)   │    │  - Shape (owned) │    │                  │
│  - Material      │    │  - Material      │    │                  │
│  - RigidBody*    │    │  - RigidBody*    │    │                  │
└──────────────────┘    └──────────────────┘    └──────────────────┘
           │                        │
           ▼                        ▼
┌─────────────────────────────────────────────────────────────────────┐
│                       rust4d_math (shapes added)                    │
│  ConvexShape4D trait                                                │
│  - Tesseract4D, Hyperplane4D, Custom                                │
│  - vertices() -> &[Vec4]                                            │
│  - tetrahedra() -> &[Tetrahedron]                                   │
│  + Vec4, Rotor4, Mat4 (existing)                                    │
└─────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────────┐
│                        rust4d_render                                │
│  RenderableGeometry: World -> GPU buffers                           │
│  SlicePipeline: 4D->3D compute                                      │
│  RenderPipeline: 3D->screen                                         │
│  Camera4D (stays here, rendering-specific)                          │
└─────────────────────────────────────────────────────────────────────┘
```

### Crate Dependency Graph

```
rust4d_math (no deps)
       │
       ▼
rust4d_core (depends on math)
       │
       ├──────────────────┐
       ▼                  ▼
rust4d_render      rust4d_input
       │                  │
       └────────┬─────────┘
                ▼
            main.rs
```

---

## Refactoring Steps

### Step 1: Extract Pure Geometry (rust4d_math)

**Move shape definitions to math crate** - shapes are mathematical, not render-specific.

**New types in rust4d_math:**
```rust
// Pure tetrahedron (indices only)
pub struct Tetrahedron {
    pub indices: [usize; 4],
}

// Convex shape trait
pub trait ConvexShape4D {
    fn vertices(&self) -> &[Vec4];
    fn tetrahedra(&self) -> &[Tetrahedron];
}

// Pure tesseract (no colors)
pub struct Tesseract4D {
    half_size: f32,
    vertices: [Vec4; 16],
    tetrahedra: Vec<Tetrahedron>,
}

// Pure hyperplane (no colors)
pub struct Hyperplane4D {
    y_level: f32,
    size: f32,
    w_extent: f32,
    vertices: Vec<Vec4>,
    tetrahedra: Vec<Tetrahedron>,
}
```

**Files:**
- Create `crates/rust4d_math/src/shape.rs`
- Create `crates/rust4d_math/src/tesseract.rs` (move from rust4d_render)
- Create `crates/rust4d_math/src/hyperplane.rs` (move from rust4d_render)

### Step 2: Create rust4d_core Crate

**New crate for shared game types:**

```toml
# crates/rust4d_core/Cargo.toml
[package]
name = "rust4d_core"
version = "0.1.0"
edition = "2021"

[dependencies]
rust4d_math = { path = "../rust4d_math" }
```

**Transform4D:**
```rust
// crates/rust4d_core/src/transform.rs
pub struct Transform4D {
    pub position: Vec4,
    pub rotation: Rotor4,
    pub scale: f32,  // uniform scale for simplicity
}

impl Transform4D {
    pub fn identity() -> Self;
    pub fn matrix(&self) -> [[f32; 4]; 4];
    pub fn transform_point(&self, p: Vec4) -> Vec4;
    pub fn inverse(&self) -> Transform4D;
}
```

**Material (minimal):**
```rust
// crates/rust4d_core/src/entity.rs
pub struct Material {
    pub base_color: [f32; 4],
}
```

**Entity with flexible shape ownership:**
```rust
pub enum ShapeRef {
    Shared(Arc<dyn ConvexShape4D>),
    Owned(Box<dyn ConvexShape4D>),
}

pub struct Entity {
    pub transform: Transform4D,
    pub shape: ShapeRef,
    pub material: Material,
    // Future: pub rigid_body: Option<RigidBodyHandle>,
}
```

**World container:**
```rust
// crates/rust4d_core/src/world.rs
pub struct World {
    entities: Vec<Entity>,
    // Future: physics_world: PhysicsWorld,
}

impl World {
    pub fn new() -> Self;
    pub fn add_entity(&mut self, entity: Entity) -> usize;
    pub fn entities(&self) -> &[Entity];
    pub fn update(&mut self, dt: f32);  // Future: steps physics
}
```

### Step 3: Update rust4d_render

**Add RenderableGeometry to bridge World to GPU:**
```rust
// crates/rust4d_render/src/renderable.rs
use rust4d_core::{World, Entity};

pub struct RenderableGeometry {
    pub vertices: Vec<Vertex4D>,
    pub tetrahedra: Vec<GpuTetrahedron>,
}

impl RenderableGeometry {
    /// Build GPU buffers from a single entity
    pub fn from_entity(entity: &Entity) -> Self;

    /// Batch all entities in a world into one buffer
    pub fn from_world(world: &World) -> Self;
}
```

**Update Cargo.toml:**
```toml
[dependencies]
rust4d_core = { path = "../rust4d_core" }
```

**Remove old geometry module** - Tesseract/Hyperplane moved to rust4d_math.

### Step 4: Update main.rs

**Before:**
```rust
let tesseract = Tesseract::new(2.0);
let (vertices, tetrahedra) = tesseract_to_tetrahedra(&mut tesseract);
// ... direct GPU upload
```

**After:**
```rust
use rust4d_core::{World, Entity, Transform4D, Material, ShapeRef};
use rust4d_math::{Tesseract4D, Hyperplane4D};
use rust4d_render::RenderableGeometry;
use std::sync::Arc;

let mut world = World::new();

// Add tesseract entity (shared shape)
let tesseract_shape = Arc::new(Tesseract4D::new(2.0));
world.add_entity(Entity {
    transform: Transform4D::identity(),
    shape: ShapeRef::Shared(tesseract_shape),
    material: Material { base_color: [1.0, 0.5, 0.2, 1.0] },
});

// Add floor entity (shared shape)
let floor_shape = Arc::new(Hyperplane4D::new(-2.0, 10.0, 5.0));
world.add_entity(Entity {
    transform: Transform4D::identity(),
    shape: ShapeRef::Shared(floor_shape),
    material: Material { base_color: [0.5, 0.5, 0.5, 1.0] },
});

// Main loop: collect geometry from world
let geometry = RenderableGeometry::from_world(&world);
slice_pipeline.upload_tetrahedra(&device, &geometry.vertices, &geometry.tetrahedra);
```

---

## File Changes Summary

| Action | File | Description |
|--------|------|-------------|
| **rust4d_math** | | |
| Create | `rust4d_math/src/shape.rs` | ConvexShape4D trait, Tetrahedron |
| Create | `rust4d_math/src/tesseract.rs` | Pure Tesseract4D (moved from render) |
| Create | `rust4d_math/src/hyperplane.rs` | Pure Hyperplane4D (moved from render) |
| Update | `rust4d_math/src/lib.rs` | Export new shape modules |
| **rust4d_core (NEW)** | | |
| Create | `crates/rust4d_core/Cargo.toml` | New crate manifest |
| Create | `crates/rust4d_core/src/lib.rs` | Crate root |
| Create | `crates/rust4d_core/src/transform.rs` | Transform4D |
| Create | `crates/rust4d_core/src/entity.rs` | Entity, Material |
| Create | `crates/rust4d_core/src/world.rs` | World container |
| **rust4d_render** | | |
| Create | `rust4d_render/src/renderable.rs` | RenderableGeometry (World -> GPU) |
| Delete | `rust4d_render/src/geometry/tesseract.rs` | Moved to math |
| Delete | `rust4d_render/src/geometry/hyperplane.rs` | Moved to math |
| Update | `rust4d_render/src/geometry/mod.rs` | Remove old geometry exports |
| Update | `rust4d_render/src/lib.rs` | Add rust4d_core dep, export renderable |
| Update | `rust4d_render/Cargo.toml` | Add rust4d_core dependency |
| **Workspace** | | |
| Update | `Cargo.toml` | Add rust4d_core to workspace members |
| Update | `src/main.rs` | Use new World/Entity API |

---

## Session Estimates

| Step | Sessions | Notes |
|------|----------|-------|
| Step 1: Extract geometry to math | 1-2 | Move Tesseract/Hyperplane, add ConvexShape4D trait |
| Step 2: Create rust4d_core | 1-2 | Transform4D, Entity, Material, World, ShapeRef |
| Step 3: Update rust4d_render | 1 | RenderableGeometry, remove old geometry module |
| Step 4: Update main.rs | 0.5-1 | Wire everything together, test visuals |
| **Total** | **4-6 sessions** | |

---

## Future Physics Integration

With this architecture, adding physics becomes straightforward:

```rust
// New crate: rust4d_physics
pub struct RigidBody4D {
    pub mass: f32,
    pub velocity: Vec4,
    pub angular_velocity: Rotor4,  // or bivector
    pub shape: Arc<dyn ConvexShape4D>,
}

pub struct PhysicsWorld {
    bodies: Vec<RigidBody4D>,
}

// Entity gains physics handle
pub struct Entity {
    pub transform: Transform4D,
    pub shape: Arc<dyn ConvexShape4D>,
    pub material: Material,
    pub rigid_body: Option<RigidBodyHandle>,  // index into PhysicsWorld
}

// World steps physics
impl World {
    pub fn update(&mut self, dt: f32) {
        self.physics_world.step(dt);
        for entity in &mut self.entities {
            if let Some(rb) = entity.rigid_body {
                entity.transform = self.physics_world.get_transform(rb);
            }
        }
    }
}
```

---

## Verification

After refactoring:
1. `cargo build` - no compilation errors
2. `cargo test` - all existing tests pass
3. Run application - visuals identical to before
4. Camera controls work as before
5. No performance regression (GPU pipeline unchanged)

---

## Design Decisions (User Confirmed)

1. **Shape ownership**: Both patterns supported
   - Default: `Arc<dyn ConvexShape4D>` for shared shapes (memory efficient)
   - Option: `.clone()` for unique copies when mutation needed
   - Shapes implement `Clone` for this flexibility

2. **Materials**: Minimal for now
   - Just `base_color: [f32; 4]`
   - Add complexity (per-vertex functions, PBR) when needed

3. **Crate structure**: New `rust4d_core` crate
   - Central home for shared game types (Entity, World, Transform4D)
   - Clean separation from math primitives and rendering specifics
   - Depends on: `rust4d_math`
   - Depended on by: `rust4d_render`, `src/main.rs`
