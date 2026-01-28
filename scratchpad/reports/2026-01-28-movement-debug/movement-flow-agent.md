# Movement Flow Agent Report

**Date:** 2026-01-28
**Task:** Trace the full movement path from input to position change

## Complete Movement Flow

### Step 1: Input Reading (main.rs:310-311)
```rust
let (forward_input, right_input) = self.controller.get_movement_input();
let w_input = self.controller.get_w_input();
```
Raw input: -1.0, 0.0, or 1.0 based on key presses.

### Step 2: Direction Calculation (main.rs:315-328)
```rust
let camera_forward = self.camera.forward();
let camera_right = self.camera.right();

// Project to XZ plane
let forward_xz = Vec4::new(camera_forward.x, 0.0, camera_forward.z, 0.0).normalized();
let right_xz = Vec4::new(camera_right.x, 0.0, camera_right.z, 0.0).normalized();

// Get camera's ana direction
let camera_ana = self.camera.ana();
let ana_xzw = Vec4::new(camera_ana.x, 0.0, camera_ana.z, camera_ana.w).normalized();

// Combine
let move_dir = forward_xz * forward_input + right_xz * right_input + ana_xzw * w_input;
```

### Step 3: Apply to Physics (main.rs:332-334)
```rust
let move_speed = self.controller.move_speed;
physics.apply_player_movement(move_dir * move_speed);
```

### Step 4: Physics Application (world.rs:149-156)
```rust
pub fn apply_player_movement(&mut self, movement: Vec4) {
    if let Some(body) = self.player_mut() {
        body.velocity.x = movement.x;
        body.velocity.z = movement.z;
        body.velocity.w = movement.w;
        // Note: Y is preserved for gravity/jumping
    }
}
```

### Step 5: Physics Step (main.rs:344)
```rust
self.scene_manager.update(dt);
```
This calls `physics.step()` which integrates velocity into position.

### Step 6: Position Sync from Physics (main.rs:364-366)
```rust
if let Some(pos) = ... physics ... .player_position() {
    self.camera.position = pos;
}
```

### Step 7: Controller Update (main.rs:371)
```rust
self.controller.update(&mut self.camera, dt, self.cursor_captured);
```
**Note:** This also applies movement via camera methods, but...

### Step 8: Re-sync Position (main.rs:374-376)
```rust
if let Some(pos) = ... physics ... .player_position() {
    self.camera.position = pos;
}
```
The controller's movement is discarded; we use physics position.

### Step 9: Titlebar Display (main.rs:380-382)
```rust
let pos = self.camera.position;
format!("{} - ({:.1}, {:.1}, {:.1}, {:.1})", ..., pos.x, pos.y, pos.z, pos.w)
```

## Key Observations

1. **Movement direction IS correctly calculated** using `camera.ana()`
2. **Physics correctly applies** the movement vector to velocity
3. **Position is synced** from physics to camera
4. **Titlebar displays** camera.position which comes from physics

## Potential Issue Found

The controller's `update()` method at step 7 also applies movement via camera methods:
```rust
camera.move_local_xz(...)
camera.move_y(...)
camera.move_w(...)  // This uses camera's internal transformation
```

But this is intentionally discarded at step 8. The comment in the code confirms this:
> "controller.update() also applies movement which we don't want"

## Conclusion

The movement flow is correct:
1. Direction calculated with `camera.ana()`
2. Applied to physics
3. Integrated into position
4. Synced back to camera
5. Displayed in titlebar

**If the direction doesn't change after 4D rotation, the issue must be in how `camera.ana()` is computed** - specifically in the SkipY transformation or rotor-to-matrix conversion.
