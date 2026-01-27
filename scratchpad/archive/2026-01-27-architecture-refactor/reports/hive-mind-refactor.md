# Hive Mind: Architecture Refactor Research

**Date:** 2026-01-27
**Task:** Research and plan major architectural refactoring

## Goals
1. Review current codebase architecture
2. Research game engine best practices
3. Plan refactoring for:
   - Scene registry (replace hardcoded entities)
   - main.rs decomposition
   - Physics improvements (friction, floor colliders, entity-entity collision)
   - Entity composition system for physics/rendering

## Agent Assignments

### Agent 1: Codebase Review
- Analyze main.rs structure and responsibilities
- Document current entity/physics/rendering architecture
- Identify pain points and coupling issues

### Agent 2: Game Engine Best Practices Research
- Research ECS (Entity-Component-System) patterns
- Research scene management patterns
- Research physics engine architecture (colliders, layers, friction)

### Agent 3: Rust Game Engine Patterns
- Look at how other Rust game engines handle these problems
- Research bevy, rapier, and similar ecosystems
- Find idiomatic Rust patterns for game architecture

## Shared Findings

### Agent 1: Codebase Review (Complete)

**Key Findings:**

1. **main.rs is a 490+ line monolith** handling window, rendering, physics, input, game loop, and scene setup. Needs decomposition into systems.

2. **Entities identified by array index only** - The code uses `if idx == 0` to determine entity type. This is fragile and breaks if entities are reordered.

3. **PhysicsWorld has a hardcoded single floor** - `floor: Plane4D` is baked in. Can't add multiple floors, walls, or arbitrary static colliders.

4. **PlayerPhysics is separate from PhysicsWorld** - Two parallel collision systems. Manual collision code in main.rs handles player-entity interaction.

5. **Full geometry rebuild on any entity movement** - No incremental updates. `build_geometry()` rebuilds everything when one entity moves.

6. **Missing friction** - PhysicsConfig has gravity and restitution but no friction coefficient.

7. **No entity lookup mechanism** - Can't query "find player" or "get all walls". Only array indices.

**Detailed report:** `2026-01-27-codebase-review.md`

---

## Questions for Discussion

- Should we adopt full ECS or a simpler component system?
  - **Agent 1 observation:** The codebase is small enough that a simpler component system may suffice. Full ECS adds complexity.

- How should static colliders (floors, walls) differ from dynamic bodies?
  - **Agent 1 observation:** Current PhysicsWorld has `is_static` flag on bodies BUT the floor is a completely separate `Plane4D` field. Suggest: either (a) static colliders list separate from bodies, or (b) treat all as bodies but efficiently skip static ones.

- What's the right abstraction boundary between physics and rendering?
  - **Agent 1 observation:** Current coupling is through Entity's `physics_body: Option<BodyHandle>`. This is good. Problem is the geometry rebuild - Entity doesn't know if it moved, so we check physics body positions manually.

### Agent 2: Best Practices Research (Complete)

**Key Findings:**

1. **ECS vs Simpler Patterns:**
   - Full ECS is best for 100+ entities needing parallelization. For smaller projects, a simpler Entity-Component (EC) approach or composition-based design is often better.
   - Recommendation: Given Rust4D's current size, use a **lightweight component system** - not full ECS with separate systems scheduling.

2. **Scene Management:**
   - Modern engines moved away from monolithic scene graphs to **multiple specialized structures**: one for hierarchy/transforms, one for spatial queries (octree), one for rendering. Keep them separate.
   - Handle-based entity references (generational indices) are preferred over raw pointers - safer, support serialization, and avoid dangling reference issues.

3. **Physics Architecture:**
   - **Broad phase** (spatial partitioning/AABB) filters potentially colliding pairs; **narrow phase** does precise collision.
   - Static vs dynamic colliders should be separate - static colliders go in acceleration structures that rarely update.
   - **Friction** is per-material property. Combined using `sqrt(friction_a * friction_b)` when objects collide.
   - **Collision layers/masks** allow efficient filtering (e.g., player vs world, enemies vs player, projectiles vs enemies).

4. **Rendering Architecture:**
   - Use **dirty flags** to avoid rebuilding unchanged geometry - can reduce frame times by 80%+.
   - Render queues allow sorting by material/shader to minimize GPU state changes.

5. **4D-Specific Considerations:**
   - 4D visualization uses either slice-based (3D cross-section of 4D object) or projection-based rendering.
   - 4D collision detection extends 3D algorithms (GJK works in 4D). Objects are meshes of tetrahedra; hyperplane intersection tests are key.
   - The axis of 4D rotation is a plane, not a line - this affects physics calculations.

**Detailed report:** `2026-01-27-best-practices-research.md`

### Agent 3: Rust Game Engine Patterns (Complete)

**Key Findings:**

1. **ECS Recommendation:**
   - Bevy uses full ECS with archetype storage for cache efficiency. Best for 1000+ entities.
   - Fyrox (production-ready engine) explicitly **does NOT use ECS** - uses generational arenas (pools) instead.
   - hecs is a minimal ECS library that could work for Rust4D if needed, but current architecture is likely sufficient.
   - **Verdict:** Stay with current approach, but adopt **generational handles** pattern.

2. **Generational Handles (High Priority):**
   - Current `BodyHandle(usize)` is vulnerable to ABA problem (stale handles point to wrong entity).
   - Use `slotmap` crate: O(1) operations, safe iteration, prevents stale handle bugs.
   ```rust
   new_key_type! { pub struct BodyKey; }
   bodies: SlotMap<BodyKey, RigidBody4D>
   ```

3. **Rapier Patterns (Physics):**
   - **Separate RigidBody from Collider** - allows multiple colliders per body, colliders without bodies.
   - **PhysicsMaterial** stores friction + restitution on colliders (not bodies).
   - **Collision groups** use bitmasks: `membership` (what groups I'm in) + `mask` (what I can collide with).
   - **Collision events** collected during physics step, drained by game logic.

4. **Friction Implementation:**
   ```rust
   struct PhysicsMaterial {
       friction: f32,      // 0.0 (ice) to 1.0 (rubber)
       restitution: f32,   // 0.0 (no bounce) to 1.0 (perfect bounce)
   }
   // Combine rule: sqrt(a.friction * b.friction) or average
   ```

5. **Scene Registry Pattern:**
   - Named entity lookup: `HashMap<String, EntityKey>` alongside `SlotMap<EntityKey, Entity>`
   - Fyrox uses strongly-typed handles: `Handle<Camera>` instead of generic `Handle<Node>`

6. **Bundle Pattern (Bevy):**
   - Groups components for convenient spawning. In non-ECS context, equivalent to builder pattern.
   - Rust4D already uses builder pattern (`Entity::with_material(...).with_physics_body(...)`)

**Detailed report:** `2026-01-27-rust-patterns-research.md`

---

## Synthesis: Recommended Architecture Changes

Based on all three agents' research:

### High Priority (Do First)
1. **Add generational handles** - Use `slotmap` for PhysicsWorld bodies and World entities
2. **Add PhysicsMaterial** - friction + restitution per collider, with combine rules
3. **Add collision events** - Vec<CollisionEvent> filled during step(), drained by game logic
4. **Add entity names/tags** - HashMap lookup for "player", "floor", etc.

### Medium Priority
5. **Separate static colliders** - List of static Collider without RigidBody for floors/walls
6. **Add collision groups** - Bitmask filtering for selective collision
7. **Add dirty flags** - Track which entities moved to avoid full geometry rebuild
8. **Decompose main.rs** - Extract into Game struct with setup/update/render phases

### Lower Priority (Future)
9. **Consider hecs** - Only if entity count grows significantly
10. **Spatial partitioning** - Octree/BVH for broad-phase collision (only if many entities)

---

## Planning Complete

**Status:** All research synthesized into implementation plans

**Plan Documents Created:**
- `scratchpad/plans/architecture-refactor/00-long-term-vision.md`
- `scratchpad/plans/architecture-refactor/01-foundation-handles.md`
- `scratchpad/plans/architecture-refactor/02-entity-identity.md`
- `scratchpad/plans/architecture-refactor/03-physics-materials.md`
- `scratchpad/plans/architecture-refactor/04-static-colliders.md`
- `scratchpad/plans/architecture-refactor/05-player-integration.md`
- `scratchpad/plans/architecture-refactor/06-collision-groups.md`
- `scratchpad/plans/architecture-refactor/07-rendering-optimization.md`
- `scratchpad/plans/architecture-refactor/08-main-decomposition.md`

**Estimated Total Sessions:** 9-14

**Next Steps:** Await user approval, then begin Sprint 1 (Phase 1: Foundation)
