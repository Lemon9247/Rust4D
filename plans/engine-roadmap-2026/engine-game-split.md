# Engine/Game Split + Full ECS Migration

## Overview

Split the Rust4D project into two repositories:
1. **Rust4D Engine** -- a generic 4D game engine library (this repo, refactored)
2. **Rust4D Shooter** -- a new repo implementing the 4D boomer shooter game, distributed via Steam

Additionally, migrate to full ECS (hecs) in the engine before building the game, since every downstream feature depends on a stable entity API.

## Goals
- Clean separation between engine and game code
- Engine is reusable for any 4D game, not just a boomer shooter
- Full hecs-based ECS replaces monolithic Entity
- New `rust4d_game` crate provides common game framework patterns (character controller, event system, state machines)
- Game-specific code (player physics, scene instantiation logic, input mapping) moves to the game repo
- Engine exposes clean public APIs that the game depends on

## Non-Goals
- Building gameplay systems (weapons, enemies, etc.) -- that's the game repo's job
- Changing the rendering pipeline architecture
- Adding new features (raycasting, audio, etc.) -- those come after the split
- Publishing engine crates to crates.io (future concern)

## Decisions

### Game repo depends on engine via git URL (hybrid approach)
The game repo's `Cargo.toml` uses git URL dependencies so it builds on any machine. For local development with both repos side-by-side, a `.cargo/config.toml` override points at the local engine checkout for fast iteration.

```toml
# Rust4D-Shooter/Cargo.toml (committed, works on any machine)
[dependencies]
rust4d_game = { git = "https://github.com/Lemon9247/Rust4D.git" }
rust4d_render = { git = "https://github.com/Lemon9247/Rust4D.git" }
rust4d_input = { git = "https://github.com/Lemon9247/Rust4D.git" }
```

```toml
# Rust4D-Shooter/.cargo/config.toml (for local iteration)
[patch.'https://github.com/Lemon9247/Rust4D.git']
rust4d_game = { path = "../Rust4D/crates/rust4d_game" }
rust4d_render = { path = "../Rust4D/crates/rust4d_render" }
rust4d_input = { path = "../Rust4D/crates/rust4d_input" }
```

### New `rust4d_game` crate in engine workspace
A higher-level crate providing common game patterns that any 4D game would use:
- **CharacterController4D** -- wraps PhysicsWorld body manipulation (movement, jump, grounded detection, edge-falling). This is where player-specific logic extracted from PhysicsWorld lands.
- **Event system** -- simple event bus for game events (collision callbacks, trigger enter/exit)
- **State machine** -- generic FSM for AI, game states, animation states
- **Scene setup helpers** -- utilities for wiring up physics from scene templates

### Collision layer presets stay in engine
`CollisionFilter::player()`, `::enemy()`, etc. are convenience presets -- just named bitflags. They're useful defaults for any game, not boomer-shooter-specific.

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
│   ├── rust4d_game              # NEW: Game framework (CharacterController4D, events, FSM)
│   ├── rust4d_render            # Refactored: works with ECS components instead of Entity
│   └── rust4d_input             # Camera controller (generic)
├── examples/                    # ECS-based examples showcasing the engine API
└── Cargo.toml                   # Workspace, no binary (library-only)

Rust4D-Shooter/                  # Game repo (new, distributed via Steam)
├── src/
│   ├── main.rs                  # Game entry point (winit app)
│   ├── config.rs                # Game configuration
│   ├── systems/                 # Game systems (simulation, render, window)
│   ├── input/                   # Game input mapper
│   └── player/                  # Character controller using rust4d_game
├── scenes/                      # Game scenes (RON)
├── config/                      # Game config (TOML)
├── Cargo.toml                   # Git URL deps on rust4d_* crates
└── .cargo/config.toml           # Local path overrides for dev iteration
```

### Dependency Chain
```
rust4d_math
  ↑
rust4d_physics (depends on math)
  ↑
rust4d_core (depends on math, physics)
  ↑
rust4d_game (depends on core, physics, math)  ← NEW
  ↑
rust4d_render (depends on core, math, input)

Game repo depends on: rust4d_game + rust4d_render + rust4d_input
(rust4d_game re-exports core/physics/math for convenience)
```

### What Stays in the Engine vs. Moves to the Game

#### Stays in Engine (generic)
- `rust4d_math`: All of it. Pure 4D math with no game assumptions.
- `rust4d_physics`: Body simulation, collision detection, collision layers/filters, materials. Collision layer constants (PLAYER, ENEMY, etc.) stay as convenience presets.
- `rust4d_core`: ECS world (hecs), Transform4D, Material, Shape types, scene loading/saving (RON), scene manager, asset cache, hierarchy.
- `rust4d_game`: CharacterController4D, event system, state machine, scene setup helpers.
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
   - **Action**: Extract to `rust4d_game::CharacterController4D`. PhysicsWorld gets generic body manipulation methods. The character controller wraps a BodyKey and calls those methods.

2. **`PhysicsConfig.jump_velocity`** (`world.rs:17`):
   - Jump velocity is gameplay, not physics.
   - **Action**: Remove from PhysicsConfig. Moves to CharacterController4D config.

3. **`ActiveScene::from_template()` tag-based physics setup** (`scene.rs:227-310`):
   - Hardcoded: "static" tag -> StaticCollider, "dynamic" tag -> RigidBody4D with mass 10.0 and PhysicsMaterial::WOOD
   - Player spawn -> kinematic sphere body
   - **Action**: Engine's `from_template()` spawns entities with core components only. `rust4d_game` provides scene setup helpers. Game calls helpers or does custom wiring.

4. **`PhysicsWorld.step()` player-specific simulation** (`world.rs:~193-280`):
   - Resets player grounded state, applies gravity specifically to player, player-specific edge-falling logic
   - **Action**: Generalize gravity to work on any body with a "has gravity" flag. Edge-falling and grounded detection move to CharacterController4D which runs its own post-step logic.

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
- Physics integration (system that syncs Transform4D <-> RigidBody4D)
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
- [ ] Update src/ binary to use new API (temporary, will be removed later)

**Verification:**
- `cargo test --workspace` passes (all 358+ tests)
- Examples run correctly
- Tech demo binary still works

### Phase 2: Extract Game Logic from Engine + Create rust4d_game
**Goal:** Remove game-specific assumptions from the physics and input crates. Create the rust4d_game framework crate. Refactor rust4d_input to use an action/axis abstraction.

**Files (engine repo):**
- `crates/rust4d_game/Cargo.toml` - NEW: depends on rust4d_core, rust4d_physics, rust4d_math
- `crates/rust4d_game/src/lib.rs` - NEW: crate root
- `crates/rust4d_game/src/character_controller.rs` - NEW: CharacterController4D (absorbs player logic from PhysicsWorld)
- `crates/rust4d_game/src/events.rs` - NEW: Simple event bus / channel system
- `crates/rust4d_physics/src/world.rs` - Remove player_body, player_jump_velocity, player-specific methods. Add generic body methods.
- `crates/rust4d_physics/src/lib.rs` - Update exports
- `crates/rust4d_input/src/camera_controller.rs` - Refactor: work with abstract input actions instead of raw KeyCodes
- `crates/rust4d_input/src/input_actions.rs` - NEW: Action/axis abstraction (InputAction enum, InputMap)
- `Cargo.toml` - Add rust4d_game to workspace members

**Specific changes to PhysicsWorld:**
- Remove fields: `player_body: Option<BodyKey>`, `player_jump_velocity: f32`
- Remove methods: `set_player_body()`, `player()`, `player_mut()`, `player_position()`, `player_is_grounded()`, `apply_player_movement()`, `player_jump()`
- Remove from `PhysicsConfig`: `jump_velocity`
- Add generic body methods: `body_is_grounded(key) -> bool`, `apply_body_movement(key, movement)`, `set_body_velocity_xzw(key, velocity)` etc.
- Generalize gravity: any body can opt in to gravity (not just "the player")
- Keep grounded detection and edge-falling as generic per-body logic

**CharacterController4D** (in rust4d_game):
- Wraps a `BodyKey` in a `PhysicsWorld`
- Owns: jump_velocity, move_speed, grounded state tracking
- Methods: `apply_movement()`, `jump()`, `is_grounded()`, `sync_position() -> Vec4`
- Internally calls generic PhysicsWorld body methods

**Input refactor** (in rust4d_input):
- Define engine-level input actions: `MoveForward`, `MoveRight`, `MoveUp`, `MoveW`, `Jump`, `Look` (mouse delta), `RotateW`, `RotateXW`
- `CameraController` operates on these abstract actions, not raw KeyCodes
- NEW: `InputMap` struct maps physical inputs (KeyCode, MouseButton, GamepadButton) to actions
- Engine provides a `default_input_map()` with WASD/mouse as a convenience starting point
- Game creates its own InputMap (or modifies the default) and owns key rebinding

**Tasks:**
- [ ] Create rust4d_game crate with Cargo.toml
- [ ] Remove `jump_velocity` from PhysicsConfig
- [ ] Remove player-specific fields and methods from PhysicsWorld
- [ ] Add generic body manipulation methods to PhysicsWorld
- [ ] Generalize grounded detection and edge-falling to work on any specified body
- [ ] Implement CharacterController4D in rust4d_game
- [ ] Implement basic event system in rust4d_game
- [ ] Create input action/axis abstraction in rust4d_input
- [ ] Refactor CameraController to use abstract actions
- [ ] Create InputMap with default bindings
- [ ] Update all physics tests (97 tests)
- [ ] Update all input tests (37 tests)
- [ ] Add CharacterController4D tests
- [ ] Update src/ binary to use CharacterController4D and InputMap

**Verification:**
- `cargo test --workspace` passes
- Physics demo example still works
- Tech demo binary still works with CharacterController4D and InputMap
- CharacterController4D provides identical behavior to old player methods
- Input behavior is identical with default InputMap

### Phase 3: Make Scene Instantiation Pluggable
**Goal:** Remove hardcoded tag-based physics setup from the engine. The engine loads scenes; the game decides what to do with tags.

**Files (engine repo):**
- `crates/rust4d_core/src/scene.rs` - Simplify ActiveScene::from_template to only spawn core components
- `crates/rust4d_game/src/scene_helpers.rs` - NEW: Helpers for common scene setup patterns (tag-based physics wiring, spawn point detection)

**Design**: Two-step approach.
1. Engine's `from_template()` creates entities with (Name, Tags, Transform4D, ShapeRef, Material) -- no physics interpretation.
2. `rust4d_game` provides `setup_physics_from_tags(world, physics)` that does the "static" -> collider, "dynamic" -> rigid body logic. Game can use this helper or do its own setup.
3. Player spawn stays in the RON format as data. The game reads it and creates CharacterController4D.

**Tasks:**
- [ ] Simplify ActiveScene::from_template to only spawn entities with core components
- [ ] Move tag-based physics setup to rust4d_game::scene_helpers
- [ ] Move player spawn handling to rust4d_game helper
- [ ] Keep Scene.player_spawn in RON format (convention, not engine behavior)
- [ ] Update scene tests
- [ ] Update src/ binary to use scene helpers

**Verification:**
- Scene loading and entity creation works
- Physics setup via game helpers produces identical behavior
- All tests pass

### Phase 4: Create Game Repository
**Goal:** Create the Rust4D-Shooter repo and move game-specific code there.

**New repo structure:**
```
Rust4D-Shooter/
├── Cargo.toml                    # Git URL deps on rust4d_* crates
├── .cargo/config.toml            # Local path overrides for dev
├── src/
│   ├── main.rs                   # From Rust4D/src/main.rs (adapted)
│   ├── config.rs                 # From Rust4D/src/config.rs
│   ├── systems/
│   │   ├── mod.rs
│   │   ├── simulation.rs         # From Rust4D/src/systems/simulation.rs
│   │   ├── render.rs             # From Rust4D/src/systems/render.rs
│   │   └── window.rs             # From Rust4D/src/systems/window.rs
│   └── input/
│       ├── mod.rs
│       └── input_mapper.rs       # From Rust4D/src/input/input_mapper.rs
├── scenes/                       # From Rust4D/scenes/
└── config/                       # From Rust4D/config/
```

**Tasks:**
- [ ] Create new GitHub repo (Rust4D-Shooter)
- [ ] Create Cargo.toml with git URL dependencies on engine crates
- [ ] Create .cargo/config.toml with local path overrides
- [ ] Move src/ files (main.rs, config.rs, systems/, input/)
- [ ] Move scenes/ and config/ directories
- [ ] Adapt all moved code to use new engine APIs (ECS queries, CharacterController4D, scene helpers)
- [ ] Remove src/ binary, scenes/, and config/ from engine repo
- [ ] Keep examples/ in engine repo
- [ ] Verify game builds and runs against engine crates

**Verification:**
- `cargo run` in game repo starts the tech demo with identical behavior
- `cargo test --workspace` in engine repo passes (no game code)
- `cargo test` in game repo passes (game-specific tests)

### Phase 5: Engine Cleanup
**Goal:** Clean up the engine repo now that game code is extracted.

**Tasks:**
- [ ] Remove the `[[bin]]` and `[lib]` sections from root Cargo.toml (workspace-only)
- [ ] Remove `src/lib.rs` (only exported config for tests)
- [ ] Clean up re-exports in crate lib.rs files
- [ ] Have rust4d_game re-export commonly needed types from core/physics/math
- [ ] Update engine README to describe it as a library
- [ ] Update CLAUDE.md for the new two-repo structure
- [ ] Verify engine has no game-specific assumptions left
- [ ] Run full test suite
- [ ] Update examples to showcase ECS + rust4d_game API

**Verification:**
- Engine repo is a pure library workspace (no binary)
- `cargo test --workspace` passes in engine repo
- Game repo builds against engine and runs correctly

## Session Estimates

| Phase | Sessions | Notes |
|-------|----------|-------|
| Phase 1: ECS Migration | 4-6 | Largest phase. 202 core tests + render/physics integration to port. hecs API is different from SlotMap-based World. |
| Phase 2: Game Logic Extraction + rust4d_game | 3-4 | Create new crate, extract player logic, implement CharacterController4D + event system, refactor input to action/axis abstraction. 97 physics + 37 input tests to update. |
| Phase 3: Pluggable Scene Instantiation | 1 | Small refactor of ActiveScene::from_template + scene helpers. |
| Phase 4: Create Game Repo | 1-2 | Moving files, adapting imports, setting up git URL deps. |
| Phase 5: Engine Cleanup | 0.5-1 | Removing dead code, updating docs/config. |
| **Total** | **9.5-14** | |

### Parallelism

Phases 1-3 are sequential (each depends on the previous). Phase 4 depends on 1-3 being complete. Phase 5 can overlap with Phase 4.

```
Phase 1 (ECS Migration)
  -> Phase 2 (Game Logic Extraction + rust4d_game)
    -> Phase 3 (Pluggable Scenes)
      -> Phase 4 (Create Game Repo) + Phase 5 (Engine Cleanup) [parallel]
```

## Resolved Decisions

1. **rust4d_input**: Stays in engine. Refactored in Phase 2 to use an action/axis abstraction. Engine provides abstract input actions + `InputMap` for binding physical inputs to actions + `default_input_map()` convenience. Game owns its InputMap and key rebinding config.

2. **Examples after the split**: Engine examples showcase ECS + rust4d_game API (spawning entities, character controller, events, input maps).

3. **rust4d_game scope**: Start minimal (CharacterController4D + events + scene helpers) and grow organically as the boomer shooter reveals common patterns. State machine, damage system, etc. get added when needed.

## Risks / Considerations

1. **hecs API differences from current World**: hecs uses query iterators, not SlotMap get/set. The hierarchy system (parent/child maps) needs to be reimplemented as components. This is the riskiest part of the migration.

2. **Test porting effort**: 202 core tests + 97 physics tests + integration tests all assume the current Entity/World API. Most will need significant rewriting, not just find-replace.

3. **RenderableGeometry coupling**: Currently iterates entities and reads shape/transform/material directly. With ECS, this becomes a query. The bridge code in `renderable.rs` will need a full rewrite.

4. **Two-repo maintenance**: Coordinating changes across engine and game repos adds friction. Breaking engine API changes require updating the game. Mitigate with semantic versioning and CI.

5. **CharacterController4D fidelity**: The player-specific step logic (grounded detection, edge falling, gravity for kinematic bodies) is non-trivial. The CharacterController4D must reproduce identical behavior to the current player methods during extraction. Test against current behavior before and after.
