# Engine4D Agent Report: Cross-Section Slicing Analysis

**Agent**: Engine4D Agent
**Date**: 2026-01-26
**Task**: Analyze how engine4d (HackerPoet/Engine4D) implements 4D cross-section rendering

---

## Executive Summary

Engine4D uses a **fundamentally different architecture** than Rust4D for 4D slicing:

1. **GPU Vertex Shader Slicing** vs Rust4D's Compute Shader Approach
2. **Texture-Based Lookup Tables** (8x8 for 4D, 128x64 for 5D)
3. **Per-Vertex Triangulation** instead of per-simplex
4. **Tetrahedra (4 vertices)** vs Rust4D's 5-cells (5 vertices)
5. **Precomputed Winding Orientation** in the LUT generation

The key insight is that Engine4D's LUT encodes not just WHICH edge to sample, but also handles orientation via a sophisticated flip detection algorithm during LUT generation.

---

## Repository Structure

**Source**: [HackerPoet/Engine4D](https://github.com/HackerPoet/Engine4D)

Key files analyzed:
- `Assets/ShadersND/Core4D.cginc` - Vertex shader with slicing logic
- `Assets/ShadersND/CoreND.cginc` - Common shader infrastructure
- `Assets/Editor/GenerateSliceLUT.cs` - LUT generation algorithm
- `Assets/Scripts/Transform4D.cs` - 4D math utilities

---

## Core Architecture Differences

### 1. Simplex Type: Tetrahedron vs 5-Cell

**Engine4D**: Uses **tetrahedra (4 vertices)** as the fundamental 4D primitive.
```csharp
float4 simplex[4] = { v.va, v.vb, v.vc, v.vd };
```

**Rust4D**: Uses **5-cells (5 vertices)**, producing more complex cross-sections:
- 5-cell sliced = up to 6 intersection points (triangular prism)
- Tetrahedron sliced = up to 3 intersection points (triangle)

This is a **critical difference**: Engine4D's cross-sections are always triangles (or degenerate), while Rust4D must handle both tetrahedra (4 points) and prisms (6 points).

### 2. Lookup Table Structure

**Engine4D 4D LUT**: 8x8 texture (64 pixels)
```csharp
const int TEX_WIDTH = 8;   // 4 vertex indices + 4 sign combinations for vertex 0
const int TEX_HEIGHT = 8;  // 2^3 combinations for vertices 1,2,3
```

Each pixel encodes TWO vertex indices (in R and G channels) that define an edge to interpolate along.

```hlsl
// X encodes: (vertexID % 4) + (simplex[0].w > 0 ? 4 : 0)
// Y encodes: vertex1_above + vertex2_above*2 + vertex3_above*4
float4 lookup = tex2Dlod(_LUT, float4((x + 0.5)/8.0, (y + 0.5)/8.0, 0.0, 0.0));
uint ix1 = (uint)(lookup.r * 4.0);
uint ix2 = (uint)(lookup.g * 4.0);
```

**Rust4D**: Uses arrays stored as GPU buffers:
- `EDGE_TABLE[32]` - Which edges are crossed (bitmask)
- `TRI_TABLE[32][24]` - How to triangulate (8 triangles max)
- `EDGES[10][2]` - Edge vertex definitions

### 3. Per-Vertex vs Per-Simplex Processing

**Engine4D**: Each vertex shader invocation processes ONE OUTPUT VERTEX.
```hlsl
v2f vert(vin v) {
    // v.vertexID determines which of the 4 output vertices (0,1,2,3) this is
    float x = (v.vertexID % 4) + (simplex[0].w > 0.0 ? 4 : 0);
    // ...
    // Output is ONE interpolated vertex on the cross-section edge
    o.vertex.xyz = v1.xyz * v2.w + v2.xyz * v1.w;
}
```

The GPU draws with 4 vertices per tetrahedron. The vertex shader converts each into a cross-section point. Triangulation happens implicitly through the index buffer.

**Rust4D**: Compute shader processes ONE SIMPLEX, outputs MULTIPLE triangles atomically.
```wgsl
@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    // Process entire simplex, output up to 8 triangles
}
```

### 4. Winding/Orientation Handling

**Engine4D**: Orientation is PRECOMPUTED in the LUT generation via a sophisticated algorithm:

```csharp
private static bool LUT4D(bool a, bool b, bool c, bool d, int ix, int[] vix, out bool slice) {
    // ...
    // Compute orientation using cross product test:
    Vector3 va = (v[ia[0]] - v[ia[1]]) * (verts[ia[0]] ? 1.0f : -1.0f);
    Vector3 vb = (v[ib[0]] - v[ib[1]]) * (verts[ib[0]] ? 1.0f : -1.0f);
    Vector3 vc = (v[ic[0]] - v[ic[1]]) * (verts[ic[0]] ? 1.0f : -1.0f);
    bool needsFlip = Vector3.Dot(va, Vector3.Cross(vb, vc)) < 0.0f;
    // ...
    return needsFlip;
}
```

The LUT generation considers geometric orientation and encodes it implicitly by choosing which vertex index goes in which channel.

**Rust4D**: Orientation is computed AT RUNTIME by comparing triangle normal to simplex centroid:

```wgsl
// Compute simplex centroid
let simplex_centroid = (transformed[0].xyz + ... + transformed[4].xyz) / 5.0;

// Check if normal points toward simplex interior
let to_centroid = simplex_centroid - tri_center;
if (dot(normal, to_centroid) > 0.0) {
    // Flip winding
    let temp = v1;
    v1 = v2;
    v2 = temp;
    normal = -normal;
}
```

---

## Key Algorithm: GenerateSliceLUT.cs

This is the heart of Engine4D's approach. Here's the detailed algorithm:

### LUT4D Generation

```csharp
private static bool LUT4D(bool a, bool b, bool c, bool d, int ix, int[] vix, out bool slice) {
    // Bit-masks for vertices
    const int A = 0x01, B = 0x02, C = 0x04, D = 0x08;

    // Default to sliced (not rendered)
    vix[0] = 0; vix[1] = 0; slice = true;

    // Create edge crossing table
    bool[] lines = new bool[] {
        a != b, a != c, a != d,  // Edges from A
        b != c, b != d,          // Edges from B
        c != d,                  // Edge from C
    };
    int[] mask = new int[] {
        A|B, A|C, A|D, B|C, B|D, C|D
    };

    // Reference geometry for orientation test
    Vector3[] v = new Vector3[] {
        new Vector3(-1, -1, -1),
        new Vector3(-1, 1, 1),
        new Vector3(1, -1, 1),
        new Vector3(1, 1, -1),
    };

    // Find valid edge chains and compute orientation
    for (int i = 0; i < lines.Length; ++i) {
        if (!lines[i]) continue;
        // ... nested loops to find connected edges
        // Compute cross product for orientation
        bool needsFlip = Vector3.Dot(va, Vector3.Cross(vb, vc)) < 0.0f;
        // Return flip status for this case
    }
}
```

### Key Points:

1. **6 edges** for a tetrahedron (not 10 like Rust4D's 5-cell)
2. **Chain connectivity** - edges must share exactly 1 vertex to be adjacent
3. **Orientation via cross product** of edge direction vectors
4. **ix parameter (0-3)** selects which of the up-to-4 output vertices this LUT entry describes
5. **flipT logic** handles symmetric cases

---

## 4D Normal Computation (Transform4D.cs)

Engine4D computes 4D normals via the 4D cross product (wedge product):

```csharp
public static Vector4 MakeNormal(Vector4 a, Vector4 b, Vector4 c) {
    return new Vector4(
       -Vector3.Dot(YZW(a), Vector3.Cross(YZW(b), YZW(c))),
        Vector3.Dot(ZWX(a), Vector3.Cross(ZWX(b), ZWX(c))),
       -Vector3.Dot(WXY(a), Vector3.Cross(WXY(b), WXY(c))),
        Vector3.Dot(XYZ(a), Vector3.Cross(XYZ(b), XYZ(c))));
}
```

This is the standard 4D cross product formula using cofactor expansion.

---

## Shader Slicing Logic (Core4D.cginc)

The actual slicing in the vertex shader:

```hlsl
v2f vert(vin v) {
    // Transform simplex vertices
    float4 simplex[4] = {
        mul(_ModelMatrix, v.va) + _ModelPosition,
        mul(_ModelMatrix, v.vb) + _ModelPosition,
        mul(_ModelMatrix, v.vc) + _ModelPosition,
        mul(_ModelMatrix, v.vd) + _ModelPosition,
    };

    // Apply camera transform
    ModelToCam(simplex[0]);
    ModelToCam(simplex[1]);
    ModelToCam(simplex[2]);
    ModelToCam(simplex[3]);

    // Compute LUT coordinates
    float x = (v.vertexID % 4) + (simplex[0].w > 0.0 ? 4 : 0);
    float y = (simplex[1].w > 0.0 ? 1 : 0) +
              (simplex[2].w > 0.0 ? 2 : 0) +
              (simplex[3].w > 0.0 ? 4 : 0);

    // Lookup edge vertices
    float4 lookup = tex2Dlod(_LUT, float4((x+0.5)/8.0, (y+0.5)/8.0, 0.0, 0.0));
    uint ix1 = (uint)(lookup.r * 4.0);
    uint ix2 = (uint)(lookup.g * 4.0);

    // Get endpoints
    float4 v1 = simplex[ix1];
    float4 v2 = simplex[ix2];

    // Interpolate (note: uses saturate to clamp t to [0,1])
    v1.w = saturate(v1.w / (v1.w - v2.w));
    v2.w = 1.0 - v1.w;

    // Final position
    o.vertex.xyz = v1.xyz * v2.w + v2.xyz * v1.w;
}
```

Key observations:
1. **saturate()** clamps interpolation factor - degenerate cases get clamped to endpoints
2. Uses **v1.w** as the interpolation factor directly (overwriting w coordinate)
3. The interpolation weights are `v1.w` and `v2.w = 1 - v1.w`

---

## Comparison with Rust4D

| Aspect | Engine4D | Rust4D |
|--------|----------|--------|
| **Primitive** | Tetrahedron (4 vertices) | 5-cell (5 vertices) |
| **Max intersection points** | 3 (triangle) | 6 (triangular prism) |
| **Slicing location** | Vertex shader | Compute shader |
| **LUT storage** | Texture (8x8) | Buffer arrays |
| **Triangulation** | Implicit (index buffer) | Explicit (TRI_TABLE) |
| **Orientation** | Precomputed in LUT | Runtime centroid test |
| **Output per primitive** | Always 1 triangle | 4-8 triangles |

---

## Why Rust4D Has Bugs

Based on this analysis, the key issues in Rust4D's approach:

### 1. TRI_TABLE Point Ordering Assumption

Rust4D's `TRI_TABLE` assumes a fixed point ordering for prism triangulation. But different cases produce points in different orders based on edge indices. The shader's point reordering logic attempts to fix this but may have edge cases.

**Engine4D avoids this entirely** by using tetrahedra (never produces prisms) and precomputing orientation in the LUT.

### 2. Runtime vs Precomputed Orientation

Rust4D computes orientation at runtime using simplex centroid. This can fail when:
- Centroid is coplanar with the triangle
- Numerical precision issues near the slice plane
- Degenerate simplices

**Engine4D precomputes** correct orientation during LUT generation using exact geometric tests.

### 3. 5-Cell Complexity

Using 5-cells instead of tetrahedra significantly increases complexity:
- 10 edges vs 6 edges
- 32 cases vs 16 cases
- Prism triangulation (8 triangles) vs single triangle
- Complex point ordering requirements

---

## Recommendations for Rust4D

### Short Term (Fix Current Approach)

1. **Verify TRI_TABLE** against all 20 prism cases manually
2. **Improve point ordering** logic in shader - ensure correct pairing by tracking above/below vertex relationships
3. **Use geometric orientation test** similar to Engine4D's cross product method instead of centroid

### Long Term (Architectural Change)

Consider switching to **tetrahedra decomposition**:
- A 5-cell can be decomposed into 5 tetrahedra
- Each tetrahedra produces at most 1 triangle
- Simplifies slicing logic significantly
- Matches Engine4D's proven approach

Alternatively, implement **dynamic triangulation** instead of lookup tables:
- For prism cases, sort points by their connectivity
- Build triangulation based on actual point relationships
- More flexible but potentially slower

---

## Appendix: Full LUT4D Function

```csharp
private static bool LUT4D(bool a, bool b, bool c, bool d, int ix, int[] vix, out bool slice) {
    const int A = 0x01, B = 0x02, C = 0x04, D = 0x08;

    vix[0] = 0; vix[1] = 0; slice = true;
    if (ix >= 4) { return false; }

    bool[] verts = new bool[] { a, b, c, d };
    bool[] lines = new bool[] { a!=b, a!=c, a!=d, b!=c, b!=d, c!=d };
    int[] mask = new int[] { A|B, A|C, A|D, B|C, B|D, C|D };

    Vector3[] v = new Vector3[] {
        new Vector3(-1, -1, -1),
        new Vector3(-1, 1, 1),
        new Vector3(1, -1, 1),
        new Vector3(1, 1, -1),
    };

    for (int i = 0; i < lines.Length; ++i) {
        if (!lines[i]) continue;
        if (ix == 0) { MaskToVIX(mask[i], vix); }
        for (int j = i + 1; j < lines.Length; ++j) {
            if (!lines[j]) continue;
            if (BitCount(mask[i] & mask[j]) != 1) continue;
            if (ix == 1) { MaskToVIX(mask[j], vix); }
            for (int k = i + 1; k < lines.Length; ++k) {
                if (!lines[k] || k == j) continue;
                if (BitCount(mask[j] & mask[k]) != 1) continue;
                if (ix >= 2) { MaskToVIX(mask[k], vix); }

                int[] ia = new int[2]; MaskToVIX(mask[i], ia);
                int[] ib = new int[2]; MaskToVIX(mask[j], ib);
                int[] ic = new int[2]; MaskToVIX(mask[k], ic);

                Vector3 va = (v[ia[0]] - v[ia[1]]) * (verts[ia[0]] ? 1f : -1f);
                Vector3 vb = (v[ib[0]] - v[ib[1]]) * (verts[ib[0]] ? 1f : -1f);
                Vector3 vc = (v[ic[0]] - v[ic[1]]) * (verts[ic[0]] ? 1f : -1f);
                bool needsFlip = Vector3.Dot(va, Vector3.Cross(vb, vc)) < 0.0f;

                if (BitCount(mask[i] & mask[k]) == 1) {
                    slice = (ix >= 3);
                    return needsFlip;
                }
                for (int p = i + 1; p < lines.Length; ++p) {
                    if (!lines[p] || p == j || p == k) continue;
                    if (BitCount(mask[k] & mask[p]) != 1) continue;
                    if (ix >= 3) { MaskToVIX(mask[p], vix); }
                    if (BitCount(mask[i] & mask[p]) == 1) {
                        slice = (ix >= 4);
                        return !needsFlip;
                    }
                    return false;
                }
            }
        }
    }
    return false;
}
```

---

## Sources

- [HackerPoet/Engine4D GitHub Repository](https://github.com/HackerPoet/Engine4D)
- Engine4D is the engine behind "4D Golf" by CodeParade
