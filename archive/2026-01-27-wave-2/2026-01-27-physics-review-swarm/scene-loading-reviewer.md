# Scene Loading Review Report

**Agent:** Scene Loading Reviewer
**Date:** 2026-01-27
**Focus:** Analyzing scene loading to identify issues with physics body creation

## Summary of Findings

After tracing through the scene loading code path, **I found no bugs in the scene loading system**. The code correctly:

1. Detects the "dynamic" tag on the tesseract entity
2. Creates a `RigidBody4D` with proper AABB collider and `BodyType::Dynamic`
3. Attaches the physics body to the entity via `with_physics_body()`
4. Creates the bounded floor collider with correct parameters

The scene loading implementation appears correct for physics setup. If the tesseract is not falling, the issue likely lies elsewhere (physics stepping, entity-body sync, or rendering).

---

## Detailed Code Trace

### 1. Scene File (`scenes/default.ron`)

The scene defines:
- **Tesseract:** position `(0, 0, 0, 0)`, size `2.0`, tag `"dynamic"`
- **Floor:** y=-2.0, size=10.0, cell_size=5.0, thickness=0.001, tag `"static"`
- **Gravity:** -20.0
- **Player spawn:** (0, 0, 5, 0)

### 2. `ActiveScene::from_template()` in `scene.rs` (lines 220-310)

The loading process:

```rust
// Line 246: Tag detection - CORRECT
let is_dynamic = entity_template.tags.contains(&"dynamic".to_string());
```

For dynamic entities (lines 262-287):

```rust
else if is_dynamic {
    // Line 264-269: Position extraction - CORRECT
    let position = Vec4::new(
        entity_template.transform.position.x,  // 0.0
        entity_template.transform.position.y,  // 0.0
        entity_template.transform.position.z,  // 0.0
        entity_template.transform.position.w,  // 0.0
    );

    // Line 272-275: Half-extent calculation - CORRECT
    let half_extent = match &entity_template.shape {
        ShapeTemplate::Tesseract { size } => size / 2.0,  // 2.0 / 2.0 = 1.0
        ...
    };

    // Line 277-283: Body creation - CORRECT
    let body = RigidBody4D::new_aabb(
        position,
        Vec4::new(half_extent, half_extent, half_extent, half_extent),  // (1.0, 1.0, 1.0, 1.0)
    )
    .with_body_type(BodyType::Dynamic)  // Explicitly set Dynamic
    .with_mass(10.0)
    .with_material(PhysicsMaterial::WOOD);

    // Line 285-286: Body attachment - CORRECT
    let body_key = physics.add_body(body);
    entity = entity.with_physics_body(body_key);
}
```

### 3. Floor Collider Creation (lines 249-261)

For static entities with Hyperplane shape:

```rust
if is_static {
    if let ShapeTemplate::Hyperplane { y, size, cell_size, thickness, .. } = &entity_template.shape {
        physics.add_static_collider(StaticCollider::floor_bounded(
            *y,           // -2.0 (floor surface level)
            *size,        // 10.0 (X/Z half-extent)
            *cell_size,   // 5.0 (W half-extent)
            *thickness,   // 0.001 (clamped to minimum 5.0 in floor_bounded)
            PhysicsMaterial::CONCRETE,
        ));
    }
}
```

### 4. `StaticCollider::floor_bounded()` in `body.rs` (lines 259-282)

```rust
pub fn floor_bounded(
    y: f32,              // -2.0
    half_size_xz: f32,   // 10.0
    half_size_w: f32,    // 5.0
    thickness: f32,      // 0.001 -> clamped to 5.0
    material: PhysicsMaterial,
) -> Self {
    let actual_thickness = thickness.max(5.0);  // 5.0 (minimum enforced)
    let half_thickness = actual_thickness / 2.0;  // 2.5

    // AABB center is below the surface
    let center = Vec4::new(0.0, y - half_thickness, 0.0, 0.0);  // (0, -4.5, 0, 0)
    let half_extents = Vec4::new(half_size_xz, half_thickness, half_size_xz, half_size_w);
    // (10.0, 2.5, 10.0, 5.0)

    // Resulting AABB:
    // min: (-10, -7, -10, -5)
    // max: (10, -2, 10, 5)
    // Top surface at y = -2.0 - CORRECT
}
```

### 5. Physics World Processing (`world.rs` lines 181-214)

The `step()` function correctly processes bodies:

```rust
pub fn step(&mut self, dt: f32) {
    for (key, body) in &mut self.bodies {
        if body.is_static() {
            continue;  // Skip static bodies
        }

        // Apply gravity to Dynamic bodies or the player
        let is_player = self.player_body == Some(key);
        if body.affected_by_gravity() || is_player {
            body.velocity.y += self.config.gravity * dt;
        }

        // Integrate velocity
        let displacement = body.velocity * dt;
        body.position = body.position + displacement;
        body.collider = body.collider.translated(displacement);
    }
    // ... collision resolution
}
```

`affected_by_gravity()` returns `true` for `BodyType::Dynamic` (line 56 in body.rs).

---

## Verification Checklist

| Question | Answer |
|----------|--------|
| Is "dynamic" tag correctly detected? | YES - Line 246 uses `.contains()` |
| Is `RigidBody4D::new_aabb()` called with correct params? | YES - Position (0,0,0,0), half_extents (1,1,1,1) |
| Is `BodyType::Dynamic` being set? | YES - Explicit `.with_body_type(BodyType::Dynamic)` |
| Is `body_key` attached to entity? | YES - `entity = entity.with_physics_body(body_key)` |
| Is floor bounded collider created correctly? | YES - Top at y=-2, extends 5 units down |
| Does `step()` apply gravity to Dynamic bodies? | YES - Checks `body.affected_by_gravity()` |

---

## Potential Issues NOT in Scene Loading

If the tesseract isn't falling, the issue might be:

1. **Entity-body position sync:** The `Entity.transform.position` may not be updated from `RigidBody4D.position` during rendering
   - Check: Is there code that syncs `world.physics().get_body(entity.physics_body).position` back to `entity.transform.position`?

2. **World update not being called:** The `ActiveScene::update()` method calls `self.world.update(dt)`, but this may not be invoked
   - Check: Is the game loop calling `scene.update(dt)` each frame?

3. **Rendering using wrong position:** The renderer might use `entity.transform.position` instead of the physics body position
   - Check: Does the render pipeline read from physics bodies for entities with `physics_body.is_some()`?

4. **Physics world not enabled:** The world might not have physics attached
   - Check: Is `ActiveScene::from_template()` being called with a valid `PhysicsConfig`?

---

## Recommendations

1. **Add integration test:** Create a test that loads the default scene, steps physics for 1 second, and verifies the tesseract's Y position has decreased

2. **Add debug logging:** Add `log::debug!` calls in `from_template()` to confirm the dynamic body is being created

3. **Verify entity-physics sync:** Ensure there's a system that copies physics body positions back to entity transforms each frame

4. **Check the render pipeline:** Ensure rendering uses physics positions for physics-enabled entities

---

## Conclusion

The scene loading code correctly creates a Dynamic physics body for the tesseract. The bug is likely in:
- The entity-body position synchronization, or
- The game loop not calling physics updates, or
- The renderer not reading physics positions

Further investigation should focus on how physics body positions propagate to rendered entity transforms.
