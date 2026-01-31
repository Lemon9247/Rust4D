# Phase 5: Advanced Features

**Created:** 2026-01-27
**Status:** Ready
**Priority:** P4
**Estimated Sessions:** 5-6
**Dependencies:** Phase 2 (Scene Management) must be complete

---

## Overview

Phase 5 adds production-ready features that enable efficient asset management, optional entity hierarchy, and advanced scene capabilities. These features take Rust4D from "functional engine" to "production-ready engine" with professional-grade scene management.

### Goals

1. **Asset Management (5A):** Implement efficient asset loading, caching, and hot reloading
2. **Entity Hierarchy (5B):** Optional parent-child relationships with hierarchical transforms
3. **Advanced Scene Features (5C):** Scene transitions, overlays, streaming, and validation

### Why This Phase Matters

- **Asset Management:** Reduces memory usage, improves loading times, enables hot reloading
- **Entity Hierarchy:** Enables complex multi-part objects (robots, vehicles, compound shapes)
- **Advanced Scenes:** Professional polish (smooth transitions, HUD overlays, background loading)

---

## Dependencies

### Required (Must Complete First)

- **Phase 1A (Scene Serialization):** Asset system needs scene file format
- **Phase 2A (Scene Manager):** Advanced features build on scene management infrastructure
- **Phase 2B (Prefab System):** Assets reference prefabs, prefabs may use hierarchy

### Optional (Improves Experience)

- **Phase 1B (Configuration System):** Asset paths, cache sizes configurable
- **Phase 3 (Documentation):** Examples showing asset/hierarchy usage

---

## Phase 5A: Asset Management

**Sessions:** 1
**Priority:** Medium
**Parallelizable:** Can run alongside 5B or 5C

### Rationale

Currently, each entity owns or shares its shape via `ShapeRef` (Arc or Box). This works but has limitations:

1. **No centralized cache:** Duplicate shapes waste memory
2. **No dependency tracking:** Can't tell which scenes use which assets
3. **No hot reloading:** Changes require engine restart
4. **No asset paths:** Everything is hardcoded or inline in scene files

An asset management system provides:
- Memory efficiency through centralized caching
- Dependency tracking for cleanup
- Hot reloading for rapid iteration
- File-based asset references (not inlined data)

### Design

#### AssetCache Architecture

```rust
// crates/rust4d_core/src/asset_cache.rs

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Handle to an asset in the cache
/// Dropping all handles allows the asset to be unloaded
#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub struct AssetHandle<T> {
    id: AssetId,
    path: PathBuf,
    _phantom: std::marker::PhantomData<T>,
}

/// Unique identifier for an asset
type AssetId = uuid::Uuid;

/// Cached asset with reference counting and metadata
struct CachedAsset<T> {
    id: AssetId,
    path: PathBuf,
    data: Arc<T>,
    load_time: std::time::SystemTime,
    dependents: Vec<String>, // Scene names that use this asset
}

/// Asset cache with hot reloading support
pub struct AssetCache {
    // Shape assets
    shapes: HashMap<AssetId, CachedAsset<dyn ConvexShape4D>>,

    // Material assets (future)
    materials: HashMap<AssetId, CachedAsset<Material>>,

    // Prefab assets (references Phase 2B)
    prefabs: HashMap<AssetId, CachedAsset<Prefab>>,

    // Path -> ID lookup for duplicate detection
    path_index: HashMap<PathBuf, AssetId>,

    // Hot reload configuration
    watch_for_changes: bool,
    last_check: std::time::SystemTime,
}

impl AssetCache {
    pub fn new() -> Self {
        Self {
            shapes: HashMap::new(),
            materials: HashMap::new(),
            prefabs: HashMap::new(),
            path_index: HashMap::new(),
            watch_for_changes: false,
            last_check: std::time::SystemTime::now(),
        }
    }

    /// Load an asset from disk, or return cached version if already loaded
    pub fn load<T: Asset>(&mut self, path: impl AsRef<Path>) -> Result<AssetHandle<T>, AssetError> {
        let path = path.as_ref();

        // Check if already cached
        if let Some(id) = self.path_index.get(path) {
            return Ok(AssetHandle {
                id: *id,
                path: path.to_path_buf(),
                _phantom: std::marker::PhantomData,
            });
        }

        // Load from disk
        let data = T::load_from_file(path)?;
        let id = AssetId::new_v4();

        // Cache it
        let cached = CachedAsset {
            id,
            path: path.to_path_buf(),
            data: Arc::new(data),
            load_time: std::time::SystemTime::now(),
            dependents: Vec::new(),
        };

        self.insert_asset(id, cached);
        self.path_index.insert(path.to_path_buf(), id);

        Ok(AssetHandle {
            id,
            path: path.to_path_buf(),
            _phantom: std::marker::PhantomData,
        })
    }

    /// Get cached asset data by handle
    pub fn get<T: Asset>(&self, handle: &AssetHandle<T>) -> Option<Arc<T>> {
        self.get_cached::<T>(handle.id)
            .map(|cached| cached.data.clone())
    }

    /// Track that a scene depends on an asset
    pub fn add_dependent(&mut self, asset_id: AssetId, scene_name: String) {
        if let Some(cached) = self.get_cached_mut(asset_id) {
            if !cached.dependents.contains(&scene_name) {
                cached.dependents.push(scene_name);
            }
        }
    }

    /// Remove a scene's dependency on an asset
    pub fn remove_dependent(&mut self, asset_id: AssetId, scene_name: &str) {
        if let Some(cached) = self.get_cached_mut(asset_id) {
            cached.dependents.retain(|dep| dep != scene_name);
        }
    }

    /// Check for file changes and reload modified assets
    pub fn check_hot_reload(&mut self) -> Vec<AssetId> {
        if !self.watch_for_changes {
            return Vec::new();
        }

        let now = std::time::SystemTime::now();
        let mut reloaded = Vec::new();

        // Check each cached asset
        for (id, cached) in &mut self.shapes {
            if let Ok(metadata) = std::fs::metadata(&cached.path) {
                if let Ok(modified) = metadata.modified() {
                    if modified > cached.load_time {
                        // File changed, reload it
                        if let Ok(new_data) = Self::load_file(&cached.path) {
                            cached.data = Arc::new(new_data);
                            cached.load_time = now;
                            reloaded.push(*id);
                        }
                    }
                }
            }
        }

        self.last_check = now;
        reloaded
    }

    /// Unload assets with no dependents (garbage collection)
    pub fn gc(&mut self) -> usize {
        let before = self.shapes.len() + self.materials.len() + self.prefabs.len();

        self.shapes.retain(|_, cached| !cached.dependents.is_empty());
        self.materials.retain(|_, cached| !cached.dependents.is_empty());
        self.prefabs.retain(|_, cached| !cached.dependents.is_empty());

        let after = self.shapes.len() + self.materials.len() + self.prefabs.len();
        before - after
    }
}
```

#### Asset Trait

```rust
/// Trait for types that can be loaded as assets
pub trait Asset: Sized {
    fn load_from_file(path: &Path) -> Result<Self, AssetError>;
}

// Example implementations
impl Asset for Prefab {
    fn load_from_file(path: &Path) -> Result<Self, AssetError> {
        let contents = std::fs::read_to_string(path)?;
        ron::from_str(&contents).map_err(|e| AssetError::ParseError(e.to_string()))
    }
}

// Future: implement for Material, Texture, etc.
```

#### Integration with Scene Files

Instead of inlining shapes in scene files, reference asset files:

```ron
// scenes/level1.ron
Scene(
    metadata: (
        name: "Level 1",
        version: "1.0.0",
    ),

    assets: [
        // Preload these assets when scene loads
        "assets/shapes/tesseract_large.ron",
        "assets/prefabs/enemy_cube.ron",
    ],

    entities: [
        (
            name: Some("floor"),
            transform: (position: (0.0, -2.0, 0.0, 0.0)),
            shape: AssetRef("assets/shapes/hyperplane_floor.ron"),  // <-- Asset reference
            material: (base_color: (0.5, 0.5, 0.5, 1.0)),
        ),

        (
            name: Some("main_tesseract"),
            transform: (position: (0.0, 0.0, 0.0, 0.0)),
            shape: AssetRef("assets/shapes/tesseract_large.ron"),  // <-- Asset reference
            material: (base_color: (1.0, 1.0, 1.0, 1.0)),
        ),
    ],
)
```

#### ShapeRef Extension

Update `ShapeRef` to support asset handles:

```rust
// crates/rust4d_core/src/entity.rs

pub enum ShapeRef {
    /// A shared reference to a shape (multiple entities can share this)
    Shared(Arc<dyn ConvexShape4D>),
    /// An owned shape (unique to this entity)
    Owned(Box<dyn ConvexShape4D>),
    /// A reference to a cached asset (NEW)
    Asset(AssetHandle<dyn ConvexShape4D>),
}

impl ShapeRef {
    /// Get a reference to the underlying shape
    pub fn as_shape(&self, asset_cache: &AssetCache) -> &dyn ConvexShape4D {
        match self {
            ShapeRef::Shared(arc) => arc.as_ref(),
            ShapeRef::Owned(boxed) => boxed.as_ref(),
            ShapeRef::Asset(handle) => {
                asset_cache.get(handle)
                    .expect("Asset not loaded")
                    .as_ref()
            }
        }
    }
}
```

### Tasks

#### Task 5A.1: Core Asset Cache (0.5 sessions)

Create the asset cache infrastructure.

**Files to create:**
- `crates/rust4d_core/src/asset_cache.rs` (NEW, ~300 lines)
- `crates/rust4d_core/src/asset_error.rs` (NEW, ~50 lines)

**Files to modify:**
- `crates/rust4d_core/src/lib.rs` - Export asset module
- `Cargo.toml` - Add `uuid` crate for asset IDs

**Implementation steps:**
1. Define `AssetHandle<T>`, `AssetId`, `CachedAsset<T>`
2. Implement `AssetCache` struct with HashMap storage
3. Add `load()`, `get()`, `add_dependent()`, `remove_dependent()`
4. Add basic error types (`AssetError`)

**Tests:**
- Load asset and verify it's cached
- Load same asset twice, verify same ID returned
- Test dependent tracking (add/remove)

#### Task 5A.2: Asset Trait and Implementations (0.25 sessions)

Define the `Asset` trait and implement for existing types.

**Files to modify:**
- `crates/rust4d_core/src/asset_cache.rs` - Add `Asset` trait
- `crates/rust4d_core/src/prefab.rs` - Implement `Asset for Prefab`

**Implementation steps:**
1. Define `Asset` trait with `load_from_file()`
2. Implement for `Prefab` (RON deserialization)
3. Document trait and examples

**Tests:**
- Load prefab as asset
- Verify error handling for missing files

#### Task 5A.3: Hot Reloading (0.25 sessions)

Add hot reload support for asset files.

**Files to modify:**
- `crates/rust4d_core/src/asset_cache.rs` - Add `check_hot_reload()`

**Implementation steps:**
1. Add `watch_for_changes` flag to `AssetCache`
2. Implement `check_hot_reload()` that checks file modification times
3. Return list of reloaded asset IDs

**Tests:**
- Modify asset file, verify hot reload detects it
- Verify `last_check` timestamp updates

**Note:** This is a simple polling-based implementation. Future versions could use `notify` crate for filesystem watching.

#### Task 5A.4: Scene Integration (0.25 sessions)

Integrate asset cache with scene loading.

**Files to modify:**
- `crates/rust4d_core/src/scene.rs` - Add asset loading to `Scene::load()`
- `crates/rust4d_core/src/entity.rs` - Extend `ShapeRef` with `Asset` variant

**Implementation steps:**
1. Add `AssetRef("path")` variant to shape deserialization
2. Load referenced assets during scene loading
3. Track scene as dependent when loading assets
4. Update `ShapeRef::as_shape()` to resolve asset handles

**Tests:**
- Load scene with asset references
- Verify assets are cached
- Verify dependency tracking works

---

## Phase 5B: Entity Hierarchy (OPTIONAL)

**Sessions:** 2-3
**Priority:** Low
**Parallelizable:** Independent from 5A and 5C

### Rationale

Currently, all entities are flat - no parent-child relationships. This works for simple objects but limits complex entities:

- **Robot with movable joints:** Arms, legs, head as separate entities
- **Vehicle with wheels:** Car body + 4 wheels that rotate independently
- **Compound shapes:** Multi-part objects that move together
- **Relative positioning:** Place torch relative to player, not world space

Entity hierarchy enables:
1. **Hierarchical transforms:** Children inherit parent's transform
2. **Relative positioning:** Child position is relative to parent
3. **Group operations:** Delete parent deletes all children
4. **Scene graph traversal:** Iterate through tree structure

### Design

#### Hierarchical Entity Structure

```rust
// crates/rust4d_core/src/entity.rs

pub struct Entity {
    pub name: Option<String>,
    pub tags: HashSet<String>,
    pub transform: Transform4D,  // Now local to parent
    pub shape: ShapeRef,
    pub material: Material,
    pub physics_body: Option<BodyKey>,
    dirty: DirtyFlags,

    // NEW: Hierarchy fields
    parent: Option<EntityKey>,
    children: Vec<EntityKey>,
}

impl Entity {
    /// Get this entity's parent (if any)
    pub fn parent(&self) -> Option<EntityKey> {
        self.parent
    }

    /// Get this entity's children
    pub fn children(&self) -> &[EntityKey] {
        &self.children
    }

    /// Check if this entity has children
    pub fn has_children(&self) -> bool {
        !self.children.is_empty()
    }
}
```

#### World Hierarchy Operations

```rust
// crates/rust4d_core/src/world.rs

impl World {
    /// Add a child entity to a parent
    pub fn add_child(&mut self, parent: EntityKey, child: EntityKey) -> Result<(), WorldError> {
        // Verify both entities exist
        if !self.entities.contains_key(parent) || !self.entities.contains_key(child) {
            return Err(WorldError::InvalidEntity);
        }

        // Detect cycles (child can't be ancestor of parent)
        if self.is_ancestor(child, parent) {
            return Err(WorldError::CyclicHierarchy);
        }

        // Remove from old parent if any
        if let Some(old_parent) = self.entities[child].parent {
            self.entities[old_parent].children.retain(|&c| c != child);
        }

        // Update parent and child
        self.entities[parent].children.push(child);
        self.entities[child].parent = Some(parent);

        Ok(())
    }

    /// Remove a child from its parent
    pub fn remove_from_parent(&mut self, child: EntityKey) {
        if let Some(parent) = self.entities[child].parent {
            self.entities[parent].children.retain(|&c| c != child);
            self.entities[child].parent = None;
        }
    }

    /// Get the world-space transform of an entity (accumulating parent transforms)
    pub fn world_transform(&self, entity: EntityKey) -> Transform4D {
        let mut transform = self.entities[entity].transform;
        let mut current = entity;

        // Walk up the hierarchy, accumulating transforms
        while let Some(parent) = self.entities[current].parent {
            transform = self.entities[parent].transform * transform;
            current = parent;
        }

        transform
    }

    /// Delete an entity and all its children recursively
    pub fn delete_recursive(&mut self, entity: EntityKey) {
        // Get all descendants first (avoid borrow issues)
        let descendants = self.descendants(entity);

        // Delete in reverse order (leaves first)
        for descendant in descendants.iter().rev() {
            self.delete_entity(*descendant);
        }

        // Delete the entity itself
        self.delete_entity(entity);
    }

    /// Get all descendants of an entity (children, grandchildren, etc.)
    fn descendants(&self, entity: EntityKey) -> Vec<EntityKey> {
        let mut result = Vec::new();
        let mut to_visit = vec![entity];

        while let Some(current) = to_visit.pop() {
            if let Some(ent) = self.entities.get(current) {
                for &child in &ent.children {
                    result.push(child);
                    to_visit.push(child);
                }
            }
        }

        result
    }

    /// Check if `ancestor` is an ancestor of `entity`
    fn is_ancestor(&self, ancestor: EntityKey, entity: EntityKey) -> bool {
        let mut current = entity;

        while let Some(parent) = self.entities.get(current).and_then(|e| e.parent) {
            if parent == ancestor {
                return true;
            }
            current = parent;
        }

        false
    }
}
```

#### Scene Serialization with Hierarchy

```ron
// scenes/robot.ron
Scene(
    metadata: (name: "Robot Test"),

    entities: [
        // Root: robot body
        (
            name: Some("robot_body"),
            transform: (position: (0.0, 0.0, 0.0, 0.0)),
            shape: Tesseract(size: 1.0),
            children: [
                // Child: left arm
                (
                    name: Some("left_arm"),
                    transform: (position: (-0.8, 0.0, 0.0, 0.0)),  // Relative to body
                    shape: Tesseract(size: 0.3),
                    children: [],
                ),
                // Child: right arm
                (
                    name: Some("right_arm"),
                    transform: (position: (0.8, 0.0, 0.0, 0.0)),  // Relative to body
                    shape: Tesseract(size: 0.3),
                    children: [],
                ),
            ],
        ),
    ],
)
```

### Tasks

#### Task 5B.1: Hierarchy Fields (0.5 sessions)

Add parent/child relationships to Entity.

**Files to modify:**
- `crates/rust4d_core/src/entity.rs` - Add `parent`, `children` fields
- `crates/rust4d_core/src/entity.rs` - Add `parent()`, `children()` accessors

**Implementation steps:**
1. Add `parent: Option<EntityKey>` field
2. Add `children: Vec<EntityKey>` field
3. Update constructors to initialize new fields
4. Add accessor methods

**Tests:**
- Create entity, verify no parent/children initially
- Test accessor methods

#### Task 5B.2: World Hierarchy Operations (1 session)

Implement hierarchy management in World.

**Files to modify:**
- `crates/rust4d_core/src/world.rs` - Add hierarchy methods

**Implementation steps:**
1. Implement `add_child()` with cycle detection
2. Implement `remove_from_parent()`
3. Implement `world_transform()` for transform accumulation
4. Implement `delete_recursive()` for cascade deletion
5. Add helper methods (`descendants()`, `is_ancestor()`)

**Tests:**
- Add child to parent, verify relationship
- Test cycle detection (prevent child becoming ancestor of itself)
- Test world transform accumulation (3-level hierarchy)
- Test recursive deletion
- Test remove from parent

#### Task 5B.3: Scene Builder Integration (0.5 sessions)

Add hierarchy support to SceneBuilder.

**Files to modify:**
- `src/scene/scene_builder.rs` - Add hierarchy methods

**Implementation steps:**
1. Add `add_entity_as_child()` method
2. Update existing methods to support parent parameter

**Example usage:**
```rust
let world = SceneBuilder::new()
    .with_physics(-20.0)
    .add_tesseract(Vec4::ZERO, 1.0, "robot_body")
    .add_entity_as_child(
        "robot_body",
        Entity::with_transform(
            ShapeRef::shared(Tesseract4D::new(0.3)),
            Transform4D::from_position(Vec4::new(-0.8, 0.0, 0.0, 0.0)),
            Material::GRAY,
        ).with_name("left_arm")
    )
    .build();
```

**Tests:**
- Build hierarchy via SceneBuilder
- Verify parent-child relationships
- Test world transforms

#### Task 5B.4: Serialization with Hierarchy (0.5-1 session)

Support hierarchical scene serialization.

**Files to modify:**
- `crates/rust4d_core/src/scene.rs` - Nested entity deserialization

**Implementation steps:**
1. Update scene format to support nested entity definitions
2. Add `children: [...]` field to entity serialization
3. Implement recursive entity instantiation
4. Preserve parent-child relationships on load

**Tests:**
- Save scene with hierarchy, verify round-trip
- Load hierarchical scene, verify relationships
- Test nested serialization (3+ levels)

**Example format:**
```ron
entities: [
    (
        name: Some("parent"),
        transform: (...),
        children: [
            (name: Some("child1"), ...),
            (name: Some("child2"), ...),
        ],
    ),
]
```

### Important Notes

**This sub-phase is OPTIONAL.** Implement only if needed for specific features:
- Complex multi-part entities (robots, vehicles)
- Relative positioning requirements
- Scene graph traversal needs

If not immediately needed, defer to future. The flat entity structure works well for most use cases.

---

## Phase 5C: Advanced Scene Features

**Sessions:** 2
**Priority:** Medium
**Parallelizable:** Can run alongside 5A or after 5B

### Rationale

Phase 2 provides basic scene management (load, unload, switch). Phase 5C adds professional polish:

1. **Scene Transitions:** Smooth fades, slides, crossfades between scenes
2. **Scene Overlays:** HUD, pause menu, debug info as separate scenes
3. **Scene Streaming:** Load scenes in background while playing
4. **Scene Validation:** Check for errors before runtime

These features distinguish a "functional" engine from a "production-ready" engine.

### Design

#### Scene Transitions

```rust
// crates/rust4d_core/src/scene_transition.rs

use std::time::Duration;

/// Transition effect between scenes
#[derive(Clone, Debug)]
pub enum TransitionEffect {
    /// Instant cut (no transition)
    Instant,
    /// Fade to black, then fade in new scene
    Fade {
        duration: Duration,
    },
    /// Crossfade between scenes (blend alpha)
    Crossfade {
        duration: Duration,
    },
    /// Slide old scene out, slide new scene in
    Slide {
        duration: Duration,
        direction: SlideDirection,
    },
}

#[derive(Clone, Debug)]
pub enum SlideDirection {
    Left, Right, Up, Down,
}

/// Active transition state
pub struct SceneTransition {
    effect: TransitionEffect,
    from_scene: String,
    to_scene: String,
    start_time: std::time::Instant,
    progress: f32,  // 0.0 = start, 1.0 = complete
}

impl SceneTransition {
    pub fn new(from: String, to: String, effect: TransitionEffect) -> Self {
        Self {
            effect,
            from_scene: from,
            to_scene: to,
            start_time: std::time::Instant::now(),
            progress: 0.0,
        }
    }

    /// Update transition, returns true when complete
    pub fn update(&mut self, dt: f32) -> bool {
        let duration = match &self.effect {
            TransitionEffect::Instant => return true,
            TransitionEffect::Fade { duration } => duration,
            TransitionEffect::Crossfade { duration } => duration,
            TransitionEffect::Slide { duration, .. } => duration,
        };

        let elapsed = self.start_time.elapsed();
        self.progress = (elapsed.as_secs_f32() / duration.as_secs_f32()).min(1.0);

        self.progress >= 1.0
    }

    /// Get current alpha for rendering fade effects
    pub fn alpha(&self) -> f32 {
        match &self.effect {
            TransitionEffect::Fade { .. } => {
                if self.progress < 0.5 {
                    1.0 - (self.progress * 2.0)  // Fade out
                } else {
                    (self.progress - 0.5) * 2.0  // Fade in
                }
            }
            TransitionEffect::Crossfade { .. } => self.progress,
            _ => 1.0,
        }
    }
}
```

#### Scene Manager Extensions

```rust
// crates/rust4d_core/src/scene_manager.rs

impl SceneManager {
    /// Switch to a new scene with transition effect
    pub fn switch_to_with_transition(
        &mut self,
        scene_name: &str,
        effect: TransitionEffect,
    ) -> Result<(), SceneError> {
        let current = self.active_scenes.last()
            .ok_or(SceneError::NoActiveScene)?
            .clone();

        self.transition = Some(SceneTransition::new(
            current,
            scene_name.to_string(),
            effect,
        ));

        Ok(())
    }

    /// Update active transition
    pub fn update_transition(&mut self, dt: f32) -> bool {
        if let Some(transition) = &mut self.transition {
            if transition.update(dt) {
                // Transition complete, actually switch scenes
                let to_scene = transition.to_scene.clone();
                self.transition = None;
                self.switch_to(&to_scene).ok();
                return true;
            }
        }
        false
    }

    /// Get current transition for rendering
    pub fn current_transition(&self) -> Option<&SceneTransition> {
        self.transition.as_ref()
    }
}
```

#### Scene Overlays

Overlays are scenes that render on top of the active scene without replacing it.

```rust
impl SceneManager {
    /// Add a scene as an overlay (doesn't replace active scene)
    pub fn push_overlay(&mut self, scene_name: &str) -> Result<(), SceneError> {
        if !self.scenes.contains_key(scene_name) {
            return Err(SceneError::SceneNotLoaded(scene_name.to_string()));
        }

        // Mark as overlay in metadata
        self.overlay_stack.push(scene_name.to_string());
        Ok(())
    }

    /// Remove the top overlay
    pub fn pop_overlay(&mut self) -> Option<String> {
        self.overlay_stack.pop()
    }

    /// Check if a scene is currently an overlay
    pub fn is_overlay(&self, scene_name: &str) -> bool {
        self.overlay_stack.contains(&scene_name.to_string())
    }
}
```

**Example usage:**
```rust
// Main game scene
scene_manager.switch_to("level1")?;

// Player pauses - add pause menu as overlay
scene_manager.push_overlay("pause_menu")?;

// Rendering: Draw level1, then draw pause_menu on top

// Player resumes - remove overlay
scene_manager.pop_overlay();
```

#### Scene Streaming

Load scenes in background without blocking.

```rust
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread;

pub struct SceneLoader {
    sender: Sender<LoadRequest>,
    receiver: Receiver<LoadResult>,
}

struct LoadRequest {
    path: String,
    scene_name: String,
}

struct LoadResult {
    scene_name: String,
    scene: Result<Scene, SceneError>,
}

impl SceneLoader {
    pub fn new() -> Self {
        let (tx_req, rx_req) = channel();
        let (tx_res, rx_res) = channel();

        // Spawn loader thread
        thread::spawn(move || {
            while let Ok(request) = rx_req.recv() {
                let scene = Scene::load(&request.path);
                tx_res.send(LoadResult {
                    scene_name: request.scene_name,
                    scene,
                }).ok();
            }
        });

        Self {
            sender: tx_req,
            receiver: rx_res,
        }
    }

    /// Request a scene to be loaded in background
    pub fn load_async(&self, path: String, scene_name: String) {
        self.sender.send(LoadRequest { path, scene_name }).ok();
    }

    /// Check if any scenes have finished loading
    pub fn poll(&self) -> Option<LoadResult> {
        self.receiver.try_recv().ok()
    }
}

// Integration with SceneManager
impl SceneManager {
    pub fn load_scene_async(&mut self, path: &str, scene_name: &str) {
        self.loader.load_async(path.to_string(), scene_name.to_string());
    }

    pub fn poll_loading(&mut self) -> Vec<String> {
        let mut loaded = Vec::new();

        while let Some(result) = self.loader.poll() {
            match result.scene {
                Ok(scene) => {
                    self.scenes.insert(result.scene_name.clone(), scene);
                    loaded.push(result.scene_name);
                }
                Err(e) => {
                    eprintln!("Failed to load scene {}: {}", result.scene_name, e);
                }
            }
        }

        loaded
    }
}
```

#### Scene Validation

Validate scenes for common errors before runtime.

```rust
// crates/rust4d_core/src/scene_validator.rs

#[derive(Debug)]
pub enum ValidationError {
    MissingAsset(String),
    InvalidEntity(String),
    CyclicPrefabReference(String),
    DuplicateName(String),
    InvalidPhysicsConfig,
}

pub struct SceneValidator;

impl SceneValidator {
    /// Validate a scene file
    pub fn validate(scene: &Scene, asset_cache: &AssetCache) -> Vec<ValidationError> {
        let mut errors = Vec::new();

        // Check for duplicate entity names
        let mut names = HashSet::new();
        for entity in scene.world.entities() {
            if let Some(name) = &entity.name {
                if !names.insert(name.clone()) {
                    errors.push(ValidationError::DuplicateName(name.clone()));
                }
            }
        }

        // Check asset references exist
        for entity in scene.world.entities() {
            if let ShapeRef::Asset(handle) = &entity.shape {
                if asset_cache.get(handle).is_none() {
                    errors.push(ValidationError::MissingAsset(
                        handle.path().to_string_lossy().to_string()
                    ));
                }
            }
        }

        // Validate physics configuration
        if let Some(physics) = scene.world.physics() {
            if physics.config.gravity.abs() > 1000.0 {
                errors.push(ValidationError::InvalidPhysicsConfig);
            }
        }

        errors
    }

    /// Validate and return Result
    pub fn validate_or_error(scene: &Scene, asset_cache: &AssetCache) -> Result<(), Vec<ValidationError>> {
        let errors = Self::validate(scene, asset_cache);
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}
```

### Tasks

#### Task 5C.1: Scene Transitions (0.75 sessions)

Implement transition effects.

**Files to create:**
- `crates/rust4d_core/src/scene_transition.rs` (NEW, ~200 lines)

**Files to modify:**
- `crates/rust4d_core/src/scene_manager.rs` - Add transition support
- `crates/rust4d_core/src/lib.rs` - Export transition module

**Implementation steps:**
1. Define `TransitionEffect` enum (Instant, Fade, Crossfade, Slide)
2. Implement `SceneTransition` state machine
3. Add `switch_to_with_transition()` to SceneManager
4. Add `update_transition()` to update loop
5. Provide alpha/progress for rendering

**Tests:**
- Create transition, verify progress updates
- Test fade calculation (0 -> 1 -> 0)
- Test transition completion
- Test instant transition (immediate)

**Example usage:**
```rust
scene_manager.switch_to_with_transition(
    "level2",
    TransitionEffect::Fade { duration: Duration::from_secs(1) },
)?;

// In game loop:
scene_manager.update_transition(dt);
if let Some(transition) = scene_manager.current_transition() {
    // Apply alpha to rendering
    let alpha = transition.alpha();
}
```

#### Task 5C.2: Scene Overlays (0.5 sessions)

Add overlay scene support.

**Files to modify:**
- `crates/rust4d_core/src/scene_manager.rs` - Add overlay stack

**Implementation steps:**
1. Add `overlay_stack: Vec<String>` to SceneManager
2. Implement `push_overlay()`, `pop_overlay()`
3. Add `is_overlay()` query
4. Update rendering to draw overlays on top

**Tests:**
- Push overlay, verify stack
- Pop overlay, verify removal
- Test multiple overlays
- Test overlay with no active scene (error)

**Example usage:**
```rust
// Game running
scene_manager.switch_to("gameplay")?;

// Pause menu
scene_manager.push_overlay("pause_menu")?;

// HUD on top of pause menu
scene_manager.push_overlay("hud")?;

// Resume (remove HUD)
scene_manager.pop_overlay();
```

#### Task 5C.3: Scene Streaming (0.5 sessions)

Add background scene loading.

**Files to create:**
- `crates/rust4d_core/src/scene_loader.rs` (NEW, ~150 lines)

**Files to modify:**
- `crates/rust4d_core/src/scene_manager.rs` - Add async loading
- `Cargo.toml` - Ensure thread safety (already has std)

**Implementation steps:**
1. Create `SceneLoader` with background thread
2. Implement `load_async()` to queue load requests
3. Implement `poll()` to check for completed loads
4. Add `load_scene_async()` to SceneManager
5. Add `poll_loading()` to game loop

**Tests:**
- Load scene async, verify it loads
- Load multiple scenes in parallel
- Test error handling (missing file)

**Example usage:**
```rust
// Start loading next level in background
scene_manager.load_scene_async("scenes/level2.ron", "level2");

// In game loop:
let loaded = scene_manager.poll_loading();
for scene_name in loaded {
    println!("Scene {} loaded!", scene_name);
}
```

#### Task 5C.4: Scene Validation (0.25 sessions)

Add scene validation tools.

**Files to create:**
- `crates/rust4d_core/src/scene_validator.rs` (NEW, ~100 lines)

**Files to modify:**
- `crates/rust4d_core/src/scene.rs` - Call validator in `load()`

**Implementation steps:**
1. Define `ValidationError` enum
2. Implement `SceneValidator::validate()`
3. Check for: duplicate names, missing assets, invalid physics
4. Add `validate_or_error()` for Result-based validation

**Tests:**
- Validate valid scene (no errors)
- Test duplicate name detection
- Test missing asset detection
- Test invalid physics detection

**Example usage:**
```rust
let scene = Scene::load("scenes/test.ron")?;
let errors = SceneValidator::validate(&scene, &asset_cache);

if !errors.is_empty() {
    for error in errors {
        eprintln!("Validation error: {:?}", error);
    }
}
```

---

## Files to Create

### Phase 5A (Asset Management)
- `crates/rust4d_core/src/asset_cache.rs` (~300 lines)
- `crates/rust4d_core/src/asset_error.rs` (~50 lines)

### Phase 5B (Entity Hierarchy - Optional)
No new files (modifies existing entity.rs, world.rs, scene.rs)

### Phase 5C (Advanced Scene Features)
- `crates/rust4d_core/src/scene_transition.rs` (~200 lines)
- `crates/rust4d_core/src/scene_loader.rs` (~150 lines)
- `crates/rust4d_core/src/scene_validator.rs` (~100 lines)

**Total new code:** ~800 lines

---

## Files to Modify

### Phase 5A
- `crates/rust4d_core/src/lib.rs` - Export asset modules
- `crates/rust4d_core/src/entity.rs` - Add `ShapeRef::Asset` variant
- `crates/rust4d_core/src/scene.rs` - Load assets during scene load
- `crates/rust4d_core/src/prefab.rs` - Implement `Asset` trait
- `Cargo.toml` - Add `uuid` dependency

### Phase 5B (Optional)
- `crates/rust4d_core/src/entity.rs` - Add parent/children fields
- `crates/rust4d_core/src/world.rs` - Add hierarchy operations
- `src/scene/scene_builder.rs` - Add hierarchy methods
- `crates/rust4d_core/src/scene.rs` - Nested entity serialization

### Phase 5C
- `crates/rust4d_core/src/scene_manager.rs` - Add transitions, overlays, async loading
- `crates/rust4d_core/src/scene.rs` - Call validator
- `crates/rust4d_core/src/lib.rs` - Export new modules

---

## Test Requirements

### Phase 5A Tests
- Asset loading and caching
- Duplicate detection (same path loaded twice)
- Dependency tracking (add/remove dependents)
- Hot reload detection (file modification times)
- Garbage collection (remove unused assets)
- Asset trait implementations (Prefab)
- Scene integration (load with asset references)

### Phase 5B Tests (Optional)
- Parent/child relationship management
- Cycle detection (prevent circular hierarchies)
- World transform accumulation (3-level hierarchy)
- Recursive deletion (delete parent deletes children)
- Remove from parent
- SceneBuilder hierarchy
- Serialization round-trip with hierarchy

### Phase 5C Tests
- Transition progress updates (0.0 -> 1.0)
- Fade alpha calculation
- Transition completion detection
- Overlay stack operations (push/pop)
- Multiple overlays
- Async scene loading
- Parallel scene loads
- Scene validation (duplicate names, missing assets, invalid physics)

**Total tests to add:** ~30-40 tests

---

## Example Code Snippets

### Asset Management Example

```rust
// Create asset cache
let mut asset_cache = AssetCache::new();
asset_cache.watch_for_changes(true);

// Load scene with asset references
let scene = Scene::load("scenes/level1.ron")?;

// Assets are automatically loaded and cached
let tesseract_shape = asset_cache.load::<Prefab>("assets/prefabs/tesseract.ron")?;

// In game loop: check for hot reloads
let reloaded = asset_cache.check_hot_reload();
for asset_id in reloaded {
    println!("Asset reloaded: {:?}", asset_id);
    // Mark affected entities as dirty
}

// Cleanup unused assets
let freed = asset_cache.gc();
println!("Freed {} unused assets", freed);
```

### Entity Hierarchy Example

```rust
// Build a robot with arms
let mut world = World::new();

// Add body
let body = world.add_entity(
    Entity::with_transform(
        ShapeRef::shared(Tesseract4D::new(1.0)),
        Transform4D::from_position(Vec4::new(0.0, 0.0, 0.0, 0.0)),
        Material::WHITE,
    ).with_name("robot_body")
);

// Add left arm as child (position is relative to body)
let left_arm = world.add_entity(
    Entity::with_transform(
        ShapeRef::shared(Tesseract4D::new(0.3)),
        Transform4D::from_position(Vec4::new(-0.8, 0.0, 0.0, 0.0)),
        Material::GRAY,
    ).with_name("left_arm")
);
world.add_child(body, left_arm)?;

// Move body - arm moves with it automatically
if let Some(body_entity) = world.get_mut(body) {
    body_entity.set_position(Vec4::new(5.0, 0.0, 0.0, 0.0));
}

// Get arm's world position (body pos + arm local pos)
let arm_world_transform = world.world_transform(left_arm);
println!("Arm world position: {:?}", arm_world_transform.position);
```

### Scene Transition Example

```rust
// Start at main menu
scene_manager.switch_to("main_menu")?;

// Player clicks "New Game" - fade to gameplay
scene_manager.switch_to_with_transition(
    "level1",
    TransitionEffect::Fade {
        duration: Duration::from_secs_f32(1.0),
    },
)?;

// In game loop:
scene_manager.update_transition(dt);

// Render with transition alpha
if let Some(transition) = scene_manager.current_transition() {
    let alpha = transition.alpha();
    // Apply fade effect to rendering
}
```

### Scene Overlay Example

```rust
// Main gameplay
scene_manager.switch_to("gameplay")?;

// Player pauses
scene_manager.push_overlay("pause_menu")?;

// Render: draw gameplay, then draw pause_menu on top with 50% alpha

// Player resumes
scene_manager.pop_overlay();
```

### Scene Streaming Example

```rust
// Start loading next level in background
scene_manager.load_scene_async("scenes/level2.ron", "level2");

// Keep playing level 1 while level 2 loads

// In game loop: check for loaded scenes
let loaded = scene_manager.poll_loading();
for scene_name in loaded {
    println!("Scene {} ready!", scene_name);
}

// Player reaches exit - switch immediately (already loaded)
scene_manager.switch_to("level2")?;  // No loading time!
```

---

## Success Criteria

### Phase 5A Success
- [ ] Asset cache loads and caches assets by path
- [ ] Duplicate loads return same cached asset
- [ ] Dependency tracking works (scenes track which assets they use)
- [ ] Hot reload detects file changes and reloads
- [ ] Garbage collection removes unused assets
- [ ] Scene files can reference external asset files
- [ ] `ShapeRef::Asset` variant resolves assets from cache
- [ ] All tests pass

### Phase 5B Success (Optional)
- [ ] Entities can have parent-child relationships
- [ ] Cycle detection prevents circular hierarchies
- [ ] World transforms accumulate through hierarchy
- [ ] Deleting parent deletes all children
- [ ] SceneBuilder supports hierarchy construction
- [ ] Scene serialization preserves hierarchy
- [ ] Round-trip serialization works with nested entities
- [ ] All tests pass

### Phase 5C Success
- [ ] Scene transitions work (fade, crossfade, slide)
- [ ] Transitions provide alpha/progress for rendering
- [ ] Scene overlays render on top of active scene
- [ ] Multiple overlays can be stacked
- [ ] Async scene loading works without blocking
- [ ] Multiple scenes can load in parallel
- [ ] Scene validator detects common errors
- [ ] Validation runs on scene load
- [ ] All tests pass

---

## Priority Notes

### Do First (High ROI)

1. **Phase 5A (Asset Management):** Immediate memory and loading time improvements
2. **Phase 5C.1 (Scene Transitions):** Big polish improvement, relatively easy

### Do If Needed

3. **Phase 5C.2 (Scene Overlays):** Essential for HUD/menus, medium effort
4. **Phase 5C.3 (Scene Streaming):** Nice for large scenes, medium effort

### Defer Unless Needed

5. **Phase 5B (Entity Hierarchy):** Only needed for complex multi-part entities
6. **Phase 5C.4 (Scene Validation):** Nice to have, but errors show up at runtime anyway

### Recommended Order

**If doing all of Phase 5:**
1. Wave 1: Asset Management (5A) - Solo, 1 session
2. Wave 2 (Parallel): Transitions (5C.1) + Overlays (5C.2) - 2 agents, 1 session
3. Wave 3: Streaming (5C.3) + Validation (5C.4) - Solo or parallel, 1 session
4. Wave 4 (Optional): Entity Hierarchy (5B) - Solo, 2-3 sessions

**If doing minimal Phase 5:**
1. Asset Management (5A) - 1 session
2. Scene Transitions (5C.1) - 0.75 sessions
3. Done! Skip the rest unless needed.

---

## Integration with Existing Code

### Phase 2 Dependencies

Phase 5 builds on Phase 2 (Scene Management):

- **Scene Manager:** Extended with transitions, overlays, async loading
- **Scene struct:** Extended with asset loading, validation
- **Prefab system:** Becomes loadable assets

### Phase 1 Dependencies

Phase 5 benefits from Phase 1 (Foundation):

- **Scene serialization:** Asset references use scene file format
- **Configuration:** Asset paths, cache size, hot reload settings

### Rendering Integration

Scene transitions need rendering support:

```rust
// In main.rs render loop:
if let Some(transition) = scene_manager.current_transition() {
    let alpha = transition.alpha();

    match transition.effect() {
        TransitionEffect::Fade { .. } => {
            // Render scene with fade alpha
            clear_color = [0.0, 0.0, 0.0, 1.0 - alpha];
        }
        TransitionEffect::Crossfade { .. } => {
            // Render old scene, then new scene with alpha
        }
        _ => {}
    }
}

// Render overlays on top
for overlay_name in scene_manager.overlays() {
    let overlay_scene = scene_manager.get_scene(overlay_name)?;
    render_scene(overlay_scene, alpha = 0.9);  // Slightly transparent
}
```

---

## Risk Assessment

### Low Risk
- Asset caching (well-understood pattern)
- Scene transitions (pure state machine)
- Scene overlays (simple stack)

### Medium Risk
- Hot reloading (file system watching can be flaky)
- Entity hierarchy (transform accumulation can have edge cases)
- Async loading (threading introduces complexity)

### Mitigation Strategies

**Hot Reloading:**
- Start with simple polling (check every 1 second)
- Use `notify` crate in future for true filesystem watching
- Make hot reload optional (off by default)

**Entity Hierarchy:**
- Extensive testing for cycle detection
- Test 5+ level deep hierarchies
- Document transform order clearly

**Async Loading:**
- Keep threading simple (single loader thread)
- Handle errors gracefully (failed loads don't crash)
- Document thread safety requirements

---

## Documentation Requirements

### API Documentation
- All new public functions must have doc comments
- Include examples in doc comments for main entry points
- Document thread safety for async loading

### User Guide
Add to `docs/user-guide.md`:
- "Working with Assets" section
- "Entity Hierarchies" section (if 5B implemented)
- "Scene Transitions and Overlays" section

### Examples
Create examples:
- `examples/05_asset_loading.rs` - Load and cache assets
- `examples/06_entity_hierarchy.rs` - Build hierarchical entities (if 5B)
- `examples/07_scene_transitions.rs` - Fade between scenes

---

## Future Enhancements (Post-Phase 5)

### Asset System Extensions
- Asset hot reload with dependency graph (reload dependents too)
- Asset compression (smaller files)
- Asset streaming (load parts of large assets)
- Asset versioning (handle format changes)

### Hierarchy Extensions
- Bone/skeleton system for animation
- Attachment points for equipment
- Physics hierarchy (compound colliders)

### Scene Extensions
- Scene pooling (reuse loaded scenes)
- Scene instancing (multiple instances of same scene)
- Scene prefabs (entire scenes as reusable templates)
- Scene scripting hooks (OnLoad, OnUnload events)

---

## Conclusion

Phase 5 transforms Rust4D from a functional engine to a production-ready engine with:
- Efficient asset management (5A)
- Optional entity hierarchy for complex objects (5B)
- Professional scene features (transitions, overlays, streaming) (5C)

**Total effort:** 5-6 sessions (1 for 5A, 2-3 for 5B if needed, 2 for 5C)

**Recommended minimal implementation:**
- 5A (Asset Management): 1 session
- 5C.1 (Transitions): 0.75 sessions
- 5C.2 (Overlays): 0.5 sessions
- **Total minimal:** 2.25 sessions

**Full implementation:**
- All of 5A: 1 session
- All of 5B: 2-3 sessions (optional)
- All of 5C: 2 sessions
- **Total full:** 5-6 sessions

The minimal implementation provides 80% of the value in 40% of the time. Hierarchy (5B) should only be implemented if specifically needed for game features.

---

**Next Phase:** After Phase 5, the engine is production-ready. Future work focuses on polish and advanced features (ECS migration, visual editor, scripting, networking).
