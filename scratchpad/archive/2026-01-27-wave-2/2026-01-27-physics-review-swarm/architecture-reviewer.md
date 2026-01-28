# Architecture Review: Physics/Rendering Integration Analysis

**Agent:** Architecture Reviewer
**Date:** 2026-01-27
**Focus:** Game loop, update flow, and physics/rendering synchronization

## Executive Summary

After thorough analysis of the game loop in `main.rs`, the `World::update()` method, and the `SceneManager`, **the architecture appears to be correctly structured**. The update order is correct, dirty tracking is properly implemented, and geometry rebuild is triggered when entities move. However, I identified a **potential timing issue** that could cause desync on the first frame or in edge cases.

## Key Findings

### 1. Game Loop Analysis (main.rs:299-517)

The `RedrawRequested` handler follows the correct update order:

```
1. Get movement input (lines 307-309)
2. Calculate movement direction (lines 311-321)
3. Apply player movement to physics (lines 324-327)
4. Handle jump (lines 329-334)
5. Step physics via scene_manager.update(dt) (line 337)
6. Check dirty entities and rebuild geometry (lines 340-354)
7. Sync camera to player position (lines 356-380)
8. Render (lines 400-511)
```

**VERDICT: Correct order** - Physics is stepped before geometry rebuild, and geometry is rebuilt before rendering.

### 2. SceneManager.update() (scene_manager.rs:215-219)

```rust
pub fn update(&mut self, dt: f32) {
    if let Some(scene) = self.active_scene_mut() {
        scene.update(dt);
    }
}
```

**VERDICT: Correctly implemented** - Delegates to `ActiveScene::update()`.

### 3. ActiveScene.update() (scene.rs:334-336)

```rust
pub fn update(&mut self, dt: f32) {
    self.world.update(dt);
}
```

**VERDICT: Correctly implemented** - Delegates to `World::update()`.

### 4. World.update() (world.rs:154-174)

```rust
pub fn update(&mut self, dt: f32) {
    // Step the physics simulation
    if let Some(ref mut physics) = self.physics_world {
        physics.step(dt);
    }

    // Sync entity transforms from their physics bodies
    if let Some(ref physics) = self.physics_world {
        for (_key, entity) in &mut self.entities {
            if let Some(body_key) = entity.physics_body {
                if let Some(body) = physics.get_body(body_key) {
                    // Only update and mark dirty if position actually changed
                    if entity.transform.position != body.position {
                        entity.transform.position = body.position;
                        entity.mark_dirty(DirtyFlags::TRANSFORM);
                    }
                }
            }
        }
    }
}
```

**VERDICT: Correctly implemented** - Steps physics, then syncs entity transforms, marking them dirty when positions change.

### 5. Dirty Entity Check (main.rs:340-354)

```rust
if self.scene_manager.active_world().map(|w| w.has_dirty_entities()).unwrap_or(false) {
    // Rebuild geometry with new transforms
    self.geometry = Self::build_geometry(self.scene_manager.active_world().unwrap());
    // Re-upload to GPU
    if let (Some(slice_pipeline), Some(ctx)) = (&mut self.slice_pipeline, &self.render_context) {
        slice_pipeline.upload_tetrahedra(
            &ctx.device,
            &self.geometry.vertices,
            &self.geometry.tetrahedra,
        );
    }
    if let Some(w) = self.scene_manager.active_world_mut() {
        w.clear_all_dirty();
    }
}
```

**VERDICT: Correctly implemented** - Checks for dirty entities, rebuilds geometry, uploads to GPU, and clears dirty flags.

## Integration Chain Verification

The full update chain is:
```
main.rs: scene_manager.update(dt)
    -> SceneManager::update(dt)
        -> ActiveScene::update(dt)
            -> World::update(dt)
                -> PhysicsWorld::step(dt)  [moves physics bodies]
                -> Sync entity transforms  [marks entities dirty]

main.rs: has_dirty_entities() check
    -> World::has_dirty_entities()
        -> Returns true if any entity has dirty flags

main.rs: build_geometry() + upload_tetrahedra()
    -> Rebuilds and uploads new geometry

main.rs: clear_all_dirty()
    -> Clears all dirty flags for next frame
```

**This chain is complete and correctly ordered.**

## Potential Issues Identified

### Issue 1: First Frame Dirty Clear (CRITICAL - POTENTIAL ROOT CAUSE)

**Location:** `App::new()` (main.rs:82) and first `RedrawRequested`

When the app starts:
1. `App::new()` calls `Self::build_geometry()` (line 82)
2. The initial geometry is uploaded in `resumed()` (lines 205-209)
3. **BUT `clear_all_dirty()` is NEVER called after initial load**

This means:
- New entities start with `DirtyFlags::ALL` (entity.rs:148)
- The first frame WILL trigger a geometry rebuild (good)
- Dirty flags ARE cleared after the rebuild (line 351-353)

**VERDICT: Not a bug** - The dirty flags system handles this correctly on the first frame.

### Issue 2: Floating Point Comparison in World::update()

**Location:** world.rs:166

```rust
if entity.transform.position != body.position {
```

This uses direct floating-point equality comparison. In theory, this could miss tiny position changes due to floating-point precision, but:
- Both positions come from the same physics system
- The comparison is exact (no tolerance)
- If positions are truly equal, no update is needed

**VERDICT: Low risk** - This is actually correct behavior. If the physics body didn't move, the position will be exactly equal.

### Issue 3: Scene Without Physics

**Location:** scene.rs:237-239

```rust
} else {
    log::debug!("No physics configured");
    World::new()  // No physics
}
```

If a scene is created without physics configuration AND without template gravity, the world will have NO physics. This is intentional but could cause confusion if a user expects physics.

**VERDICT: By design** - Documented behavior.

## Critical Path Verification

For a tesseract to fall to the floor:

1. **Scene must have physics enabled** - Verified in `ActiveScene::from_template()`:
   - Uses `physics_config` if provided
   - OR uses `template.gravity` if set

2. **Tesseract must have a physics body** - Verified in `ActiveScene::from_template()`:
   - Entities with "dynamic" tag get `RigidBody4D::new_aabb()` body
   - Body is linked via `entity.with_physics_body(body_key)`

3. **Floor must have a static collider** - Verified in `ActiveScene::from_template()`:
   - Entities with "static" tag AND `Hyperplane` shape get `StaticCollider::floor_bounded()`

4. **Physics must step** - Verified in call chain above

5. **Entity transform must sync** - Verified in `World::update()`

6. **Dirty flag must be set** - Verified in `World::update()` when position changes

7. **Geometry must rebuild** - Verified in `RedrawRequested` handler

**All steps in the critical path are implemented correctly.**

## Recommendations

### 1. Add Debug Logging for Physics Sync

Consider adding temporary debug logging to trace the physics-to-entity sync:

```rust
// In World::update()
if entity.transform.position != body.position {
    log::trace!("Entity {:?} synced: {:?} -> {:?}",
        entity.name, entity.transform.position, body.position);
    entity.transform.position = body.position;
    entity.mark_dirty(DirtyFlags::TRANSFORM);
}
```

### 2. Verify Scene File Configuration

The bug may be in the scene configuration rather than the code. Check:
- Does the scene file have `gravity` set?
- Does the tesseract entity have the `"dynamic"` tag?
- Does the floor entity have the `"static"` tag?
- Is the tesseract's Y position above the floor's Y position?

### 3. Check Physics Body Type

In `ActiveScene::from_template()` (line 281), dynamic entities are created with:
```rust
.with_body_type(BodyType::Dynamic)
```

Verify that `BodyType::Dynamic` bodies receive gravity. Check `PhysicsWorld::step()` (world.rs:198-201):
```rust
if body.affected_by_gravity() || is_player {
    body.velocity.y += self.config.gravity * dt;
}
```

The condition uses `affected_by_gravity()`. Verify this returns `true` for `BodyType::Dynamic`.

## Conclusion

**The architecture is sound.** The game loop, physics stepping, transform syncing, dirty tracking, and geometry rebuild are all correctly implemented and properly ordered.

If the tesseract is not falling, the issue is likely:
1. **Scene configuration** - Missing gravity, tags, or incorrect positions
2. **Physics body configuration** - Body type or gravity flag
3. **Collision detection** - Floor collider not being detected (different agent reviewing)

The architecture review found no bugs in the integration between physics and rendering systems.
