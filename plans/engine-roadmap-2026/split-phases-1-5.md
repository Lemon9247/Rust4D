# Phases 1-5: Engine/Game Split Plan Summary

**Full Plan**: `scratchpad/plans/engine-roadmap-2026/engine-game-split.md`
**Status**: Planned (depends on Phase 0 Foundation completion)
**Total Estimated Effort**: 9.5-14 sessions

---

## Overview

This document is a **summary reference** to the full engine/game split plan. The split transforms Rust4D from a single repository containing both engine and game code into two separate repositories:

1. **Rust4D Engine** (this repo, refactored) -- a generic 4D game engine library
2. **Rust4D Shooter** (new repo) -- the 4D boomer shooter game, distributed via Steam

The full plan lives at `scratchpad/plans/engine-roadmap-2026/engine-game-split.md` and contains complete file-level task lists, code examples, and implementation details. This summary captures the key milestones, estimates, and decisions for quick reference.

---

## Architecture: Current State to Target State

### Current State
```
Rust4D/
├── crates/
│   ├── rust4d_math      # Pure 4D math (Vec4, Rotor4, shapes)
│   ├── rust4d_physics   # 4D physics (player-specific code mixed in)
│   ├── rust4d_core      # Entity, World, Scene (game-specific scene logic)
│   ├── rust4d_render    # GPU rendering pipeline
│   └── rust4d_input     # Camera controller
├── src/                 # Tech demo binary + systems (GAME-SPECIFIC)
├── scenes/              # RON scene files (GAME-SPECIFIC)
└── config/              # TOML config files (GAME-SPECIFIC)
```

### Target State
```
Rust4D/                              # Engine repo (library-only)
├── crates/
│   ├── rust4d_math                  # Unchanged (pure math)
│   ├── rust4d_physics               # Refactored: generic body API
│   ├── rust4d_core                  # Refactored: ECS via hecs
│   ├── rust4d_game                  # NEW: CharacterController4D, events, FSM
│   ├── rust4d_render                # Refactored: ECS queries
│   └── rust4d_input                 # Refactored: action/axis abstraction
├── examples/                        # ECS-based examples
└── Cargo.toml                       # Workspace, no binary

Rust4D-Shooter/                      # Game repo (new)
├── src/                             # Game entry point, systems, input
├── scenes/                          # Game scenes (RON)
├── config/                          # Game config (TOML)
├── Cargo.toml                       # Git URL deps on rust4d_* crates
└── .cargo/config.toml               # Local path overrides for dev
```

### Dependency Chain
```
rust4d_math
  ^
rust4d_physics (depends on math)
  ^
rust4d_core (depends on math, physics)
  ^
rust4d_game (depends on core, physics, math)  <-- NEW
  ^
rust4d_render (depends on core, math, input)

Game repo depends on: rust4d_game + rust4d_render + rust4d_input
(rust4d_game re-exports core/physics/math for convenience)
```

---

## Phase Summary Table

| Phase | Name | Sessions | Depends On | Parallelizable? |
|-------|------|----------|------------|-----------------|
| 1 | ECS Migration | 4-6 | Phase 0 Foundation | No (sequential) |
| 2 | Game Logic Extraction + rust4d_game | 3-4 | Phase 1 | No (sequential) |
| 3 | Pluggable Scene Instantiation | 1 | Phase 2 | No (sequential) |
| 4 | Create Game Repository | 1-2 | Phases 1-3 | Yes (with Phase 5) |
| 5 | Engine Cleanup | 0.5-1 | Phase 3 | Yes (with Phase 4) |
| | **Total** | **9.5-14** | | |

### Execution Order
```
Phase 0 (Foundation, 1 session)
  -> Phase 1 (ECS Migration, 4-6 sessions)
    -> Phase 2 (Game Logic Extraction, 3-4 sessions)
      -> Phase 3 (Pluggable Scenes, 1 session)
        -> Phase 4 (Game Repo) + Phase 5 (Cleanup)  [parallel]
```

---

## Phase 1: ECS Migration

**Goal**: Replace monolithic Entity/World with hecs-based ECS

### Key Milestones
- hecs added to workspace dependencies
- Engine component types created: `Name`, `Tags`, `Transform4D`, `Material`, `ShapeRef`, `PhysicsBody`, `DirtyFlags`, `Parent`, `Children`
- `World` rewritten as thin wrapper around `hecs::World`
- `ActiveScene::from_template` spawns ECS entities with component bundles
- `RenderableGeometry::from_world()` uses ECS queries instead of Entity iteration
- Hierarchy system ported to Parent/Children components

### Verification
- `cargo test --workspace` passes (all 358+ tests ported)
- Examples run correctly with new ECS API
- Tech demo binary still works

### What This Enables
- Game-defined components (Health, Weapon, AIState) can be added without modifying engine code
- Query-based iteration for systems (render, physics sync, etc.)
- Foundation for all downstream phases

---

## Phase 2: Game Logic Extraction + rust4d_game

**Goal**: Remove game-specific assumptions from engine. Create `rust4d_game` framework crate.

### Key Milestones
- New `rust4d_game` crate created with `CharacterController4D`
- Player-specific methods removed from `PhysicsWorld` (`set_player_body`, `player_jump`, `apply_player_movement`, etc.)
- Generic body manipulation methods added to `PhysicsWorld` (`body_is_grounded`, `apply_body_movement`, etc.)
- `jump_velocity` removed from `PhysicsConfig` (moves to CharacterController4D)
- Gravity generalized to work on any body (not just "the player")
- Input refactored to action/axis abstraction (`InputAction`, `InputMap`)
- Basic event system in `rust4d_game`

### Verification
- `cargo test --workspace` passes (97 physics + 37 input tests updated)
- `CharacterController4D` produces identical behavior to old player methods
- Tech demo works with `CharacterController4D` and `InputMap`

### What This Enables
- Engine has no game-specific assumptions
- Any game can build its own character controller using generic physics APIs
- Input rebinding becomes possible via `InputMap`
- Physics type serialization cascade can be done here (the deferred Task 2 from Phase 0)

---

## Phase 3: Pluggable Scene Instantiation

**Goal**: Remove hardcoded tag-based physics setup from the engine

### Key Milestones
- `ActiveScene::from_template` simplified to spawn only core components (Name, Tags, Transform4D, ShapeRef, Material)
- Tag-based physics wiring ("static" -> collider, "dynamic" -> rigid body) moved to `rust4d_game::scene_helpers`
- Player spawn handling moved to `rust4d_game` helper
- Scene.player_spawn remains in RON format as data (convention, not engine behavior)

### Verification
- Scene loading and entity creation works with simplified `from_template`
- Physics setup via game helpers produces identical behavior
- All scene tests pass

### What This Enables
- Games can define their own tag interpretations
- Custom physics setup logic per game
- Engine scenes are purely data, not behavior

---

## Phase 4: Create Game Repository

**Goal**: Create the Rust4D-Shooter repo and move game-specific code there

### Key Milestones
- New GitHub repo created (Rust4D-Shooter)
- `Cargo.toml` with git URL dependencies on engine crates
- `.cargo/config.toml` with local path overrides for dev iteration
- Files moved: `src/` (main.rs, config.rs, systems/, input/), `scenes/`, `config/`
- All moved code adapted to use new engine APIs (ECS queries, CharacterController4D, scene helpers)
- `src/` binary, `scenes/`, and `config/` removed from engine repo

### Verification
- `cargo run` in game repo starts the tech demo with identical behavior
- `cargo test --workspace` passes in engine repo (no game code)
- `cargo test` passes in game repo

### What This Enables
- Two-repo development: engine changes don't break game, game changes don't pollute engine
- Game can add gameplay components (Health, Weapon, Enemy) without modifying engine
- Engine is a reusable library for any 4D game project

---

## Phase 5: Engine Cleanup

**Goal**: Clean up the engine repo now that game code is extracted

### Key Milestones
- `[[bin]]` and `[lib]` sections removed from root `Cargo.toml` (workspace-only)
- `src/lib.rs` removed
- Re-exports cleaned up; `rust4d_game` re-exports common types from core/physics/math
- Engine README updated to describe it as a library
- CLAUDE.md updated for two-repo structure
- Examples showcase ECS + `rust4d_game` API

### Verification
- Engine repo is a pure library workspace (no binary)
- `cargo test --workspace` passes
- Game repo builds and runs against engine

### What This Enables
- Clean engine API surface for external consumers
- Engine documentation reflects its actual purpose
- Ready for post-split feature development (combat, audio, editor, etc.)

---

## Key Decisions Already Made

1. **ECS: hecs** -- Full hecs migration chosen over partial ECS (ComponentStore). The partial approach was considered but superseded by the full split plan.

2. **Dependency model: Git URL hybrid** -- Game repo uses `git = "https://..."` deps in committed Cargo.toml (works on any machine), with `.cargo/config.toml` path overrides for local dev iteration.

3. **rust4d_game scope**: Start minimal -- `CharacterController4D` + events + scene helpers. Grows organically as the boomer shooter reveals common patterns. State machine, damage system, etc. added when needed.

4. **Collision layer presets stay in engine** -- `CollisionFilter::player()`, `::enemy()`, etc. are useful defaults for any game, not boomer-shooter-specific.

5. **Input action/axis abstraction in engine** -- `CameraController` works with abstract `InputAction` enum, not raw `KeyCode`. Engine provides `default_input_map()`, game owns its `InputMap` and key rebinding.

---

## Risks and Considerations

1. **hecs API differences**: hecs uses query iterators, not SlotMap get/set. The hierarchy system needs reimplementation as components. This is the riskiest part of the migration.

2. **Test porting effort**: 202 core tests + 97 physics tests + integration tests assume current Entity/World API. Most need significant rewriting, not find-replace.

3. **RenderableGeometry coupling**: Currently iterates entities directly. With ECS, becomes a query. The `renderable.rs` bridge code needs a full rewrite.

4. **Two-repo maintenance**: Coordinating changes across engine and game repos adds friction. Breaking engine API changes require updating the game. Mitigated with semantic versioning and CI.

5. **CharacterController4D fidelity**: The player-specific step logic (grounded detection, edge falling, kinematic body gravity) is non-trivial. Must reproduce identical behavior during extraction. Test against current behavior before and after.

---

## What Post-Split Phases Become Possible

After the split is complete (Phases 1-5), the following feature development phases from the engine roadmap can proceed. Each assumes the split is done and `rust4d_game` exists:

| Post-Split Phase | Engine Work | Game Work | Depends On |
|------------------|-------------|-----------|------------|
| Combat Core (P1) | Raycasting in `rust4d_physics`, collision events, trigger fix | Health, damage, weapons | Split Phase 2 (generic physics API) |
| Weapons & Feedback (P2) | `rust4d_audio` crate (kira), particle system, egui overlay in `rust4d_render` | Weapon types, HUD widgets, screen shake | Split Phase 1 (ECS for components) |
| Enemies & AI (P3) | Sprite billboard pipeline in `rust4d_render`, spatial queries, FSM in `rust4d_game` | Enemy types, AI behaviors, spawn logic | Split Phase 1 (ECS), P1 (raycasting) |
| Level Design (P4) | Shape types in `rust4d_math`, RON preview tool, tween system in `rust4d_game` | Door/elevator logic, pickups, level scripts | Split Phase 3 (pluggable scenes) |
| Editor & Polish (P5) | `rust4d_editor` crate (egui), point lights, textures, input rebinding | Game-specific editor panels | Split Phases 1-5 complete |

The engine/game split is the critical path that gates all gameplay feature development.
