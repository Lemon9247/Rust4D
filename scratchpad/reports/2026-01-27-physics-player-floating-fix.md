# Session Report: Physics Bug Fix - Player Floating on Tesseract

**Date:** 2026-01-27
**Branch:** feature/physics
**Status:** Complete

---

## Problem

When the player walked into the tesseract, they would float on top instead of pushing it. The player could then float indefinitely. Additionally, Q/E (W-axis movement) and Space (jump) controls were broken.

## Root Cause Analysis

The bug had **three interconnected causes**:

### 1. Kinematic Body Collision Handling (Physics)

**File:** `crates/rust4d_physics/src/world.rs`

The initial analysis correctly identified that kinematic bodies (the player) were being treated like dynamic bodies in collision resolution. However, the first fix was too aggressive - it made kinematic bodies completely immovable, which broke floor collision (player fell through the floor).

**Correct fix:** Kinematic bodies need nuanced collision handling:
- **vs Static (floors/walls):** Kinematic gets pushed out (static always wins)
- **vs Dynamic (tesseract):** Dynamic gets pushed, kinematic doesn't move

```rust
// Position correction: kinematic is immovable only vs dynamic
let can_correct_a = !is_static_a && (!is_kinematic_a || is_static_b);
let can_correct_b = !is_static_b && (!is_kinematic_b || is_static_a);
```

### 2. Camera W Position Reset (Main Loop)

**File:** `src/main.rs`

The game loop was syncing the **entire** camera position from physics (including W), then adding W input, then calling `controller.update()` which applied movement again, then re-syncing the entire position. This caused W movement to never accumulate.

**Fix:** Only sync X, Y, Z from physics; preserve W for 4D navigation:
```rust
// Before: self.camera.position = pos;  // Reset everything including W
// After:
self.camera.position.x = pos.x;
self.camera.position.y = pos.y;
self.camera.position.z = pos.z;
// W preserved for 4D navigation
```

### 3. No Gravity for Kinematic Player (Physics)

**File:** `crates/rust4d_physics/src/world.rs`

Kinematic bodies don't have gravity by design (they're meant to be moved by code, not physics). But the player needs gravity to fall and become grounded for jumping to work.

**Fix:** Apply gravity to the player body specifically:
```rust
let is_player = self.player_body == Some(key);
if body.affected_by_gravity() || is_player {
    body.velocity.y += self.config.gravity * dt;
}
```

## Design Insight: Player as Hybrid Body

The player character needs hybrid behavior that doesn't fit neatly into standard physics body types:

| Behavior | Like Kinematic | Like Dynamic |
|----------|----------------|--------------|
| Horizontal movement | User-controlled | - |
| Vertical movement | - | Gravity + jump |
| vs Static geometry | Pushed out | Pushed out |
| vs Dynamic objects | NOT pushed | - |
| Pushes dynamic objects | Yes | Yes |

Most engines solve this with a dedicated "Character Controller" component. Our solution treats the player as kinematic for collision purposes but with explicit gravity application.

## Commits

1. `3f1051b` - Fix kinematic bodies being pushed by dynamic bodies
2. `c8a9e57` - Fix camera W position being reset by physics sync
3. `3168df4` - Apply gravity to player body even though it's kinematic

## Files Modified

| File | Changes |
|------|---------|
| `crates/rust4d_physics/src/world.rs` | Collision hierarchy + player gravity |
| `src/main.rs` | Camera sync preserves W position |

## Tests Added

- `test_kinematic_pushes_dynamic` - Kinematic can push dynamic
- `test_kinematic_not_pushed_by_dynamic` - Kinematic isn't pushed by dynamic
- `test_kinematic_velocity_not_modified` - Kinematic velocity unchanged by collision

## Verification

All 263 workspace tests pass. Manual testing confirms:
- Q/E moves player in W axis (movement accumulates)
- Space makes player jump when grounded
- Walking into tesseract pushes it without floating on top
- Player falls to floor and stops (gravity + floor collision work)

## Lessons Learned

1. **Investigate the full system, not just the reported symptom.** The initial plan focused on physics collision, but the actual bugs spanned physics, the game loop, and the camera sync.

2. **Kinematic vs Dynamic is a spectrum.** Player characters often need behaviors from both categories, requiring special handling.

3. **Position syncing must be surgical.** Syncing the entire Vec4 position when only XYZ comes from physics caused W to be reset every frame.
