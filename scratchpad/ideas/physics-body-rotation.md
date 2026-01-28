# Physics Body Rotation

**Date:** 2026-01-28
**Status:** Idea for future consideration
**Source:** Movement analysis swarm (Coordinate Agent)

## Observation

Currently `RigidBody4D` has no rotation field:

```rust
pub struct RigidBody4D {
    pub position: Vec4,
    pub velocity: Vec4,
    pub mass: f32,
    pub material: PhysicsMaterial,
    pub collider: Collider,
    pub body_type: BodyType,
    pub grounded: bool,
    pub filter: CollisionFilter,
    // NO ROTATION FIELD
}
```

The camera has full orientation (`pitch` + `rotation_4d: Rotor4`), but this information isn't stored in the physics system.

## Potential Benefits

1. **Physics-driven rotation:** Bodies could rotate from collisions/forces
2. **Oriented colliders:** Non-spherical colliders could rotate
3. **Angular velocity:** Could add spinning objects
4. **Unified player state:** Player orientation would live in physics, camera would just follow

## Current Workaround

For movement, we use the camera's orientation directly (`camera.ana()`, `camera.forward()`, etc.) rather than storing orientation in physics. This works fine for first-person games where camera = player orientation.

## When to Implement

Consider adding physics rotation when:
- Adding rotating physics objects (spinning platforms, tumbling debris)
- Adding non-axis-aligned colliders
- Moving to third-person where camera ≠ player orientation
- Implementing angular momentum / torque

For now, the camera-based approach is sufficient.
