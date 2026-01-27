# Physics System Review Report

**Agent:** Physics Reviewer
**Date:** 2026-01-27
**Focus:** Why the tesseract does not fall correctly to the floor

## Executive Summary

After thorough analysis of the physics system, **the core physics code appears to be correct**. The `aabb_vs_aabb` collision detection, gravity application, collision resolution, and velocity response all work correctly based on the code and comprehensive unit tests. The issue is likely in how the scene loading pipeline transforms the scene entities or in runtime entity-physics synchronization.

## Scene Configuration Analysis

From `scenes/default.ron`:
- **Floor:** Hyperplane at y=-2, size=10, thickness=0.001
- **Tesseract:** Position (0,0,0,0), size=2.0 (half_extent=1.0), tagged "dynamic"
- **Gravity:** -20.0

From `crates/rust4d_core/src/scene.rs` (lines 250-287):
- The floor is created via `StaticCollider::floor_bounded(-2.0, 10.0, 5.0, 0.001, CONCRETE)`
- The tesseract is created as `RigidBody4D::new_aabb(position, half_extents)` with `BodyType::Dynamic`

### Floor AABB Construction

The `floor_bounded` function (body.rs lines 259-281):
- Enforces minimum thickness of 5.0 units (0.001 gets clamped to 5.0)
- Center: (0, y - half_thickness, 0, 0) = (0, -4.5, 0, 0)
- Half_extents: (10.0, 2.5, 10.0, 5.0)
- Resulting AABB: min=(-10, -7, -10, -5), max=(10, -2, 10, 5)
- **Floor top surface is at y=-2** (correct)

### Tesseract AABB Construction

For the tesseract:
- Position: (0, 0, 0, 0)
- Size: 2.0, half_extent: 1.0
- AABB: min=(-1, -1, -1, -1), max=(1, 1, 1, 1)
- **Bottom of tesseract at y=-1** (correct)

### Expected Collision Behavior

1. Tesseract falls from y=0
2. At y=-1 center (bottom at y=-2), it should touch the floor
3. `aabb_vs_aabb` detects penetration when tesseract.min.y < floor.max.y
4. Normal should be +Y (upward), pushing the tesseract up
5. After resolution, tesseract should rest at y=-1 (bottom at y=-2)

## Code Review Findings

### 1. `aabb_vs_aabb` Function (collision.rs lines 284-344)

**Status: CORRECT**

The algorithm:
1. Correctly checks for separation on all 4 axes (lines 286-297)
2. Computes overlap on each axis (lines 300-303)
3. Finds minimum overlap axis for resolution (lines 305-336)
4. Normal direction based on center comparison (line 315: `if a.center().y < b.center().y`)

**Key observation:** When tesseract (center at falling position) collides with floor (center at y=-4.5), the tesseract's center y will be HIGHER than floor's center y, so normal will be `+Vec4::Y` (pushing tesseract UP). This is correct.

**Test verification:** The test `test_tesseract_vs_bounded_floor` (lines 564-615) explicitly tests this scenario and passes.

### 2. Gravity Application (world.rs lines 190-207)

**Status: CORRECT**

```rust
if body.affected_by_gravity() || is_player {
    body.velocity.y += self.config.gravity * dt;
}
```

- Dynamic bodies have `affected_by_gravity() == true`
- Gravity is correctly applied before position integration

### 3. Collision Resolution (world.rs lines 269-324)

**Status: CORRECT**

```rust
if contact.is_colliding() {
    let correction = contact.normal * contact.penetration;
    body.apply_correction(correction);

    if contact.normal.y > GROUND_NORMAL_THRESHOLD {
        body.grounded = true;
    }

    let velocity_along_normal = body.velocity.dot(contact.normal);
    if velocity_along_normal < 0.0 {
        // Remove normal component and apply restitution
        let normal_velocity = contact.normal * velocity_along_normal;
        body.velocity = body.velocity - normal_velocity * (1.0 + combined.restitution);
    }
}
```

- Position correction is in the right direction (normal * penetration)
- Grounded detection checks if normal.y > 0.7 (correct for upward-facing surface)
- Velocity response correctly removes downward velocity component

### 4. Grounded Detection (world.rs lines 294-296)

**Status: CORRECT**

```rust
if contact.normal.y > GROUND_NORMAL_THRESHOLD {  // 0.7
    body.grounded = true;
}
```

When collision normal is +Y (upward), `normal.y = 1.0 > 0.7`, so grounded is set.

## Potential Issues Identified

### Issue 1: Entity-Physics Position Synchronization (CRITICAL)

**Location:** Not in physics code - likely in the render/update loop

The physics system correctly updates `RigidBody4D.position` and `RigidBody4D.collider`. However, there is no visible code that synchronizes the physics body position back to the entity's `Transform4D`.

In `ActiveScene::from_template` (scene.rs lines 277-286):
```rust
let body = RigidBody4D::new_aabb(position, half_extents)
    .with_body_type(BodyType::Dynamic);
let body_key = physics.add_body(body);
entity = entity.with_physics_body(body_key);
```

The entity stores a `body_key` reference, but I don't see where the entity's transform is updated from the physics body position each frame. **If this synchronization is missing, the entity will render at its original position while the physics body falls correctly in the background.**

**Cross-cutting concern:** The Scene Reviewer agent should verify the entity-physics sync in the render loop.

### Issue 2: Body-Body Collision Normal Direction (POTENTIAL)

**Location:** world.rs lines 375-381

```rust
(Collider::AABB(a), Collider::AABB(b)) => {
    // aabb_vs_aabb returns normal pointing from B toward A
    // We want normal from A toward B, so flip it
    aabb_vs_aabb(a, b).map(|mut c| {
        c.normal = -c.normal;
        c
    })
}
```

The comment says `aabb_vs_aabb` returns normal "from B toward A", but looking at the actual code (collision.rs line 315), the normal points FROM the body with the LOWER center TOWARD the body with the HIGHER center on the minimum overlap axis.

For tesseract vs floor:
- Tesseract is body A, floor is body B
- On Y axis: tesseract center y > floor center y (e.g., -0.5 > -4.5)
- So normal = +Y (from floor toward tesseract)
- After flip: normal = -Y (pointing downward)

Wait - this is for BODY-BODY collisions, not static collisions. The tesseract vs floor collision uses `resolve_static_collisions`, not `resolve_body_collisions`. This is fine because floor is a StaticCollider.

**Status: NOT AN ISSUE** - Static collisions don't go through `resolve_body_collisions`.

### Issue 3: Collision Layer Filtering

**Location:** world.rs lines 279-283

```rust
if !body.filter.collides_with(&static_col.filter) {
    continue;
}
```

- Dynamic bodies have `CollisionFilter::default()` (layer=DEFAULT, mask=ALL)
- Static colliders have `CollisionFilter::static_world()` (layer=STATIC, mask=ALL)

Checking: `DEFAULT.intersects(ALL) && STATIC.intersects(ALL)` = `true && true` = `true`

**Status: NOT AN ISSUE** - Layers allow collision.

## Verification Tests Run

The existing test `test_tesseract_vs_bounded_floor` (collision.rs lines 564-615) verifies:
1. Tesseract at y=0 does NOT collide with floor at y=-2 (correct gap)
2. Tesseract at y=-0.9 does NOT collide (bottom at -1.9, still above floor top at -2)
3. Tesseract at y=-1.1 DOES collide (bottom at -2.1 penetrates floor top at -2)
4. Tesseract at y=-1.001 DOES collide (slightly below resting position)

All tests pass, confirming AABB collision detection is working correctly.

## Recommendations

### Priority 1: Verify Entity-Physics Synchronization

Check if `World::update()` or the main render loop synchronizes entity transforms from their physics body positions. Look for code that:
```rust
if let Some(body_key) = entity.physics_body_key() {
    if let Some(body) = physics.get_body(body_key) {
        entity.transform.position = body.position;
    }
}
```

If this is missing, that's the bug.

### Priority 2: Add Debug Logging

Add logging to verify the physics simulation is running:
```rust
// In PhysicsWorld::step
log::debug!("Physics step: {} bodies", self.bodies.len());
for (key, body) in &self.bodies {
    log::debug!("Body {:?}: pos={:?}, vel={:?}", key, body.position, body.velocity);
}
```

### Priority 3: Verify Scene Loading

Confirm the "dynamic" tag is being recognized and the tesseract is being added to the physics world as a Dynamic body.

## Summary Table

| Component | Status | Notes |
|-----------|--------|-------|
| `aabb_vs_aabb` collision detection | OK | Comprehensive tests pass |
| Gravity application | OK | Correctly applied to Dynamic bodies |
| Position correction | OK | Correct direction and magnitude |
| Velocity response | OK | Removes normal component, applies restitution |
| Grounded detection | OK | Threshold-based, works for AABB vs AABB |
| Collision layer filtering | OK | DEFAULT collides with STATIC |
| Entity-physics sync | UNKNOWN | Not in physics crate, needs main loop review |

## Files Reviewed

- `/home/lemoneater/Projects/Rust4D/crates/rust4d_physics/src/collision.rs`
- `/home/lemoneater/Projects/Rust4D/crates/rust4d_physics/src/world.rs`
- `/home/lemoneater/Projects/Rust4D/crates/rust4d_physics/src/body.rs`
- `/home/lemoneater/Projects/Rust4D/crates/rust4d_physics/src/shapes.rs`
- `/home/lemoneater/Projects/Rust4D/crates/rust4d_core/src/scene.rs`
- `/home/lemoneater/Projects/Rust4D/scenes/default.ron`
