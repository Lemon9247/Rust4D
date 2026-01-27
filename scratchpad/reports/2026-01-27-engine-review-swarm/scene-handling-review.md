# Scene Handling Review Report

**Agent:** Scene Agent
**Date:** 2026-01-27
**Task:** Review current scene handling and recommend improvements

---

## Executive Summary

Rust4D currently has a minimal but functional scene handling system based on the newly added `SceneBuilder`. The declarative builder pattern is a good foundation, but the engine lacks critical scene management features needed for a production game engine: scene persistence (loading/saving), scene switching, prefab/template systems, and hierarchical scene organization.

**Current State:** Basic procedural scene construction via builder pattern
**Recommendation:** Evolve toward a hybrid approach combining ECS-inspired flexibility with scene graph hierarchy and file-based scene persistence.

---

## 1. Current Scene Handling Analysis

### 1.1 SceneBuilder Implementation

**Location:** `/home/lemoneater/Projects/Personal/Rust4D/src/scene/scene_builder.rs`

The SceneBuilder provides a fluent API for constructing 4D scenes:

```rust
let world = SceneBuilder::with_capacity(2)
    .with_physics(GRAVITY)
    .add_floor(FLOOR_Y, 10.0, PhysicsMaterial::CONCRETE)
    .add_player(player_start, 0.5)
    .add_tesseract(Vec4::ZERO, 2.0, "tesseract")
    .build();
```

**Strengths:**
- Clean, declarative API that's easy to read and understand
- Type-safe construction with compile-time guarantees
- Good separation of concerns (scene construction vs. runtime)
- Well-tested (254 lines of code, 95 lines of tests)
- Reduces boilerplate in main.rs (from ~55 lines to ~6 lines)

**Limitations:**
- Purely programmatic - no file-based scene persistence
- No scene hierarchy or parent-child relationships
- No prefab/template system for reusable entity configurations
- Limited to a fixed set of entity types (floor, player, tesseract, wall)
- No runtime scene switching or multi-scene support
- Builder is consumed on `build()`, making incremental scene construction awkward

### 1.2 World and Entity System

**Location:** `/home/lemoneater/Projects/Personal/Rust4D/crates/rust4d_core/src/world.rs`

The World uses SlotMap for generational entity keys:

```rust
pub struct World {
    entities: SlotMap<EntityKey, Entity>,
    name_index: HashMap<String, EntityKey>,
    physics_world: Option<PhysicsWorld>,
}
```

**Strengths:**
- Generational keys prevent the ABA problem (stale references)
- Name-based lookup for important entities
- Tag-based filtering (`get_by_tag()`)
- Dirty tracking for efficient rendering updates
- Integrated physics synchronization
- 700 lines of well-tested code (54 tests)

**Limitations:**
- Flat structure - no entity hierarchy or parent-child transforms
- No component-based architecture (entities are monolithic)
- Single world only - no multi-scene support
- No serialization/deserialization capabilities
- Tags are strings (HashSet<String>) - less efficient than bitflags

### 1.3 Entity Structure

**Location:** `/home/lemoneater/Projects/Personal/Rust4D/crates/rust4d_core/src/entity.rs`

```rust
pub struct Entity {
    pub name: Option<String>,
    pub tags: HashSet<String>,
    pub transform: Transform4D,
    pub shape: ShapeRef,
    pub material: Material,
    pub physics_body: Option<BodyKey>,
    dirty: DirtyFlags,
}
```

**Strengths:**
- Simple, cohesive entity representation
- Optional physics integration via `physics_body`
- Dirty tracking for optimization
- Support for named entities and tags

**Limitations:**
- Monolithic design - all entities have the same fields
- No component composition (can't have custom entity types)
- ShapeRef is either Owned or Shared, but no template/prefab system
- No support for entity variants (e.g., light sources, cameras, triggers)
- No way to store custom game-specific data on entities

### 1.4 Usage in main.rs

**Location:** `/home/lemoneater/Projects/Personal/Rust4D/src/main.rs`

Scene construction happens once in `App::new()`:

```rust
fn new() -> Self {
    let player_start = Vec4::new(0.0, 0.0, 5.0, 0.0);
    let world = SceneBuilder::with_capacity(2)
        .with_physics(GRAVITY)
        .add_floor(FLOOR_Y, 10.0, PhysicsMaterial::CONCRETE)
        .add_player(player_start, 0.5)
        .add_tesseract(Vec4::ZERO, 2.0, "tesseract")
        .build();
    // ...
}
```

**Observations:**
- No scene loading from files
- No ability to change scenes at runtime
- Hardcoded scene constants (GRAVITY, FLOOR_Y)
- Scene is built once and never modified structurally (entities aren't added/removed at runtime)

---

## 2. What's Missing

### 2.1 Scene Persistence (Critical)

**Problem:** Scenes are hardcoded in Rust. No way to edit scenes without recompiling.

**Missing Features:**
- Scene serialization (save scene to file)
- Scene deserialization (load scene from file)
- Hot reloading (reload scene while engine is running)
- Asset references (textures, models, physics materials)

**Impact:**
- Artists and designers can't create content without programming
- Iteration time is slow (recompile for every scene change)
- No way to ship multiple levels/scenes with the game

### 2.2 Scene Switching (High Priority)

**Problem:** Only one world exists. No concept of multiple scenes or levels.

**Missing Features:**
- Scene manager to handle multiple scenes
- Ability to load/unload scenes at runtime
- Transition between scenes (e.g., main menu → gameplay → pause menu)
- Scene stacking (e.g., pause menu overlay on gameplay scene)

**Impact:**
- Can't implement menus, level selection, or multi-level games
- All content must exist in a single world
- No support for UI overlays or HUD scenes

### 2.3 Prefab/Template System (High Priority)

**Problem:** No way to define reusable entity templates.

**Missing Features:**
- Prefab definitions (template entities)
- Prefab instantiation (spawn copies from template)
- Prefab variants (override template properties)
- Prefab nesting (prefabs containing other prefabs)

**Impact:**
- Copy-paste code for similar entities (e.g., multiple enemies)
- Hard to maintain consistency across similar entities
- No way to "spawn" entities at runtime from templates

### 2.4 Entity Hierarchy (Medium Priority)

**Problem:** Entities are flat. No parent-child relationships.

**Missing Features:**
- Parent-child transform hierarchy (children inherit parent's transform)
- Relative transforms (position child relative to parent)
- Hierarchical operations (delete parent deletes children)
- Scene graph traversal

**Impact:**
- Can't model complex objects (e.g., robot with movable joints)
- No compound entities (e.g., car with wheels)
- Transform updates are more complex than necessary

### 2.5 Scene File Format (Critical)

**Problem:** No standardized format for scene files.

**Need to Choose:**
- Binary format (fast, compact, not human-readable)
- Text format (slow, verbose, human-readable and editable)
- Which text format: JSON, YAML, RON, TOML?

### 2.6 Component-Based Architecture (Low Priority - Defer)

**Observation:** Current monolithic Entity works for now, but limits extensibility.

**Future Consideration:**
- Move to ECS-style component composition
- Allow custom components for game-specific behavior
- Better separation of rendering, physics, and gameplay data

**Note:** This is a major architectural change and should be deferred until the current design's limitations become painful.

---

## 3. Recommended Scene File Format

After researching modern scene serialization practices, I recommend **RON (Rusty Object Notation)** as the primary scene format for Rust4D.

### 3.1 Format Comparison

| Format | Pros | Cons | Verdict |
|--------|------|------|---------|
| **RON** | Rust-native, type-safe, human-readable, supports Rust types (tuples, enums, structs) | Rust-specific, smaller ecosystem | **RECOMMENDED** |
| **JSON** | Universal, great tooling, widely understood | Verbose, no comments, limited types (no tuples) | Good fallback |
| **YAML** | Very readable, supports comments, anchors/aliases for reuse | Indentation-sensitive, slow parsing, error-prone | Not recommended |
| **Binary** | Fast, compact | Not human-readable, versioning issues | For internal use only |

### 3.2 Why RON?

1. **Rust Integration:** RON is designed for Rust with first-class support for Rust types via serde
2. **Readability:** More readable than JSON for complex nested structures
3. **Type Safety:** Supports enums, tuples, and newtypes that JSON can't represent
4. **Comments:** Supports comments for documentation
5. **Production Use:** Used successfully in Bevy, Amethyst, and other Rust game engines

**Sources:**
- [Scene serialization formats comparison](https://peerdh.com/blogs/programming-insights/comparing-different-serialization-formats-for-game-state-management-1)
- [Serialization for game engines](https://jorenjoestar.github.io/post/serialization_for_games/)

### 3.3 Example RON Scene Format

```ron
Scene(
    metadata: (
        name: "Test Level",
        version: "1.0.0",
        author: "Willow",
    ),

    physics: Some((
        gravity: -20.0,
    )),

    entities: [
        // Floor entity
        (
            name: Some("floor"),
            tags: ["static", "environment"],
            transform: (
                position: (0.0, -2.0, 0.0, 0.0),
                rotation: Identity,
                scale: (1.0, 1.0, 1.0, 1.0),
            ),
            shape: Hyperplane(
                y: -2.0,
                size: 10.0,
                subdivisions: 10,
                cell_size: 5.0,
                thickness: 0.001,
            ),
            material: (
                base_color: (0.5, 0.5, 0.5, 1.0),
            ),
            physics: Some(Static(
                collider: Floor(
                    y: -2.0,
                    material: Concrete,
                ),
            )),
        ),

        // Tesseract entity
        (
            name: Some("tesseract"),
            tags: ["dynamic", "interactable"],
            transform: (
                position: (0.0, 0.0, 0.0, 0.0),
            ),
            shape: Tesseract(size: 2.0),
            material: (
                base_color: (1.0, 1.0, 1.0, 1.0),
            ),
            physics: Some(Dynamic(
                mass: 10.0,
                material: Wood,
            )),
        ),

        // Player spawn point (not a visual entity, just a marker)
        (
            name: Some("player_start"),
            tags: ["spawn_point"],
            transform: (
                position: (0.0, 0.0, 5.0, 0.0),
            ),
            shape: None,
            physics: None,
        ),
    ],

    // Optional: prefab instances
    prefabs: [
        (
            prefab: "assets/prefabs/enemy_cube.ron",
            transform: (position: (5.0, 0.0, 0.0, 0.0)),
            overrides: (
                material: (base_color: (1.0, 0.0, 0.0, 1.0)),
            ),
        ),
    ],
)
```

**Benefits of this format:**
- Human-readable and editable by hand
- Can add comments to document scene elements
- Type-safe deserialization via serde
- Supports both visual entities and logical markers (spawn points)
- Easy to extend with new fields without breaking old scenes

---

## 4. Scene Management Architecture Recommendations

Based on research of modern game engine architectures in 2026, I recommend the following architecture:

**Sources:**
- [Game engine architecture overview](https://grier.hashnode.dev/tinker-engine)
- [Isetta Engine scene architecture](https://isetta.io/blogs/engine-architecture/)
- [Scene system design patterns](https://rivermanmedia.com/object-oriented-game-programming-the-scene-system/)

### 4.1 Architecture Overview

```
┌─────────────────────────────────────────┐
│         SceneManager (Singleton)        │
│  - Owns all scenes                      │
│  - Handles scene transitions            │
│  - Manages active scenes stack          │
└─────────────────────────────────────────┘
                   │
                   │ owns
                   ▼
┌─────────────────────────────────────────┐
│           Scene                         │
│  - World (entities)                     │
│  - Scene metadata (name, version)       │
│  - Scene-specific systems               │
│  - Load/Save from/to RON files          │
└─────────────────────────────────────────┘
                   │
                   │ contains
                   ▼
┌─────────────────────────────────────────┐
│         World (Entity Container)        │
│  - SlotMap<EntityKey, Entity>           │
│  - Name index, tag index                │
│  - Physics integration                  │
│  - Dirty tracking                       │
└─────────────────────────────────────────┘
                   │
                   │ contains
                   ▼
┌─────────────────────────────────────────┐
│        Entity (Game Object)             │
│  - Transform, Shape, Material           │
│  - Optional physics body                │
│  - Name, tags, dirty flags              │
└─────────────────────────────────────────┘
```

### 4.2 Core Components

#### SceneManager

```rust
pub struct SceneManager {
    /// All loaded scenes (by name)
    scenes: HashMap<String, Scene>,

    /// Stack of active scenes (top = currently running)
    /// Allows scene overlays (e.g., pause menu over gameplay)
    active_scenes: Vec<String>,

    /// Transition state (e.g., fading between scenes)
    transition: Option<SceneTransition>,
}

impl SceneManager {
    pub fn load_scene(&mut self, path: &str) -> Result<(), SceneError>;
    pub fn unload_scene(&mut self, name: &str);
    pub fn push_scene(&mut self, name: &str);  // Add to active stack
    pub fn pop_scene(&mut self) -> Option<String>;  // Remove from active stack
    pub fn switch_to(&mut self, name: &str);  // Replace top of stack
    pub fn active_scene(&self) -> Option<&Scene>;
    pub fn active_scene_mut(&mut self) -> Option<&mut Scene>;
}
```

#### Scene

```rust
pub struct Scene {
    /// Scene metadata
    pub metadata: SceneMetadata,

    /// The world containing all entities
    pub world: World,

    /// Scene-local asset cache
    pub assets: AssetCache,

    /// Prefab registry for this scene
    pub prefabs: HashMap<String, Prefab>,
}

pub struct SceneMetadata {
    pub name: String,
    pub version: String,
    pub author: Option<String>,
    pub description: Option<String>,
}

impl Scene {
    /// Load a scene from a RON file
    pub fn load(path: &str) -> Result<Self, SceneError>;

    /// Save the scene to a RON file
    pub fn save(&self, path: &str) -> Result<(), SceneError>;

    /// Update the scene (physics, systems)
    pub fn update(&mut self, dt: f32);

    /// Instantiate a prefab into this scene
    pub fn instantiate_prefab(&mut self, prefab_name: &str) -> EntityKey;
}
```

#### Prefab System

```rust
/// A prefab is a template for creating entities
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Prefab {
    pub name: String,
    pub template: EntityTemplate,
    pub children: Vec<Prefab>,  // For nested prefabs
}

/// Template for an entity (similar to Entity but without runtime data)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EntityTemplate {
    pub name: Option<String>,
    pub tags: HashSet<String>,
    pub transform: Transform4D,
    pub shape: ShapeTemplate,
    pub material: Material,
    pub physics: Option<PhysicsTemplate>,
}

impl Prefab {
    /// Load a prefab from a RON file
    pub fn load(path: &str) -> Result<Self, PrefabError>;

    /// Instantiate this prefab into a world
    pub fn instantiate(&self, world: &mut World) -> EntityKey;

    /// Instantiate with overrides
    pub fn instantiate_with(&self, world: &mut World, overrides: &EntityTemplate) -> EntityKey;
}
```

### 4.3 Scene Lifecycle

```
   [Load Scene]
        │
        ▼
   [Deserialize RON] ──► Parse scene file
        │
        ▼
   [Create World] ──────► Instantiate entities
        │
        ▼
   [Initialize Physics] ─► Set up physics bodies
        │
        ▼
   [Scene Active] ◄─────► Update loop (update, render)
        │
        ▼
   [Unload Scene] ──────► Cleanup resources
```

### 4.4 Incremental Implementation Strategy

**Phase 1: Scene Structure (1-2 sessions)**
- Create `Scene` and `SceneMetadata` types
- Implement RON serialization for basic scenes
- Add `Scene::load()` and `Scene::save()`
- Add tests for serialization roundtrip
- **Goal:** Can save/load a simple scene from a file

**Phase 2: Scene Manager (1 session)**
- Create `SceneManager` for managing multiple scenes
- Implement scene stack (push/pop/switch)
- Add scene transitions
- **Goal:** Can switch between multiple scenes

**Phase 3: Prefab System (2 sessions)**
- Create `Prefab` and `EntityTemplate` types
- Implement prefab loading and instantiation
- Add prefab overrides
- Add tests for prefab instantiation
- **Goal:** Can spawn entities from templates

**Phase 4: Hierarchy (2-3 sessions - Optional)**
- Add parent-child relationships to entities
- Implement hierarchical transforms
- Add scene graph traversal
- **Goal:** Entities can have children with relative transforms

**Phase 5: Integration (1 session)**
- Update main.rs to use SceneManager
- Create example scene files
- Add hot reloading support
- **Goal:** Fully integrated scene system

**Total Estimate:** 6-8 sessions for full implementation
**Critical Path:** Phases 1-3 (4-5 sessions)

---

## 5. Priority Features to Add

### 5.1 Immediate Priorities (Next 1-2 sessions)

**1. Scene Serialization (Critical - 1 session)**
- Add serde derives to Entity, World, Transform4D, Material
- Implement Scene struct with metadata
- Add Scene::load() and Scene::save() using RON
- Create example scene file

**Why:** Unlocks content creation without recompiling. Biggest productivity boost.

**2. SceneManager Basics (High - 1 session)**
- Create SceneManager singleton
- Implement basic load/unload/switch
- Update main.rs to use SceneManager
- Support multiple scene files

**Why:** Enables multi-level games and menu systems.

### 5.2 Near-Term Priorities (Sessions 3-5)

**3. Prefab System (High - 2 sessions)**
- Create Prefab and EntityTemplate types
- Implement prefab loading from RON files
- Add prefab instantiation with overrides
- Add tests

**Why:** Essential for spawning enemies, collectibles, and reusable entities at runtime.

**4. Asset Management (Medium - 1 session)**
- Create AssetCache for loading shared resources
- Track asset dependencies (which scenes use which assets)
- Add asset hot reloading

**Why:** Improves loading times and memory usage by sharing assets between entities.

### 5.3 Future Priorities (Sessions 6+)

**5. Entity Hierarchy (Medium - 2-3 sessions)**
- Add parent-child relationships
- Implement hierarchical transforms
- Scene graph traversal and operations

**Why:** Needed for complex multi-part entities. Not urgent for current use cases.

**6. Scene Editor Integration (Low - Future)**
- Create visual scene editor (egui-based)
- Drag-and-drop entity placement
- Visual prefab editing

**Why:** Nice to have, but manual RON editing works for now.

---

## 6. Implementation Risks and Mitigations

### 6.1 Serialization Complexity

**Risk:** Entity serialization is complex due to ShapeRef (trait objects).

**Mitigation:**
- Use enum-based shapes instead of trait objects for serialization
- Create ShapeTemplate enum with variants: Tesseract, Hyperplane, etc.
- Convert ShapeTemplate → ShapeRef on deserialization

### 6.2 Physics Body Synchronization

**Risk:** Deserializing entities with physics bodies requires coordinating World and PhysicsWorld.

**Mitigation:**
- Deserialize entities first (without physics bodies)
- Then create physics bodies and link them
- Two-pass deserialization: entities, then physics

### 6.3 Backward Compatibility

**Risk:** Scene file format changes break existing scenes.

**Mitigation:**
- Add version field to scene metadata
- Implement format migration for old versions
- Provide format validation and helpful error messages

### 6.4 Performance

**Risk:** Loading large scenes is slow with text formats.

**Mitigation:**
- Profile scene loading and optimize hot paths
- Consider binary format for final builds (RON for development)
- Stream large scenes (load visible portions first)

---

## 7. References and Research

This review draws on research into modern game engine scene management practices:

### Industry Practices (2026)
- [Game Engine Architecture Overview](https://grier.hashnode.dev/tinker-engine) - Scene graph and management layer design
- [Isetta Engine Architecture](https://isetta.io/blogs/engine-architecture/) - Modern engine structure
- [Scene Management Patterns](https://rivermanmedia.com/object-oriented-game-programming-the-scene-system/) - Scene system design

### Scene Serialization
- [Scene Serialization Formats Comparison](https://peerdh.com/blogs/programming-insights/comparing-different-serialization-formats-for-game-state-management-1) - JSON vs YAML vs binary
- [Serialization for Game Engines](https://jorenjoestar.github.io/post/serialization_for_games/) - Best practices
- [Unity Scene Format](https://docs.unity3d.com/Manual/FormatDescription.html) - YAML-based reference

### Prefab Systems
- [Prefab System Overview](https://ezengine.net/pages/docs/prefabs/prefabs-overview.html) - Template-instance architecture
- [Unity Prefabs Guide](https://docs.unity3d.com/6000.3/Documentation/Manual/Prefabs.html) - Industry standard
- [O3DE Prefab Architecture](https://docs.o3de.org/docs/engine-dev/architecture/prefabs/) - Modern approach

### ECS and Bevy (Reference)
- [Bevy ECS](https://bevy.org/learn/quick-start/getting-started/ecs/) - Component-based architecture
- [Bevy Documentation](https://docs.rs/bevy_ecs/latest/bevy_ecs/) - Scene patterns in ECS
- [ECS Intro](https://bevy-cheatbook.github.io/programming/ecs-intro.html) - Entity component patterns

---

## 8. Conclusion

Rust4D's scene handling is functional but minimal. The SceneBuilder provides a good foundation for programmatic scene construction, but the engine needs file-based scene persistence, scene switching, and a prefab system to be production-ready.

### Key Recommendations:

1. **Add RON-based scene serialization (Critical)** - Enables content creation without recompiling
2. **Implement SceneManager (High)** - Enables multi-scene games and menus
3. **Create Prefab system (High)** - Essential for runtime entity spawning
4. **Defer entity hierarchy (Medium)** - Nice to have, not urgent
5. **Defer full ECS (Low)** - Current design works well for now

### Implementation Priority:

**Short term (1-2 sessions):** Scene serialization + SceneManager
**Medium term (3-5 sessions):** Prefab system + Asset management
**Long term (6+ sessions):** Entity hierarchy + Scene editor

The recommended architecture follows proven patterns from Unity, Godot, and Bevy while staying true to Rust4D's simple, physics-focused design. RON provides the best balance of readability, type safety, and Rust integration for scene files.

---

**End of Report**
