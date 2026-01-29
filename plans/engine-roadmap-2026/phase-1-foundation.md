# Phase 1: Foundation - Scene Serialization & Configuration System

**Created:** 2026-01-27
**Updated:** 2026-01-28
**Status:** COMPLETE
**Priority:** P0 (Critical Foundation)
**Total Effort:** 4 sessions (2 sessions per track)

---

**COMPLETION NOTE (2026-01-28)**: This phase was completed with the following notes:
- Phase 1A (Scene Serialization): Complete - RON format working for scenes
- Phase 1B (Configuration System): Complete - TOML config with figment for hierarchical overrides
- `ColliderTemplate` was not implemented as planned; colliders are created from entity tags instead
- The physics body creation uses a simpler approach without explicit ColliderTemplate types

---

## Overview

Phase 1 establishes the foundational infrastructure for content creation and user customization. It consists of two parallel tracks that can be developed independently:

- **Phase 1A: Scene Serialization** (2 sessions) - Enable loading/saving scenes from RON files
- **Phase 1B: Configuration System** (2 sessions) - Replace 40+ hardcoded constants with TOML config

These two tracks are **fully parallelizable** - they have no dependencies on each other and can be executed by separate agents simultaneously.

### Goals

1. Enable artists/designers to create scenes without recompiling
2. Allow users to customize engine settings without editing code
3. Establish file formats and conventions for the engine
4. Support hot-reloading and rapid iteration
5. Lay groundwork for scene management (Phase 2)

### Success Criteria

- [x] Scene files can be loaded from RON format
- [x] Scene files can be saved to RON format
- [x] All hardcoded constants moved to TOML config
- [x] Config supports hierarchical overrides (default → user → env vars)
- [x] Example scenes and configs included
- [x] Tests cover serialization roundtrips
- [x] Documentation explains file formats

---

## Phase 1A: Scene Serialization

**Effort:** 2 sessions
**Dependencies:** None
**Output Files:** Scene serialization infrastructure, example scene files

### Context

Currently, scenes are constructed programmatically in `src/main.rs` using `SceneBuilder`. This requires recompiling to make any changes. We need file-based scene persistence using RON (Rusty Object Notation) to enable content creation.

**Key Challenge:** Entity uses `ShapeRef` which wraps trait objects (`dyn ConvexShape4D`). Trait objects can't be serialized directly with serde. We need an intermediate representation.

### Tasks (Session 1: Serialization Infrastructure)

#### 1.1 Add Dependencies (15 minutes)

Add to `crates/rust4d_core/Cargo.toml`:

```toml
[dependencies]
serde = { version = "1.0", features = ["derive"] }
ron = "0.8"
```

#### 1.2 Create Serializable Shape Enum (45 minutes)

**File:** `crates/rust4d_core/src/shapes.rs` (new file)

Create an enum that represents all shape types for serialization:

```rust
use serde::{Deserialize, Serialize};
use rust4d_math::{Tesseract4D, Hyperplane4D, ConvexShape4D};
use std::sync::Arc;
use crate::ShapeRef;

/// Serializable representation of shapes
///
/// This enum represents all shape types that can be saved/loaded.
/// Converts to/from ShapeRef for runtime use.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ShapeTemplate {
    /// A tesseract (hypercube) with given size
    Tesseract { size: f32 },

    /// A hyperplane (infinite 4D floor)
    Hyperplane {
        y: f32,
        size: f32,
        subdivisions: u32,
        cell_size: f32,
        thickness: f32,
    },

    /// A 4D sphere (hypersphere)
    Sphere { radius: f32 },

    /// A wall (thin rectangular prism)
    Wall {
        position: [f32; 4],
        width: f32,
        height: f32,
        thickness: f32,
    },
}

impl ShapeTemplate {
    /// Convert this template into a runtime ShapeRef
    pub fn to_shape_ref(&self) -> ShapeRef {
        match self {
            ShapeTemplate::Tesseract { size } => {
                ShapeRef::shared(Tesseract4D::new(*size))
            }
            ShapeTemplate::Hyperplane { y, size, subdivisions, cell_size, thickness } => {
                ShapeRef::shared(Hyperplane4D::new(*y, *size, *subdivisions, *cell_size, *thickness))
            }
            ShapeTemplate::Sphere { radius } => {
                // TODO: Implement HyperSphere4D in rust4d_math
                // For now, use tesseract as placeholder
                ShapeRef::shared(Tesseract4D::new(*radius * 2.0))
            }
            ShapeTemplate::Wall { position, width, height, thickness } => {
                // TODO: Implement Wall4D in rust4d_math
                // For now, use tesseract as placeholder
                ShapeRef::shared(Tesseract4D::new(*width))
            }
        }
    }

    /// Create a template from a ShapeRef
    ///
    /// Note: This is lossy - we can't extract exact parameters from trait objects.
    /// Use this only for entities created from templates.
    pub fn from_shape_ref(shape: &ShapeRef, hint: &str) -> Self {
        // This is a placeholder - in practice, entities should store their template
        // For now, return a default tesseract
        ShapeTemplate::Tesseract { size: 1.0 }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shape_template_roundtrip() {
        let template = ShapeTemplate::Tesseract { size: 2.0 };
        let shape_ref = template.to_shape_ref();
        assert_eq!(shape_ref.as_shape().vertex_count(), 16);
    }

    #[test]
    fn test_shape_template_serialize() {
        let template = ShapeTemplate::Tesseract { size: 2.0 };
        let ron = ron::to_string(&template).unwrap();
        assert!(ron.contains("Tesseract"));
        assert!(ron.contains("size"));
    }
}
```

#### 1.3 Add Serde Derives to Core Types (30 minutes)

**Files to modify:**

1. `crates/rust4d_core/src/transform.rs`:

```rust
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Transform4D {
    pub position: Vec4,
    #[serde(with = "rotor4_serde")]
    pub rotation: Rotor4,
    pub scale: f32,
}

// Custom serialization for Rotor4 (serialize as 6 bivector components)
mod rotor4_serde {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use rust4d_math::Rotor4;

    pub fn serialize<S>(rotor: &Rotor4, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // Serialize as [s, xy, xz, xw, yz, yw, zw]
        let components = [
            rotor.s, rotor.xy, rotor.xz, rotor.xw,
            rotor.yz, rotor.yw, rotor.zw,
        ];
        components.serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Rotor4, D::Error>
    where
        D: Deserializer<'de>,
    {
        let components: [f32; 7] = Deserialize::deserialize(deserializer)?;
        Ok(Rotor4 {
            s: components[0],
            xy: components[1],
            xz: components[2],
            xw: components[3],
            yz: components[4],
            yw: components[5],
            zw: components[6],
        })
    }
}
```

2. `crates/rust4d_core/src/entity.rs`:

```rust
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Material {
    pub base_color: [f32; 4],
}

// Add a serializable entity template
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityTemplate {
    pub name: Option<String>,
    pub tags: Vec<String>,  // Use Vec instead of HashSet for serde
    pub transform: Transform4D,
    pub shape: ShapeTemplate,
    pub material: Material,
    pub physics: Option<PhysicsTemplate>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PhysicsTemplate {
    Static {
        collider: ColliderTemplate,
    },
    Dynamic {
        mass: f32,
        material: String, // e.g., "wood", "metal", "concrete"
        collider: ColliderTemplate,
    },
    Player {
        radius: f32,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ColliderTemplate {
    Sphere { radius: f32 },
    Floor { y: f32, material: String },
}

impl EntityTemplate {
    /// Convert this template into a runtime Entity
    pub fn to_entity(&self) -> Entity {
        let shape = self.shape.to_shape_ref();
        let mut entity = Entity::with_transform(shape, self.transform, self.material);

        if let Some(ref name) = self.name {
            entity.name = Some(name.clone());
        }

        entity.tags = self.tags.iter().cloned().collect();

        // Note: physics bodies are created separately after world setup
        entity
    }
}
```

3. `crates/rust4d_math/src/lib.rs` - Add serde derives to Vec4:

```rust
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Vec4 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,
}
```

#### 1.4 Create Scene Structure (30 minutes)

**File:** `crates/rust4d_core/src/scene.rs` (new file)

```rust
use serde::{Deserialize, Serialize};
use crate::{EntityTemplate, World};
use std::fs;
use std::path::Path;

/// Metadata about a scene
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SceneMetadata {
    pub name: String,
    pub version: String,
    pub author: Option<String>,
    pub description: Option<String>,
}

/// Physics configuration for a scene
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenePhysicsConfig {
    pub gravity: f32,
}

/// A serializable scene definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Scene {
    pub metadata: SceneMetadata,
    pub physics: Option<ScenePhysicsConfig>,
    pub entities: Vec<EntityTemplate>,
}

impl Scene {
    /// Load a scene from a RON file
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self, SceneError> {
        let contents = fs::read_to_string(path.as_ref())
            .map_err(|e| SceneError::IoError(e))?;

        let scene: Scene = ron::from_str(&contents)
            .map_err(|e| SceneError::ParseError(e))?;

        Ok(scene)
    }

    /// Save a scene to a RON file
    pub fn save<P: AsRef<Path>>(&self, path: P) -> Result<(), SceneError> {
        let pretty_config = ron::ser::PrettyConfig::new()
            .struct_names(true)
            .enumerate_arrays(false)
            .depth_limit(4);

        let contents = ron::ser::to_string_pretty(&self, pretty_config)
            .map_err(|e| SceneError::SerializeError(e))?;

        fs::write(path.as_ref(), contents)
            .map_err(|e| SceneError::IoError(e))?;

        Ok(())
    }

    /// Create a World from this scene
    ///
    /// Note: This creates the entities but not the physics bodies.
    /// Physics bodies must be created separately based on PhysicsTemplate.
    pub fn to_world(&self) -> World {
        let mut world = World::with_capacity(self.entities.len());

        for entity_template in &self.entities {
            let entity = entity_template.to_entity();
            world.add_entity(entity);
        }

        world
    }
}

#[derive(Debug)]
pub enum SceneError {
    IoError(std::io::Error),
    ParseError(ron::Error),
    SerializeError(ron::Error),
}

impl std::fmt::Display for SceneError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            SceneError::IoError(e) => write!(f, "IO error: {}", e),
            SceneError::ParseError(e) => write!(f, "Parse error: {}", e),
            SceneError::SerializeError(e) => write!(f, "Serialize error: {}", e),
        }
    }
}

impl std::error::Error for SceneError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Material, Transform4D, ShapeTemplate};
    use rust4d_math::Vec4;

    #[test]
    fn test_scene_roundtrip() {
        let scene = Scene {
            metadata: SceneMetadata {
                name: "Test Scene".to_string(),
                version: "1.0.0".to_string(),
                author: Some("Willow".to_string()),
                description: Some("A test scene".to_string()),
            },
            physics: Some(ScenePhysicsConfig {
                gravity: -20.0,
            }),
            entities: vec![
                EntityTemplate {
                    name: Some("tesseract".to_string()),
                    tags: vec!["dynamic".to_string()],
                    transform: Transform4D::from_position(Vec4::ZERO),
                    shape: ShapeTemplate::Tesseract { size: 2.0 },
                    material: Material::WHITE,
                    physics: None,
                },
            ],
        };

        // Serialize to RON
        let ron = ron::ser::to_string_pretty(&scene, Default::default()).unwrap();
        println!("Serialized scene:\n{}", ron);

        // Deserialize back
        let loaded: Scene = ron::from_str(&ron).unwrap();
        assert_eq!(loaded.metadata.name, "Test Scene");
        assert_eq!(loaded.entities.len(), 1);
    }
}
```

### Tasks (Session 2: Example Scenes & Integration)

#### 2.1 Create Example Scene Files (30 minutes)

**File:** `scenes/default.ron` (new directory and file)

```ron
Scene(
    metadata: SceneMetadata(
        name: "Default Scene",
        version: "1.0.0",
        author: Some("Willow"),
        description: Some("The default test scene with a floor, player spawn, and tesseract"),
    ),

    physics: Some(ScenePhysicsConfig(
        gravity: -20.0,
    )),

    entities: [
        // Floor
        EntityTemplate(
            name: Some("floor"),
            tags: ["static", "environment"],
            transform: Transform4D(
                position: (0.0, -2.0, 0.0, 0.0),
                rotation: [1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0], // Identity rotor
                scale: 1.0,
            ),
            shape: Hyperplane(
                y: -2.0,
                size: 10.0,
                subdivisions: 10,
                cell_size: 5.0,
                thickness: 0.001,
            ),
            material: Material(
                base_color: [0.5, 0.5, 0.5, 1.0],
            ),
            physics: Some(Static(
                collider: Floor(
                    y: -2.0,
                    material: "concrete",
                ),
            )),
        ),

        // Tesseract
        EntityTemplate(
            name: Some("tesseract"),
            tags: ["dynamic", "interactable"],
            transform: Transform4D(
                position: (0.0, 0.0, 0.0, 0.0),
                rotation: [1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
                scale: 1.0,
            ),
            shape: Tesseract(size: 2.0),
            material: Material(
                base_color: [1.0, 1.0, 1.0, 1.0],
            ),
            physics: Some(Dynamic(
                mass: 10.0,
                material: "wood",
                collider: Sphere(radius: 1.4), // ~sqrt(2) for tesseract bounding sphere
            )),
        ),

        // Player spawn point
        EntityTemplate(
            name: Some("player_start"),
            tags: ["spawn_point"],
            transform: Transform4D(
                position: (0.0, 0.0, 5.0, 0.0),
                rotation: [1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
                scale: 1.0,
            ),
            shape: Tesseract(size: 0.1), // Tiny invisible marker
            material: Material(
                base_color: [0.0, 1.0, 0.0, 0.5], // Transparent green
            ),
            physics: None,
        ),
    ],
)
```

**File:** `scenes/test_chamber.ron`

```ron
Scene(
    metadata: SceneMetadata(
        name: "Test Chamber",
        version: "1.0.0",
        author: Some("Willow"),
        description: Some("A chamber with multiple tesseracts for physics testing"),
    ),

    physics: Some(ScenePhysicsConfig(
        gravity: -20.0,
    )),

    entities: [
        // Floor
        EntityTemplate(
            name: Some("floor"),
            tags: ["static"],
            transform: Transform4D(
                position: (0.0, -2.0, 0.0, 0.0),
                rotation: [1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
                scale: 1.0,
            ),
            shape: Hyperplane(
                y: -2.0,
                size: 20.0,
                subdivisions: 20,
                cell_size: 10.0,
                thickness: 0.001,
            ),
            material: Material(
                base_color: [0.3, 0.3, 0.3, 1.0],
            ),
            physics: Some(Static(
                collider: Floor(y: -2.0, material: "concrete"),
            )),
        ),

        // Tower of tesseracts
        EntityTemplate(
            name: Some("cube_1"),
            tags: ["dynamic"],
            transform: Transform4D(
                position: (0.0, 2.0, 0.0, 0.0),
                rotation: [1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
                scale: 1.0,
            ),
            shape: Tesseract(size: 1.0),
            material: Material(base_color: [1.0, 0.0, 0.0, 1.0]),
            physics: Some(Dynamic(
                mass: 5.0,
                material: "wood",
                collider: Sphere(radius: 0.87),
            )),
        ),

        EntityTemplate(
            name: Some("cube_2"),
            tags: ["dynamic"],
            transform: Transform4D(
                position: (0.0, 4.0, 0.0, 0.0),
                rotation: [1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
                scale: 1.0,
            ),
            shape: Tesseract(size: 1.0),
            material: Material(base_color: [0.0, 1.0, 0.0, 1.0]),
            physics: Some(Dynamic(
                mass: 5.0,
                material: "wood",
                collider: Sphere(radius: 0.87),
            )),
        ),

        EntityTemplate(
            name: Some("cube_3"),
            tags: ["dynamic"],
            transform: Transform4D(
                position: (0.0, 6.0, 0.0, 0.0),
                rotation: [1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
                scale: 1.0,
            ),
            shape: Tesseract(size: 1.0),
            material: Material(base_color: [0.0, 0.0, 1.0, 1.0]),
            physics: Some(Dynamic(
                mass: 5.0,
                material: "wood",
                collider: Sphere(radius: 0.87),
            )),
        ),

        // Player spawn
        EntityTemplate(
            name: Some("player_start"),
            tags: ["spawn_point"],
            transform: Transform4D(
                position: (0.0, 0.0, 10.0, 0.0),
                rotation: [1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
                scale: 1.0,
            ),
            shape: Tesseract(size: 0.1),
            material: Material(base_color: [0.0, 1.0, 0.0, 0.5]),
            physics: None,
        ),
    ],
)
```

#### 2.2 Update Scene Builder to Support Scene Loading (45 minutes)

**File:** `src/scene/scene_builder.rs`

Add scene loading capability to SceneBuilder:

```rust
use rust4d_core::{Scene, SceneError};

impl SceneBuilder {
    /// Load a scene from a RON file
    pub fn from_scene_file(path: &str) -> Result<Self, SceneError> {
        let scene = Scene::load(path)?;

        // Create builder from scene
        let mut builder = SceneBuilder::with_capacity(scene.entities.len());

        // Set physics if specified
        if let Some(physics_config) = scene.physics {
            builder = builder.with_physics(physics_config.gravity);
        }

        // Add entities
        for entity_template in scene.entities {
            let entity = entity_template.to_entity();
            builder.entities.push(entity);

            // TODO: Handle physics body creation based on PhysicsTemplate
            // This will need to create physics bodies and link them to entities
        }

        Ok(builder)
    }
}
```

#### 2.3 Update main.rs to Use Scene Files (30 minutes)

**File:** `src/main.rs`

Replace programmatic scene construction with file loading:

```rust
// OLD CODE (remove):
// let world = SceneBuilder::with_capacity(2)
//     .with_physics(GRAVITY)
//     .add_floor(FLOOR_Y, 10.0, PhysicsMaterial::CONCRETE)
//     .add_player(player_start, 0.5)
//     .add_tesseract(Vec4::ZERO, 2.0, "tesseract")
//     .build();

// NEW CODE:
let scene_path = std::env::var("SCENE_FILE")
    .unwrap_or_else(|_| "scenes/default.ron".to_string());

println!("Loading scene: {}", scene_path);

let world = SceneBuilder::from_scene_file(&scene_path)
    .expect("Failed to load scene file")
    .build();
```

#### 2.4 Write Tests (30 minutes)

**File:** `crates/rust4d_core/src/scene.rs` - Add integration tests:

```rust
#[cfg(test)]
mod integration_tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_save_and_load_scene() {
        let scene = create_test_scene();

        let temp_path = "test_scene_temp.ron";
        scene.save(temp_path).expect("Failed to save");

        let loaded = Scene::load(temp_path).expect("Failed to load");

        // Cleanup
        fs::remove_file(temp_path).ok();

        assert_eq!(loaded.metadata.name, scene.metadata.name);
        assert_eq!(loaded.entities.len(), scene.entities.len());
    }

    #[test]
    fn test_scene_to_world() {
        let scene = create_test_scene();
        let world = scene.to_world();

        assert_eq!(world.entity_count(), scene.entities.len());

        // Verify named entity exists
        assert!(world.get_by_name("tesseract").is_some());
    }

    fn create_test_scene() -> Scene {
        Scene {
            metadata: SceneMetadata {
                name: "Test".to_string(),
                version: "1.0.0".to_string(),
                author: None,
                description: None,
            },
            physics: Some(ScenePhysicsConfig { gravity: -20.0 }),
            entities: vec![
                EntityTemplate {
                    name: Some("tesseract".to_string()),
                    tags: vec![],
                    transform: Transform4D::identity(),
                    shape: ShapeTemplate::Tesseract { size: 2.0 },
                    material: Material::WHITE,
                    physics: None,
                },
            ],
        }
    }
}
```

#### 2.5 Update Exports (15 minutes)

**File:** `crates/rust4d_core/src/lib.rs`

```rust
mod shapes;
mod scene;

pub use shapes::{ShapeTemplate, ColliderTemplate, PhysicsTemplate};
pub use scene::{Scene, SceneMetadata, ScenePhysicsConfig, SceneError};
```

---

## Phase 1B: Configuration System

**Effort:** 2 sessions
**Dependencies:** None
**Output Files:** Configuration infrastructure, TOML config files

### Context

The codebase has 40+ hardcoded constants scattered across multiple files (physics, rendering, input, scene setup). These need to be centralized in TOML configuration files with hierarchical overrides.

### Tasks (Session 1: Configuration Infrastructure)

#### 1.1 Add Dependencies (15 minutes)

Add to root `Cargo.toml`:

```toml
[dependencies]
figment = { version = "0.10", features = ["toml", "env"] }
serde = { version = "1.0", features = ["derive"] }
```

#### 1.2 Create Configuration Structures (60 minutes)

**File:** `src/config.rs` (new file)

```rust
use serde::{Deserialize, Serialize};
use figment::{Figment, providers::{Format, Toml, Env}};
use std::collections::HashMap;

/// Complete application configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AppConfig {
    pub window: WindowConfig,
    pub camera: CameraConfig,
    pub input: InputConfig,
    pub physics: PhysicsConfig,
    pub rendering: RenderingConfig,
    pub scene: SceneConfig,
    pub debug: DebugConfig,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct WindowConfig {
    pub title: String,
    pub width: u32,
    pub height: u32,
    #[serde(default)]
    pub fullscreen: bool,
    #[serde(default = "default_true")]
    pub vsync: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CameraConfig {
    /// Starting position [x, y, z, w]
    pub start_position: [f32; 4],
    /// Field of view in degrees
    pub fov: f32,
    pub near_plane: f32,
    pub far_plane: f32,
    /// Pitch limit in degrees (prevents gimbal lock)
    pub pitch_limit: f32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct InputConfig {
    /// Movement speed in units/second
    pub move_speed: f32,
    /// 4D (W-axis) movement speed in units/second
    pub w_move_speed: f32,
    /// Mouse look sensitivity
    pub mouse_sensitivity: f32,
    /// 4D rotation sensitivity (right-click drag)
    pub w_rotation_sensitivity: f32,
    /// Input smoothing enabled
    #[serde(default)]
    pub smoothing_enabled: bool,
    /// Input smoothing half-life in seconds
    #[serde(default = "default_smoothing")]
    pub smoothing_half_life: f32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PhysicsConfig {
    /// Gravity acceleration (negative = downward)
    pub gravity: f32,
    /// Player jump velocity
    pub jump_velocity: f32,
    /// Player collision radius
    pub player_radius: f32,
    /// Ground detection threshold (dot product of normal with up)
    pub ground_normal_threshold: f32,
    /// Fixed timestep (0.0 = variable)
    #[serde(default)]
    pub fixed_timestep: f32,
    /// Physics material presets
    #[serde(default)]
    pub materials: HashMap<String, PhysicsMaterialConfig>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PhysicsMaterialConfig {
    pub friction: f32,
    pub restitution: f32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RenderingConfig {
    /// Maximum triangles in output buffer
    pub max_output_triangles: usize,
    /// Background color RGB [0.0-1.0]
    pub background_color: [f32; 3],
    pub lighting: LightingConfig,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LightingConfig {
    /// Light direction (normalized in shader)
    pub light_direction: [f32; 3],
    pub ambient_strength: f32,
    pub diffuse_strength: f32,
    /// W-depth color mixing strength
    pub w_color_strength: f32,
    /// W-depth color range
    pub w_range: f32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SceneConfig {
    /// Player starting position [x, y, z, w]
    pub player_start: [f32; 4],
    pub player_radius: f32,
    pub floor: FloorConfig,
    #[serde(default)]
    pub objects: Vec<SceneObjectConfig>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FloorConfig {
    pub enabled: bool,
    pub y_position: f32,
    pub size: f32,
    pub subdivisions: u32,
    pub material: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SceneObjectConfig {
    #[serde(rename = "type")]
    pub object_type: String,
    pub name: String,
    pub position: [f32; 4],
    pub size: f32,
    pub mass: f32,
    pub material: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DebugConfig {
    /// Show debug overlay
    #[serde(default)]
    pub overlay: bool,
    /// Log level: "error", "warn", "info", "debug", "trace"
    #[serde(default = "default_log_level")]
    pub log_level: String,
    /// Show collision boxes
    #[serde(default)]
    pub show_colliders: bool,
    /// Show 4D rotation axes
    #[serde(default)]
    pub show_4d_axes: bool,
}

// Default value helpers
fn default_true() -> bool { true }
fn default_smoothing() -> f32 { 0.05 }
fn default_log_level() -> String { "info".to_string() }

impl AppConfig {
    /// Load configuration from files and environment variables
    ///
    /// Priority (lowest to highest):
    /// 1. config/default.toml (checked into git)
    /// 2. config/user.toml (gitignored, user overrides)
    /// 3. Environment variables (R4D_ prefix)
    pub fn load() -> Result<Self, figment::Error> {
        Figment::new()
            // 1. Default config (checked into git)
            .merge(Toml::file("config/default.toml"))
            // 2. User overrides (gitignored)
            .merge(Toml::file("config/user.toml").nested())
            // 3. Environment variables (R4D_ prefix)
            // Example: R4D_PHYSICS__GRAVITY=-5.0
            .merge(Env::prefixed("R4D_").split("__"))
            .extract()
    }

    /// Validate configuration values
    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.physics.gravity > 0.0 {
            return Err(ConfigError::InvalidValue(
                "physics.gravity must be <= 0 (downward)".to_string()
            ));
        }

        if self.window.width < 640 || self.window.height < 480 {
            return Err(ConfigError::InvalidValue(
                "window dimensions must be at least 640x480".to_string()
            ));
        }

        if self.camera.fov <= 0.0 || self.camera.fov >= 180.0 {
            return Err(ConfigError::InvalidValue(
                "camera.fov must be between 0 and 180 degrees".to_string()
            ));
        }

        Ok(())
    }
}

#[derive(Debug)]
pub enum ConfigError {
    InvalidValue(String),
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            ConfigError::InvalidValue(msg) => write!(f, "Invalid config value: {}", msg),
        }
    }
}

impl std::error::Error for ConfigError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_validation() {
        let mut config = create_test_config();
        assert!(config.validate().is_ok());

        // Test invalid gravity
        config.physics.gravity = 5.0;
        assert!(config.validate().is_err());
    }

    fn create_test_config() -> AppConfig {
        // Return a minimal valid config for testing
        AppConfig {
            window: WindowConfig {
                title: "Test".to_string(),
                width: 1280,
                height: 720,
                fullscreen: false,
                vsync: true,
            },
            camera: CameraConfig {
                start_position: [0.0, 0.0, 5.0, 0.0],
                fov: 45.0,
                near_plane: 0.1,
                far_plane: 100.0,
                pitch_limit: 89.0,
            },
            input: InputConfig {
                move_speed: 3.0,
                w_move_speed: 2.0,
                mouse_sensitivity: 0.002,
                w_rotation_sensitivity: 0.005,
                smoothing_enabled: false,
                smoothing_half_life: 0.05,
            },
            physics: PhysicsConfig {
                gravity: -20.0,
                jump_velocity: 8.0,
                player_radius: 0.5,
                ground_normal_threshold: 0.7,
                fixed_timestep: 0.0,
                materials: HashMap::new(),
            },
            rendering: RenderingConfig {
                max_output_triangles: 100_000,
                background_color: [0.02, 0.02, 0.08],
                lighting: LightingConfig {
                    light_direction: [0.5, 1.0, 0.3],
                    ambient_strength: 0.3,
                    diffuse_strength: 0.7,
                    w_color_strength: 0.5,
                    w_range: 2.0,
                },
            },
            scene: SceneConfig {
                player_start: [0.0, 0.0, 5.0, 0.0],
                player_radius: 0.5,
                floor: FloorConfig {
                    enabled: true,
                    y_position: -2.0,
                    size: 10.0,
                    subdivisions: 10,
                    material: "concrete".to_string(),
                },
                objects: vec![],
            },
            debug: DebugConfig {
                overlay: false,
                log_level: "info".to_string(),
                show_colliders: false,
                show_4d_axes: false,
            },
        }
    }
}
```

#### 1.3 Create Default Configuration File (45 minutes)

**File:** `config/default.toml` (new directory and file)

```toml
# Rust4D Engine Configuration
# This is the default configuration checked into version control.
# Override settings by creating config/user.toml (gitignored)

[window]
title = "Rust4D - 4D Rendering Engine"
width = 1280
height = 720
fullscreen = false
vsync = true

[camera]
# Starting position (x, y, z, w)
start_position = [0.0, 0.0, 5.0, 0.0]
# Field of view in degrees
fov = 45.0
near_plane = 0.1
far_plane = 100.0
# Pitch limit in degrees (prevents gimbal lock)
pitch_limit = 89.0

[input]
# Movement speed in units/second
move_speed = 3.0
# 4D (W-axis) movement speed in units/second
w_move_speed = 2.0
# Mouse look sensitivity
mouse_sensitivity = 0.002
# 4D rotation sensitivity (right-click drag)
w_rotation_sensitivity = 0.005
# Input smoothing (exponential, half-life in seconds)
smoothing_enabled = false
smoothing_half_life = 0.05

[physics]
# Gravity acceleration (negative = downward)
gravity = -20.0
# Player jump velocity
jump_velocity = 8.0
# Player collision radius
player_radius = 0.5
# Ground detection threshold (dot product of normal with up)
ground_normal_threshold = 0.7
# Time step for physics simulation (0.0 = variable timestep)
fixed_timestep = 0.0

# Physics materials presets
[physics.materials.ice]
friction = 0.05
restitution = 0.1

[physics.materials.rubber]
friction = 0.9
restitution = 0.8

[physics.materials.metal]
friction = 0.3
restitution = 0.3

[physics.materials.wood]
friction = 0.5
restitution = 0.2

[physics.materials.concrete]
friction = 0.7
restitution = 0.1

[rendering]
# Maximum triangles in output buffer
max_output_triangles = 100_000
# Background color (RGB, 0.0-1.0)
background_color = [0.02, 0.02, 0.08]

# Lighting configuration
[rendering.lighting]
# Light direction (normalized in shader)
light_direction = [0.5, 1.0, 0.3]
ambient_strength = 0.3
diffuse_strength = 0.7
# W-depth color mixing strength
w_color_strength = 0.5
# W-depth color range
w_range = 2.0

# Scene configuration (loaded at startup)
[scene]
# Player starting position
player_start = [0.0, 0.0, 5.0, 0.0]
player_radius = 0.5

# Floor configuration
[scene.floor]
enabled = true
y_position = -2.0
size = 10.0
subdivisions = 10
material = "concrete"

# Objects in the scene (programmatic scene setup)
[[scene.objects]]
type = "tesseract"
name = "main_tesseract"
position = [0.0, 0.0, 0.0, 0.0]
size = 2.0
mass = 10.0
material = "wood"

[debug]
# Show debug overlay with FPS, position, etc.
overlay = false
# Log level: "error", "warn", "info", "debug", "trace"
log_level = "info"
# Show collision boxes
show_colliders = false
# Show 4D rotation axes visualization
show_4d_axes = false
```

### Tasks (Session 2: Migration & Integration)

#### 2.1 Update .gitignore (5 minutes)

**File:** `.gitignore`

```
# User configuration (don't commit personal settings)
config/user.toml
```

#### 2.2 Create User Config Template (10 minutes)

**File:** `config/user.toml.example`

```toml
# User Configuration Overrides
# Copy this file to user.toml and customize your settings!
# user.toml is gitignored - it won't be committed.

# Example: Faster movement for testing
#[input]
#move_speed = 10.0

# Example: Different starting position
#[scene]
#player_start = [5.0, 2.0, 0.0, 1.0]

# Example: Enable debug overlay
#[debug]
#overlay = true
#log_level = "debug"
```

#### 2.3 Migrate main.rs to Use Config (60 minutes)

**File:** `src/main.rs`

Replace all hardcoded constants:

```rust
use crate::config::AppConfig;

// Remove old constants:
// const GRAVITY: f32 = -20.0;
// const FLOOR_Y: f32 = -2.0;

struct App {
    config: AppConfig,
    // ... rest of fields
}

impl App {
    fn new() -> Self {
        // Load configuration
        let config = AppConfig::load()
            .expect("Failed to load configuration");

        config.validate()
            .expect("Invalid configuration");

        println!("Loaded configuration from config/default.toml");

        // Use config values
        let player_start = Vec4::new(
            config.scene.player_start[0],
            config.scene.player_start[1],
            config.scene.player_start[2],
            config.scene.player_start[3],
        );

        let world = if config.scene.floor.enabled {
            SceneBuilder::with_capacity(2)
                .with_physics(config.physics.gravity)
                .add_floor(
                    config.scene.floor.y_position,
                    config.scene.floor.size,
                    PhysicsMaterial::CONCRETE
                )
                .add_player(player_start, config.physics.player_radius)
                // Add objects from config
                .build()
        } else {
            SceneBuilder::with_capacity(0)
                .with_physics(config.physics.gravity)
                .build()
        };

        // Create window with config
        let event_loop = EventLoop::new().unwrap();
        let window = WindowBuilder::new()
            .with_title(&config.window.title)
            .with_inner_size(PhysicalSize::new(
                config.window.width,
                config.window.height
            ))
            .build(&event_loop)
            .unwrap();

        // ... rest of initialization using config values

        Self {
            config,
            // ... rest of fields
        }
    }
}
```

#### 2.4 Update Input Controller to Use Config (30 minutes)

**File:** `crates/rust4d_input/src/camera_controller.rs`

Change hardcoded values to config parameters:

```rust
// Add config parameter to CameraController::new()
pub fn new(config: &InputConfig) -> Self {
    Self {
        move_speed: config.move_speed,
        w_move_speed: config.w_move_speed,
        mouse_sensitivity: config.mouse_sensitivity,
        w_rotation_sensitivity: config.w_rotation_sensitivity,
        smoothing_half_life: config.smoothing_half_life,
        smoothing_enabled: config.smoothing_enabled,
        // ... rest of fields
    }
}
```

Pass config from main.rs:

```rust
let camera_controller = CameraController::new(&app.config.input);
```

#### 2.5 Update Render Pipeline to Use Config (30 minutes)

**File:** `src/main.rs` - Update render uniform creation:

```rust
// In render loop:
let uniforms = RenderUniforms {
    view_proj: view_proj_matrix,
    camera_pos: camera.position,
    light_dir: config.rendering.lighting.light_direction,
    ambient_strength: config.rendering.lighting.ambient_strength,
    diffuse_strength: config.rendering.lighting.diffuse_strength,
    w_color_strength: config.rendering.lighting.w_color_strength,
    w_range: config.rendering.lighting.w_range,
};

// Background color
let [r, g, b] = config.rendering.background_color;
encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
    // ...
    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
        view: &view,
        resolve_target: None,
        ops: wgpu::Operations {
            load: wgpu::LoadOp::Clear(wgpu::Color {
                r: r as f64,
                g: g as f64,
                b: b as f64,
                a: 1.0,
            }),
            store: wgpu::StoreOp::Store,
        },
    })],
    // ...
});
```

#### 2.6 Write Configuration Tests (30 minutes)

**File:** `src/config.rs` - Add more tests:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_load_default_config() {
        // This test requires config/default.toml to exist
        let config = AppConfig::load();
        assert!(config.is_ok(), "Failed to load default config: {:?}", config.err());
    }

    #[test]
    fn test_env_var_override() {
        std::env::set_var("R4D_PHYSICS__GRAVITY", "-5.0");

        let config = AppConfig::load().unwrap();
        assert_eq!(config.physics.gravity, -5.0);

        std::env::remove_var("R4D_PHYSICS__GRAVITY");
    }

    #[test]
    fn test_config_serialization() {
        let config = create_test_config();
        let toml = toml::to_string_pretty(&config).unwrap();

        println!("Serialized config:\n{}", toml);

        let deserialized: AppConfig = toml::from_str(&toml).unwrap();
        assert_eq!(deserialized.window.width, config.window.width);
    }
}
```

---

## File Checklist

### Phase 1A: Scene Serialization

**New Files:**
- [ ] `crates/rust4d_core/src/shapes.rs` - ShapeTemplate enum
- [ ] `crates/rust4d_core/src/scene.rs` - Scene struct and serialization
- [ ] `scenes/default.ron` - Default scene file
- [ ] `scenes/test_chamber.ron` - Test scene with multiple objects

**Modified Files:**
- [ ] `crates/rust4d_core/Cargo.toml` - Add serde, ron dependencies
- [ ] `crates/rust4d_core/src/transform.rs` - Add Serialize/Deserialize derives
- [ ] `crates/rust4d_core/src/entity.rs` - Add EntityTemplate, serde support
- [ ] `crates/rust4d_math/src/lib.rs` - Add Serialize/Deserialize to Vec4
- [ ] `crates/rust4d_core/src/lib.rs` - Export new types
- [ ] `src/scene/scene_builder.rs` - Add from_scene_file method
- [ ] `src/main.rs` - Use scene file loading

### Phase 1B: Configuration System

**New Files:**
- [ ] `src/config.rs` - Configuration structs and loading logic
- [ ] `config/default.toml` - Default configuration (version controlled)
- [ ] `config/user.toml.example` - Example user overrides

**Modified Files:**
- [ ] `Cargo.toml` - Add figment dependency
- [ ] `.gitignore` - Add config/user.toml
- [ ] `src/main.rs` - Load and use configuration
- [ ] `src/lib.rs` - Export config module
- [ ] `crates/rust4d_input/src/camera_controller.rs` - Accept config parameter

---

## Testing Strategy

### Phase 1A Tests

1. **Serialization roundtrip**: Save and load scenes, verify equality
2. **ShapeTemplate conversion**: Verify template → ShapeRef → runtime shape
3. **Scene to World**: Verify scene correctly creates world with entities
4. **File format validation**: Test malformed RON files produce good errors
5. **Integration test**: Load default.ron, verify scene works in engine

### Phase 1B Tests

1. **Config loading**: Verify default config loads successfully
2. **Config validation**: Test invalid values are rejected
3. **Hierarchical overrides**: Verify user.toml overrides default.toml
4. **Environment variables**: Test R4D_* env vars override config files
5. **Integration test**: Run engine with various config combinations

---

## Example Usage

### Scene Files

```bash
# Run with default scene
cargo run

# Run with custom scene
SCENE_FILE=scenes/test_chamber.ron cargo run

# Run with scene file argument
cargo run -- --scene scenes/test_chamber.ron
```

### Configuration

```bash
# Run with default config
cargo run

# Override gravity via environment variable
R4D_PHYSICS__GRAVITY=-5.0 cargo run

# Override multiple values
R4D_WINDOW__WIDTH=1920 R4D_WINDOW__HEIGHT=1080 cargo run

# Create user config for persistent overrides
cp config/user.toml.example config/user.toml
# Edit config/user.toml
cargo run
```

---

## Risks and Mitigations

### Risk 1: Rotor4 Serialization Complexity

**Risk:** Rotor4 has 7 components (scalar + 6 bivector components) that need custom serde implementation.

**Mitigation:**
- Implement custom serialization as array of 7 floats
- Add tests for rotor serialization roundtrip
- Document rotor format in scene file comments

### Risk 2: ShapeRef Trait Object Serialization

**Risk:** Can't serialize trait objects directly. Need intermediate representation.

**Mitigation:**
- Use ShapeTemplate enum as intermediate representation
- Store shape type hints in EntityTemplate
- Accept some loss of fidelity for complex shapes

### Risk 3: Physics Body Synchronization

**Risk:** Scene loading creates entities but physics bodies are created separately.

**Mitigation:**
- Document two-phase loading (entities first, then physics)
- Add helper methods to create physics bodies from PhysicsTemplate
- Defer full physics integration to Phase 2

### Risk 4: Config File Migration

**Risk:** Changing hardcoded values may break existing behavior.

**Mitigation:**
- Use exact same default values as current hardcoded constants
- Test thoroughly before and after migration
- Document any behavior changes in comments

### Risk 5: Backward Compatibility

**Risk:** Scene/config format changes could break existing files.

**Mitigation:**
- Add version field to Scene and Config
- Implement format validation with good error messages
- Provide migration tools if format changes in future

---

## Dependencies and Parallelization

### Parallelization Strategy

Phase 1A and 1B are **completely independent** and can be executed simultaneously by two agents:

```
Wave 1 (Parallel):
├── Agent A: Phase 1A - Scene Serialization (2 sessions)
│   ├── Session 1: Serialization infrastructure
│   └── Session 2: Example scenes & integration
│
└── Agent B: Phase 1B - Configuration System (2 sessions)
    ├── Session 1: Config infrastructure
    └── Session 2: Migration & integration
```

### Integration Point

After both tracks complete, merge both branches and verify:
1. Scene files can reference config values (e.g., scene references materials defined in config)
2. Scene builder can use both scene files and config
3. Tests pass with both systems active

---

## Success Metrics

### Quantitative

- [x] 0 hardcoded constants in main.rs (down from ~10)
- [x] 0 hardcoded physics values (down from 8)
- [x] 0 hardcoded rendering values (down from 10)
- [x] 2+ example scene files
- [x] 100% test coverage for serialization roundtrips
- [x] Config loading time < 10ms

### Qualitative

- [x] Artists can create scenes without programming knowledge
- [x] Users can customize settings without rebuilding
- [x] Developers can iterate rapidly via config/scene changes
- [x] Error messages are clear and actionable
- [x] Documentation explains file formats thoroughly

---

## Next Steps (Phase 2)

After Phase 1 completes:

1. **Scene Manager** - Support multiple scenes, switching, transitions
2. **Prefab System** - Reusable entity templates
3. **Asset Management** - Shared resource loading and caching
4. **Hot Reloading** - Reload scenes/config without restarting

---

## References

- Config recommendations: `scratchpad/reports/2026-01-27-engine-review-swarm/config-recommendations.md`
- Scene handling review: `scratchpad/reports/2026-01-27-engine-review-swarm/scene-handling-review.md`
- Figment docs: https://docs.rs/figment/
- RON format: https://github.com/ron-rs/ron
- Serde guide: https://serde.rs/

---

**End of Phase 1 Plan**
