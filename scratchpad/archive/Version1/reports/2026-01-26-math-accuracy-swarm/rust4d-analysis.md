# Rust4D Mathematical Analysis Report

**Agent:** Rust4D Analysis Agent
**Date:** 2026-01-26
**Focus:** Mathematical accuracy of 4D rendering pipeline

---

## 1. Hyperplane Slicing

### 1.1 Hyperplane Equation

The slicing hyperplane is defined as a **constant-W hyperplane**:

$$\Pi: \quad w = w_{\text{slice}}$$

This is an axis-aligned hyperplane perpendicular to the W-axis. In the code (`slice.wgsl`):

```wgsl
let slice_w = params.slice_w;
```

The hyperplane has implicit equation:

$$\Pi: \quad w - w_{\text{slice}} = 0$$

**Note:** This is a simple case. A general hyperplane in 4D would be:

$$ax + by + cz + dw + e = 0$$

Rust4D uses the simpler axis-aligned form for performance and intuitive "depth" slicing.

### 1.2 Vertex Classification

A vertex $\mathbf{v} = (x, y, z, w)$ is classified as:

- **Above** the hyperplane if $w > w_{\text{slice}}$
- **Below** (or on) the hyperplane if $w \leq w_{\text{slice}}$

From `slice.wgsl` (lines 231-236):

```wgsl
var case_idx: u32 = 0u;
if (transformed[0].w > slice_w) { case_idx |= 1u; }
if (transformed[1].w > slice_w) { case_idx |= 2u; }
if (transformed[2].w > slice_w) { case_idx |= 4u; }
if (transformed[3].w > slice_w) { case_idx |= 8u; }
if (transformed[4].w > slice_w) { case_idx |= 16u; }
```

**Mathematical interpretation:** The case index is a 5-bit binary number where bit $i$ is set iff vertex $i$ is above the slice plane:

$$\text{case\_idx} = \sum_{i=0}^{4} \mathbb{1}[w_i > w_{\text{slice}}] \cdot 2^i$$

### 1.3 Edge Intersection Formula

For an edge connecting vertices $\mathbf{p}_0 = (x_0, y_0, z_0, w_0)$ and $\mathbf{p}_1 = (x_1, y_1, z_1, w_1)$, the intersection with the slice plane is computed via linear interpolation.

**Parameterization:** The edge is parameterized as:

$$\mathbf{p}(t) = (1-t)\mathbf{p}_0 + t\mathbf{p}_1, \quad t \in [0, 1]$$

**Intersection condition:** Find $t$ such that $w(t) = w_{\text{slice}}$:

$$w(t) = (1-t)w_0 + t \cdot w_1 = w_{\text{slice}}$$

**Solving for $t$:**

$$t = \frac{w_{\text{slice}} - w_0}{w_1 - w_0}$$

From `slice.wgsl` (lines 139-147):

```wgsl
let w0 = p0.w;
let w1 = p1.w;
let dw = w1 - w0;
// Protect against division by zero when edge is parallel to slice plane
let t = select((slice_w - w0) / dw, 0.5, abs(dw) < 0.0001);

// Interpolate position
let pos = mix(p0, p1, t);
```

**Edge case handling:** When $|w_1 - w_0| < 0.0001$ (edge nearly parallel to slice plane), $t = 0.5$ is used as a fallback. This is a reasonable approximation for degenerate cases.

**Final intersection point:**

$$\mathbf{p}_{\text{intersect}} = \left( x_0 + t(x_1 - x_0), \; y_0 + t(y_1 - y_0), \; z_0 + t(z_1 - z_0), \; w_{\text{slice}} \right)$$

Only the $(x, y, z)$ components are used for the 3D output vertex.

---

## 2. Simplex Processing

### 2.1 5-Cell (Pentatope) Structure

A 5-cell is the 4D analogue of a tetrahedron. It has:
- **5 vertices** (indexed 0-4)
- **10 edges** (all pairs of vertices: $\binom{5}{2} = 10$)
- **10 triangular faces** ($\binom{5}{3} = 10$)
- **5 tetrahedral cells** ($\binom{5}{4} = 5$)

### 2.2 Case Index Computation

The case index is a 5-bit number encoding which vertices are above the slice plane:

| Bit | Vertex | Weight |
|-----|--------|--------|
| 0   | v0     | $2^0 = 1$ |
| 1   | v1     | $2^1 = 2$ |
| 2   | v2     | $2^2 = 4$ |
| 3   | v3     | $2^3 = 8$ |
| 4   | v4     | $2^4 = 16$ |

**Total cases:** $2^5 = 32$ (indices 0-31)

**Case symmetry:**
- Case 0 (all below) and Case 31 (all above): No intersection
- Cases with same popcount (number of vertices above) produce similar geometry

### 2.3 Edge Crossing Determination

An edge is "crossed" (intersects the slice plane) if and only if its endpoints are on opposite sides:

$$\text{edge crossed} \iff (v_a \text{ above}) \oplus (v_b \text{ above})$$

From `lookup_tables.rs` (lines 53-66):

```rust
let v0_above = (case_idx >> v0) & 1;
let v1_above = (case_idx >> v1) & 1;

// Edge is crossed if vertices are on opposite sides
if v0_above != v1_above {
    edge_mask |= 1 << edge_idx;
}
```

### 2.4 Cross-Section Geometry by Case

| Vertices Above | Edges Crossed | Cross-Section Shape | Triangles |
|----------------|---------------|---------------------|-----------|
| 0 or 5         | 0             | Empty               | 0         |
| 1 or 4         | 4             | Tetrahedron         | 4         |
| 2 or 3         | 6             | Triangular Prism    | 8         |

**Tetrahedron cases (4 points):**
- 1 vertex above: 4 edges from that vertex cross (cases 1, 2, 4, 8, 16)
- 4 vertices above: 4 edges to the 1 vertex below cross (cases 15, 23, 27, 29, 30)

**Prism cases (6 points):**
- 2 vertices above: 6 edges cross (cases 3, 5, 6, 9, 10, 12, 17, 18, 20, 24)
- 3 vertices above: 6 edges cross (cases 7, 11, 13, 14, 19, 21, 22, 25, 26, 28)

---

## 3. Normal Calculation

### 3.1 Triangle Normal Formula

For a triangle with vertices $\mathbf{p}_0$, $\mathbf{p}_1$, $\mathbf{p}_2$, the normal is computed using the cross product of edge vectors.

**Edge vectors:**

$$\mathbf{e}_1 = \mathbf{p}_1 - \mathbf{p}_0$$
$$\mathbf{e}_2 = \mathbf{p}_2 - \mathbf{p}_0$$

**Normal (unnormalized):**

$$\mathbf{n} = \mathbf{e}_1 \times \mathbf{e}_2 = \begin{vmatrix} \hat{i} & \hat{j} & \hat{k} \\ e_{1x} & e_{1y} & e_{1z} \\ e_{2x} & e_{2y} & e_{2z} \end{vmatrix}$$

**Expanded:**

$$\mathbf{n} = \left( e_{1y} \cdot e_{2z} - e_{1z} \cdot e_{2y}, \; e_{1z} \cdot e_{2x} - e_{1x} \cdot e_{2z}, \; e_{1x} \cdot e_{2y} - e_{1y} \cdot e_{2x} \right)$$

**Normalized normal:**

$$\hat{\mathbf{n}} = \frac{\mathbf{n}}{|\mathbf{n}|}$$

From `slice.wgsl` (lines 187-191):

```wgsl
fn compute_normal(p0: vec3<f32>, p1: vec3<f32>, p2: vec3<f32>) -> vec3<f32> {
    let e1 = p1 - p0;
    let e2 = p2 - p0;
    return normalize(cross(e1, e2));
}
```

### 3.2 Orientation Check

The code ensures normals face **outward** from the origin (where the tesseract is centered).

**Reference point:** Origin $(0, 0, 0)$

**Triangle centroid:**

$$\mathbf{c} = \frac{\mathbf{p}_0 + \mathbf{p}_1 + \mathbf{p}_2}{3}$$

**Orientation test:** If $\hat{\mathbf{n}} \cdot \mathbf{c} < 0$, the normal points toward the origin (inward), so it must be flipped.

From `slice.wgsl` (lines 429-441):

```wgsl
let tri_center = (p0 + p1 + p2) / 3.0;
// If normal points opposite to tri_center direction, it points inward - flip it
// DEBUG: Skip flipping for prisms to test if that's causing issues
if (!is_prism && dot(normal, tri_center) < 0.0) {
    // Normal points toward origin (inward), flip to point outward
    let temp = v1;
    v1 = v2;
    v2 = temp;
    normal = -normal;
}
```

**Important note:** The code currently has a debug condition `!is_prism` that skips orientation fixing for prism cases. This may be a bug being investigated.

### 3.3 Winding Order Adjustment

When the normal is flipped, the vertex winding order is also swapped ($v_1 \leftrightarrow v_2$) to maintain consistency between normal direction and winding order (right-hand rule).

---

## 4. Lookup Table Structure

### 4.1 Edge Numbering

The 10 edges of a 5-cell are numbered based on vertex pair ordering:

| Edge Index | Vertices | Description |
|------------|----------|-------------|
| 0 | (0, 1) | First edge from v0 |
| 1 | (0, 2) | Second edge from v0 |
| 2 | (0, 3) | Third edge from v0 |
| 3 | (0, 4) | Fourth edge from v0 |
| 4 | (1, 2) | First edge from v1 (not to v0) |
| 5 | (1, 3) | Second edge from v1 |
| 6 | (1, 4) | Third edge from v1 |
| 7 | (2, 3) | First edge from v2 (not to v0,v1) |
| 8 | (2, 4) | Second edge from v2 |
| 9 | (3, 4) | Edge between v3 and v4 |

From `lookup_tables.rs` (lines 14-25):

```rust
pub const EDGES: [[usize; 2]; 10] = [
    [0, 1], // Edge 0
    [0, 2], // Edge 1
    [0, 3], // Edge 2
    [0, 4], // Edge 3
    [1, 2], // Edge 4
    [1, 3], // Edge 5
    [1, 4], // Edge 6
    [2, 3], // Edge 7
    [2, 4], // Edge 8
    [3, 4], // Edge 9
];
```

### 4.2 EDGE_TABLE Format

`EDGE_TABLE[case_idx]` is a 16-bit bitmask where bit $i$ is set if edge $i$ is crossed:

$$\text{EDGE\_TABLE}[\text{case}] = \sum_{i=0}^{9} \mathbb{1}[\text{edge } i \text{ crossed}] \cdot 2^i$$

**Example:** Case 1 (only v0 above):
- Edges 0, 1, 2, 3 connect v0 to other vertices
- All four are crossed
- `EDGE_TABLE[1] = 0b0000001111 = 15`

### 4.3 TRI_TABLE Format

`TRI_TABLE[case_idx]` is a 24-element array of signed bytes:
- Every 3 consecutive indices form a triangle
- Indices reference intersection points (0-based, in edge order)
- Value `-1` indicates end of triangle list

**Tetrahedron triangulation (4 points, 4 triangles):**

```rust
let tetra_4pts: [i8; 24] = [
    0, 1, 2,  // face 0
    0, 2, 3,  // face 1
    0, 3, 1,  // face 2
    1, 3, 2,  // face 3
    -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1  // unused
];
```

**Prism triangulation (6 points, 8 triangles):**

```rust
let prism_6pts: [i8; 24] = [
    0, 2, 4,  // cap A
    1, 5, 3,  // cap B (opposite winding)
    0, 2, 3,  // side 1a
    0, 3, 1,  // side 1b
    2, 4, 5,  // side 2a
    2, 5, 3,  // side 2b
    4, 0, 1,  // side 3a
    4, 1, 5,  // side 3b
];
```

### 4.4 Winding Convention

The winding convention follows the **right-hand rule**:
- Vertices are ordered counter-clockwise when viewed from the front (normal pointing toward viewer)
- Cross product $(\mathbf{p}_1 - \mathbf{p}_0) \times (\mathbf{p}_2 - \mathbf{p}_0)$ gives outward-pointing normal

For prism cases:
- Cap A (points 0, 2, 4) has one winding
- Cap B (points 1, 5, 3) has **reversed** winding to face the opposite direction

### 4.5 Point Sorting for Prisms

The shader performs special sorting for prism cases (6 intersection points) to ensure consistent indexing. Points are reordered based on their relationship to "above" and "below" vertices.

**For 2-above cases:**
- Points are grouped by which "below" vertex they connect to
- Within each group, points are ordered by "above" vertex index

**For 3-above cases:**
- Points are grouped by which "above" vertex they connect to
- Within each group, points are ordered by "below" vertex index

This ensures the TRI_TABLE indices work correctly regardless of which specific case is being processed.

---

## 5. Coordinate System

### 5.1 Axis Definitions

| Axis | Component | Range | Meaning |
|------|-----------|-------|---------|
| X | `v.x` | -1 to +1 (for unit tesseract) | First spatial dimension |
| Y | `v.y` | -1 to +1 | Second spatial dimension |
| Z | `v.z` | -1 to +1 | Third spatial dimension |
| W | `v.w` | -1 to +1 | Fourth spatial dimension (ana/kata) |

From `vec4.rs` (lines 6-14):

```rust
/// 4D Vector with x, y, z, w components
/// The w component represents the 4th spatial dimension (ana/kata)
#[repr(C)]
pub struct Vec4 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,
}
```

### 5.2 Handedness

The 4D coordinate system uses a **right-handed convention** extended to 4D:
- In the XYZ subspace, the standard right-hand rule applies
- W is the additional "perpendicular" dimension

From `rotor4.rs`, the rotation planes follow standard geometric algebra conventions:
- XY plane: rotation in the first two dimensions
- ZW plane: rotation in the last two dimensions (the "4D-specific" rotation)

### 5.3 Origin and Tesseract Centering

The tesseract is centered at the origin $(0, 0, 0, 0)$.

From `tesseract.rs` (lines 21-43):

```rust
let h = size * 0.5;

// All 16 vertices are combinations of +/-h for each coordinate
let vertices = [
    Vec4::new(-h, -h, -h, -h), // 0  = 0b0000
    // ...
    Vec4::new( h,  h,  h,  h), // 15 = 0b1111
];
```

The vertices span from $(-h, -h, -h, -h)$ to $(h, h, h, h)$, symmetric about the origin.

### 5.4 Vertex Binary Encoding

Tesseract vertices are indexed by binary encoding:

$$\text{vertex index} = b_x \cdot 1 + b_y \cdot 2 + b_z \cdot 4 + b_w \cdot 8$$

where $b_i = 0$ means coordinate $-h$, and $b_i = 1$ means coordinate $+h$.

| Index | Binary | Coordinates |
|-------|--------|-------------|
| 0 | 0000 | $(-h, -h, -h, -h)$ |
| 1 | 0001 | $(+h, -h, -h, -h)$ |
| 2 | 0010 | $(-h, +h, -h, -h)$ |
| 7 | 0111 | $(+h, +h, +h, -h)$ |
| 15 | 1111 | $(+h, +h, +h, +h)$ |

---

## 6. 4D Rotation Mathematics

### 6.1 Rotation Planes in 4D

In 4D, rotations occur in **planes**, not around axes. There are 6 rotation planes:

| Plane | Bivector | Affects Axes |
|-------|----------|--------------|
| XY | $e_{12}$ | X, Y |
| XZ | $e_{13}$ | X, Z |
| XW | $e_{14}$ | X, W |
| YZ | $e_{23}$ | Y, Z |
| YW | $e_{24}$ | Y, W |
| ZW | $e_{34}$ | Z, W |

### 6.2 Rotor Formula

A rotation by angle $\theta$ in plane $B$ is represented by the rotor:

$$R = \cos\left(\frac{\theta}{2}\right) - \sin\left(\frac{\theta}{2}\right) \cdot B$$

From `rotor4.rs` (lines 80-99):

```rust
pub fn from_plane_angle(plane: RotationPlane, angle: f32) -> Self {
    let half = angle * 0.5;
    let cos_h = half.cos();
    let sin_h = half.sin();

    let mut r = Self::IDENTITY;
    r.s = cos_h;

    match plane {
        RotationPlane::XY => r.b_xy = -sin_h,
        // ... etc
    }
    r
}
```

### 6.3 Sandwich Product

To rotate a vector $\mathbf{v}$:

$$\mathbf{v}' = R \cdot \mathbf{v} \cdot R^{\dagger}$$

where $R^{\dagger}$ is the reverse (conjugate) of the rotor.

The `rotate()` method implements this using explicit formulas derived from the geometric product. For a unit rotor with components $(s, b_{12}, b_{13}, b_{14}, b_{23}, b_{24}, b_{34}, p)$:

$$x' = x(s^2 - b_{12}^2 - b_{13}^2 - b_{14}^2 + b_{23}^2 + b_{24}^2 + b_{34}^2 - p^2) + \ldots$$

(Full formula in `rotor4.rs` lines 197-218)

---

## 7. Kuhn Triangulation for Tesseract

### 7.1 Simplex Decomposition

The tesseract is decomposed into 24 simplices (5-cells) using Kuhn triangulation.

**Key insight:** Each simplex corresponds to a permutation of the 4 dimensions.

For permutation $(\pi_0, \pi_1, \pi_2, \pi_3)$:
1. Start at vertex $(-, -, -, -)$ (index 0)
2. Flip dimension $\pi_0$ to get vertex $2^{\pi_0}$
3. Flip dimension $\pi_1$ to get vertex $2^{\pi_0} + 2^{\pi_1}$
4. Continue until reaching $(+, +, +, +)$ (index 15)

**Example:** Permutation $(0, 1, 2, 3)$:
- Vertex 0: index 0 (0000)
- Vertex 1: index 1 (0001) - flipped X
- Vertex 2: index 3 (0011) - flipped Y
- Vertex 3: index 7 (0111) - flipped Z
- Vertex 4: index 15 (1111) - flipped W

### 7.2 Properties

- **24 simplices** (one per permutation of 4 elements: $4! = 24$)
- All simplices share vertex 0 (index 0) and vertex 4 (index 15)
- Consecutive vertices in each simplex differ by exactly one bit
- Total simplex edges include both tesseract edges AND internal diagonals

---

## 8. Potential Issues Identified

### 8.1 Prism Normal Orientation

The code has a debug condition that skips normal flipping for prism cases:

```wgsl
if (!is_prism && dot(normal, tri_center) < 0.0) {
```

This may cause incorrect face orientations for prism cross-sections (2 or 3 vertices above/below the slice plane).

### 8.2 Edge Case: Parallel Edges

When an edge is nearly parallel to the slice plane ($|w_1 - w_0| < 0.0001$), the code uses $t = 0.5$. This could produce incorrect intersection points if the edge actually crosses the plane at a different parameter value.

### 8.3 Internal Triangles from Simplex Decomposition

The Kuhn triangulation creates internal edges (diagonals through the tesseract interior). When sliced, these produce intersection points that are NOT on the tesseract surface. The resulting "internal triangles" should cancel out between adjacent simplices, but this depends on correct winding order consistency.

---

## 9. Summary of Mathematical Formulas

| Component | Formula |
|-----------|---------|
| Hyperplane | $w = w_{\text{slice}}$ |
| Edge parameter | $t = \frac{w_{\text{slice}} - w_0}{w_1 - w_0}$ |
| Intersection point | $\mathbf{p}(t) = (1-t)\mathbf{p}_0 + t\mathbf{p}_1$ |
| Case index | $\sum_{i=0}^{4} \mathbb{1}[w_i > w_{\text{slice}}] \cdot 2^i$ |
| Triangle normal | $\hat{\mathbf{n}} = \frac{(\mathbf{p}_1 - \mathbf{p}_0) \times (\mathbf{p}_2 - \mathbf{p}_0)}{|(\mathbf{p}_1 - \mathbf{p}_0) \times (\mathbf{p}_2 - \mathbf{p}_0)|}$ |
| Orientation check | $\hat{\mathbf{n}} \cdot \mathbf{c} > 0$ for outward |
| Rotor | $R = \cos(\theta/2) - \sin(\theta/2) \cdot B$ |
| Rotation | $\mathbf{v}' = R \mathbf{v} R^{\dagger}$ |

---

*Report generated by Rust4D Analysis Agent*
