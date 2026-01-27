# Physics Implementation Plan for Rust4D

## Goal
Add physics to the 4D game engine:
- Hyperplane acts as a solid floor
- Tesseract is a rigid body (falls, slides, can be pushed)
- Player has gravity, can jump, and can push the tesseract

## Branch
All work on a new branch: `feature/physics`

---

## Architecture Decision

**Create new `rust4d_physics` crate** (not add to rust4d_core)
- Keeps physics optional and testable in isolation
- Follows existing crate separation pattern

```
rust4d_math (no deps)
       │
       ▼
rust4d_core ──────────────────┐
       │                      │
       ▼                      ▼
rust4d_physics (NEW)    rust4d_render
       │
       └──────────┬───────────┘
                  ▼
              main.rs
```

---

## Key Design Choices

| Question | Decision |
|----------|----------|
| Collision shapes | Sphere4D (player), AABB4D (tesseract), Plane4D (floor) |
| Physics library | Custom (no 4D libraries exist, simple needs) |
| Gravity direction | Y-axis only (matches existing Y-as-up convention) |
| Collisions | Full 4D (objects can collide in W dimension) |
| Player representation | Camera IS the player; physics position syncs to camera |

---

## Implementation Phases (with Parallel Work)

### Wave 1: Foundation (Sequential - Lead Agent)
Must be done first to establish shared types.

**Tasks:**
1. Create branch `feature/physics`
2. Create `rust4d_physics` crate structure
3. Add `clamp_components()` to Vec4
4. Implement collision shapes (AABB4D, Sphere4D, Plane4D)
5. Implement collision detection (Contact, sphere_vs_plane, aabb_vs_plane, sphere_vs_aabb)

**Files:**
- `crates/rust4d_physics/Cargo.toml`
- `crates/rust4d_physics/src/lib.rs`
- `crates/rust4d_physics/src/shapes.rs`
- `crates/rust4d_physics/src/collision.rs`
- `crates/rust4d_math/src/vec4.rs` (modification)

---

### Wave 2: Core Physics (PARALLEL - 2 Agents)

Once Wave 1 is complete, these can run in parallel:

#### Agent A: Physics World Agent
**Scope:** PhysicsWorld, RigidBody4D, body management, gravity simulation

**Files to create:**
- `crates/rust4d_physics/src/body.rs` - RigidBody4D, Collider enum, BodyHandle
- `crates/rust4d_physics/src/world.rs` - PhysicsWorld, PhysicsConfig, step()

**Responsibilities:**
- Implement RigidBody4D with velocity, mass, restitution, gravity flag
- Implement PhysicsWorld with body storage and floor plane
- Implement `step(dt)` - gravity integration, floor collision response
- Write unit tests for physics simulation

#### Agent B: Player Physics Agent
**Scope:** PlayerPhysics, input handling, jump mechanics

**Files to create:**
- `crates/rust4d_physics/src/player.rs` - PlayerPhysics

**Files to modify:**
- `crates/rust4d_input/src/camera_controller.rs` - Add `get_movement_input()`, `consume_jump()`

**Responsibilities:**
- Implement PlayerPhysics with position, velocity, grounded state
- Implement `apply_movement()`, `jump()`, `step()` for player
- Add jump input handling to CameraController
- Write unit tests for player movement

---

### Wave 3: Integration (PARALLEL - 2 Agents)

Once Wave 2 is complete, these can run in parallel:

#### Agent C: Core Integration Agent
**Scope:** Integrate physics into rust4d_core

**Files to modify:**
- `crates/rust4d_core/src/entity.rs` - Add `physics_body: Option<BodyHandle>`
- `crates/rust4d_core/src/world.rs` - Add physics field, implement transform sync
- `crates/rust4d_core/Cargo.toml` - Add rust4d_physics dependency
- `Cargo.toml` (workspace) - Add rust4d_physics member

**Responsibilities:**
- Add optional physics_body handle to Entity
- Add optional PhysicsWorld to World
- Implement `World::update()` to step physics and sync transforms
- Update existing tests

#### Agent D: Main Integration Agent
**Scope:** Wire everything together in main.rs

**Files to modify:**
- `src/main.rs` - Add PlayerPhysics, modify game loop, connect physics

**Responsibilities:**
- Add PlayerPhysics to App struct
- Create PhysicsWorld and add tesseract as dynamic body
- Modify game loop: process input → step player physics → step world physics → sync camera
- Handle push interaction (player→tesseract impulse transfer)
- Manual testing and tuning

---

### Wave 4: Polish (Sequential - Any Agent)
- Tune physics parameters
- Add integration tests
- Write session report

---

## Dependency Graph

```
                    Wave 1 (Sequential)
                    ┌─────────────────┐
                    │ Foundation      │
                    │ - Crate setup   │
                    │ - Shapes        │
                    │ - Collision     │
                    └────────┬────────┘
                             │
              ┌──────────────┴──────────────┐
              ▼                             ▼
    ┌─────────────────┐           ┌─────────────────┐
    │ Agent A         │           │ Agent B         │
    │ Physics World   │           │ Player Physics  │
    │ - RigidBody4D   │           │ - PlayerPhysics │
    │ - PhysicsWorld  │           │ - Input changes │
    └────────┬────────┘           └────────┬────────┘
             │   Wave 2 (Parallel)         │
             └──────────────┬──────────────┘
                            │
              ┌─────────────┴─────────────┐
              ▼                           ▼
    ┌─────────────────┐         ┌─────────────────┐
    │ Agent C         │         │ Agent D         │
    │ Core Integration│         │ Main Integration│
    │ - Entity/World  │         │ - main.rs       │
    └────────┬────────┘         └────────┬────────┘
             │   Wave 3 (Parallel)        │
             └─────────────┬──────────────┘
                           │
                           ▼
                    Wave 4 (Polish)
```

---

## Hive Mind Coordination

Agents should coordinate via: `scratchpad/reports/physics-implementation/hive-mind-physics.md`

**Shared decisions to document:**
- PhysicsConfig defaults (gravity strength, floor Y level)
- BodyHandle type definition (used by both Agent A and Agent C)
- Contact struct definition (used by all collision code)
- Player radius and jump velocity values

---

## Critical Files

| File | Changes |
|------|---------|
| `crates/rust4d_physics/` | **NEW** - Entire physics crate |
| `crates/rust4d_math/src/vec4.rs` | Add `clamp_components()` |
| `crates/rust4d_core/src/entity.rs` | Add `physics_body: Option<BodyHandle>` |
| `crates/rust4d_core/src/world.rs` | Add `PhysicsWorld`, implement physics stepping |
| `crates/rust4d_input/src/camera_controller.rs` | Add jump input, `get_movement_input()` |
| `src/main.rs` | Add PlayerPhysics, modify game loop |

---

## Input Changes

Current Space key behavior: Move up (Y-axis)

New behavior:
- **Grounded**: Space = Jump
- **In air**: Space = No effect (or optional fly mode for debugging)

---

## Collision Algorithms (Summary)

**Sphere vs Plane:**
```
penetration = sphere.radius - plane.signed_distance(sphere.center)
```

**AABB vs Plane:**
```
Find vertex closest to plane (center - half_extents * sign(normal))
penetration = -plane.signed_distance(closest_vertex)
```

**Sphere vs AABB:**
```
closest_point = clamp(sphere.center, aabb.min, aabb.max)
penetration = sphere.radius - distance(sphere.center, closest_point)
```

---

## Verification

1. **Unit tests**: Run `cargo test -p rust4d_physics`
2. **Visual testing**:
   - Launch app: `cargo run`
   - Verify tesseract falls and lands on floor
   - Verify player stands on floor (doesn't fall through)
   - Verify Space key makes player jump
   - Walk into tesseract - verify it gets pushed
3. **Regression**: Run full test suite `cargo test`
