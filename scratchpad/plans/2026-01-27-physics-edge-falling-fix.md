# 4D Physics Edge Falling Bug Investigation

## Problem Statement
When the player walks off the W edge of the hyperplane (4th dimension), they should fall into the void. Currently, the player can walk off the edge and walk back without falling.

## Analysis Summary

### What's Working Correctly
1. **Collision detection** - `sphere_vs_aabb` correctly handles 4D collision
2. **Bounded floor creation** - `StaticCollider::floor_bounded` creates an AABB with proper W bounds
3. **Unit tests pass** - `test_player_falls_off_w_edge` verifies the physics code works in isolation
4. **Visual mesh** - The hyperplane disappears when the player walks past W bounds

### Suspected Issue: Collision Normal Direction at Edges

When the player walks off the W edge and tries to return:

1. Player starts at W=0, Y=0.5 (on floor)
2. Player walks to W=3 (past floor W bounds of -2 to +2)
3. Player begins falling (no collision when W > 2.5)
4. Player presses E to return, moving toward W=0
5. At W≈2.4, collision resumes BUT with **normal pointing in +W direction** (not +Y)
6. Player is pushed back in +W, Y continues to fall
7. Player oscillates at edge, slowly falling

The key issue: When the player re-enters the collision zone from the W edge, the collision normal points outward in W (pushing them back out) rather than upward in Y (correcting their fall). This allows them to "hover" at the edge while slowly falling.

The player CAN eventually fall below Y=-5 (AABB bottom) and fall completely, but this takes ~0.77 seconds of oscillating at the edge - long enough to not feel like "falling off an edge".

### Scene Configuration (test_chamber.ron)
- Floor W extent: `cell_size: 2.0` → W bounds from -2 to +2
- Player radius: 0.5 → Collision zone extends to W = ±2.5
- W move speed: 2.0 units/s

### Fix Options

**Option A: Add Edge Falling Detection**
Add explicit logic to detect when the player walks off a bounded surface edge and apply falling physics. This would check if the player's XZW position is outside all floor bounds and mark them as "in void" regardless of collision.

**Option B: Modify Collision Response for Kinematic Bodies**
When a kinematic player body is at an edge (collision normal not pointing +Y), don't reset grounded state and allow gravity to accumulate properly. This ensures falling continues even during W-direction collisions.

**Option C: Create Invisible Barrier Colliders at Edges** (Not recommended)
Add invisible walls at floor edges that would push the player back instead of letting them walk off. This changes the gameplay semantics.

## Recommended Approach: Option A

Add edge detection to the physics step that explicitly handles the "player off all floors" case:

1. After collision resolution, check if player is grounded
2. If not grounded, check if player is within the XZW bounds of ANY floor
3. If not within any floor bounds, apply normal falling physics
4. If within floor bounds but not touching (e.g., jumped), allow normal physics

## Files to Modify

1. `crates/rust4d_physics/src/world.rs` - Add edge detection in `step()` or `resolve_static_collisions()`
2. `crates/rust4d_physics/src/body.rs` - May need new method on `StaticCollider` to check if a position is "over" the collider (XZW bounds check ignoring Y)

## Implementation Steps

### Step 1: Add "over bounds" check to StaticCollider
```rust
impl StaticCollider {
    /// Check if a position (ignoring Y) is within the XZW bounds of this collider
    pub fn is_position_over(&self, position: Vec4) -> bool {
        match &self.collider {
            Collider::AABB(aabb) => {
                position.x >= aabb.min.x && position.x <= aabb.max.x &&
                position.z >= aabb.min.z && position.z <= aabb.max.z &&
                position.w >= aabb.min.w && position.w <= aabb.max.w
            }
            Collider::Plane(_) => true, // Infinite planes extend forever
            _ => false,
        }
    }
}
```

### Step 2: Modify grounded detection in PhysicsWorld
After collision resolution, if the player is not grounded, check if they're over ANY floor:

```rust
// In resolve_static_collisions or step:
// If player was not grounded by collision detection,
// and player is not over any floor bounds, ensure they keep falling
if let Some(player_key) = self.player_body {
    let player = &self.bodies[player_key];
    if !player.grounded {
        let is_over_any_floor = self.static_colliders.iter()
            .any(|col| col.is_position_over(player.position));

        // If not over any floor, they're in the void - gravity will handle it
        // If over a floor but not grounded, they're jumping or haven't landed yet
    }
}
```

### Step 3: Add tests
- Test that player falls when walking off W edge
- Test that player doesn't fall when over floor but jumping
- Test that player lands correctly when jumping over floor

## Verification

After implementation:
1. Run `cargo test -p rust4d_physics` to verify existing tests still pass
2. Run the game and walk off the W edge - player should fall
3. Verify jumping still works correctly
4. Verify walking on the floor still works

## Alternative: Debug Current Behavior First

Before implementing a fix, it may be worth adding debug logging to trace exactly what's happening during the edge case. This would confirm the analysis above.

Add to `resolve_static_collisions`:
```rust
if is_player {
    log::debug!("Player collision: normal={:?}, penetration={}, grounded={}",
        contact.normal, contact.penetration, body.grounded);
}
```
