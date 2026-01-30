# Engine/Game Split + Full ECS Migration

## Overview

Split the Rust4D project into two repositories:
1. **Rust4D Engine** -- a generic 4D game engine library (this repo, refactored)
2. **Rust4D Shooter** -- a new repo implementing the 4D boomer shooter game

Additionally, migrate to full ECS (hecs) in the engine before building the game, since every downstream feature depends on a stable entity API.

## Goals
- Clean separation between engine and game code
- Engine is reusable for any 4D game, not just a boomer shooter
- Full hecs-based ECS replaces monolithic Entity
- Game-specific code (player physics, scene instantiation logic, input mapping) moves to the game repo
- Engine exposes clean public APIs that the game depends on

## Non-Goals
- Building gameplay systems (weapons, enemies, etc.) -- that's the game repo's job
- Changing the rendering pipeline architecture
- Adding new features (raycasting, audio, etc.) -- those come after the split
- Publishing engine crates to crates.io (future concern)

## Architecture / Design

### Current State
```
Rust4D/
├── crates/
│   ├── rust4d_math      # Pure 4D math (Vec4, Rotor4, shapes)       ← ENGINE
│   ├── rust4d_physics   # 4D physics (bodies, collision, world)     ← ENGINE (with player-specific code mixed in)
│   ├── rust4d_core      # Entity, World, Scene, Assets              ← ENGINE (with game-specific scene logic)
│   ├── rust4d_render    # GPU rendering pipeline                    ← ENGINE
│   └── rust4d_input     # Camera controller                        ← ENGINE
├── src/                 # Tech demo binary + systems                ← GAME-SPECIFIC
├── examples/            # Example programs                          ← ENGINE (stays)
├── scenes/              # RON scene files                           ← GAME-SPECIFIC
└── config/              # TOML config files                         ← GAME-SPECIFIC
```

### Target State
```
Rust4D/                          # Engine repo (this repo, refactored)
├── crates/
│   ├── rust4d_math              # Unchanged (pure math)
│   ├── rust4d_physics           # Refactored: player logic extracted to generic API
│   ├── rust4d_core              # Refactored: ECS via hecs, generic scene instantiation
│   ├── rust4d_render            # Refactored: works with ECS components instead of Entity
│   └── rust4d_input             # Refactored: action/axis abstraction (optional)
├── examples/                    # Simplified examples using new ECS API
└── Cargo.toml                   # Workspace, no binary (library-only)

Rust4D-Shooter/                  # Game repo (new)
├── src/
│   ├── main.rs                  # Game entry point (winit app)
│   ├── config.rs                # Game configuration (from current src/config.rs)
│   ├── systems/                 # Game systems (simulation, render, window)
│   └── input/                   # Game input mapper
├── scenes/                      # Game scenes (from current scenes/)
├── config/                      # Game config (from current config/)
└── Cargo.toml                   # Depends on rust4d engine crates
```

### What Stays in the Engine vs. Moves to the Game

#### Stays in Engine (generic)
- `rust4d_math`: All of it. Pure 4D math with no game assumptions.
- `rust4d_physics`: Body simulation, collision detection, collision layers/filters, materials. The collision layer constants (PLAYER, ENEMY, etc.) stay as **convenience presets** -- they're just named bitflags, not game logic.
- `rust4d_core`: ECS world (hecs), Transform4D, Material, Shape types, scene loading/saving (RON), scene manager, asset cache, hierarchy.
- `rust4d_render`: GPU slicing pipeline, Camera4D, RenderableGeometry (refactored for ECS queries).
- `rust4d_input`: CameraController, CameraControl trait.

#### Moves to Game (game-specific)
- `src/main.rs` -- the winit application, event loop
- `src/systems/` -- SimulationSystem, RenderSystem, WindowSystem
- `src/config.rs` -- game configuration structs
- `src/input/input_mapper.rs` -- key-to-action mapping
- `scenes/` -- RON scene files
- `config/` -- TOML config files

#### Code That Needs Refactoring (currently game-specific, embedded in engine)

1. **`PhysicsWorld` player methods** (`world.rs:54-57, plus ~200 lines of player-specific logic`):
   - `set_player_body()`, `player()`, `player_mut()`, `player_position()`
   - `apply_player_movement()`, `player_jump()`, `player_is_grounded()`
   - `player_body: Option<BodyKey>`, `player_jump_velocity: f32`
   - **Action**: Extract to a generic "character body" API or remove entirely. The PhysicsWorld should not have special knowledge of "the player." Instead, expose generic body manipulation methods that the game's character controller calls.

2. **`PhysicsConfig.jump_velocity`** (`world.rs:17`):
   - Jump velocity is gameplay, not physics.
   - **Action**: Remove from PhysicsConfig. The game's character controller owns jump velocity.

3. **`ActiveScene::from_template()` tag-based physics setup** (`scene.rs:227-310`):
   - Hardcoded: "static" tag → StaticCollider, "dynamic" tag → RigidBody4D with mass 10.0 and PhysicsMaterial::WOOD
   - Player spawn → kinematic sphere body
   - **Action**: Make scene instantiation pluggable. The engine provides a generic `ActiveScene::from_template()` that creates entities but does NOT set up physics. The game provides a callback or builder that wires up physics based on its own tag conventions.

4. **`PhysicsWorld.step()` player-specific simulation** (`world.rs:~193-280`):
   - Resets player grounded state, applies gravity specifically to player, player-specific edge-falling logic
   - **Action**: Generalize to "character bodies" -- any kinematic body with gravity can be a character. Or move the step-player logic into a separate `CharacterPhysics` struct that the game owns.

### ECS Migration Design

Replace the monolithic `Entity` struct and custom `World` with hecs:

**Current Entity struct** (7 fields):
```rust
struct Entity {
    name: Option<String>,
    tags: HashSet<String>,
    transform: Transform4D,
    shape: ShapeRef,
    material: Material,
    physics_body: Option<BodyKey>,
    dirty: DirtyFlags,
}
```

**New ECS components** (in `rust4d_core`):
```rust
// Engine-provided components
struct Name(String);
struct Tags(HashSet<String>);   // Or bitflag tags
struct Transform4D { ... }      // Already exists
struct Material { ... }         // Already exists
struct ShapeRef { ... }         // Already exists
struct PhysicsBody(BodyKey);
struct DirtyFlags { ... }       // Already exists
struct Parent(hecs::Entity);
struct Children(Vec<hecs::Entity>);
```

The game adds its own components:
```rust
// Game-defined components (in Rust4D-Shooter)
struct Health { current: f32, max: f32 }
struct Weapon { ... }
struct AIState { ... }
struct Pickup { ... }
```

**World becomes a thin wrapper around hecs::World** that provides:
- Physics integration (system that syncs Transform4D ↔ RigidBody4D)
- Hierarchy management (Parent/Children components + utility functions)
- Name index (maintained as a side-table or via query)
- Dirty tracking (DirtyFlags component, set by systems)

## Implementation Phases

### Phase 1: ECS Migration in Engine
**Goal:** Replace monolithic Entity/World with hecs-based ECS while keeping everything compiling and tests passing.

**Files (engine repo):**
- `crates/rust4d_core/Cargo.toml` - Add hecs dependency
- `crates/rust4d_core/src/components.rs` - NEW: Engine component types (Name, Tags, Transform4D, Material, ShapeRef, PhysicsBody, DirtyFlags)
- `crates/rust4d_core/src/world.rs` - Rewrite: Thin wrapper around hecs::World with physics integration, hierarchy, name index
- `crates/rust4d_core/src/entity.rs` - Refactor: EntityTemplate stays for serialization, Entity struct removed (replaced by component bundles)
- `crates/rust4d_core/src/scene.rs` - Refactor: ActiveScene::from_template spawns ECS entities with component bundles
- `crates/rust4d_core/src/lib.rs` - Update exports
- `crates/rust4d_render/src/renderable.rs` - Refactor: from_world() uses ECS queries `(Transform4D, ShapeRef, Material)` instead of iterating Entity structs

**Tasks:**
- [ ] Add hecs to workspace dependencies
- [ ] Create engine component types as simple structs
- [ ] Create EntityBundle type for spawning (Transform4D, ShapeRef, Material + optional Name, Tags, PhysicsBody)
- [ ] Rewrite World as hecs::World wrapper with physics integration
- [ ] Port hierarchy system to Parent/Children components
- [ ] Port name index to side-table maintained on spawn/despawn
- [ ] Port dirty tracking to DirtyFlags component
- [ ] Refactor ActiveScene::from_template to spawn ECS entities
- [ ] Refactor RenderableGeometry::from_world to use ECS queries
- [ ] Update EntityTemplate serialization to produce component bundles
- [ ] Port all rust4d_core tests (202 tests)
- [ ] Port all integration tests
- [ ] Update examples to use new ECS API
- [ ] Update src/ binary to use new API (temporary, will be removed in Phase 2)

**Verification:**
- `cargo test --workspace` passes (all 358+ tests)
- Examples run correctly
- Tech demo binary still works

### Phase 2: Extract Player Logic from Physics
**Goal:** Remove game-specific player assumptions from the physics engine.

**Files (engine repo):**
- `crates/rust4d_physics/src/world.rs` - Remove player_body, player_jump_velocity, player-specific methods. Add generic character body API.
- `crates/rust4d_physics/src/body.rs` - No change (RigidBody4D is already generic)
- `crates/rust4d_physics/src/lib.rs` - Update exports

**Specific changes to PhysicsWorld:**
- Remove fields: `player_body: Option<BodyKey>`, `player_jump_velocity: f32`
- Remove methods: `set_player_body()`, `player()`, `player_mut()`, `player_position()`, `player_is_grounded()`, `apply_player_movement()`, `player_jump()`
- Remove from `PhysicsConfig`: `jump_velocity`
- Generalize the step logic: any kinematic body can have gravity applied. The "grounded" state becomes a per-body property or query result.
- Add: `body_is_grounded(key) -> bool`, `apply_body_movement(key, movement)`, `apply_body_jump(key, velocity) -> bool` -- generic versions that work on any body, not just "the player"
- Alternatively: extract a `CharacterController4D` struct that the game instantiates and calls `step()` on, which internally manipulates a `BodyKey` in `PhysicsWorld`

**Tasks:**
- [ ] Remove `jump_velocity` from PhysicsConfig
- [ ] Remove player-specific fields from PhysicsWorld
- [ ] Replace player methods with generic body methods (or CharacterController4D)
- [ ] Generalize grounded detection and edge-falling to work on any specified body
- [ ] Refactor ActiveScene::from_template to not create player body (game does this)
- [ ] Remove player_spawn from ActiveScene (game manages spawn points)
- [ ] Update all physics tests (97 tests)
- [ ] Update src/ binary to use new generic API

**Verification:**
- `cargo test --workspace` passes
- Physics demo example still works
- Tech demo binary still works with refactored player controller

### Phase 3: Make Scene Instantiation Pluggable
**Goal:** Remove hardcoded tag-based physics setup from the engine. The engine loads scenes; the game decides what to do with tags.

**Files (engine repo):**
- `crates/rust4d_core/src/scene.rs` - Refactor ActiveScene::from_template to be generic. Spawn entities with components from template, but don't interpret tags for physics. Provide a hook/callback or builder for the game to add physics.

**Design options:**
1. **Callback approach**: `from_template(template, |entity_template, world| { /* game adds physics */ })`
2. **Two-step approach**: `from_template()` spawns bare entities, game iterates and adds physics components
3. **Builder approach**: `SceneBuilder::new(template).with_physics_setup(|...| {...}).build()`

Option 2 is simplest and most flexible. The engine's `from_template()` creates entities with (Name, Tags, Transform4D, ShapeRef, Material). The game queries for tags and adds PhysicsBody, Health, AIState, etc.

**Tasks:**
- [ ] Simplify ActiveScene::from_template to only spawn entities with core components
- [ ] Remove player_spawn handling from ActiveScene (game manages this via Tags or a SpawnPoint component)
- [ ] Keep Scene.player_spawn in the RON format as a convention the game reads
- [ ] Update scene tests
- [ ] Update src/ binary

**Verification:**
- Scene loading and entity creation works
- Physics is set up by the binary (simulating what the game would do)
- All tests pass

### Phase 4: Create Game Repository
**Goal:** Create the Rust4D-Shooter repo and move game-specific code there.

**New repo structure:**
```
Rust4D-Shooter/
├── Cargo.toml                    # Depends on rust4d_* crates via path or git
├── src/
│   ├── main.rs                   # From Rust4D/src/main.rs (adapted)
│   ├── config.rs                 # From Rust4D/src/config.rs
│   ├── systems/
│   │   ├── mod.rs
│   │   ├── simulation.rs         # From Rust4D/src/systems/simulation.rs
│   │   ├── render.rs             # From Rust4D/src/systems/render.rs
│   │   └── window.rs             # From Rust4D/src/systems/window.rs
│   ├── input/
│   │   ├── mod.rs
│   │   └── input_mapper.rs       # From Rust4D/src/input/input_mapper.rs
│   └── player/
│       ├── mod.rs
│       └── controller.rs         # Character controller using generic physics API
├── scenes/                       # From Rust4D/scenes/
└── config/                       # From Rust4D/config/
```

**Tasks:**
- [ ] Create new repo with Cargo.toml depending on rust4d crates
- [ ] Move src/ files (main.rs, config.rs, systems/, input/)
- [ ] Move scenes/ and config/ directories
- [ ] Create player/controller.rs implementing character controller using generic physics API
- [ ] Adapt all moved code to use new engine APIs (ECS queries, generic physics)
- [ ] Remove src/ binary, scenes/, and config/ from engine repo
- [ ] Keep examples/ in engine repo (simplified ECS examples)
- [ ] Verify game builds and runs against engine crates

**Verification:**
- `cargo run` in game repo starts the tech demo with identical behavior
- `cargo test --workspace` in engine repo passes (no game code)
- `cargo test` in game repo passes (game-specific tests)

### Phase 5: Engine Cleanup
**Goal:** Clean up the engine repo now that game code is extracted.

**Tasks:**
- [ ] Remove the `[[bin]]` section and `src/` from engine Cargo.toml
- [ ] Remove `src/lib.rs` (only exported config for tests)
- [ ] Clean up re-exports in crate lib.rs files (rust4d_core re-exports physics types -- decide if this stays)
- [ ] Update engine README to describe it as a library
- [ ] Update CLAUDE.md for the new two-repo structure
- [ ] Verify engine has no game-specific assumptions left
- [ ] Run full test suite
- [ ] Update examples to showcase the new clean API

**Verification:**
- Engine repo is a pure library workspace (no binary)
- `cargo test --workspace` passes in engine repo
- Game repo builds against engine and runs correctly

## Session Estimates

| Phase | Sessions | Notes |
|-------|----------|-------|
| Phase 1: ECS Migration | 4-6 | Largest phase. 202 core tests + render/physics integration to port. hecs API is different from SlotMap-based World. |
| Phase 2: Extract Player Logic | 1-2 | Focused refactor of PhysicsWorld. ~200 lines of player code to generalize. 97 physics tests to update. |
| Phase 3: Pluggable Scene Instantiation | 1 | Small refactor of ActiveScene::from_template. Scene tests to update. |
| Phase 4: Create Game Repo | 1-2 | Mostly moving files and adapting imports. Character controller wrapper. |
| Phase 5: Engine Cleanup | 0.5-1 | Removing dead code, updating docs/config. |
| **Total** | **7.5-12** | |

### Parallelism

Phases 1-3 are sequential (each depends on the previous). Phase 4 depends on 1-3 being complete. Phase 5 can overlap with Phase 4.

```
Phase 1 (ECS Migration)
  → Phase 2 (Extract Player Logic)
    → Phase 3 (Pluggable Scenes)
      → Phase 4 (Create Game Repo) + Phase 5 (Engine Cleanup) [parallel]
```

## Open Questions

1. **Engine crate dependency method**: Should the game repo depend on engine crates via git URL, path (monorepo-adjacent), or local registry? Git URL is cleanest for separate repos. Path works if they share a parent directory.

2. **Where does `rust4d_input` go?** The CameraController is fairly generic (WASD + mouse), but the key bindings are hardcoded. Options: keep in engine as-is, or split into generic input framework (engine) + specific bindings (game).

3. **Collision layer presets**: `CollisionFilter::player()`, `::enemy()`, etc. are game-convention-flavored but harmless as convenience functions. Keep or remove?

4. **Should the engine provide a "game framework" crate?** A `rust4d_game` crate could provide common game patterns (character controller, state machine, event system) without being specific to any game. This could live in the engine repo but depend on ECS + physics.

5. **Examples after the split**: Should engine examples demonstrate ECS usage patterns, or just raw rendering/physics? They should probably show the ECS API since that's what game developers would use.

## Risks / Considerations

1. **hecs API differences from current World**: hecs uses query iterators, not SlotMap get/set. The hierarchy system (parent/child maps) needs to be reimplemented as components. This is the riskiest part of the migration.

2. **Test porting effort**: 202 core tests + 97 physics tests + integration tests all assume the current Entity/World API. Most will need significant rewriting, not just find-replace.

3. **RenderableGeometry coupling**: Currently iterates entities and reads shape/transform/material directly. With ECS, this becomes a query. The bridge code in `renderable.rs` will need a full rewrite.

4. **Two-repo maintenance**: Coordinating changes across engine and game repos adds friction. Breaking engine API changes require updating the game. Mitigate with semantic versioning and CI.

5. **PhysicsWorld generalization**: The player-specific step logic (grounded detection, edge falling, gravity for kinematic bodies) is non-trivial. Making it generic without losing the behavior requires careful API design. The `CharacterController4D` approach keeps the logic but moves ownership to the game.
