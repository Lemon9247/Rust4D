# Wave 3: Bug Fixes & Testing

**Effort**: 1-2 sessions
**Priority**: MEDIUM
**Dependencies**: Wave 1 (for serial_test pattern reference)

---

## Overview

Fix the orphaned physics bodies bug and add unit tests for the completely untested `rust4d_input` crate. This wave improves correctness and prevents regressions.

---

## Task 1: Fix Orphaned Physics Bodies

**Priority**: HIGH
**Effort**: 30 minutes
**Files**:
- `crates/rust4d_core/src/world.rs`
- `crates/rust4d_core/tests/physics_integration.rs` (new test)

### Problem
When `World::remove_entity()` is called, the entity is removed from the World's SlotMap, but if the entity had a physics body, that body remains in `PhysicsWorld`. This causes:
- Memory leak (physics bodies accumulate)
- Potential stale collisions
- Incorrect physics simulation

### Current Code (`world.rs:89-100`)
```rust
pub fn remove_entity(&mut self, key: EntityKey) -> Option<Entity> {
    if let Some(entity) = self.entities.remove(key) {
        // Remove from name index
        if let Some(name) = &entity.name {
            self.name_index.remove(name);
        }
        Some(entity)
        // BUG: entity.physics_body is not cleaned up!
    } else {
        None
    }
}
```

### Solution
Check if the entity has a physics body and remove it from PhysicsWorld before removing the entity.

### Steps

1. **Update `World::remove_entity()`** in `crates/rust4d_core/src/world.rs`:

```rust
pub fn remove_entity(&mut self, key: EntityKey) -> Option<Entity> {
    if let Some(entity) = self.entities.remove(key) {
        // Remove from name index
        if let Some(name) = &entity.name {
            self.name_index.remove(name);
        }

        // Clean up physics body if present
        if let Some(body_key) = entity.physics_body {
            if let Some(physics) = &mut self.physics_world {
                physics.remove_body(body_key);
            }
        }

        Some(entity)
    } else {
        None
    }
}
```

2. **Verify PhysicsWorld has `remove_body()` method**:
Check `crates/rust4d_physics/src/world.rs`. If it doesn't exist, add it:

```rust
impl PhysicsWorld {
    /// Remove a rigid body from the physics world
    pub fn remove_body(&mut self, key: BodyKey) -> Option<RigidBody4D> {
        self.bodies.remove(key)
    }
}
```

3. **Add integration test** in `crates/rust4d_core/tests/physics_integration.rs`:

```rust
#[test]
fn test_remove_entity_cleans_up_physics_body() {
    let physics_config = PhysicsConfig {
        gravity: -20.0,
        jump_velocity: 8.0,
    };

    let mut world = World::new().with_physics(physics_config);

    // Create entity with physics body
    let mut entity = Entity::new(Tesseract4D::new(1.0));
    entity.transform.position = Vec4::new(0.0, 5.0, 0.0, 0.0);

    let entity_key = world.add_entity(entity);

    // Add physics body to entity
    let body = RigidBody4D::new_aabb(/* params */);
    let body_key = world.physics_mut().unwrap().add_body(body);
    world.get_entity_mut(entity_key).unwrap().physics_body = Some(body_key);

    // Verify body exists
    assert!(world.physics().unwrap().get_body(body_key).is_some());

    // Remove entity
    world.remove_entity(entity_key);

    // Verify physics body was also removed
    assert!(world.physics().unwrap().get_body(body_key).is_none());
}
```

### Verification
```bash
cargo test -p rust4d_core test_remove_entity_cleans_up_physics_body
cargo test -p rust4d_core  # All existing tests still pass
```

---

## Task 2: Add Unit Tests for rust4d_input

**Priority**: MEDIUM
**Effort**: 1 session
**Files**:
- `crates/rust4d_input/src/camera_controller.rs`

### Problem
The `CameraController` struct has complex logic for:
- Input smoothing with exponential decay
- Movement direction calculation
- Key state tracking
- Mouse delta accumulation

None of this is tested. A regression could break all player movement.

### Test Coverage Plan

#### 2.1 Builder Pattern Tests
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_values() {
        let controller = CameraController::new();
        assert_eq!(controller.move_speed(), 3.0);
        assert_eq!(controller.w_move_speed(), 2.0);
        assert!(!controller.is_smoothing_enabled());
    }

    #[test]
    fn test_builder_move_speed() {
        let controller = CameraController::new()
            .with_move_speed(5.0);
        assert_eq!(controller.move_speed(), 5.0);
    }

    #[test]
    fn test_builder_chaining() {
        let controller = CameraController::new()
            .with_move_speed(5.0)
            .with_w_move_speed(3.0)
            .with_mouse_sensitivity(0.005)
            .with_smoothing(true)
            .with_smoothing_half_life(0.1);

        assert_eq!(controller.move_speed(), 5.0);
        assert_eq!(controller.w_move_speed(), 3.0);
        assert!(controller.is_smoothing_enabled());
    }
}
```

#### 2.2 Key State Tests
```rust
#[test]
fn test_key_pressed() {
    let mut controller = CameraController::new();

    // Initially no keys pressed
    assert!(!controller.is_moving());

    // Press W key
    controller.key_pressed(KeyCode::KeyW);
    assert!(controller.is_moving());

    // Release W key
    controller.key_released(KeyCode::KeyW);
    assert!(!controller.is_moving());
}

#[test]
fn test_multiple_keys() {
    let mut controller = CameraController::new();

    controller.key_pressed(KeyCode::KeyW);
    controller.key_pressed(KeyCode::KeyA);
    assert!(controller.is_moving());

    // Release one key, still moving
    controller.key_released(KeyCode::KeyW);
    assert!(controller.is_moving());

    // Release all keys
    controller.key_released(KeyCode::KeyA);
    assert!(!controller.is_moving());
}
```

#### 2.3 Movement Direction Tests
```rust
#[test]
fn test_forward_movement() {
    let mut controller = CameraController::new();
    controller.key_pressed(KeyCode::KeyW);

    let movement = controller.get_movement_input();
    assert!(movement.z < 0.0); // Forward is -Z
    assert_eq!(movement.x, 0.0);
}

#[test]
fn test_diagonal_movement_normalized() {
    let mut controller = CameraController::new();
    controller.key_pressed(KeyCode::KeyW);
    controller.key_pressed(KeyCode::KeyD);

    let movement = controller.get_movement_input();
    // Diagonal movement should be normalized
    let length = (movement.x * movement.x + movement.z * movement.z).sqrt();
    assert!((length - 1.0).abs() < 0.01);
}

#[test]
fn test_opposing_keys_cancel() {
    let mut controller = CameraController::new();
    controller.key_pressed(KeyCode::KeyW);
    controller.key_pressed(KeyCode::KeyS);

    let movement = controller.get_movement_input();
    assert_eq!(movement.z, 0.0); // Forward and back cancel
}
```

#### 2.4 Smoothing Tests
```rust
#[test]
fn test_smoothing_disabled() {
    let mut controller = CameraController::new()
        .with_smoothing(false);

    controller.key_pressed(KeyCode::KeyW);
    let movement = controller.update(0.016); // ~60fps

    // Without smoothing, should immediately reach full speed
    assert!(movement.z.abs() > 0.9);
}

#[test]
fn test_smoothing_enabled_gradual() {
    let mut controller = CameraController::new()
        .with_smoothing(true)
        .with_smoothing_half_life(0.1);

    controller.key_pressed(KeyCode::KeyW);

    // First frame - should be less than full speed
    let movement1 = controller.update(0.016);

    // After several frames - should approach full speed
    for _ in 0..20 {
        controller.update(0.016);
    }
    let movement2 = controller.update(0.016);

    assert!(movement2.z.abs() > movement1.z.abs());
}
```

#### 2.5 Mouse Input Tests
```rust
#[test]
fn test_mouse_delta_accumulation() {
    let mut controller = CameraController::new();

    controller.mouse_moved(10.0, 5.0);
    controller.mouse_moved(5.0, 3.0);

    let (dx, dy) = controller.consume_mouse_delta();
    assert_eq!(dx, 15.0);
    assert_eq!(dy, 8.0);

    // After consume, should be zero
    let (dx2, dy2) = controller.consume_mouse_delta();
    assert_eq!(dx2, 0.0);
    assert_eq!(dy2, 0.0);
}
```

### Steps

1. Add `#[cfg(test)]` module to `camera_controller.rs`
2. Implement tests from sections 2.1-2.5
3. Add any necessary getter methods for testing (e.g., `move_speed()`)
4. Run tests and fix any issues found

### Verification
```bash
cargo test -p rust4d_input
# Should have 15+ new tests
```

---

## Task 3: Clean Up Test Warnings

**Priority**: LOW
**Effort**: 15 minutes
**Files**: Various test modules

### Items to Fix

1. **Unused imports in scene_manager tests**:
```rust
// crates/rust4d_core/src/scene_manager.rs:225-226
// Remove unused: Material, Vec4
```

2. **Unused variable in collision tests**:
```rust
// crates/rust4d_physics/src/collision.rs:603
// Remove or use: tesseract_resting
```

3. **Debug println! in tests**:
Search and remove or gate behind a flag:
```bash
grep -r "println!" crates/*/src/*.rs --include="*.rs" | grep "#\[test\]" -A 5
```

4. **Soft skip pattern**:
Convert `test_load_default_scene_file` from soft skip to `#[ignore]`:
```rust
// Before:
#[test]
fn test_load_default_scene_file() {
    if !Path::new("scenes/default.ron").exists() {
        return; // Soft skip
    }
    // ...
}

// After:
#[test]
#[ignore = "Requires scenes/default.ron to exist"]
fn test_load_default_scene_file() {
    // ...
}
```

5. **Fix or document ignored doc-test**:
```rust
// crates/rust4d_core/src/scene_manager.rs:9
// Either make the doc-test work or add explanation
/// ```ignore
/// // This example requires a running event loop
/// ```
```

### Steps
1. Fix each warning/issue
2. Run `cargo test --workspace` to verify no regressions
3. Run `cargo test --workspace 2>&1 | grep -i warning` to verify warnings eliminated

---

## Checklist

- [ ] Add `remove_body()` to PhysicsWorld if missing
- [ ] Update `World::remove_entity()` to clean up physics body
- [ ] Add integration test for physics body cleanup
- [ ] Create test module in camera_controller.rs
- [ ] Add builder pattern tests (5+ tests)
- [ ] Add key state tests (3+ tests)
- [ ] Add movement direction tests (4+ tests)
- [ ] Add smoothing tests (2+ tests)
- [ ] Add mouse input tests (1+ test)
- [ ] Clean up unused imports in tests
- [ ] Clean up unused variables in tests
- [ ] Convert soft skip to `#[ignore]`
- [ ] Fix or document ignored doc-test
- [ ] Verify all tests pass: `cargo test --workspace`
- [ ] Verify no test warnings: `cargo test --workspace 2>&1 | grep warning`

---

## Commits

1. `Fix orphaned physics bodies when entity removed from World`
2. `Add unit tests for CameraController input handling`
3. `Clean up test warnings and fix test organization`

---

## Impact

### Before
- Physics bodies leaked when entities removed
- CameraController had 0 tests - any change could break movement
- 4+ test warnings in codebase

### After
- Physics bodies properly cleaned up
- CameraController has 15+ tests covering core functionality
- Clean test output with no warnings
