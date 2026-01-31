# Post-Split Phase 1: Combat Core (Engine Side)

**Date**: 2026-01-31
**Updated 2026-01-31**: Integrated Lua scripting amendments -- Lua bindings for raycasting, collision events, and trigger callbacks are now part of this plan.
**Status**: Planning Document (Implementation-Ready)
**Depends On**: Engine/Game Split Plan (Phases 1-3 complete: ECS migration done, `rust4d_game` exists, game repo exists); `rust4d_scripting` crate with mlua integration (for Sub-Phase C)
**Engine Estimate**: 2.25-2.5 sessions (originally 1.75; +0.5-0.75 for Lua bindings)
**Game Estimate**: Lua scripts replace compiled Rust game code; game-side work is writing Lua scripts that call engine APIs

---

## 1. Overview

Combat Core is the first post-split engine phase. It provides the fundamental physics primitives that every combat game needs: raycasting (for weapons, line-of-sight, picking) and collision event reporting (for triggers, damage zones, gameplay callbacks).

The original cross-swarm synthesis scoped Phase 1 as four features across 3-4 sessions:
1. 4D raycasting
2. Event system
3. Health/damage system
4. Trigger zone callbacks

After Agent P1's detailed review of the actual engine source code and the engine/game boundary, the scope narrows significantly on the engine side:

- **Raycasting** is entirely engine work, split across `rust4d_math` (geometric primitive) and `rust4d_physics` (world queries). This is the most substantial item.
- **Event system**: The engine does NOT need a general-purpose event bus. The physics engine must *produce* collision event DATA. With the Lua scripting architecture, the game consumes these events through registered Lua callbacks rather than a Rust-side `EventBus`.
- **Health/damage** is purely game-side (Lua scripts). The engine needs nothing.
- **Trigger callbacks**: The trigger infrastructure is half-built but has a design bug (asymmetric detection never fires). The engine must fix this and report trigger overlaps as events. With Lua, triggers invoke Lua functions directly instead of routing through a Rust `GameEvent(String)` pattern.

**What the Lua architecture changes**: The engine's internal Rust implementation is completely unchanged. What changes is the *surface area the engine exposes*. Everything that was "game implements in Rust" now needs Lua bindings so scripts can call it. Additionally, some game-framework types become unnecessary because Lua provides equivalent capability natively.

**What gets simpler with Lua**:
- **Trigger system**: `GameEvent(String)` was a workaround for not having a scripting language. With Lua, triggers call Lua functions directly. The string-to-game-event translation layer is unnecessary. Triggers become: detect overlap -> call Lua function. Much cleaner.
- **EventBus in rust4d_game**: The general-purpose `EventBus` with typed `GameEvent` enum is no longer needed as a Rust type for game-side consumption. Lua scripts register callbacks directly with the engine. A Lua-side event system (a simple pub/sub table) replaces the Rust `EventBus` for game events.

**What is removed from engine scope**:
- **`rust4d_game::events::EventBus`**: No longer needed. The engine's collision event reporting remains (internal Rust), but game-side event dispatch is now Lua-native.
- **`GameEvent` enum**: This typed Rust enum (Damage, Pickup, TriggerEnter, etc.) is replaced by Lua tables and strings -- no Rust enum needed.
- **`StateMachine` dependency**: This phase no longer depends on or feeds into a Rust-side FSM. Lua handles state management natively.

**Why this matters**: Without raycasting, no hitscan weapons, no line-of-sight, no picking. Without collision events, no way for the game to know when things collide or enter trigger zones. Without Lua bindings, none of this is accessible to game scripts. These are the minimum primitives the engine must expose for any combat gameplay to exist.

---

## 2. Engine vs Game Boundary

This phase draws a clear line. With the Lua scripting architecture, the boundary shifts: items previously implemented as game-side Rust code now need Lua bindings so game scripts can call them.

### Engine Provides (this plan -- Rust implementation)
- `Ray4D` geometric primitive in `rust4d_math`
- Ray-shape intersection functions in `rust4d_physics` (ray vs sphere, AABB, hyperplane)
- `PhysicsWorld::raycast()` with layer mask filtering
- `CollisionEvent` data structs reporting what collided during `step()`
- `PhysicsWorld::drain_collision_events()` poll API
- Asymmetric trigger overlap detection (fixing the current bug)
- Trigger enter/stay/exit tracking
- `Vec4` utility additions (`distance()`, `distance_squared()`, `f32 * Vec4`)
- `sphere_vs_sphere` visibility fix (currently private, should be public standalone)

### Engine Provides (this plan -- Lua bindings)
- `world:raycast(origin, direction, max_dist, layer_mask)` returning Lua table of hits
- `world:raycast_nearest(origin, dir, max_dist, mask)` returning hit table or `nil`
- `CollisionLayer` constants exposed to Lua scope (`LAYER.PLAYER`, `LAYER.ENEMY`, `LAYER.STATIC`, etc.)
- Lua callback registration: `on_collision(callback)`, `on_trigger_enter(callback)`, `on_trigger_stay(callback)`, `on_trigger_exit(callback)`
- Event dispatch: engine calls `drain_collision_events()` internally each frame and dispatches to registered Lua callbacks

### Lua Binding Boundary Details

The following table shows what was previously game-side Rust and is now exposed through Lua bindings:

| Was (Rust game code) | Becomes (Engine Lua binding) |
|---------------------|------------------------------|
| Game calls `physics_world.raycast()` in Rust | `world:raycast(origin, direction, max_dist, layer_mask)` returns Lua table of hits |
| Game calls `physics_world.raycast_nearest()` in Rust | `world:raycast_nearest(origin, dir, max_dist, mask)` returns hit or nil |
| Game calls `physics_world.drain_collision_events()` in Rust | Engine dispatches events to registered Lua callbacks |
| Game reads `WorldRayHit.hit.distance`, `.hit.point`, `.hit.normal`, `.target` | Lua hit table: `{ distance=N, point=vec4, normal=vec4, target={type="body", key=K} }` |
| Game reads `CollisionEvent.kind` variants | Lua event tables: `{ type="body_vs_body", body_a=K, body_b=K, contact={...} }` |
| Game iterates `CollisionLayer` bitflags for filtering | `CollisionLayer` exposed as Lua constants: `LAYER.PLAYER`, `LAYER.ENEMY`, `LAYER.STATIC`, etc. |
| `EventBus` in `rust4d_game::events` dispatches typed Rust events | Engine provides Lua callback registration: `on_collision(function(event) ... end)` |

### Game Implements (Lua scripts, not in this plan)
- `Health { current, max }` -- purely game-defined Lua table
- Damage system -- Lua scripts read collision events + raycasts, modify health tables
- Death handling -- Lua game logic triggered by `health.current <= 0`
- Weapon hitscan/projectile logic that calls `world:raycast()`
- AI line-of-sight wrappers around `world:line_of_sight()`
- Trigger-to-gameplay translation (what happens when player enters a trigger zone -- now a Lua callback function)

Agent P1 was emphatic: **Health/damage is 100% game-side.** Different games have wildly different health models (HP bars, shield + health, damage types, armor, invulnerability frames). The engine provides collision events and raycasting; the game defines what "taking damage" means. With Lua, this remains true -- the game implements health in Lua scripts, not Rust types.

---

## 3. Sub-Phase A: Raycasting

### Rationale

Raycasting is a fundamental engine primitive. Every game needs it for weapons, line-of-sight, picking, UI interaction. This is not game-specific.

### Current State

- `rust4d_math` has `Vec4`, and the physics crate has `Sphere4D`, `AABB4D`, `Plane4D` in `rust4d_physics::shapes`. No ray type exists anywhere.
- `rust4d_physics` has collision detection between shapes (`sphere_vs_plane`, `sphere_vs_aabb`, `aabb_vs_aabb`, `aabb_vs_plane`) but zero ray intersection tests.
- `rust4d_physics::PhysicsWorld` has `SlotMap<BodyKey, RigidBody4D>` for dynamic bodies and `Vec<StaticCollider>` for static geometry. A world raycast must check both.
- The collision filter system (`CollisionLayer`, `CollisionFilter`) is ready to be reused for ray filtering.

### Design

The raycasting system is split across two crates following the existing pattern where `rust4d_math` holds pure math types and `rust4d_physics` holds physics simulation types:

**Layer 1 -- `rust4d_math`**: `Ray4D` struct and utility methods. This belongs in the math crate because rays are a geometric primitive, like vectors and rotations. Non-physics uses (debug visualization, picking, UI raycasting) should not require depending on the physics crate.

**Layer 2 -- `rust4d_physics`**: Ray-shape intersection functions and `PhysicsWorld::raycast()`. Since collision shapes (`Sphere4D`, `AABB4D`, `Plane4D`) live in `rust4d_physics::shapes`, the ray-shape intersection functions must also live in `rust4d_physics`.

**Alternative considered and rejected**: Put `Ray4D` in `rust4d_physics`. Simpler but architecturally wrong -- a ray is a math primitive, not a physics concept.

### API Design

#### `rust4d_math::ray` (new module)

```rust
// File: crates/rust4d_math/src/ray.rs

/// A ray in 4D space defined by an origin point and a normalized direction
#[derive(Clone, Copy, Debug)]
pub struct Ray4D {
    /// Starting point of the ray
    pub origin: Vec4,
    /// Direction of the ray (should be normalized)
    pub direction: Vec4,
}

impl Ray4D {
    /// Create a new ray. Direction will be normalized automatically.
    pub fn new(origin: Vec4, direction: Vec4) -> Self {
        Self {
            origin,
            direction: direction.normalized(),
        }
    }

    /// Get the point along the ray at parameter t.
    /// point(t) = origin + direction * t
    #[inline]
    pub fn point_at(&self, t: f32) -> Vec4 {
        self.origin + self.direction * t
    }
}
```

#### `rust4d_physics::raycast` (new module)

```rust
// File: crates/rust4d_physics/src/raycast.rs

use rust4d_math::{Ray4D, Vec4};
use crate::shapes::{Sphere4D, AABB4D, Plane4D, Collider};

/// Result of a ray intersection test
#[derive(Clone, Copy, Debug)]
pub struct RayHit {
    /// Distance along the ray to the hit point (parameter t)
    pub distance: f32,
    /// World-space position of the hit
    pub point: Vec4,
    /// Surface normal at the hit point
    pub normal: Vec4,
}

/// Cast a ray against a 4D sphere.
///
/// Returns the nearest intersection point, or None if the ray misses.
/// Only returns hits with t >= 0 (in front of the ray origin).
pub fn ray_vs_sphere(ray: &Ray4D, sphere: &Sphere4D) -> Option<RayHit> {
    // Standard ray-sphere intersection, dimension-agnostic:
    // |origin + t*dir - center|^2 = r^2
    // Expand: t^2(dir.dir) + 2t(dir.(origin-center)) + (origin-center).(origin-center) - r^2 = 0
    let oc = ray.origin - sphere.center;
    let a = ray.direction.dot(ray.direction); // should be 1.0 if normalized
    let b = 2.0 * ray.direction.dot(oc);
    let c = oc.dot(oc) - sphere.radius * sphere.radius;
    let discriminant = b * b - 4.0 * a * c;

    if discriminant < 0.0 {
        return None;
    }

    let sqrt_disc = discriminant.sqrt();
    let t1 = (-b - sqrt_disc) / (2.0 * a);
    let t2 = (-b + sqrt_disc) / (2.0 * a);

    // Pick the nearest positive t
    let t = if t1 >= 0.0 { t1 } else if t2 >= 0.0 { t2 } else { return None; };

    let point = ray.point_at(t);
    let normal = (point - sphere.center).normalized();

    Some(RayHit { distance: t, point, normal })
}

/// Cast a ray against a 4D AABB using the slab method.
///
/// The slab method generalizes trivially to any number of dimensions.
/// For each axis, compute entry/exit t values and intersect the intervals.
pub fn ray_vs_aabb(ray: &Ray4D, aabb: &AABB4D) -> Option<RayHit> {
    let mut t_min = f32::NEG_INFINITY;
    let mut t_max = f32::INFINITY;
    let mut hit_axis = 0usize;
    let mut hit_sign = 1.0f32;

    // Check each of the 4 axes (x, y, z, w)
    for axis in 0..4 {
        let (origin, dir, min_val, max_val) = match axis {
            0 => (ray.origin.x, ray.direction.x, aabb.min.x, aabb.max.x),
            1 => (ray.origin.y, ray.direction.y, aabb.min.y, aabb.max.y),
            2 => (ray.origin.z, ray.direction.z, aabb.min.z, aabb.max.z),
            3 => (ray.origin.w, ray.direction.w, aabb.min.w, aabb.max.w),
            _ => unreachable!(),
        };

        if dir.abs() < 1e-8 {
            // Ray is parallel to this slab
            if origin < min_val || origin > max_val {
                return None; // Outside the slab, no intersection
            }
            // Inside the slab, this axis doesn't constrain t
            continue;
        }

        let t1 = (min_val - origin) / dir;
        let t2 = (max_val - origin) / dir;

        let (t_near, t_far, near_sign) = if t1 < t2 {
            (t1, t2, -1.0f32) // entering from min side
        } else {
            (t2, t1, 1.0f32)  // entering from max side
        };

        if t_near > t_min {
            t_min = t_near;
            hit_axis = axis;
            hit_sign = near_sign;
        }
        t_max = t_max.min(t_far);

        if t_min > t_max {
            return None; // No intersection
        }
    }

    if t_max < 0.0 {
        return None; // AABB is behind the ray
    }

    // If ray origin is inside AABB (t_min < 0), use distance 0 convention
    let t = if t_min >= 0.0 { t_min } else { 0.0 };

    let point = ray.point_at(t);

    // Normal is along the axis we entered
    let mut normal = Vec4::ZERO;
    match hit_axis {
        0 => normal.x = hit_sign,
        1 => normal.y = hit_sign,
        2 => normal.z = hit_sign,
        3 => normal.w = hit_sign,
        _ => unreachable!(),
    }

    Some(RayHit { distance: t, point, normal })
}

/// Cast a ray against a 4D plane (hyperplane).
///
/// Returns the intersection point if the ray is not parallel to the plane
/// and the intersection is in front of the ray origin.
pub fn ray_vs_plane(ray: &Ray4D, plane: &Plane4D) -> Option<RayHit> {
    let denom = ray.direction.dot(plane.normal);

    if denom.abs() < 1e-8 {
        return None; // Ray is parallel to the plane
    }

    let t = (plane.distance - ray.origin.dot(plane.normal)) / denom;

    if t < 0.0 {
        return None; // Intersection is behind the ray
    }

    let point = ray.point_at(t);
    // Normal faces toward the ray origin side
    let normal = if denom < 0.0 { plane.normal } else { -plane.normal };

    Some(RayHit { distance: t, point, normal })
}

/// Cast a ray against any Collider variant.
pub fn ray_vs_collider(ray: &Ray4D, collider: &Collider) -> Option<RayHit> {
    match collider {
        Collider::Sphere(sphere) => ray_vs_sphere(ray, sphere),
        Collider::AABB(aabb) => ray_vs_aabb(ray, aabb),
        Collider::Plane(plane) => ray_vs_plane(ray, plane),
    }
}
```

#### `PhysicsWorld::raycast()` (addition to `world.rs`)

```rust
// Added to: crates/rust4d_physics/src/world.rs

use crate::raycast::{self, RayHit};
use crate::collision::CollisionLayer;
use rust4d_math::Ray4D;

/// Result of a world-level raycast, identifying what was hit
#[derive(Clone, Copy, Debug)]
pub struct WorldRayHit {
    /// The ray intersection details (distance, point, normal)
    pub hit: RayHit,
    /// What was hit
    pub target: RayTarget,
}

/// What a world raycast hit
#[derive(Clone, Copy, Debug)]
pub enum RayTarget {
    /// Hit a dynamic/kinematic body
    Body(BodyKey),
    /// Hit a static collider (index into static_colliders vec)
    Static(usize),
}

impl PhysicsWorld {
    /// Cast a ray through the physics world, returning all hits sorted by distance.
    ///
    /// `layer_mask` filters which collision layers the ray interacts with.
    /// Only bodies/colliders whose layer intersects the mask will be tested.
    pub fn raycast(
        &self,
        ray: &Ray4D,
        max_distance: f32,
        layer_mask: CollisionLayer,
    ) -> Vec<WorldRayHit> {
        let mut hits = Vec::new();

        // Test against all dynamic/kinematic bodies
        for (key, body) in &self.bodies {
            if body.is_static() {
                continue;
            }
            if !body.filter.layer.intersects(layer_mask) {
                continue;
            }
            if let Some(hit) = raycast::ray_vs_collider(ray, &body.collider) {
                if hit.distance <= max_distance {
                    hits.push(WorldRayHit {
                        hit,
                        target: RayTarget::Body(key),
                    });
                }
            }
        }

        // Test against all static colliders
        for (i, static_col) in self.static_colliders.iter().enumerate() {
            if !static_col.filter.layer.intersects(layer_mask) {
                continue;
            }
            if let Some(hit) = raycast::ray_vs_collider(ray, &static_col.collider) {
                if hit.distance <= max_distance {
                    hits.push(WorldRayHit {
                        hit,
                        target: RayTarget::Static(i),
                    });
                }
            }
        }

        // Sort by distance (nearest first)
        hits.sort_by(|a, b| a.hit.distance.partial_cmp(&b.hit.distance).unwrap());
        hits
    }

    /// Cast a ray and return only the nearest hit.
    ///
    /// More efficient than `raycast()` when you only need the first hit,
    /// though the current implementation is equivalent. Future optimization
    /// with spatial acceleration structures would benefit this path.
    pub fn raycast_nearest(
        &self,
        ray: &Ray4D,
        max_distance: f32,
        layer_mask: CollisionLayer,
    ) -> Option<WorldRayHit> {
        self.raycast(ray, max_distance, layer_mask).into_iter().next()
    }
}
```

### Vec4 Utility Additions

The raycasting work reveals two gaps in `Vec4` that should be fixed alongside this work:

```rust
// Added to crates/rust4d_math/src/vec4.rs

impl Vec4 {
    /// Euclidean distance between two points
    #[inline]
    pub fn distance(self, other: Self) -> f32 {
        (self - other).length()
    }

    /// Squared Euclidean distance between two points (avoids sqrt)
    #[inline]
    pub fn distance_squared(self, other: Self) -> f32 {
        (self - other).length_squared()
    }
}

// Missing commutative multiplication: f32 * Vec4
impl std::ops::Mul<Vec4> for f32 {
    type Output = Vec4;
    #[inline]
    fn mul(self, v: Vec4) -> Vec4 {
        v * self
    }
}
```

### `sphere_vs_sphere` Visibility Fix

`sphere_vs_sphere` currently exists as a private method on `PhysicsWorld` (line 265 of `world.rs`). It should be a public standalone function like the other collision tests (`sphere_vs_plane`, `sphere_vs_aabb`, etc.). This is a quick fix to do alongside the raycasting work.

### File List

| File | Action | Crate |
|------|--------|-------|
| `crates/rust4d_math/src/ray.rs` | NEW | rust4d_math |
| `crates/rust4d_math/src/lib.rs` | EDIT: add `pub mod ray;` and `pub use ray::Ray4D;` | rust4d_math |
| `crates/rust4d_math/src/vec4.rs` | EDIT: add `distance()`, `distance_squared()`, `f32 * Vec4` | rust4d_math |
| `crates/rust4d_physics/src/raycast.rs` | NEW | rust4d_physics |
| `crates/rust4d_physics/src/world.rs` | EDIT: add `raycast()`, `raycast_nearest()`, `WorldRayHit`, `RayTarget` types | rust4d_physics |
| `crates/rust4d_physics/src/world.rs` | EDIT: make `sphere_vs_sphere` a public standalone function | rust4d_physics |
| `crates/rust4d_physics/src/lib.rs` | EDIT: add `pub mod raycast;` + re-exports | rust4d_physics |

### Tests Required

- `ray_vs_sphere`: miss, tangent, through center, origin inside sphere, behind ray
- `ray_vs_aabb`: miss, hit each face pair (8 faces in 4D), parallel to axis, origin inside AABB
- `ray_vs_plane`: hit from above, hit from below, parallel miss, behind ray
- `ray_vs_collider`: dispatch test for each variant
- `PhysicsWorld::raycast`: hit body, hit static, miss all, layer filtering, max_distance cutoff, multiple hits sorted by distance
- `PhysicsWorld::raycast_nearest`: returns closest hit
- `Vec4::distance`: basic distance test
- `f32 * Vec4`: commutativity test

### Session Estimate

**1 session.** The math is straightforward (dimension-agnostic ray-shape tests). The world raycast is a linear scan (no spatial acceleration needed with current body counts). The bulk of the work is writing tests.

---

## 4. Sub-Phase B: Collision Events & Trigger System

### Rationale

The physics engine currently resolves collisions silently -- it pushes bodies apart and modifies velocities, but produces no data about what collided with what. For the game to fire events (damage, trigger enter/exit, pickups), the engine must report collision data.

Additionally, the existing trigger infrastructure has a **design bug** that must be fixed: triggers never detect players due to the symmetric `collides_with()` check.

### Current State -- Collision Events

- `PhysicsWorld::step()` resolves collisions but discards all information about what collided.
- No `CollisionEvent` type exists.
- The game has no way to know what hit what.

### Current State -- Trigger System (Bug Analysis)

The trigger infrastructure is **half-built**:

1. `CollisionLayer::TRIGGER` exists (bit 4 in the bitflags).
2. `CollisionFilter::trigger(detects: CollisionLayer)` exists -- creates a filter where the trigger detects specified layers.
3. The asymmetric collision design is intentional: triggers detect overlaps, but the detected objects should not push against triggers.
4. **BUG**: In `world.rs`, the collision filter check `body.filter.collides_with(&static_col.filter)` uses the SYMMETRIC `collides_with()` which requires BOTH filters to agree. Since `CollisionFilter::player()` excludes TRIGGER from its mask, the trigger-player collision is never detected.

From `collision.rs` line 117-123 (trigger filter):
```rust
pub fn trigger(detects: CollisionLayer) -> Self {
    Self {
        layer: CollisionLayer::TRIGGER,
        mask: detects,
    }
}
```

From player filter (excludes TRIGGER):
```rust
pub fn player() -> Self {
    Self {
        layer: CollisionLayer::PLAYER,
        mask: CollisionLayer::ALL
            & !CollisionLayer::PLAYER
            & !CollisionLayer::PROJECTILE
            & !CollisionLayer::TRIGGER,  // Player ignores triggers
    }
}
```

The test at line 530 documents this:
```rust
// The trigger layer is not in player's mask, so symmetric check fails
// This is intentional: triggers detect but don't push
assert!(!trigger.collides_with(&player));
```

The comment says "intentional" but the consequence is that triggers NEVER detect players. The design intent (triggers detect without pushing) requires an ASYMMETRIC overlap check, separate from the push/response collision check.

### Design: CollisionEvent Data

```rust
// Added to: crates/rust4d_physics/src/collision.rs (or new collision_event.rs)

/// A collision event that occurred during a physics step
#[derive(Clone, Debug)]
pub struct CollisionEvent {
    /// The type of collision
    pub kind: CollisionEventKind,
    /// Contact information
    pub contact: Contact,
}

/// What kind of collision occurred
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CollisionEventKind {
    /// Two bodies collided (physics response was applied)
    BodyVsBody {
        body_a: BodyKey,
        body_b: BodyKey,
    },
    /// A body collided with a static collider (physics response was applied)
    BodyVsStatic {
        body: BodyKey,
        static_index: usize,
    },
    /// A body entered a trigger zone this frame
    TriggerEnter {
        body: BodyKey,
        trigger_index: usize,
    },
    /// A body is still inside a trigger zone (ongoing overlap)
    TriggerStay {
        body: BodyKey,
        trigger_index: usize,
    },
    /// A body exited a trigger zone this frame
    TriggerExit {
        body: BodyKey,
        trigger_index: usize,
    },
}
```

**Design note on `TriggerStay`**: Included because it is trivial to add (any overlap also in `active_triggers`) and some games need it (continuous damage zones). Can be omitted if performance profiling shows event volume is too high.

### Design: Drain API (Preferred over Return Value)

The recommended approach is poll/drain rather than changing `step()`'s return type:

```rust
// In PhysicsWorld struct definition, add:
collision_events: Vec<CollisionEvent>,

// In step(), after resolving each collision, push an event.
// After step returns, game code calls drain_collision_events().

impl PhysicsWorld {
    /// Get collision events from the last step, emptying the buffer.
    pub fn drain_collision_events(&mut self) -> Vec<CollisionEvent> {
        std::mem::take(&mut self.collision_events)
    }
}
```

**Why drain/poll over return value**: Simpler, does not change the `step()` signature (which would be a breaking change across many call sites).

**With Lua**: The engine calls `drain_collision_events()` internally each frame and dispatches events to registered Lua callbacks. Game scripts never call drain directly -- they register callbacks and the engine invokes them. See Sub-Phase C for the Lua dispatch design.

Game-side usage pattern (Lua):
1. Engine calls `physics_world.step(dt)` internally
2. Engine calls `physics_world.drain_collision_events()` internally
3. For each collision event, engine dispatches to registered Lua callbacks
4. Lua callbacks handle game logic (damage, pickups, effects, etc.)

### Design: Two-Pass Collision Detection (Trigger Fix)

Fix the trigger bug by separating **trigger detection** from **collision response**:

```rust
// In PhysicsWorld::step(), add a new phase:
// Phase 2b: Detect trigger overlaps (no physics response)
self.detect_trigger_overlaps();
```

```rust
impl PhysicsWorld {
    fn detect_trigger_overlaps(&mut self) {
        // Check each body against trigger-tagged static colliders
        for (key, body) in &self.bodies {
            if body.is_static() {
                continue;
            }

            for (i, static_col) in self.static_colliders.iter().enumerate() {
                // Asymmetric check: does this static collider's mask include the body's layer?
                // (One-way detection: trigger wants to detect the body)
                if !static_col.filter.layer.contains(CollisionLayer::TRIGGER) {
                    continue;
                }
                if !static_col.filter.mask.intersects(body.filter.layer) {
                    continue;
                }

                // Check geometric overlap
                let contact = Self::check_static_collision(&body.collider, &static_col.collider);
                if let Some(contact) = contact {
                    if contact.is_colliding() {
                        self.collision_events.push(CollisionEvent {
                            kind: CollisionEventKind::TriggerOverlap {
                                body: key,
                                trigger_index: i,
                            },
                            contact,
                        });
                    }
                }
            }

            // Body-body trigger detection deferred until a game use case requires it
        }
    }
}
```

### Design: Enter/Exit Tracking

For `TriggerEnter` / `TriggerStay` / `TriggerExit` events (analogous to Unity's `OnTriggerEnter` / `OnTriggerStay` / `OnTriggerExit`), the physics engine needs to track which body-trigger pairs were overlapping in the previous frame:

```rust
// In PhysicsWorld struct:
/// Active trigger overlaps from the previous step (for enter/exit detection)
active_triggers: HashSet<(BodyKey, usize)>,

// In detect_trigger_overlaps():
let mut current_overlaps = HashSet::new();
// ... for each overlap detected:
current_overlaps.insert((key, i));

// Compare with previous frame:
for &pair in &current_overlaps {
    if !self.active_triggers.contains(&pair) {
        // NEW overlap: emit TriggerEnter event
        self.collision_events.push(CollisionEvent {
            kind: CollisionEventKind::TriggerEnter {
                body: pair.0,
                trigger_index: pair.1,
            },
            contact,
        });
    } else {
        // Ongoing overlap: emit TriggerStay event
        self.collision_events.push(CollisionEvent {
            kind: CollisionEventKind::TriggerStay {
                body: pair.0,
                trigger_index: pair.1,
            },
            contact,
        });
    }
}
for &pair in &self.active_triggers {
    if !current_overlaps.contains(&pair) {
        // ENDED overlap: emit TriggerExit event
        self.collision_events.push(CollisionEvent {
            kind: CollisionEventKind::TriggerExit {
                body: pair.0,
                trigger_index: pair.1,
            },
            contact: Contact::new(Vec4::ZERO, Vec4::ZERO, 0.0), // No contact data for exits
        });
    }
}
self.active_triggers = current_overlaps;
```

### File List

| File | Action | Crate |
|------|--------|-------|
| `crates/rust4d_physics/src/collision.rs` | EDIT: add `CollisionEvent`, `CollisionEventKind` | rust4d_physics |
| `crates/rust4d_physics/src/world.rs` | EDIT: add `collision_events: Vec<CollisionEvent>` field | rust4d_physics |
| `crates/rust4d_physics/src/world.rs` | EDIT: add `active_triggers: HashSet<(BodyKey, usize)>` field | rust4d_physics |
| `crates/rust4d_physics/src/world.rs` | EDIT: accumulate collision events during `step()` | rust4d_physics |
| `crates/rust4d_physics/src/world.rs` | EDIT: add `detect_trigger_overlaps()` method | rust4d_physics |
| `crates/rust4d_physics/src/world.rs` | EDIT: add `drain_collision_events()` method | rust4d_physics |
| `crates/rust4d_physics/src/lib.rs` | EDIT: add re-exports for new types | rust4d_physics |

### Tests Required

- Collision event emitted when two bodies collide (`BodyVsBody`)
- Collision event emitted when body hits static collider (`BodyVsStatic`)
- Trigger overlap detected when body enters trigger zone
- Trigger overlap NOT detected when body is outside trigger zone
- `TriggerEnter` fires only on first frame of overlap
- `TriggerStay` fires on subsequent frames while body remains in trigger zone
- `TriggerExit` fires when body leaves trigger zone
- Layer filtering: trigger only detects configured layers
- Player body correctly detected by player-detecting trigger (validates bug fix)
- Multiple triggers on same body
- Body removed while in trigger zone (should fire `TriggerExit` or clean up gracefully)
- `drain_collision_events()` empties the buffer after draining

### Session Estimate

**0.75 sessions** (0.25 for collision events, 0.5 for trigger detection + enter/exit tracking).

The collision detection code already exists. The work is adding event accumulation alongside existing collision resolution, and adding the asymmetric trigger detection pass with frame-over-frame state tracking.

---

## 5. Sub-Phase C: Lua Bindings for Combat Core APIs

### Rationale

With the Lua scripting architecture, the engine must expose raycasting, collision events, and trigger callbacks to Lua so game scripts can use them. Without Lua bindings, the Rust-side physics primitives from Sub-Phases A and B are inaccessible to game code.

This sub-phase depends on the `rust4d_scripting` crate (mlua integration, script loading, hot-reload) already existing. It also depends on Sub-Phases A and B being complete so the Rust APIs exist to bind.

### Design: Lua Raycasting Wrappers (~0.25 session)

Bind `PhysicsWorld::raycast()` and `raycast_nearest()` to Lua:

```lua
-- Lua API: world:raycast(origin, direction, max_dist, layer_mask)
-- Returns: array of hit tables, sorted by distance (nearest first)
local hits = world:raycast(
    vec4(0, 1, 0, 0),      -- origin
    vec4(1, 0, 0, 0),      -- direction
    100.0,                   -- max_distance
    LAYER.ENEMY | LAYER.STATIC  -- layer_mask
)

for _, hit in ipairs(hits) do
    print(hit.distance)     -- f32
    print(hit.point)        -- vec4
    print(hit.normal)       -- vec4
    print(hit.target.type)  -- "body" or "static"
    print(hit.target.key)   -- body key or static index
end

-- Lua API: world:raycast_nearest(origin, dir, max_dist, mask)
-- Returns: single hit table or nil
local hit = world:raycast_nearest(origin, dir, 100.0, LAYER.ALL)
if hit then
    -- process nearest hit
end
```

Implementation notes:
- `RayHit` and `WorldRayHit` are converted to Lua tables on return (not userdata -- scripts need to read fields freely)
- `CollisionLayer` constants exposed to Lua scope as `LAYER.PLAYER`, `LAYER.ENEMY`, `LAYER.STATIC`, `LAYER.TRIGGER`, `LAYER.PROJECTILE`, `LAYER.ALL`
- Bitwise OR on layer constants works via Lua 5.4 integer bitwise operators

### Design: Lua Collision Event Dispatch (~0.25 session)

The engine calls `drain_collision_events()` each frame internally and dispatches to registered Lua callbacks:

```lua
-- Lua API: register callbacks for collision events
on_collision(function(event)
    -- event = { type="body_vs_body", body_a=K, body_b=K, contact={...} }
    -- or:     { type="body_vs_static", body=K, static_index=N, contact={...} }
    if event.type == "body_vs_body" then
        -- handle body collision
    end
end)

on_trigger_enter(function(event)
    -- event = { body=K, trigger_index=N }
    local body_key = event.body
    local trigger_idx = event.trigger_index
    -- apply game logic: damage, pickup, etc.
end)

on_trigger_stay(function(event)
    -- continuous damage zone, etc.
end)

on_trigger_exit(function(event)
    -- cleanup when leaving trigger zone
end)
```

Implementation notes:
- Alternative design: single `on_physics_event(callback)` with event type discrimination in Lua. The per-type callback approach is recommended because it is more intuitive and avoids unnecessary dispatch overhead in Lua.
- Multiple Lua callbacks can be registered for the same event type (engine maintains a list per event type)
- Error in a Lua callback does not crash the engine -- the error is logged and execution continues to the next callback

### Design: Lua Trigger Callbacks (~0.25 session)

Triggers can invoke Lua functions directly instead of routing through `GameEvent(String)`:

```lua
-- Define trigger handler functions
function on_health_pickup(trigger_index, body_key)
    local entity = world:entity_for_body(body_key)
    local health = entity:get("health")
    health.current = math.min(health.current + 25, health.max)
    entity:set("health", health)
    audio:play_oneshot("pickup_health", "sfx")
    world:despawn(trigger_index)  -- remove the pickup
end

-- Triggers in RON reference Lua functions by name:
-- TriggerAction::Callback("on_health_pickup")
```

This is far more powerful than the `GameEvent(String)` escape hatch:
- Instead of the game needing to match on string event names and dispatch, the trigger directly invokes arbitrary game logic
- Any trigger action can be arbitrary Lua code
- No string-to-game-event translation layer needed

`TriggerAction::LuaCallback(String)` is added as a variant -- it calls a named Lua function when the trigger fires, passing the trigger index and body key as arguments.

### File List

| File | Action | Crate |
|------|--------|-------|
| `crates/rust4d_scripting/src/bindings/physics.rs` | NEW: Lua bindings for raycasting and collision events | rust4d_scripting |
| `crates/rust4d_scripting/src/bindings/mod.rs` | EDIT: add `pub mod physics;` | rust4d_scripting |
| `crates/rust4d_scripting/src/lib.rs` | EDIT: register physics bindings in Lua state | rust4d_scripting |

Note: Exact file paths depend on how `rust4d_scripting` is structured. The above assumes a `bindings/` module pattern.

### Tests Required (Lua Integration Tests)

- Lua script calls `world:raycast()` and receives correct hit table
- Lua script calls `world:raycast_nearest()` and gets `nil` on miss
- `on_trigger_enter` callback fires when entity enters trigger zone
- `on_trigger_exit` callback fires when entity leaves trigger zone
- `CollisionLayer` constants accessible from Lua (`LAYER.PLAYER`, etc.)
- Lua callback receives correct event table fields (body keys, contact data)
- Multiple Lua callbacks can be registered for the same event type
- Error in Lua callback does not crash engine (logged, execution continues)
- `TriggerAction::LuaCallback("func_name")` correctly invokes the named Lua function

### Session Estimate

**0.5-0.75 sessions** (0.25 raycasting wrappers + 0.25 collision event dispatch + 0.25 trigger callbacks). The lower end assumes `rust4d_scripting` has well-established patterns for binding Rust APIs to Lua; the upper end accounts for working through mlua type conversion details.

---

## 6. Session Estimates

| Sub-Phase | Scope | Estimate |
|-----------|-------|----------|
| A: Raycasting | Ray4D, ray-shape intersections, world raycast, Vec4 utilities, sphere_vs_sphere fix | 1 session |
| B: Collision Events & Triggers | CollisionEvent types, event accumulation, trigger detection bug fix, enter/exit tracking, drain API | 0.75 session |
| C: Lua Bindings | Raycasting Lua wrappers, collision event Lua dispatch, trigger Lua callbacks, CollisionLayer constants | 0.5-0.75 session |
| **Total Engine Work** | | **2.25-2.5 sessions** |

Comparison with pre-Lua estimates:

| Sub-Phase | Original | Amended | Delta |
|-----------|----------|---------|-------|
| A: Raycasting | 1.0 | 1.0 | 0 (no change to Rust impl) |
| B: Collision Events & Triggers | 0.75 | 0.75 | 0 (no change to Rust impl) |
| NEW C: Lua bindings for P1 APIs | -- | 0.5-0.75 | +0.5-0.75 |
| **Total** | **1.75** | **2.25-2.5** | **+0.5-0.75** |

For context, the game-side work that builds on these primitives is now Lua scripts rather than compiled Rust:

| Game Feature | Engine Dependency | Notes |
|--------------|-------------------|-------|
| Health/damage system | Collision events + raycasting | Lua tables and callbacks, not Rust types |
| Event dispatch | Collision events | Lua pub/sub tables replace Rust `EventBus` |
| Trigger-to-gameplay | Trigger Lua callbacks | Direct Lua function invocation, no string dispatch |

---

## 7. Dependencies

### On the Engine/Game Split Plan

This phase assumes the split plan is complete through at least Phase 2:
- **Phase 1 (ECS Migration)**: Completed. `hecs`-based ECS is in place. Entity system is component-based.
- **Phase 2 (Game Logic Extraction + `rust4d_game`)**: Completed. `rust4d_game` crate exists with `CharacterController4D`, basic event system, scene helpers. `PhysicsWorld` has generic body methods.

### On Lua Scripting Infrastructure

Sub-Phase C depends on the `rust4d_scripting` crate already existing and providing:
- mlua integration with Lua 5.4 runtime
- Script loading and execution lifecycle
- Hot-reload support for Lua scripts
- Error handling and reporting
- The global Lua state with engine API tables registered

Sub-Phases A and B (pure Rust) have no dependency on the scripting crate and can proceed independently. Sub-Phase C can only begin once the scripting foundation exists AND Sub-Phases A and B are complete.

### On Foundation Phase

- **Fixed timestep**: Not strictly required for raycasting (instantaneous query). Trigger enter/exit detection benefits from fixed timestep because variable dt can cause missed overlaps if bodies move through trigger zones in a single frame. Plan works without fixed timestep but works better with it.
- **Rotor4 serialization**: Not required for this phase (no serialization of ray/collision types needed).
- **Diagonal normalization fix**: Not required for this phase but related to physics correctness.

### External Dependencies

Sub-Phases A and B: None. These add to `rust4d_math` and `rust4d_physics` only, which have no external dependencies beyond `std`.

Sub-Phase C: Depends on `mlua` (via `rust4d_scripting`).

---

## 8. Parallelization

Sub-Phases A and B can be implemented **fully in parallel** by separate agents or in separate worktrees. Sub-Phase C must wait for both A and B to complete, plus the `rust4d_scripting` crate to exist.

```
Wave 1 (Parallel -- No Dependencies Between Sub-Phases)
├── Agent/Worktree 1: Sub-Phase A (Raycasting)
│   ├── Vec4 utility additions (distance, f32 * Vec4)
│   ├── Ray4D struct in rust4d_math
│   ├── ray-shape intersection functions in rust4d_physics
│   ├── PhysicsWorld::raycast() + raycast_nearest()
│   ├── sphere_vs_sphere visibility fix
│   └── All raycasting tests
│
└── Agent/Worktree 2: Sub-Phase B (Collision Events & Triggers)
    ├── CollisionEvent + CollisionEventKind types
    ├── collision_events field + drain_collision_events()
    ├── Event accumulation during step()
    ├── detect_trigger_overlaps() (asymmetric detection fix)
    ├── active_triggers HashSet + enter/exit/stay tracking
    └── All collision event + trigger tests

Wave 2 (Sequential -- Depends on Wave 1 + rust4d_scripting)
└── Agent/Worktree 3: Sub-Phase C (Lua Bindings)
    ├── Lua raycasting wrappers (world:raycast, world:raycast_nearest)
    ├── CollisionLayer Lua constants
    ├── Lua collision event dispatch (on_collision, on_trigger_enter, etc.)
    ├── Lua trigger callback invocation (TriggerAction::LuaCallback)
    └── All Lua integration tests
```

**Why A and B parallelize cleanly**:
- Sub-Phase A touches `rust4d_math/src/ray.rs` (new file), `rust4d_math/src/vec4.rs`, `rust4d_physics/src/raycast.rs` (new file), and adds methods to `world.rs`.
- Sub-Phase B touches `rust4d_physics/src/collision.rs` and adds different methods + fields to `world.rs`.
- The only shared file is `world.rs`. The additions are orthogonal (raycasting queries vs event accumulation in step), but merge coordination is needed for `world.rs` and `lib.rs`.

**Why C must be sequential**: It binds APIs created in A and B. It also depends on the scripting crate infrastructure.

**Merge strategy**: If running Wave 1 in parallel, one agent should merge first, then the second agent rebases. The `world.rs` changes do not conflict semantically (raycasting reads bodies; collision events writes to a separate field during step).

Within Sub-Phase A, the implementation order is:
1. Vec4 utilities (unblocks nothing, but quick)
2. Ray4D struct in rust4d_math
3. Ray-shape intersection functions (depends on Ray4D)
4. PhysicsWorld::raycast() (depends on ray-shape functions)

Within Sub-Phase B, the implementation order is:
1. CollisionEvent + CollisionEventKind types
2. collision_events field + drain_collision_events()
3. Event accumulation in step() for BodyVsBody and BodyVsStatic
4. detect_trigger_overlaps() with asymmetric check
5. active_triggers + enter/exit/stay tracking (depends on detect_trigger_overlaps)

Within Sub-Phase C, the implementation order is:
1. CollisionLayer Lua constants (quick, unblocks raycasting wrappers)
2. Raycasting Lua wrappers (depends on Sub-Phase A complete)
3. Collision event Lua dispatch (depends on Sub-Phase B complete)
4. Trigger Lua callbacks (depends on collision event dispatch)

---

## 9. Verification Criteria

### Sub-Phase A: Raycasting

- [ ] `Ray4D::new()` normalizes direction automatically
- [ ] `Ray4D::point_at(t)` returns correct points along the ray
- [ ] `ray_vs_sphere` correctly detects: miss, tangent, through-center, origin-inside, behind-ray
- [ ] `ray_vs_aabb` correctly detects: miss, hit each face pair (8 faces in 4D), parallel-to-axis, origin-inside
- [ ] `ray_vs_plane` correctly detects: hit-from-above, hit-from-below, parallel-miss, behind-ray
- [ ] `ray_vs_collider` dispatches correctly for all `Collider` variants
- [ ] `PhysicsWorld::raycast()` returns hits sorted by distance
- [ ] `PhysicsWorld::raycast()` respects `layer_mask` filtering (bodies with non-matching layers are skipped)
- [ ] `PhysicsWorld::raycast()` respects `max_distance` cutoff
- [ ] `PhysicsWorld::raycast()` checks both dynamic bodies and static colliders
- [ ] `PhysicsWorld::raycast_nearest()` returns only the closest hit
- [ ] `Vec4::distance()` and `Vec4::distance_squared()` return correct values
- [ ] `f32 * Vec4` produces same result as `Vec4 * f32` (commutativity)
- [ ] `sphere_vs_sphere` is a public standalone function
- [ ] All ray intersection functions return `None` for rays pointing away from the shape
- [ ] All new code compiles with `cargo build --workspace`
- [ ] All new tests pass with `cargo test --workspace`

### Sub-Phase B: Collision Events & Triggers

- [ ] `CollisionEvent` structs are emitted for body-body collisions
- [ ] `CollisionEvent` structs are emitted for body-static collisions
- [ ] `drain_collision_events()` returns all events and empties the buffer
- [ ] Calling `drain_collision_events()` twice returns empty vec on second call
- [ ] Trigger zones with `CollisionFilter::trigger(CollisionLayer::PLAYER)` detect player bodies (validates bug fix)
- [ ] Trigger zones do NOT apply physics response (no pushing)
- [ ] `TriggerEnter` fires only on the first frame a body overlaps a trigger
- [ ] `TriggerStay` fires on every subsequent frame of continued overlap
- [ ] `TriggerExit` fires on the first frame after a body stops overlapping a trigger
- [ ] Layer filtering works for triggers (trigger configured for PLAYER does not detect ENEMY)
- [ ] Multiple bodies can be in the same trigger simultaneously
- [ ] A single body can be in multiple triggers simultaneously
- [ ] `active_triggers` is cleaned up properly when bodies are removed
- [ ] All new code compiles with `cargo build --workspace`
- [ ] All new tests pass with `cargo test --workspace`
- [ ] All existing tests still pass (no regressions)

### Sub-Phase C: Lua Bindings

- [ ] Lua script calls `world:raycast()` and receives correct hit table with distance, point, normal, target fields
- [ ] Lua script calls `world:raycast_nearest()` and gets `nil` on miss
- [ ] `CollisionLayer` constants accessible from Lua (`LAYER.PLAYER`, `LAYER.ENEMY`, `LAYER.STATIC`, etc.)
- [ ] Bitwise OR on `CollisionLayer` constants works for building layer masks
- [ ] `on_collision` callback fires with correct event table for body-body and body-static collisions
- [ ] `on_trigger_enter` callback fires when entity enters trigger zone
- [ ] `on_trigger_stay` callback fires on subsequent frames of continued overlap
- [ ] `on_trigger_exit` callback fires when entity leaves trigger zone
- [ ] Lua callback receives correct event table fields (body keys, trigger index, contact data)
- [ ] Multiple Lua callbacks can be registered for the same event type
- [ ] Error in Lua callback does not crash engine (logged, execution continues)
- [ ] `TriggerAction::LuaCallback("func_name")` correctly invokes the named Lua function
- [ ] All Lua integration tests pass
- [ ] All existing Rust tests still pass (no regressions from binding layer)

### Integration

- [ ] A complete workflow functions: create physics world -> add bodies and static triggers -> step -> engine dispatches events to Lua callbacks -> see TriggerEnter/TriggerStay/TriggerExit cycle in Lua
- [ ] A complete workflow functions: Lua script calls `world:raycast()` -> get hit table with correct target and distance
- [ ] No changes to `step()` return type (drain pattern only)
- [ ] Engine manages the drain-and-dispatch cycle internally; Lua scripts only register callbacks

---

## 10. Cross-Phase Dependencies

Other post-split phases depend on Combat Core's outputs:

### Phase 2 (Weapons & Feedback) -- Needs Raycasting
- Hitscan weapons call `world:raycast()` from Lua scripts to detect hits.
- Weapon impact effects spawn at hit point with orientation from hit normal.
- Audio triggers come from Lua callbacks on collision events, not from a Rust-side event chain. (Confirmed in hive-mind by Agent P1 and Agent P2.)

### Phase 3 (Enemies & AI) -- Needs Raycasting
- Enemy line-of-sight checks use `world:line_of_sight()` (built on raycasting) from Lua AI scripts.
- Agent P3 stubs LOS as `true` until raycasting is ready. Once Combat Core is done, P3 drops the stub.
- For W-phasing enemies, the AI checks LOS considering W-distance attenuation, not just geometric occlusion. (Note from Agent P3 in hive-mind.)
- Hyperspherical area damage uses spatial queries (not raycasting), but benefits from collision events to know when explosions hit.

### Phase 4 (Level Design Pipeline) -- Needs Trigger Events
- Declarative trigger system (`TriggerDef` in RON) uses `TriggerAction::Callback("lua_func")` to invoke Lua functions when triggers fire.
- Pickup triggers use `CollisionFilter::trigger(CollisionLayer::PLAYER)`.
- Door/elevator mechanics triggered by Lua callbacks on trigger zones -- far simpler than the Rust struct approach.
- The question of string-named events is resolved: with Lua, triggers call Lua functions directly. No `GameEvent(String)` dispatch needed.

### Phase 5 (Editor & Polish) -- No Direct Dependency
- Editor does not directly depend on raycasting or collision events.
- However, a future editor feature (click-to-select entities in viewport) would use raycasting.
- Editor's Lua console can call `world:raycast()` for debugging.

### Foundation -- Feeds Into This Phase
- Fixed timestep improves trigger enter/exit reliability.
- `sphere_vs_sphere` fix is a foundation-level cleanup done here as a convenience.

### Scripting Phase -- Feeds Into Sub-Phase C
- `rust4d_scripting` crate must provide mlua integration, script loading, hot-reload, error handling, and the global Lua state before Sub-Phase C can begin.
- Sub-Phases A and B have no dependency on the scripting crate.

---

## 11. Open Questions

1. **Dynamic trigger bodies**: The current design only covers static trigger colliders (triggers as world geometry). Should dynamic bodies also be able to act as triggers? (e.g., a moving damage field.) If so, the body-body collision loop also needs asymmetric trigger detection. **Recommendation**: Defer until a game use case requires it.

2. **CollisionEvent memory**: With many bodies and triggers, event vectors could grow large. The drain pattern is good (no persistent allocation growth), but should be monitored. If needed, add a `max_events` cap or pre-allocated ring buffer.

3. **Spatial acceleration for raycasting**: The current linear scan is O(n) per raycast. For a boomer shooter with ~50-100 entities, this is fine. A BVH or grid would help at scale but is premature optimization now. Defer to Phase 6 (Advanced).

4. **`sphere_vs_sphere` refactoring scope**: Should this be a public function in `collision.rs` (like the other shape-shape tests) or in a separate `intersections.rs` module? Follow the existing pattern in the crate.

5. **`TriggerStay` performance**: If many bodies are inside many triggers simultaneously, `TriggerStay` events could dominate the event buffer each frame. Consider making `TriggerStay` opt-in or providing a `drain_collision_events_filtered()` variant. For now, include it and monitor.

6. **Lua callback dispatch overhead**: Calling Lua functions from Rust on every collision event adds per-frame overhead. For a boomer shooter (20-50 enemies, a few hundred collisions per frame), this is negligible. Monitor if event volumes grow. Consider batching (pass all events to a single Lua call as a table) if per-call overhead becomes measurable.

7. **Single vs per-type callbacks**: The design above offers per-type callbacks (`on_collision`, `on_trigger_enter`, etc.). An alternative is a single `on_physics_event(callback)` with type discrimination in Lua. Per-type is recommended for clarity and to avoid unnecessary Lua-side dispatch, but both could be supported.

8. **Lua trigger callback error recovery**: When a `TriggerAction::LuaCallback("func_name")` fails (function not found, runtime error), the engine should log the error and continue. Should it also disable the callback to prevent log spam? **Recommendation**: Log on first error, then silence repeated errors for the same trigger+function pair until the script is hot-reloaded.
