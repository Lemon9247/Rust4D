# Agent P1: Combat Core -- Engine Implementation Plan

**Date**: 2026-01-30
**Phase**: Phase 1 (Combat Core) from cross-swarm synthesis
**Scope**: What the Rust4D ENGINE needs to support combat, accounting for the engine/game split

---

## Executive Summary

Phase 1 of the original roadmap identified four items: 4D raycasting, event system, health/damage, and trigger zone callbacks. After reviewing the actual engine source code and the engine/game split plan, the picture is clear:

- **Raycasting** is entirely engine work, split across `rust4d_math` (primitives) and `rust4d_physics` (world queries). This is the most substantial item.
- **Event system** already has a home in the split plan: `rust4d_game::events`. The engine core does NOT need an event bus. However, the physics engine needs to *produce* collision event data that the event system can consume.
- **Health/damage** is purely game-side. The engine needs nothing for this.
- **Trigger callbacks** require the physics engine to *report* trigger overlaps (currently detected but silently discarded). This is engine work.

**Engine work: ~1.5 sessions. Game work: ~1.5 sessions.**

---

## Feature 1: 4D Raycasting

### Engine vs Game Verdict: **ENGINE** (both `rust4d_math` and `rust4d_physics`)

Raycasting is a fundamental engine primitive. Every game needs it -- for weapons, line-of-sight, picking, UI interaction, anything. This is not game-specific.

### Current State

- `rust4d_math` has `Vec4`, `Sphere4D` (in physics), `AABB4D` (in physics), `Plane4D` (in physics). No ray type exists anywhere.
- `rust4d_physics` has collision detection between shapes (`sphere_vs_plane`, `sphere_vs_aabb`, `aabb_vs_aabb`, `aabb_vs_plane`) but zero ray intersection tests.
- `rust4d_physics::PhysicsWorld` has `SlotMap<BodyKey, RigidBody4D>` for dynamic bodies and `Vec<StaticCollider>` for static geometry. A world raycast must check both.
- The collision filter system (`CollisionLayer`, `CollisionFilter`) is ready to be reused for ray filtering.

### Design

The raycasting system is split across two crates following the existing pattern where `rust4d_math` holds pure math types and `rust4d_physics` holds physics simulation types:

**Layer 1 -- `rust4d_math`**: `Ray4D` struct and utility methods. This belongs in the math crate because rays are a geometric primitive, like vectors and rotations. However, since the collision shapes (`Sphere4D`, `AABB4D`, `Plane4D`) currently live in `rust4d_physics::shapes`, the ray-shape intersection functions must also live in `rust4d_physics`.

**Alternative considered**: Move `Ray4D` into `rust4d_physics`. This is simpler but wrong architecturally -- a ray is a math primitive, not a physics concept. Non-physics uses (debug visualization, picking, UI raycasting) should not require depending on the physics crate.

**Layer 2 -- `rust4d_physics`**: Ray-shape intersection functions and `PhysicsWorld::raycast()`.

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

**File changes**:
- NEW: `crates/rust4d_math/src/ray.rs`
- EDIT: `crates/rust4d_math/src/lib.rs` -- add `pub mod ray;` and `pub use ray::Ray4D;`

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

    let t = if t_min >= 0.0 { t_min } else { t_max }; // if inside AABB, use exit point... or t_min
    // Actually if t_min < 0, ray starts inside -- use t_min = 0 convention or return the exit?
    // For raycasting (weapons), we want the entry point. If origin is inside, distance = 0.
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

#### `PhysicsWorld::raycast()` (addition to world.rs)

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

### File Locations

| File | Action | Crate |
|------|--------|-------|
| `crates/rust4d_math/src/ray.rs` | NEW | rust4d_math |
| `crates/rust4d_math/src/lib.rs` | EDIT: add module + re-export | rust4d_math |
| `crates/rust4d_physics/src/raycast.rs` | NEW | rust4d_physics |
| `crates/rust4d_physics/src/world.rs` | EDIT: add raycast methods + WorldRayHit/RayTarget types | rust4d_physics |
| `crates/rust4d_physics/src/lib.rs` | EDIT: add `pub mod raycast;` + re-exports | rust4d_physics |

### Vec4 Utility Additions

The raycasting work reveals that `Vec4` is missing a `distance()` method that would be convenient. Adding it while we're here:

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
```

Also add the missing `f32 * Vec4` operator:

```rust
// Added to crates/rust4d_math/src/vec4.rs

impl std::ops::Mul<Vec4> for f32 {
    type Output = Vec4;
    #[inline]
    fn mul(self, v: Vec4) -> Vec4 {
        v * self
    }
}
```

### Tests Required

- `ray_vs_sphere`: miss, tangent, through center, origin inside sphere, behind ray
- `ray_vs_aabb`: miss, hit each face pair (8 faces in 4D), parallel to axis, origin inside AABB
- `ray_vs_plane`: hit from above, hit from below, parallel miss, behind ray
- `ray_vs_collider`: dispatch test for each variant
- `PhysicsWorld::raycast`: hit body, hit static, miss all, layer filtering, max_distance cutoff, multiple hits sorted by distance
- `PhysicsWorld::raycast_nearest`: returns closest hit
- `Vec4::distance`: basic distance test
- `f32 * Vec4`: commutativity test

### Session Estimate: **1 session**

The math is straightforward (dimension-agnostic). The world raycast is a linear scan (no spatial acceleration needed yet with the current body counts). The bulk of the work is writing tests.

### What the Game Needs to Build on Top

The game uses `PhysicsWorld::raycast()` for:
- **Hitscan weapons**: Cast ray from camera position in look direction, check if it hits an enemy body.
- **Line-of-sight**: AI checks if it can see the player by raycasting from enemy to player.
- **Picking/interaction**: Raycast from player to check for interactive objects.

The game does NOT need additional engine support for these -- it just calls `raycast()` and acts on the results.

---

## Feature 2: Event System

### Engine vs Game Verdict: **GAME** (mostly), with engine-level collision event DATA

The engine/game split plan already designates an event system for `rust4d_game::events`. The engine core (`rust4d_math`, `rust4d_physics`, `rust4d_core`) should NOT have a general-purpose event bus. Here's why:

1. **An event bus is an application-level pattern**, not a math/physics/ECS primitive. It coordinates between systems, which is the game's job.
2. **The split plan already decided this**: `rust4d_game` provides "Event system -- simple event bus for game events (collision callbacks, trigger enter/exit)".
3. **Engine crates should be composable utilities**, not opinionated frameworks.

However, the engine DOES need to produce event-worthy DATA:

### What the Engine Needs to Provide

The physics engine currently resolves collisions silently -- it pushes bodies apart and modifies velocities, but produces no data about what collided with what. For the game to fire events, the engine must report:

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
    /// Two dynamic/kinematic bodies collided
    BodyVsBody {
        body_a: BodyKey,
        body_b: BodyKey,
    },
    /// A body collided with a static collider
    BodyVsStatic {
        body: BodyKey,
        static_index: usize,
    },
}
```

And `PhysicsWorld::step()` needs to return or accumulate these:

```rust
impl PhysicsWorld {
    /// Step the physics simulation and return all collision events that occurred.
    pub fn step(&mut self, dt: f32) -> Vec<CollisionEvent> {
        let mut events = Vec::new();
        // ... existing simulation code ...
        // At each collision resolution point, push an event:
        // events.push(CollisionEvent { kind: ..., contact });
        events
    }

    // OR: poll-based approach

    /// Get collision events from the last step.
    pub fn drain_collision_events(&mut self) -> Vec<CollisionEvent> {
        std::mem::take(&mut self.collision_events)
    }
}
```

**Recommendation**: Use the drain/poll approach. It's simpler and doesn't change the `step()` signature, which would be a breaking change across many call sites.

```rust
// In PhysicsWorld struct definition, add:
collision_events: Vec<CollisionEvent>,

// In step(), after resolving each collision, push event.
// After step returns, game code calls drain_collision_events().
```

### What the Game Builds on Top

`rust4d_game::events` provides:

```rust
// Game-level event system (in rust4d_game or game repo)
pub struct EventBus {
    // Type-erased event channels
}

// Game-defined events
pub enum GameEvent {
    Damage { target: Entity, amount: f32, source: Entity },
    Pickup { entity: Entity, item: PickupType },
    TriggerEnter { trigger: Entity, entity: Entity },
    TriggerExit { trigger: Entity, entity: Entity },
    Death { entity: Entity },
}
```

The game's simulation loop:
1. Call `physics_world.step(dt)`
2. Call `physics_world.drain_collision_events()`
3. For each collision event, translate into game events (damage, trigger, etc.)
4. Dispatch game events through the event bus

### File Locations

| File | Action | Crate |
|------|--------|-------|
| `crates/rust4d_physics/src/collision.rs` | EDIT: add CollisionEvent, CollisionEventKind | rust4d_physics |
| `crates/rust4d_physics/src/world.rs` | EDIT: add collision_events field, accumulate during step, add drain method | rust4d_physics |
| `crates/rust4d_physics/src/lib.rs` | EDIT: add re-exports | rust4d_physics |

### Session Estimate: **0.5 session** (engine side)

Accumulating collision events during `step()` is straightforward -- the collision detection is already happening, we just need to record the results alongside the resolution.

---

## Feature 3: Health/Damage System

### Engine vs Game Verdict: **PURELY GAME** -- Engine needs NOTHING

Health and damage are gameplay concepts. The engine provides:
- **ECS** (from the split plan): The game defines `Health { current: f32, max: f32 }` as a component.
- **Collision events** (from Feature 2): The game detects hits.
- **Raycasting** (from Feature 1): The game detects hitscan weapon impacts.

The engine should NOT define Health, Damage, or Death. These are game-specific because:
1. Different games have wildly different health models (HP bars, shield + health, damage types, armor, invulnerability frames, etc.)
2. The split plan explicitly lists `Health { current: f32, max: f32 }` as a game-defined component.
3. Putting gameplay components in the engine violates the "generic 4D engine" principle.

### What the Game Implements

```rust
// In game repo (Rust4D-Shooter)

/// Health component
pub struct Health {
    pub current: f32,
    pub max: f32,
}

impl Health {
    pub fn new(max: f32) -> Self {
        Self { current: max, max }
    }

    pub fn apply_damage(&mut self, amount: f32) {
        self.current = (self.current - amount).max(0.0);
    }

    pub fn is_dead(&self) -> bool {
        self.current <= 0.0
    }

    pub fn heal(&mut self, amount: f32) {
        self.current = (self.current + amount).min(self.max);
    }
}

/// Damage system: processes collision events and raycasts to deal damage
fn damage_system(world: &mut hecs::World, events: &[CollisionEvent]) {
    // For each collision event, check if either body has a DealsDamage component
    // and the other has a Health component, then apply damage.
}
```

### Session Estimate: **0 sessions** (engine) / **1 session** (game)

---

## Feature 4: Trigger Zone Callbacks

### Engine vs Game Verdict: **ENGINE** (physics event data) + **GAME** (callback dispatch)

### Current State Analysis

The trigger infrastructure is **half-built**:

1. `CollisionLayer::TRIGGER` exists (bit 4 in the bitflags).
2. `CollisionFilter::trigger(detects: CollisionLayer)` exists -- creates a filter where the trigger detects specified layers.
3. The asymmetric collision design is intentional: triggers detect overlaps, but the detected objects DON'T push against triggers. From `collision.rs` line 117-123:
   ```rust
   pub fn trigger(detects: CollisionLayer) -> Self {
       Self {
           layer: CollisionLayer::TRIGGER,
           mask: detects,
       }
   }
   ```
4. BUT -- in `world.rs`, the collision filter check `body.filter.collides_with(&static_col.filter)` uses the SYMMETRIC `collides_with()` which requires BOTH filters to agree. Since `CollisionFilter::player()` excludes TRIGGER from its mask, the trigger-player collision is never detected! Line 84-91:
   ```rust
   pub fn player() -> Self {
       Self {
           layer: CollisionLayer::PLAYER,
           mask: CollisionLayer::ALL
               & !CollisionLayer::PLAYER
               & !CollisionLayer::PROJECTILE
               & !CollisionLayer::TRIGGER,  // <-- Player ignores triggers
       }
   }
   ```

**This is a design bug.** The trigger system was designed for asymmetric detection (trigger detects player, player doesn't push trigger), but the symmetric `collides_with()` check prevents it from working at all. The test at line 530 explicitly documents this:
```rust
// The trigger layer is not in player's mask, so symmetric check fails
// This is intentional: triggers detect but don't push
assert!(!trigger.collides_with(&player));
```

The comment says "intentional" but the consequence is that triggers NEVER detect players. The design intent (triggers detect without pushing) requires an ASYMMETRIC overlap check, separate from the push/response collision check.

### Design: Two-Pass Collision Detection

The fix is to separate **trigger detection** from **collision response**:

```rust
// In PhysicsWorld::step(), add a new phase between static and body collisions:

// Phase 2b: Detect trigger overlaps (no physics response)
self.detect_trigger_overlaps();
```

```rust
impl PhysicsWorld {
    /// A trigger overlap detected during physics step
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

            // Also check body-body trigger overlaps
            // (for dynamic trigger zones, e.g., moving damage fields)
        }
    }
}
```

Update `CollisionEventKind`:

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CollisionEventKind {
    BodyVsBody { body_a: BodyKey, body_b: BodyKey },
    BodyVsStatic { body: BodyKey, static_index: usize },
    TriggerOverlap { body: BodyKey, trigger_index: usize },
}
```

### Enter/Exit Tracking

For "trigger enter" and "trigger exit" events (like Unity's `OnTriggerEnter`/`OnTriggerExit`), the physics engine needs to track which body-trigger pairs were overlapping last frame:

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
            kind: CollisionEventKind::TriggerEnter { body: pair.0, trigger_index: pair.1 },
            contact,
        });
    }
}
for &pair in &self.active_triggers {
    if !current_overlaps.contains(&pair) {
        // ENDED overlap: emit TriggerExit event
        self.collision_events.push(CollisionEvent {
            kind: CollisionEventKind::TriggerExit { body: pair.0, trigger_index: pair.1 },
            contact: Contact::new(Vec4::ZERO, Vec4::ZERO, 0.0), // No contact data for exits
        });
    }
}
self.active_triggers = current_overlaps;
```

Final `CollisionEventKind`:

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CollisionEventKind {
    /// Two bodies collided (physics response was applied)
    BodyVsBody { body_a: BodyKey, body_b: BodyKey },
    /// A body collided with a static collider (physics response was applied)
    BodyVsStatic { body: BodyKey, static_index: usize },
    /// A body entered a trigger zone this frame
    TriggerEnter { body: BodyKey, trigger_index: usize },
    /// A body is still inside a trigger zone (ongoing overlap)
    TriggerStay { body: BodyKey, trigger_index: usize },
    /// A body exited a trigger zone this frame
    TriggerExit { body: BodyKey, trigger_index: usize },
}
```

**Design choice**: I'm including `TriggerStay` because it's trivial to add (it's any overlap that was also in `active_triggers`) and some games need it (continuous damage zones). It can be omitted if performance profiling shows the event volume is too high.

### File Locations

| File | Action | Crate |
|------|--------|-------|
| `crates/rust4d_physics/src/collision.rs` | EDIT: add CollisionEvent, CollisionEventKind | rust4d_physics |
| `crates/rust4d_physics/src/world.rs` | EDIT: add collision_events, active_triggers, detect_trigger_overlaps(), drain_collision_events() | rust4d_physics |

### Tests Required

- Trigger overlap detected when body enters trigger zone
- Trigger overlap NOT detected when body is outside trigger zone
- TriggerEnter fires only on first frame of overlap
- TriggerStay fires on subsequent frames
- TriggerExit fires when body leaves trigger zone
- Layer filtering: trigger only detects configured layers
- Player body correctly detected by player-detecting trigger
- Multiple triggers on same body
- Body removed while in trigger zone (should fire TriggerExit or just clean up)

### Session Estimate: **0.5 session** (engine side)

The collision detection code already exists. This is adding event accumulation and enter/exit tracking on top of existing geometry checks.

### What the Game Builds on Top

The game translates trigger events into gameplay:

```rust
// In game simulation loop
let events = physics_world.drain_collision_events();
for event in &events {
    match event.kind {
        CollisionEventKind::TriggerEnter { body, trigger_index } => {
            // Look up what this trigger does (pickup? damage zone? door opener?)
            // Dispatch appropriate game event
        }
        CollisionEventKind::TriggerExit { body, trigger_index } => {
            // End ongoing effects
        }
        _ => {}
    }
}
```

---

## Dependency Analysis

### Internal Dependencies (within this phase)

```
Feature 1 (Raycasting) -- no dependencies, can start immediately
Feature 2 (Collision Events) -- no dependencies, can start immediately
Feature 4 (Trigger Detection) -- depends on Feature 2 (uses CollisionEvent types)
Feature 3 (Health/Damage) -- game-only, depends on Features 1+2 being available
```

### External Dependencies

- **ECS migration** (from split plan Phase 1): Features 1, 2, and 4 work with the current non-ECS architecture. They operate on `PhysicsWorld` which is independent of the entity system. After ECS migration, the game-side integration changes but the physics APIs stay the same.
- **rust4d_game crate** (from split plan Phase 2): The event bus lives there. Features 2 and 4 produce raw event data; the event bus in `rust4d_game` wraps it for game-level dispatch.
- **Fixed timestep** (Foundation): Physics events will be more reliable with fixed timestep, but functionally work without it.

### Parallelism Opportunity

Features 1 and 2+4 can be implemented in parallel by different agents:
- **Agent A**: Ray4D + ray-shape intersections + world raycast + Vec4 utilities
- **Agent B**: CollisionEvent types + trigger detection + enter/exit tracking + drain API

---

## Summary Table

| Feature | Engine Work | Game Work | Engine Session Est. | Game Session Est. |
|---------|-----------|-----------|--------------------|--------------------|
| 4D Raycasting | Ray4D struct, ray-shape intersections, world raycast API | Hitscan weapons, LOS checks | 1 | 0 (uses API) |
| Event System | CollisionEvent data types, event accumulation in step() | EventBus, GameEvent types, dispatch logic | 0.25 | 0.75 |
| Health/Damage | Nothing | Health component, damage system, death handling | 0 | 1 |
| Trigger Callbacks | Trigger overlap detection, enter/exit tracking | Trigger-to-gameplay translation | 0.5 | 0.25 |
| **Totals** | | | **1.75** | **2** |

### Engine-Side Implementation Order

1. **Vec4 utilities** (distance, f32*Vec4) -- 15 minutes, unblocks nothing but good to have
2. **Ray4D + ray-shape intersections** -- core raycasting math
3. **CollisionEvent types** -- data structures for both collision and trigger events
4. **PhysicsWorld::raycast()** -- world-level raycasting
5. **Trigger overlap detection + enter/exit tracking** -- builds on CollisionEvent
6. **PhysicsWorld::drain_collision_events()** -- event accumulation in step()

Steps 2-3 can run in parallel. Steps 4 and 5-6 can run in parallel.

---

## Open Questions

1. **Should `sphere_vs_sphere` be made public?** It currently exists as a private method on `PhysicsWorld` (line 265 of world.rs). It should be a public standalone function like the other collision tests. This is a quick fix that could be done alongside the raycasting work.

2. **Trigger body-body detection**: The design above only covers static trigger colliders (triggers as world geometry). Should dynamic bodies also be able to be triggers? (e.g., a moving damage field). If so, the body-body collision loop also needs asymmetric trigger detection. I recommend deferring this until a game use case requires it.

3. **CollisionEvent memory**: With many bodies and triggers, event vectors could grow large. The drain pattern is good (no persistent allocation growth), but we should monitor. If needed, add a `max_events` cap or pre-allocated ring buffer.

4. **Spatial acceleration for raycasting**: The current linear scan is O(n) per raycast. For a boomer shooter with ~50-100 entities, this is fine. A BVH or grid would help at scale but is premature optimization now. Defer to Phase 6 (Advanced).
