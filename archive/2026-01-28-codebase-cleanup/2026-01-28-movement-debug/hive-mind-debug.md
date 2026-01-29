# Hive Mind: Movement Debug Investigation

**Task:** Investigate why using `camera.ana()` for W-axis movement still doesn't rotate with 4D orientation

## Context

A fix was attempted that changed:
```rust
// Before:
+ Vec4::W * w_input

// After:
let camera_ana = self.camera.ana();
let ana_xzw = Vec4::new(camera_ana.x, 0.0, camera_ana.z, camera_ana.w).normalized();
+ ana_xzw * w_input
```

But the movement still doesn't follow 4D rotation. Need to understand why.

## Agent Assignments

| Agent | Focus Area | Status |
|-------|------------|--------|
| Camera Matrix Agent | How camera_matrix() is built, what ana() actually returns | Pending |
| Controller Agent | How 4D rotation is triggered, what rotate_w/rotate_xw do | Pending |
| Movement Flow Agent | Trace the full path from input to position change | Pending |
| Debug Agent | Add debug output to understand actual values at runtime | Pending |

## Key Questions

1. When the user rotates in 4D, does `rotation_4d` actually change?
2. Does `camera_matrix()` incorporate `rotation_4d` correctly?
3. Does `ana()` return a different vector after 4D rotation?
4. Is the movement being calculated correctly but applied incorrectly?
5. Is the titlebar showing the wrong coordinates?

## Findings

(To be filled as agents complete)

---
