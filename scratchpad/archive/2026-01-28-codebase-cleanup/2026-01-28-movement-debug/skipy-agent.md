# SkipY Agent Report

**Date:** 2026-01-28
**Task:** Analyze the SkipY transformation and how it affects 4D rotations

## Deep Analysis of SkipY

### 1. SkipY Function Overview

**File:** `crates/rust4d_math/src/mat4.rs` lines 49-106

SkipY remaps a 3D rotation to operate in the XZW hyperplane:
- Input indices [0,1,2] (XYZ) → Output indices [0,2,3] (XZW)
- Output Y (index 1) stays identity

```rust
let src_idx = [0usize, 1, 2];  // Input: X, Y, Z
let dst_idx = [0usize, 2, 3];  // Output: X, Z, W

for i in 0..3 {
    for j in 0..3 {
        result[dst_idx[j]][dst_idx[i]] = m[src_idx[j]][src_idx[i]];
    }
}
result[1][1] = 1.0;  // Y stays identity
```

### 2. Critical Insight: SkipY Does NOT Lock W

Common misconception: SkipY keeps both Y and W fixed.

**Reality:** SkipY only locks Y. It ALLOWS W to be rotated by remapping Z-related rotations to affect W.

### 3. Rotation Plane Transformations

| Input Plane | Before SkipY | After SkipY |
|-------------|--------------|-------------|
| XY | X↔Y rotation | X↔Z rotation (horizontal turn) |
| XZ | X↔Z rotation | **X↔W rotation** |
| YZ | Y↔Z rotation | **Z↔W rotation** |

### 4. Mathematical Trace: 90° XZ Rotation

Input: XZ plane rotation by 90° (what `rotate_w(PI/2)` creates before SkipY)

Before SkipY matrix (column-major):
```
Col 0 (X): [0, 0, 1, 0]   // X → Z
Col 1 (Y): [0, 1, 0, 0]   // Y → Y
Col 2 (Z): [-1, 0, 0, 0]  // Z → -X
Col 3 (W): [0, 0, 0, 1]   // W → W
```

After SkipY (remapping cols/rows 0,1,2 → 0,2,3):
```
Col 0 (X): [0, 0, 0, 1]   // X → W
Col 1 (Y): [0, 1, 0, 0]   // Y → Y (identity)
Col 2 (Z): [0, 0, 1, 0]   // Z → Z (identity from init)
Col 3 (W): [-1, 0, 0, 0]  // W → -X
```

### 5. What ana() Returns After This Rotation

```rust
ana() = transform(camera_matrix, (0, 0, 0, 1))
      = Col 3 of matrix
      = (-1, 0, 0, 0)
```

**After 90° rotation via rotate_w(PI/2), ana() returns (-1, 0, 0, 0) instead of (0, 0, 0, 1)**

This is a dramatic change! The W-axis movement should now go in the -X direction.

### 6. Verification Through Code

The rotor-to-matrix conversion in `rotor4.rs` and the SkipY transformation in `mat4.rs` are mathematically correct. The above trace confirms that:

1. `rotate_w()` creates an XZ plane rotation
2. SkipY transforms this to XW plane
3. `ana()` returns the correctly rotated W direction

### 7. Why Might It Not Work in Practice?

Possible issues outside the mathematical transformation:

1. **Rotor normalization** - accumulated floating point errors might reduce rotation magnitude
2. **Small rotation angles** - user might not be rotating enough to notice
3. **Projection in movement code** - `ana_xzw` projection might cancel out certain components

Looking at main.rs:324:
```rust
let ana_xzw = Vec4::new(camera_ana.x, 0.0, camera_ana.z, camera_ana.w).normalized();
```

For `ana() = (-1, 0, 0, 0)` after 90° rotation:
- `ana_xzw = (-1, 0, 0, 0).normalized() = (-1, 0, 0, 0)`
- This has X=-1, W=0 - so movement would go in -X direction!

## Conclusion

**The SkipY transformation is correct.**

After `rotate_w(PI/2)`:
- `ana()` should return approximately `(-1, 0, 0, 0)` or `(1, 0, 0, 0)`
- W-axis movement should go in ±X direction, not W direction
- This IS a significant change that should be visible in the titlebar

**If the user doesn't see this change, the rotor might not be accumulating correctly, or the rotation input is too small.**

## Recommendation

Add debug output to verify:
1. What `rotation_4d` rotor contains after user rotates
2. What `camera.ana()` returns
3. What `ana_xzw` becomes after projection
