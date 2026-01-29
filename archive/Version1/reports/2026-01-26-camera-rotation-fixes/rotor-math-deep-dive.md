# Technical Deep Dive: 4D Rotor Sandwich Product

**Date:** 2026-01-26

This document explains the mathematics behind the rotor rotation fix.

---

## Background: Geometric Algebra Rotations

In 4D geometric algebra, rotations are represented by **rotors** - even-grade multivectors with 8 components:

```
R = s + b₁₂e₁₂ + b₁₃e₁₃ + b₁₄e₁₄ + b₂₃e₂₃ + b₂₄e₂₄ + b₃₄e₃₄ + pe₁₂₃₄
```

Where:
- `s` is the scalar component
- `bᵢⱼ` are bivector components (6 total, one per rotation plane)
- `p` is the pseudoscalar component (e₁₂₃₄)

To rotate a vector `v`, we use the **sandwich product**:

```
v' = R v R̃
```

Where `R̃` is the **reverse** of R (bivectors negated, scalar and pseudoscalar unchanged).

---

## The Bug: Incorrect Explicit Formula

The original implementation tried to compute `R v R̃` using an explicit matrix formula:

```rust
// INCORRECT - this formula has errors for composed rotors
let new_x = x * (s² - b₁₂² - b₁₃² - b₁₄² + b₂₃² + b₂₄² + b₃₄² - p²)
    + 2.0 * y * (s*b₁₂ + b₁₃*b₂₃ + b₁₄*b₂₄ + b₃₄*p)
    + 2.0 * z * (s*b₁₃ - b₁₂*b₂₃ + b₁₄*b₃₄ - b₂₄*p)
    + 2.0 * w * (s*b₁₄ - b₁₂*b₂₄ - b₁₃*b₃₄ + b₂₃*p);
```

This formula was derived assuming certain simplifications that only hold for **simple rotors** (single plane rotations). When multiple bivector components are non-zero (as happens when composing rotations), the formula produces incorrect results.

### Evidence of the Bug

```rust
// Compose XZ (yaw 90°) and YZ (pitch 90°) rotations
let r_xz = Rotor4::from_plane_angle(RotationPlane::XZ, π/2);
let r_yz = Rotor4::from_plane_angle(RotationPlane::YZ, π/2);
let composed = r_yz.compose(&r_xz);

// Sequential application (correct):
// X → (via XZ) → Z → (via YZ) → -Y
// Result: (0, -1, 0, 0)

// Composed rotor with old formula gave:
// Result: (0, 0, 1, 0)  // WRONG!

// The bug also broke orthogonality:
// X.Y should be 0, but was 0.5
```

---

## The Fix: Step-by-Step Sandwich Product

The correct approach computes the sandwich product in two steps:

### Step 1: Compute R * v

When a rotor R multiplies a vector v, the result has both **vector** and **trivector** components:

```
R * v = (vector part) + (trivector part)
```

**Vector part coefficients:**
```rust
rv_e1 = s*vx + b₁₂*vy + b₁₃*vz + b₁₄*vw
rv_e2 = s*vy - b₁₂*vx + b₂₃*vz + b₂₄*vw
rv_e3 = s*vz - b₁₃*vx - b₂₃*vy + b₃₄*vw
rv_e4 = s*vw - b₁₄*vx - b₂₄*vy - b₃₄*vz
```

**Trivector part coefficients:**
```rust
rv_e123 = b₁₂*vz - b₁₃*vy + b₂₃*vx + p*vw
rv_e124 = b₁₂*vw - b₁₄*vy + b₂₄*vx - p*vz
rv_e134 = b₁₃*vw - b₁₄*vz + b₃₄*vx + p*vy
rv_e234 = b₂₃*vw - b₂₄*vz + b₃₄*vy - p*vx
```

### Step 2: Compute (R*v) * R̃

Now multiply the intermediate result by the reverse rotor. The reverse is:

```
R̃ = s - b₁₂e₁₂ - b₁₃e₁₃ - b₁₄e₁₄ - b₂₃e₂₃ - b₂₄e₂₄ - b₃₄e₃₄ + pe₁₂₃₄
```

The key insight is that we only need the **vector part** of the final result. This comes from:
- vector * scalar → vector
- vector * bivector → vector (and trivector, which we ignore)
- trivector * bivector → vector (and other grades)
- trivector * pseudoscalar → vector

**Final vector coefficients:**

```rust
// e₁ coefficient (new x)
new_x = rv_e1*s + rv_e2*b₁₂ + rv_e3*b₁₃ + rv_e4*b₁₄
      + rv_e123*b₂₃ + rv_e124*b₂₄ + rv_e134*b₃₄ - rv_e234*p

// e₂ coefficient (new y)
new_y = rv_e2*s - rv_e1*b₁₂ + rv_e3*b₂₃ + rv_e4*b₂₄
      - rv_e123*b₁₃ - rv_e124*b₁₄ + rv_e234*b₃₄ + rv_e134*p

// e₃ coefficient (new z)
new_z = rv_e3*s - rv_e1*b₁₃ - rv_e2*b₂₃ + rv_e4*b₃₄
      + rv_e123*b₁₂ - rv_e134*b₁₄ - rv_e234*b₂₄ - rv_e124*p

// e₄ coefficient (new w)
new_w = rv_e4*s - rv_e1*b₁₄ - rv_e2*b₂₄ - rv_e3*b₃₄
      + rv_e124*b₁₂ + rv_e134*b₁₃ + rv_e234*b₂₃ + rv_e123*p
```

---

## Why the Trivector Terms Matter

The bug in the old formula came from ignoring or incorrectly handling the trivector terms. Consider:

1. When R has only one non-zero bivector (simple rotation), the trivector part of R*v is small
2. When R has multiple bivector components (composed rotation), the trivector part is significant
3. The trivector terms interact with bivectors in R̃ to produce vector contributions

The explicit matrix formula tried to combine these into a single expression but got the cross-terms wrong.

---

## Verification

After the fix, all these properties hold:

1. **Length preservation**: `|R v R̃| = |v|`
2. **Orthogonality preservation**: If `u ⊥ v`, then `(R u R̃) ⊥ (R v R̃)`
3. **Composition correctness**: `R₂(R₁ v R̃₁)R̃₂ = (R₂R₁) v (R₂R₁)̃`
4. **Identity**: `I v Ĩ = v` where I is the identity rotor

---

## Performance Consideration

The step-by-step approach requires more operations than a correct explicit formula would:
- Step 1: ~32 multiplications, ~24 additions
- Step 2: ~32 multiplications, ~28 additions

A correct explicit formula could potentially be more efficient, but deriving it correctly for the full 4D case with pseudoscalar is complex and error-prone. The step-by-step approach is:
- Easier to verify
- Matches the mathematical definition directly
- Still fast enough for real-time use

---

## References

- Geometric Algebra for Computer Science (Dorst, Fontijne, Mann)
- 4D Rotors in Geometric Algebra
- engine4d source code (for behavioral reference)
