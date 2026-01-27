# Phase 2: Scene Management

**Created:** 2026-01-27
**Status:** Ready
**Sessions:** 3 (1 for 2A, 2 for 2B)
**Priority:** P1 - High

---

## Overview

Phase 2 adds multi-scene support and a prefab system to Rust4D. This phase enables scene switching (menus, levels, overlays) and runtime entity spawning from templates.

**Key Goals:**
- Scene stack with push/pop/switch operations
- SceneManager singleton for managing multiple scenes
- Prefab system for entity templates
- RON-based prefab files with override support

**What This Enables:**
- Multi-level games with scene transitions
- Menu systems (main menu, pause overlay, settings)
- Runtime entity spawning (enemies, projectiles, collectibles)
- Reusable entity templates
- Scene composition and overlays

---

## Dependencies

**Blocks Phase 2:**
- **Phase 1A (Scene Serialization)** - REQUIRED
  - Needs: `Scene` struct with RON serialization
  - Needs: Serializable `Entity`, `World`, `Transform4D`, `Material`
  - Needs: `ShapeTemplate` enum for serialization
  - Needs: Working `Scene::load()` and `Scene::save()`

**Blocked by Phase 2:**
- Phase 5 features that spawn entities (particle systems, enemies)
- Advanced scene transitions and effects

**Parallelization:**
- Phase 2A can run in parallel with Phase 3A (Documentation - Examples)
- Phase 2B can run in parallel with Phase 3B (Documentation - Guides)

---

## Phase 2A: Scene Manager (1 session)

### Overview

Create a SceneManager singleton that manages a stack of active scenes. Supports push (overlay), pop (dismiss), and switch (replace) operations.

### Architecture

```
┌─────────────────────────────────────────┐
│       SceneManager (Singleton)          │
│  - scenes: HashMap<String, Scene>       │
│  - active_stack: Vec<String>            │
│  - transition: Option<SceneTransition>  │
│                                         │
│  + load_scene(path) -> Result           │
│  + unload_scene(name)                   │
│  + push_scene(name)    // Overlay       │
│  + pop_scene() -> Option<String>        │
│  + switch_to(name)     // Replace top   │
│  + active_scene() -> &Scene             │
│  + active_scene_mut() -> &mut Scene     │
│  + update(dt)                           │
└─────────────────────────────────────────┘
              │ owns multiple
              ▼
┌─────────────────────────────────────────┐
│              Scene                      │
│  - metadata: SceneMetadata              │
│  - world: World                         │
│                                         │
│  + load(path) -> Result<Scene>          │
│  + save(&self, path) -> Result<()>      │
│  + update(&mut self, dt)                │
└─────────────────────────────────────────┘
```

### Usage Patterns

#### Single Scene (Basic)

```rust
// Load and activate a single scene
let mut scene_manager = SceneManager::new();
scene_manager.load_scene("scenes/level_1.ron")?;
scene_manager.switch_to("level_1");

// Update loop
loop {
    scene_manager.update(dt);
    // Render active scene
}
```

#### Scene Transitions (Levels)

```rust
// Switch between levels
scene_manager.load_scene("scenes/level_1.ron")?;
scene_manager.load_scene("scenes/level_2.ron")?;

// Start with level 1
scene_manager.switch_to("level_1");

// Later: switch to level 2 (unloads level 1 from active stack)
scene_manager.switch_to("level_2");
```

#### Scene Stack (Overlays)

```rust
// Gameplay scene is active
scene_manager.switch_to("gameplay");

// Player pauses - push pause menu on top
scene_manager.push_scene("pause_menu");
// Stack is now: [gameplay, pause_menu]
// Only pause_menu updates/renders

// Player unpauses - pop pause menu
scene_manager.pop_scene();
// Stack is now: [gameplay]
```

#### Multiple Overlays

```rust
// Main menu -> Settings -> Controls
scene_manager.switch_to("main_menu");     // [main_menu]
scene_manager.push_scene("settings");     // [main_menu, settings]
scene_manager.push_scene("controls");     // [main_menu, settings, controls]

// Back button: pop controls
scene_manager.pop_scene();                // [main_menu, settings]

// Back button: pop settings
scene_manager.pop_scene();                // [main_menu]
```

### Tasks

#### Task 2A.1: Create SceneManager struct (30 min)

**File:** `crates/rust4d_core/src/scene_manager.rs` (NEW)

```rust
use std::collections::HashMap;
use crate::{Scene, SceneError};

/// Manages multiple scenes and the active scene stack
pub struct SceneManager {
    /// All loaded scenes (by name)
    scenes: HashMap<String, Scene>,

    /// Stack of active scene names (top = currently active)
    /// Allows overlays: [gameplay, pause_menu, settings]
    active_stack: Vec<String>,

    /// Optional scene transition state (for fade effects, etc.)
    transition: Option<SceneTransition>,
}

/// Transition state for scene changes (future use)
#[derive(Debug, Clone)]
pub struct SceneTransition {
    pub from: Option<String>,
    pub to: String,
    pub progress: f32,  // 0.0 to 1.0
}

impl SceneManager {
    /// Create a new empty scene manager
    pub fn new() -> Self {
        Self {
            scenes: HashMap::new(),
            active_stack: Vec::new(),
            transition: None,
        }
    }

    /// Load a scene from a file and store it by name
    ///
    /// The scene name is taken from the scene metadata.
    pub fn load_scene(&mut self, path: &str) -> Result<String, SceneError> {
        let scene = Scene::load(path)?;
        let name = scene.metadata.name.clone();
        self.scenes.insert(name.clone(), scene);
        Ok(name)
    }

    /// Unload a scene by name
    ///
    /// Returns the scene if it existed. If the scene is in the active stack,
    /// it is removed from the stack first.
    pub fn unload_scene(&mut self, name: &str) -> Option<Scene> {
        // Remove from active stack if present
        self.active_stack.retain(|n| n != name);

        // Remove and return the scene
        self.scenes.remove(name)
    }

    /// Push a scene onto the active stack (overlay)
    ///
    /// The new scene becomes active, but the previous scene remains loaded.
    /// Use this for overlays like pause menus.
    pub fn push_scene(&mut self, name: &str) -> Result<(), SceneError> {
        if !self.scenes.contains_key(name) {
            return Err(SceneError::NotLoaded(name.to_string()));
        }
        self.active_stack.push(name.to_string());
        Ok(())
    }

    /// Pop the top scene from the active stack
    ///
    /// Returns the name of the popped scene, or None if the stack is empty.
    pub fn pop_scene(&mut self) -> Option<String> {
        self.active_stack.pop()
    }

    /// Switch to a scene (replace the top of the stack)
    ///
    /// If the stack is empty, the scene is pushed.
    /// If the stack is not empty, the top scene is replaced.
    pub fn switch_to(&mut self, name: &str) -> Result<(), SceneError> {
        if !self.scenes.contains_key(name) {
            return Err(SceneError::NotLoaded(name.to_string()));
        }

        if self.active_stack.is_empty() {
            self.active_stack.push(name.to_string());
        } else {
            let last = self.active_stack.len() - 1;
            self.active_stack[last] = name.to_string();
        }

        Ok(())
    }

    /// Get a reference to the active scene (top of stack)
    pub fn active_scene(&self) -> Option<&Scene> {
        let name = self.active_stack.last()?;
        self.scenes.get(name)
    }

    /// Get a mutable reference to the active scene (top of stack)
    pub fn active_scene_mut(&mut self) -> Option<&mut Scene> {
        let name = self.active_stack.last()?.clone();
        self.scenes.get_mut(&name)
    }

    /// Get a reference to a scene by name
    pub fn get_scene(&self, name: &str) -> Option<&Scene> {
        self.scenes.get(name)
    }

    /// Get a mutable reference to a scene by name
    pub fn get_scene_mut(&mut self, name: &str) -> Option<&mut Scene> {
        self.scenes.get_mut(name)
    }

    /// Get the active scene stack (for debugging/UI)
    pub fn active_stack(&self) -> &[String] {
        &self.active_stack
    }

    /// Check if a scene is loaded
    pub fn is_loaded(&self, name: &str) -> bool {
        self.scenes.contains_key(name)
    }

    /// Check if a scene is active
    pub fn is_active(&self, name: &str) -> bool {
        self.active_stack.last().map_or(false, |n| n == name)
    }

    /// Update the active scene
    ///
    /// Only the top scene in the stack is updated.
    pub fn update(&mut self, dt: f32) {
        if let Some(scene) = self.active_scene_mut() {
            scene.update(dt);
        }
    }
}

impl Default for SceneManager {
    fn default() -> Self {
        Self::new()
    }
}
```

**Changes:**
- Add `pub mod scene_manager;` to `crates/rust4d_core/src/lib.rs`
- Add `pub use scene_manager::{SceneManager, SceneTransition};` to exports

#### Task 2A.2: Add SceneError variants (10 min)

**File:** `crates/rust4d_core/src/scene.rs` (modify existing)

Add new error variants for scene management:

```rust
#[derive(Debug, thiserror::Error)]
pub enum SceneError {
    #[error("Failed to load scene: {0}")]
    LoadError(String),

    #[error("Failed to save scene: {0}")]
    SaveError(String),

    #[error("Scene not loaded: {0}")]
    NotLoaded(String),

    #[error("Scene already loaded: {0}")]
    AlreadyLoaded(String),

    #[error("No active scene")]
    NoActiveScene,

    #[error("Serialization error: {0}")]
    SerializationError(#[from] ron::Error),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}
```

#### Task 2A.3: Add Scene::update method (10 min)

**File:** `crates/rust4d_core/src/scene.rs` (modify existing)

```rust
impl Scene {
    /// Update the scene (physics, systems)
    pub fn update(&mut self, dt: f32) {
        self.world.update(dt);
    }
}
```

#### Task 2A.4: Update main.rs to use SceneManager (20 min)

**File:** `src/main.rs`

Replace direct World construction with SceneManager:

```rust
use rust4d_core::{SceneManager, SceneError};

struct App {
    scene_manager: SceneManager,
    // ... other fields
}

impl App {
    fn new() -> Result<Self, SceneError> {
        let mut scene_manager = SceneManager::new();

        // Load initial scene (or create programmatically for now)
        let initial_scene = create_test_scene();
        scene_manager.scenes.insert("main".to_string(), initial_scene);
        scene_manager.switch_to("main")?;

        Ok(Self {
            scene_manager,
            // ... initialize other fields
        })
    }

    fn update(&mut self, dt: f32) {
        self.scene_manager.update(dt);

        // Get active world for rendering
        if let Some(scene) = self.scene_manager.active_scene() {
            // Update camera based on player position
            // ... existing camera logic
        }
    }
}

// Temporary: programmatic scene creation until Scene::load works
fn create_test_scene() -> Scene {
    use rust4d_core::{Scene, SceneMetadata};
    use crate::scene::SceneBuilder;

    let world = SceneBuilder::with_capacity(2)
        .with_physics(GRAVITY)
        .add_floor(FLOOR_Y, 10.0, PhysicsMaterial::CONCRETE)
        .add_player(player_start, 0.5)
        .add_tesseract(Vec4::ZERO, 2.0, "tesseract")
        .build();

    Scene {
        metadata: SceneMetadata {
            name: "main".to_string(),
            version: "1.0.0".to_string(),
            author: Some("Willow".to_string()),
            description: Some("Test scene".to_string()),
        },
        world,
    }
}
```

#### Task 2A.5: Add tests for SceneManager (30 min)

**File:** `crates/rust4d_core/src/scene_manager.rs`

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Scene, SceneMetadata, World};

    fn make_test_scene(name: &str) -> Scene {
        Scene {
            metadata: SceneMetadata {
                name: name.to_string(),
                version: "1.0.0".to_string(),
                author: None,
                description: None,
            },
            world: World::new(),
        }
    }

    #[test]
    fn test_scene_manager_new() {
        let manager = SceneManager::new();
        assert!(manager.active_scene().is_none());
        assert_eq!(manager.active_stack().len(), 0);
    }

    #[test]
    fn test_push_scene() {
        let mut manager = SceneManager::new();
        manager.scenes.insert("scene1".to_string(), make_test_scene("scene1"));

        manager.push_scene("scene1").unwrap();
        assert_eq!(manager.active_stack().len(), 1);
        assert!(manager.is_active("scene1"));
    }

    #[test]
    fn test_pop_scene() {
        let mut manager = SceneManager::new();
        manager.scenes.insert("scene1".to_string(), make_test_scene("scene1"));
        manager.scenes.insert("scene2".to_string(), make_test_scene("scene2"));

        manager.push_scene("scene1").unwrap();
        manager.push_scene("scene2").unwrap();

        assert_eq!(manager.active_stack().len(), 2);

        let popped = manager.pop_scene();
        assert_eq!(popped, Some("scene2".to_string()));
        assert_eq!(manager.active_stack().len(), 1);
        assert!(manager.is_active("scene1"));
    }

    #[test]
    fn test_switch_to() {
        let mut manager = SceneManager::new();
        manager.scenes.insert("scene1".to_string(), make_test_scene("scene1"));
        manager.scenes.insert("scene2".to_string(), make_test_scene("scene2"));

        manager.switch_to("scene1").unwrap();
        assert!(manager.is_active("scene1"));

        manager.switch_to("scene2").unwrap();
        assert!(manager.is_active("scene2"));
        assert_eq!(manager.active_stack().len(), 1);  // Replaced, not pushed
    }

    #[test]
    fn test_scene_stack_overlay() {
        let mut manager = SceneManager::new();
        manager.scenes.insert("gameplay".to_string(), make_test_scene("gameplay"));
        manager.scenes.insert("pause".to_string(), make_test_scene("pause"));

        // Start gameplay
        manager.switch_to("gameplay").unwrap();

        // Overlay pause menu
        manager.push_scene("pause").unwrap();

        assert_eq!(manager.active_stack(), &["gameplay", "pause"]);
        assert!(manager.is_active("pause"));

        // Resume
        manager.pop_scene();
        assert!(manager.is_active("gameplay"));
    }

    #[test]
    fn test_unload_scene() {
        let mut manager = SceneManager::new();
        manager.scenes.insert("scene1".to_string(), make_test_scene("scene1"));

        manager.push_scene("scene1").unwrap();
        assert!(manager.is_loaded("scene1"));

        manager.unload_scene("scene1");
        assert!(!manager.is_loaded("scene1"));
        assert_eq!(manager.active_stack().len(), 0);  // Removed from stack
    }

    #[test]
    fn test_scene_not_loaded_error() {
        let mut manager = SceneManager::new();

        let result = manager.push_scene("nonexistent");
        assert!(result.is_err());
    }
}
```

### Files to Create/Modify

| File | Action | Description |
|------|--------|-------------|
| `crates/rust4d_core/src/scene_manager.rs` | CREATE | SceneManager singleton |
| `crates/rust4d_core/src/scene.rs` | MODIFY | Add SceneError variants, Scene::update |
| `crates/rust4d_core/src/lib.rs` | MODIFY | Export SceneManager |
| `src/main.rs` | MODIFY | Use SceneManager instead of direct World |

### Testing Requirements

**Unit Tests (in scene_manager.rs):**
- ✅ Create empty SceneManager
- ✅ Push scene to stack
- ✅ Pop scene from stack
- ✅ Switch scene (replace top)
- ✅ Scene stack overlay pattern (gameplay + pause)
- ✅ Unload scene removes from stack
- ✅ Error when pushing non-loaded scene
- ✅ Active scene retrieval
- ✅ Scene query methods (is_loaded, is_active)

**Integration Test:**
- Create SceneManager, load scene, switch between scenes
- Verify only active scene updates

### Success Criteria

- ✅ SceneManager compiles without errors
- ✅ All unit tests pass
- ✅ Can push/pop/switch scenes
- ✅ Active scene updates correctly
- ✅ main.rs uses SceneManager successfully
- ✅ Scene stack behavior works (overlays)

---

## Phase 2B: Prefab System (2 sessions)

### Overview

Create a prefab (template) system for defining reusable entity configurations in RON files. Prefabs can be instantiated into scenes with optional property overrides.

### Architecture

```
┌─────────────────────────────────────────┐
│           Prefab                        │
│  - name: String                         │
│  - template: EntityTemplate             │
│  - children: Vec<Prefab>                │
│                                         │
│  + load(path) -> Result<Prefab>         │
│  + instantiate(world) -> EntityKey      │
│  + instantiate_with(world, overrides)   │
└─────────────────────────────────────────┘
                  │ contains
                  ▼
┌─────────────────────────────────────────┐
│        EntityTemplate                   │
│  - name: Option<String>                 │
│  - tags: HashSet<String>                │
│  - transform: Transform4D               │
│  - shape: ShapeTemplate                 │
│  - material: Material                   │
│  - physics: Option<PhysicsTemplate>     │
│                                         │
│  + to_entity(world) -> Entity           │
│  + merge_overrides(other) -> Self       │
└─────────────────────────────────────────┘
                  │ uses
                  ▼
┌─────────────────────────────────────────┐
│       PhysicsTemplate                   │
│  Enum:                                  │
│  - Static { collider, material }        │
│  - Dynamic { mass, material }           │
│  - Kinematic { material }               │
│                                         │
│  + to_physics_body() -> RigidBody4D     │
└─────────────────────────────────────────┘
```

### Example Prefab RON Files

#### Simple Prefab: Wooden Crate

**File:** `prefabs/wooden_crate.ron`

```ron
Prefab(
    name: "wooden_crate",
    template: (
        name: Some("crate"),
        tags: ["dynamic", "interactable"],
        transform: (
            position: (0.0, 0.0, 0.0, 0.0),
            rotation: Identity,
            scale: (1.0, 1.0, 1.0, 1.0),
        ),
        shape: Tesseract(size: 1.0),
        material: (
            base_color: (0.6, 0.4, 0.2, 1.0),  // Brown wood color
        ),
        physics: Some(Dynamic(
            mass: 5.0,
            material: Wood,
        )),
    ),
    children: [],
)
```

**Usage:**

```rust
// Load prefab
let crate_prefab = Prefab::load("prefabs/wooden_crate.ron")?;

// Instantiate at specific position
let overrides = EntityTemplate {
    transform: Transform4D::from_position(Vec4::new(5.0, 0.0, 0.0, 0.0)),
    ..Default::default()
};
let crate_entity = crate_prefab.instantiate_with(&mut world, &overrides);

// Or instantiate at origin
let another_crate = crate_prefab.instantiate(&mut world);
```

#### Complex Prefab: Enemy Cube

**File:** `prefabs/enemy_cube.ron`

```ron
Prefab(
    name: "enemy_cube",
    template: (
        name: Some("enemy"),
        tags: ["dynamic", "enemy", "ai"],
        transform: (
            position: (0.0, 0.0, 0.0, 0.0),
        ),
        shape: Tesseract(size: 1.0),
        material: (
            base_color: (1.0, 0.0, 0.0, 1.0),  // Red
        ),
        physics: Some(Dynamic(
            mass: 3.0,
            material: Metal,
        )),
    ),
    children: [],
)
```

#### Prefab with Variants

```rust
// Spawn red enemy
let enemy = enemy_prefab.instantiate(&mut world);

// Spawn blue variant
let blue_overrides = EntityTemplate {
    material: Material::new([0.0, 0.0, 1.0, 1.0]),
    ..Default::default()
};
let blue_enemy = enemy_prefab.instantiate_with(&mut world, &blue_overrides);

// Spawn giant variant
let giant_overrides = EntityTemplate {
    transform: Transform4D::from_position(Vec4::new(10.0, 0.0, 0.0, 0.0))
        .with_scale(Vec4::new(2.0, 2.0, 2.0, 2.0)),
    physics: Some(PhysicsTemplate::Dynamic {
        mass: 20.0,  // Heavier
        material: PhysicsMaterial::METAL,
    }),
    ..Default::default()
};
let giant_enemy = enemy_prefab.instantiate_with(&mut world, &giant_overrides);
```

### Tasks

#### Task 2B.1: Create EntityTemplate struct (45 min)

**File:** `crates/rust4d_core/src/entity_template.rs` (NEW)

```rust
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use crate::{Entity, Material, Transform4D, ShapeRef};
use rust4d_math::ShapeTemplate;
use rust4d_physics::PhysicsMaterial;

/// Template for creating entities
///
/// Similar to Entity but used for serialization and as a blueprint.
/// Can be instantiated multiple times with different overrides.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EntityTemplate {
    #[serde(default)]
    pub name: Option<String>,

    #[serde(default)]
    pub tags: HashSet<String>,

    #[serde(default)]
    pub transform: Transform4D,

    pub shape: ShapeTemplate,

    #[serde(default)]
    pub material: Material,

    #[serde(default)]
    pub physics: Option<PhysicsTemplate>,
}

/// Physics body template for serialization
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum PhysicsTemplate {
    /// Static collider (walls, floors)
    Static {
        material: PhysicsMaterial,
    },

    /// Dynamic body (affected by forces)
    Dynamic {
        mass: f32,
        material: PhysicsMaterial,
    },

    /// Kinematic body (user-controlled, no physics forces)
    Kinematic {
        material: PhysicsMaterial,
    },
}

impl EntityTemplate {
    /// Convert template to an entity (without physics)
    ///
    /// Physics bodies must be created separately and linked to the entity.
    pub fn to_entity(&self) -> Entity {
        let shape = self.shape.to_shape_ref();

        let mut entity = Entity::new(shape)
            .with_transform(self.transform.clone())
            .with_material(self.material.clone());

        if let Some(ref name) = self.name {
            entity = entity.with_name(name);
        }

        for tag in &self.tags {
            entity = entity.with_tag(tag);
        }

        entity
    }

    /// Merge overrides into this template
    ///
    /// Fields in `overrides` replace fields in `self`.
    /// Uses Option fields to detect which values to override.
    pub fn merge_overrides(&self, overrides: &EntityTemplateOverrides) -> Self {
        let mut result = self.clone();

        if let Some(ref name) = overrides.name {
            result.name = Some(name.clone());
        }

        if let Some(ref tags) = overrides.tags {
            result.tags = tags.clone();
        }

        if let Some(ref transform) = overrides.transform {
            result.transform = transform.clone();
        }

        if let Some(ref shape) = overrides.shape {
            result.shape = shape.clone();
        }

        if let Some(ref material) = overrides.material {
            result.material = material.clone();
        }

        if let Some(ref physics) = overrides.physics {
            result.physics = physics.clone();
        }

        result
    }
}

/// Optional overrides for entity templates
///
/// All fields are Option so we can detect which values to override.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct EntityTemplateOverrides {
    #[serde(default)]
    pub name: Option<String>,

    #[serde(default)]
    pub tags: Option<HashSet<String>>,

    #[serde(default)]
    pub transform: Option<Transform4D>,

    #[serde(default)]
    pub shape: Option<ShapeTemplate>,

    #[serde(default)]
    pub material: Option<Material>,

    #[serde(default)]
    pub physics: Option<PhysicsTemplate>,
}

impl Default for EntityTemplate {
    fn default() -> Self {
        Self {
            name: None,
            tags: HashSet::new(),
            transform: Transform4D::default(),
            shape: ShapeTemplate::Tesseract { size: 1.0 },
            material: Material::WHITE,
            physics: None,
        }
    }
}
```

**Changes:**
- Add `pub mod entity_template;` to `crates/rust4d_core/src/lib.rs`
- Add `pub use entity_template::{EntityTemplate, EntityTemplateOverrides, PhysicsTemplate};`

#### Task 2B.2: Create ShapeTemplate enum (30 min)

**File:** `crates/rust4d_math/src/shape_template.rs` (NEW)

```rust
use serde::{Deserialize, Serialize};
use crate::{Tesseract4D, Hyperplane4D, ShapeRef};

/// Serializable shape template
///
/// Used for scene/prefab serialization. Trait objects (dyn Shape4D) can't be
/// directly serialized, so we use an enum of known shape types.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ShapeTemplate {
    /// 4D hypercube (tesseract)
    Tesseract {
        size: f32,
    },

    /// 3D hyperplane (infinite flat surface in 4D)
    Hyperplane {
        y: f32,
        size: f32,
        subdivisions: usize,
        cell_size: f32,
        thickness: f32,
    },
}

impl ShapeTemplate {
    /// Convert template to a ShapeRef (owned shape)
    pub fn to_shape_ref(&self) -> ShapeRef {
        match self {
            ShapeTemplate::Tesseract { size } => {
                ShapeRef::shared(Tesseract4D::new(*size))
            }
            ShapeTemplate::Hyperplane { y, size, subdivisions, cell_size, thickness } => {
                ShapeRef::shared(Hyperplane4D::new(*y, *size, *subdivisions, *cell_size, *thickness))
            }
        }
    }
}

impl From<&Tesseract4D> for ShapeTemplate {
    fn from(t: &Tesseract4D) -> Self {
        ShapeTemplate::Tesseract {
            size: t.size,
        }
    }
}

impl From<&Hyperplane4D> for ShapeTemplate {
    fn from(h: &Hyperplane4D) -> Self {
        ShapeTemplate::Hyperplane {
            y: h.y,
            size: h.size,
            subdivisions: h.subdivisions,
            cell_size: h.cell_size,
            thickness: h.thickness,
        }
    }
}
```

**Changes:**
- Add `pub mod shape_template;` to `crates/rust4d_math/src/lib.rs`
- Add `pub use shape_template::ShapeTemplate;`

#### Task 2B.3: Create Prefab struct (45 min)

**File:** `crates/rust4d_core/src/prefab.rs` (NEW)

```rust
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::fs;
use crate::{EntityTemplate, EntityTemplateOverrides, World, EntityKey};

/// A prefab (template) for creating entities
///
/// Prefabs define reusable entity configurations that can be instantiated
/// multiple times with optional overrides.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Prefab {
    /// Prefab name/identifier
    pub name: String,

    /// Entity template
    pub template: EntityTemplate,

    /// Child prefabs (for nested hierarchies)
    #[serde(default)]
    pub children: Vec<Prefab>,
}

#[derive(Debug, thiserror::Error)]
pub enum PrefabError {
    #[error("Failed to load prefab: {0}")]
    LoadError(String),

    #[error("Failed to save prefab: {0}")]
    SaveError(String),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] ron::Error),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

impl Prefab {
    /// Create a new prefab from a template
    pub fn new(name: String, template: EntityTemplate) -> Self {
        Self {
            name,
            template,
            children: Vec::new(),
        }
    }

    /// Load a prefab from a RON file
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self, PrefabError> {
        let content = fs::read_to_string(path)?;
        let prefab = ron::from_str(&content)?;
        Ok(prefab)
    }

    /// Save the prefab to a RON file
    pub fn save<P: AsRef<Path>>(&self, path: P) -> Result<(), PrefabError> {
        let content = ron::ser::to_string_pretty(self, ron::ser::PrettyConfig::default())?;
        fs::write(path, content)?;
        Ok(())
    }

    /// Instantiate this prefab into a world at the origin
    pub fn instantiate(&self, world: &mut World) -> EntityKey {
        self.instantiate_with(world, &EntityTemplateOverrides::default())
    }

    /// Instantiate this prefab with property overrides
    pub fn instantiate_with(
        &self,
        world: &mut World,
        overrides: &EntityTemplateOverrides,
    ) -> EntityKey {
        // Merge overrides into template
        let final_template = self.template.merge_overrides(overrides);

        // Convert template to entity (without physics yet)
        let mut entity = final_template.to_entity();

        // Create physics body if specified
        if let Some(ref physics_template) = final_template.physics {
            if let Some(physics_world) = world.physics_mut() {
                use rust4d_physics::{RigidBody4D, BodyType};

                let body = match physics_template {
                    crate::PhysicsTemplate::Static { material } => {
                        // For static entities, we typically use StaticCollider
                        // This is a simplified version - real implementation needs proper collider shapes
                        RigidBody4D::new_sphere(final_template.transform.position, 0.5)
                            .with_body_type(BodyType::Static)
                            .with_material(*material)
                    }
                    crate::PhysicsTemplate::Dynamic { mass, material } => {
                        // Create dynamic body from shape
                        // TODO: Extract size from ShapeTemplate for proper AABB/sphere
                        RigidBody4D::new_sphere(final_template.transform.position, 0.5)
                            .with_body_type(BodyType::Dynamic)
                            .with_mass(*mass)
                            .with_material(*material)
                    }
                    crate::PhysicsTemplate::Kinematic { material } => {
                        RigidBody4D::new_sphere(final_template.transform.position, 0.5)
                            .with_body_type(BodyType::Kinematic)
                            .with_material(*material)
                    }
                };

                let body_key = physics_world.add_body(body);
                entity = entity.with_physics_body(body_key);
            }
        }

        // Add entity to world
        world.add_entity(entity)

        // TODO: Instantiate children (nested prefabs)
    }
}
```

**Changes:**
- Add `pub mod prefab;` to `crates/rust4d_core/src/lib.rs`
- Add `pub use prefab::{Prefab, PrefabError};`

#### Task 2B.4: Add prefab registry to Scene (20 min)

**File:** `crates/rust4d_core/src/scene.rs` (modify)

```rust
use std::collections::HashMap;
use crate::{Prefab, PrefabError, EntityKey};

pub struct Scene {
    pub metadata: SceneMetadata,
    pub world: World,

    /// Prefabs registered in this scene
    #[serde(default)]
    pub prefabs: HashMap<String, Prefab>,
}

impl Scene {
    /// Register a prefab in this scene
    pub fn register_prefab(&mut self, prefab: Prefab) {
        self.prefabs.insert(prefab.name.clone(), prefab);
    }

    /// Load and register a prefab from a file
    pub fn load_prefab<P: AsRef<Path>>(&mut self, path: P) -> Result<String, PrefabError> {
        let prefab = Prefab::load(path)?;
        let name = prefab.name.clone();
        self.register_prefab(prefab);
        Ok(name)
    }

    /// Instantiate a registered prefab by name
    pub fn instantiate_prefab(&mut self, name: &str) -> Result<EntityKey, PrefabError> {
        let prefab = self.prefabs.get(name)
            .ok_or_else(|| PrefabError::LoadError(format!("Prefab not found: {}", name)))?
            .clone();

        Ok(prefab.instantiate(&mut self.world))
    }

    /// Instantiate a prefab with overrides
    pub fn instantiate_prefab_with(
        &mut self,
        name: &str,
        overrides: &EntityTemplateOverrides,
    ) -> Result<EntityKey, PrefabError> {
        let prefab = self.prefabs.get(name)
            .ok_or_else(|| PrefabError::LoadError(format!("Prefab not found: {}", name)))?
            .clone();

        Ok(prefab.instantiate_with(&mut self.world, overrides))
    }
}
```

#### Task 2B.5: Create example prefab files (30 min)

**Directory:** `prefabs/` (NEW)

Create three example prefabs:

1. **`prefabs/wooden_crate.ron`** - Simple dynamic object
2. **`prefabs/enemy_cube.ron`** - Enemy template
3. **`prefabs/metal_wall.ron`** - Static obstacle

(Content as shown in "Example Prefab RON Files" section above)

#### Task 2B.6: Add prefab tests (45 min)

**File:** `crates/rust4d_core/src/prefab.rs`

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Material, World};
    use rust4d_math::{Vec4, Transform4D, ShapeTemplate};

    fn make_test_template() -> EntityTemplate {
        EntityTemplate {
            name: Some("test".to_string()),
            tags: ["test".to_string()].into_iter().collect(),
            transform: Transform4D::default(),
            shape: ShapeTemplate::Tesseract { size: 1.0 },
            material: Material::WHITE,
            physics: None,
        }
    }

    #[test]
    fn test_prefab_new() {
        let template = make_test_template();
        let prefab = Prefab::new("test_prefab".to_string(), template);

        assert_eq!(prefab.name, "test_prefab");
        assert_eq!(prefab.children.len(), 0);
    }

    #[test]
    fn test_prefab_instantiate() {
        let template = make_test_template();
        let prefab = Prefab::new("test".to_string(), template);

        let mut world = World::new();
        let entity_key = prefab.instantiate(&mut world);

        assert!(world.get_entity(entity_key).is_some());
        assert_eq!(world.entity_count(), 1);
    }

    #[test]
    fn test_prefab_instantiate_with_overrides() {
        let template = make_test_template();
        let prefab = Prefab::new("test".to_string(), template);

        let overrides = EntityTemplateOverrides {
            transform: Some(Transform4D::from_position(Vec4::new(5.0, 0.0, 0.0, 0.0))),
            material: Some(Material::RED),
            ..Default::default()
        };

        let mut world = World::new();
        let entity_key = prefab.instantiate_with(&mut world, &overrides);

        let entity = world.get_entity(entity_key).unwrap();
        assert_eq!(entity.transform.position.x, 5.0);
        assert_eq!(entity.material.base_color, [1.0, 0.0, 0.0, 1.0]);
    }

    #[test]
    fn test_prefab_serialization() {
        let template = make_test_template();
        let prefab = Prefab::new("test".to_string(), template);

        // Serialize to RON
        let ron_string = ron::ser::to_string_pretty(&prefab, Default::default()).unwrap();

        // Deserialize back
        let deserialized: Prefab = ron::from_str(&ron_string).unwrap();

        assert_eq!(deserialized.name, "test");
    }

    #[test]
    fn test_scene_register_prefab() {
        use crate::{Scene, SceneMetadata};

        let mut scene = Scene {
            metadata: SceneMetadata {
                name: "test".to_string(),
                version: "1.0.0".to_string(),
                author: None,
                description: None,
            },
            world: World::new(),
            prefabs: HashMap::new(),
        };

        let template = make_test_template();
        let prefab = Prefab::new("crate".to_string(), template);

        scene.register_prefab(prefab);
        assert!(scene.prefabs.contains_key("crate"));
    }

    #[test]
    fn test_scene_instantiate_prefab() {
        use crate::{Scene, SceneMetadata};

        let mut scene = Scene {
            metadata: SceneMetadata {
                name: "test".to_string(),
                version: "1.0.0".to_string(),
                author: None,
                description: None,
            },
            world: World::new(),
            prefabs: HashMap::new(),
        };

        let template = make_test_template();
        let prefab = Prefab::new("crate".to_string(), template);
        scene.register_prefab(prefab);

        let entity_key = scene.instantiate_prefab("crate").unwrap();
        assert!(scene.world.get_entity(entity_key).is_some());
    }
}
```

### Files to Create/Modify

| File | Action | Description |
|------|--------|-------------|
| `crates/rust4d_core/src/entity_template.rs` | CREATE | EntityTemplate, PhysicsTemplate |
| `crates/rust4d_math/src/shape_template.rs` | CREATE | ShapeTemplate enum |
| `crates/rust4d_core/src/prefab.rs` | CREATE | Prefab struct, load/save/instantiate |
| `crates/rust4d_core/src/scene.rs` | MODIFY | Add prefab registry |
| `crates/rust4d_core/src/lib.rs` | MODIFY | Export prefab types |
| `crates/rust4d_math/src/lib.rs` | MODIFY | Export ShapeTemplate |
| `prefabs/wooden_crate.ron` | CREATE | Example prefab |
| `prefabs/enemy_cube.ron` | CREATE | Example prefab |
| `prefabs/metal_wall.ron` | CREATE | Example prefab |

### Testing Requirements

**Unit Tests (in prefab.rs):**
- ✅ Create prefab from template
- ✅ Instantiate prefab at origin
- ✅ Instantiate with transform override
- ✅ Instantiate with material override
- ✅ Prefab serialization roundtrip
- ✅ Scene registers prefab
- ✅ Scene instantiates prefab by name
- ✅ Error when instantiating non-existent prefab

**Integration Test:**
- Load prefab from file
- Instantiate multiple copies with different overrides
- Verify physics bodies created correctly

**Example Validation:**
- All example prefab files load without errors
- Each prefab can be instantiated into a world

### Success Criteria

- ✅ Prefab system compiles without errors
- ✅ All unit tests pass
- ✅ Can load prefabs from RON files
- ✅ Can instantiate prefabs with overrides
- ✅ Example prefabs load and instantiate correctly
- ✅ Scene prefab registry works
- ✅ ShapeTemplate converts to ShapeRef correctly
- ✅ PhysicsTemplate creates physics bodies

---

## Risks and Mitigations

### Risk 1: Physics Body Creation Complexity

**Problem:** Creating physics bodies from templates requires determining collider shapes from ShapeTemplate.

**Impact:** Medium - Physics bodies might not match visual shapes

**Mitigation:**
- Start with simplified colliders (spheres/AABBs) for all prefabs
- Add proper collider shape extraction in Phase 4
- Document limitation in code comments

### Risk 2: Nested Prefab Complexity

**Problem:** Nested prefabs (prefabs containing other prefabs) add significant complexity.

**Impact:** Low - Nice to have, but not essential

**Mitigation:**
- Implement basic nested prefab support (children list)
- Defer advanced features (prefab references, lazy loading) to Phase 5
- Test with simple 2-level hierarchies only

### Risk 3: Override Merging Edge Cases

**Problem:** Merging overrides might have unexpected behavior with complex types.

**Impact:** Medium - Could cause confusion

**Mitigation:**
- Use explicit Option fields in EntityTemplateOverrides
- Clear documentation of merge semantics
- Comprehensive tests for override scenarios
- Start with simple overrides (position, color)

### Risk 4: Scene File Format Changes

**Problem:** Adding prefab support changes scene file format, might break existing scenes.

**Impact:** Low - No existing scene files yet

**Mitigation:**
- Make prefabs field optional with `#[serde(default)]`
- Scene files without prefabs remain valid
- Add version checking in Phase 3

---

## Post-Phase 2 Capabilities

After completing Phase 2, Rust4D will support:

**Scene Management:**
- ✅ Load multiple scenes from RON files
- ✅ Switch between scenes (level transitions)
- ✅ Scene stack for overlays (pause menus)
- ✅ Scene metadata (name, version, author)

**Prefab System:**
- ✅ Define entity templates in RON files
- ✅ Instantiate entities from prefabs
- ✅ Override prefab properties (position, color, physics)
- ✅ Register prefabs in scenes
- ✅ Example prefabs (crate, enemy, wall)

**Developer Experience:**
- ✅ Create content without recompiling
- ✅ Iterate quickly on level design
- ✅ Reuse entity configurations
- ✅ Experiment with entity variants

---

## Integration with Future Phases

**Phase 3 (Documentation):**
- Document SceneManager API
- Tutorial: Creating and switching scenes
- Tutorial: Creating custom prefabs
- Example: Multi-level game with pause menu

**Phase 4 (Architecture):**
- Extract scene system to `rust4d_scene` crate
- Improve physics body creation from templates
- Add asset management for prefab resources

**Phase 5 (Advanced Features):**
- Scene transitions with effects (fade, wipe)
- Nested prefab references (prefab contains other prefabs by path)
- Prefab variants system (inheritance)
- Hot reloading for prefabs
- Visual prefab editor

---

## Dependencies to Install

Add to `Cargo.toml`:

```toml
[dependencies]
ron = "0.8"          # Already added in Phase 1A
serde = { version = "1.0", features = ["derive"] }
thiserror = "1.0"
```

---

## Commit Strategy

**Phase 2A commits:**
1. "Add SceneManager singleton with scene stack"
2. "Add Scene::update and SceneError variants"
3. "Update main.rs to use SceneManager"
4. "Add SceneManager tests"

**Phase 2B commits:**
1. "Add EntityTemplate and PhysicsTemplate types"
2. "Add ShapeTemplate enum for serialization"
3. "Add Prefab struct with load/save/instantiate"
4. "Add prefab registry to Scene"
5. "Add example prefab files"
6. "Add prefab system tests"

---

## Success Metrics

**Quantitative:**
- All tests pass (20+ new tests)
- Code compiles without warnings
- Example prefabs load successfully
- Scene switching works in main.rs

**Qualitative:**
- SceneManager API feels natural to use
- Prefab files are human-readable and editable
- Override system is intuitive
- Documentation is clear and actionable

---

**End of Phase 2 Plan**
