# Codebase Agent Report: Code Structure Review

**Agent:** Codebase Agent
**Date:** 2026-01-28
**Task:** Review current codebase structure and identify what's implemented

---

## Crate Structure Overview

```
rust4d/ (workspace root)
├── crates/
│   ├── rust4d_math/      - 4D vector/matrix math, shapes
│   ├── rust4d_core/      - World, entities, transforms, scenes
│   ├── rust4d_render/    - GPU rendering pipeline
│   ├── rust4d_input/     - Input handling
│   └── rust4d_physics/   - 4D physics simulation
├── src/
│   ├── main.rs           - Main application
│   └── config.rs         - Configuration system
├── config/               - Configuration files
├── scenes/               - Scene RON files
├── examples/             - Example programs
└── scratchpad/          - Work notes, reports, plans
```

---

## Crate Details

### rust4d_math
**Purpose:** Foundation 4D mathematics

**Key Types:**
- `Vec4` - 4D vector with x, y, z, w
- `Rotor4` - 4D rotation representation
- `Mat4` - 4x4 matrix
- `Tesseract4D` - 4D hypercube shape
- `Hyperplane4D` - Infinite 4D floor/surface
- `Tetrahedron` - 4D simplex

**Traits:**
- `ConvexShape4D` - Interface for 4D shapes

### rust4d_core
**Purpose:** Entity/World management, scene system

**Key Types:**
- `Entity` - Game object with shape, transform, material, physics body
- `EntityKey` - Generational key (SlotMap)
- `World` - Container for entities with physics integration
- `Transform4D` - Position, rotation, scale
- `Material` - RGBA color
- `DirtyFlags` - Tracks what needs rebuilding

**Scene System:**
- `Scene` - Serializable template (RON format)
- `ActiveScene` - Runtime instance with World and physics
- `SceneManager` - Scene stack management
- `EntityTemplate` - Serializable entity definition
- `ShapeTemplate` - Enum of serializable shapes

### rust4d_render
**Purpose:** GPU-based rendering pipeline

**Key Types:**
- `RenderContext` - WGPU device and surface
- `Camera4D` - 4D camera with position, rotation
- `SlicePipeline` - Compute shader for 4D→3D slicing
- `RenderPipeline` - 3D rendering with lighting
- `RenderableGeometry` - Converts World to GPU buffers

**Camera Methods:**
- `ana()` - Returns W-axis direction (4th dimension forward)
- `right()`, `up()`, `forward()` - 3D basis vectors
- `rotate_yaw()`, `rotate_pitch()`, `rotate_w()` - Rotation methods

### rust4d_input
**Purpose:** User input handling

**Key Types:**
- `CameraController` - FPS-style movement and look controls

**Configuration:**
- move_speed: 3.0 units/sec
- w_move_speed: 2.0 units/sec
- mouse_sensitivity: 0.002
- w_rotation_sensitivity: 0.005
- smoothing_half_life: 0.05 seconds

### rust4d_physics
**Purpose:** 4D physics simulation

**Key Types:**
- `PhysicsWorld` - Physics container
- `RigidBody4D` - Dynamic body with velocity, mass
- `StaticCollider` - Static collision shapes
- `PlayerPhysics` - Player-specific physics
- `PhysicsConfig` - Gravity, jump velocity
- `PhysicsMaterial` - Friction, restitution
- `Contact` - Collision response data
- `BodyKey` - Generational key for bodies

**Collision Shapes:**
- Sphere (radius)
- AABB (min, max bounds)
- Plane (normal, distance)

---

## Scene System Architecture

### Scene Template (Serializable)
```rust
Scene {
    name: String,
    gravity: Option<f32>,
    player_spawn: Option<[f32; 4]>,
    entities: Vec<EntityTemplate>,
}
```

### EntityTemplate (Serializable)
```rust
EntityTemplate {
    name: Option<String>,
    tags: Vec<String>,
    transform: Transform4D,
    shape: ShapeTemplate,
    material: Material,
}
```

### ShapeTemplate (Enum)
```rust
enum ShapeTemplate {
    Tesseract { size: f32 },
    Hyperplane { y, size, subdivisions, cell_size, thickness },
}
```

### ActiveScene (Runtime)
```rust
ActiveScene {
    name: String,
    player_spawn: Option<Vec4>,
    world: World,  // Contains entities and physics
}
```

### SceneManager
```rust
SceneManager {
    templates: HashMap<String, Scene>,
    active_scenes: HashMap<String, ActiveScene>,
    scene_stack: Vec<String>,
    default_physics: Option<PhysicsConfig>,
    player_radius: f32,
}
```

---

## Configuration System

### AppConfig Structure
```
AppConfig
├── window: WindowConfig
│   ├── title: String
│   ├── width: u32
│   ├── height: u32
│   ├── fullscreen: bool
│   └── vsync: bool
├── camera: CameraConfig
│   ├── start_position: [f32; 4]
│   ├── fov: f32
│   ├── near: f32
│   ├── far: f32
│   └── pitch_limit: f32
├── input: InputConfig
│   ├── move_speed: f32
│   ├── w_move_speed: f32
│   ├── mouse_sensitivity: f32
│   ├── w_rotation_sensitivity: f32
│   └── smoothing_half_life: f32
├── physics: PhysicsConfig
│   ├── gravity: f32
│   ├── jump_velocity: f32
│   └── player_radius: f32
├── rendering: RenderingConfig
│   ├── max_output_triangles: usize
│   ├── background_color: [f32; 3]
│   └── lighting: LightingConfig
├── debug: DebugConfig
│   ├── show_overlay: bool
│   ├── log_level: String
│   └── show_colliders: bool
└── scene: SceneConfig
    ├── path: String
    └── player_radius: f32
```

### Loading Priority
1. `config/default.toml` - Default values (version controlled)
2. `config/user.toml` - User overrides (gitignored)
3. Environment variables - `R4D_SECTION__KEY` format

---

## Documentation Files

| File | Lines | Content |
|------|-------|---------|
| README.md | ~150 | Project overview, features, getting started |
| ARCHITECTURE.md | 251 | System design, 7 Mermaid diagrams |
| CLAUDE.md | ~120 | Development instructions for Claude |
| examples/README.md | ~50 | Example index and learning path |

---

## Recent Additions (Last Week)

From git history:
- **Jan 28:** Fix all movement directions to rotate in 4D space
- **Jan 28:** Add verification tests for W-axis movement rotation
- **Jan 27:** Fix W-axis movement to follow camera 4D rotation
- **Jan 27:** Add physics review swarm reports
- **Jan 27:** Fix player edge falling, enable true 4D physics
- **Jan 27:** Add physics integration tests
- **Jan 27:** Wave 2 swarm: SceneManager, examples, documentation

---

## File Locations Summary

| Component | Location |
|-----------|----------|
| Scene Manager | `crates/rust4d_core/src/scene_manager.rs` |
| Scene Serialization | `crates/rust4d_core/src/scene.rs` |
| Entity System | `crates/rust4d_core/src/entity.rs` |
| Shape Templates | `crates/rust4d_core/src/shapes.rs` |
| World | `crates/rust4d_core/src/world.rs` |
| Configuration | `src/config.rs` |
| Camera | `crates/rust4d_render/src/camera4d.rs` |
| Physics World | `crates/rust4d_physics/src/world.rs` |
| Input Controller | `crates/rust4d_input/src/camera_controller.rs` |

---

## Conclusion

The codebase is well-organized with clear separation of concerns:
- Math library provides foundation
- Core provides entity/scene management
- Physics handles simulation
- Render handles GPU pipeline
- Input handles user controls

All major Phase 1-2 features are implemented and working. The architecture supports the planned Phase 3-5 features.
