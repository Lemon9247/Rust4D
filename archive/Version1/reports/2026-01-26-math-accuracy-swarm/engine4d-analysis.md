# Engine4D Mathematical Analysis Report

**Agent**: Engine4D Analysis Agent
**Date**: 2026-01-26
**Repository**: https://github.com/HackerPoet/Engine4D
**Cloned to**: /home/lemoneater/Projects/Personal/Engine4D

## Executive Summary

Engine4D is a Unity-based 4D/5D game engine by HackerPoet (CodeParade), used in 4D Golf. This report provides a rigorous mathematical analysis of its 4D rendering pipeline, focusing on hyperplane slicing, lookup tables, and orientation determination.

---

## 1. Hyperplane Slicing

### 1.1 Hyperplane Equation

Engine4D uses a **W=0 hyperplane** in camera space for slicing:

```
Hyperplane: W = 0
```

The camera transformation converts world coordinates to camera-relative coordinates where `W` represents the perpendicular distance from the viewing hyperplane.

**From Core4D.cginc (lines 63-71):**
```hlsl
#if defined(IS_EDITOR)
#define ModelToCam(V) V = float4(mul(UNITY_MATRIX_V, float4(V.xyz, 1.0)).xyz, V.w - _EditorSliceW)
#elif defined(IS_EDITOR_V)
#define ModelToCam(V) V = float4(mul(UNITY_MATRIX_V, float4(V.x, -V.w, V.z, 1.0)).xyz, V.y - _EditorSliceW)
#else
#define ModelToCam(V) V = mul(_CamMatrix, V + _CamPosition)
#endif
```

In runtime mode, `_CamMatrix` is a 4x4 rotation matrix and `_CamPosition` is the camera's 4D position. The transformation:
```
V_cam = _CamMatrix * (V_world + _CamPosition)
```

### 1.2 Edge Intersection Formula

For a tetrahedron with vertices classified above/below the W=0 plane, edges crossing the plane are interpolated.

**From Core4D.cginc (lines 119-127):**
```hlsl
float4 v1 = simplex[ix1];
float4 v2 = simplex[ix2];

// Create interpolation factors and clamp during undefined behavior
v1.w = saturate(v1.w / (v1.w - v2.w));
v2.w = 1.0 - v1.w;

// Slice the edge for the final vertex position
o.vertex.xyz = v1.xyz * v2.w + v2.xyz * v1.w;
```

**Mathematical Formula:**

For edge from vertex A to vertex B where A.w and B.w have opposite signs:

```
t = A.w / (A.w - B.w)          // Parameter along edge
t = saturate(t)                 // Clamp to [0,1]
P_intersection = A * (1-t) + B * t
```

Equivalently:
```
P = A + t(B - A)  where  t = w_A / (w_A - w_B)
```

The `saturate()` function clamps edge cases to prevent NaN/Inf.

### 1.3 Camera Space Projection

After slicing, the 3D intersection point is projected using Unity's view matrix:

```hlsl
o.vertex = mul(UNITY_MATRIX_V, float4(o.vertex.x, o.vertex.y, -o.vertex.z, 1.0));
o.vertex = mul(UNITY_MATRIX_P, float4(o.vertex.xyz, 1.0));
```

Note the Z-negation (`-o.vertex.z`) which suggests a right-handed coordinate system conversion.

---

## 2. Simplex Processing

### 2.1 Primitive Type

Engine4D uses **tetrahedra (3-simplices)** as the fundamental 4D primitive.

**From Mesh4D.cs (line 157-192):**
```csharp
public void AddTetrahedron(Vector4 a, Vector4 b, Vector4 c, Vector4 d, PackedNormal p, uint ao) {
    vArray.Add(new Vertex4D(a, b, c, d, p, ao));
    vArray.Add(new Vertex4D(a, b, c, d, p, ao));
    vArray.Add(new Vertex4D(a, b, c, d, p, ao));
    vArray.Add(new Vertex4D(a, b, c, d, p, ao));
    vIndices[curSubMesh].Add(vArray.Count - 4);
    vIndices[curSubMesh].Add(vArray.Count - 3);
    vIndices[curSubMesh].Add(vArray.Count - 2);
    vIndices[curSubMesh].Add(vArray.Count - 4);
    vIndices[curSubMesh].Add(vArray.Count - 2);
    vIndices[curSubMesh].Add(vArray.Count - 1);
}
```

Each tetrahedron is stored as **4 identical vertex instances** with indices forming 2 triangles for rendering the cross-section.

### 2.2 Vertex Structure

**From Mesh4D.cs (lines 76-91):**
```csharp
public struct Vertex4D {
    public Vector4 va;           // First vertex of tetrahedron
    public PackedNormal normal;  // Packed 4D normals for each vertex
    public Vector4 vb;           // Second vertex
    public Vector4 vc;           // Third vertex
    public Vector4 vd;           // Fourth vertex
    public uint ao;              // Ambient occlusion per-vertex
}
```

The entire tetrahedron is stored per-vertex to allow GPU-side slicing.

### 2.3 Vertex Classification

Vertices are classified by their W coordinate sign:

**From Core4D.cginc (lines 114-115):**
```hlsl
float x = (v.vertexID % 4) + (simplex[0].w > 0.0 ? 4 : 0);
float y = (simplex[1].w > 0.0 ? 1 : 0) + (simplex[2].w > 0.0 ? 2 : 0) + (simplex[3].w > 0.0 ? 4 : 0);
```

**Classification:**
- Vertex i is "above" if `simplex[i].w > 0`
- Vertex i is "below" if `simplex[i].w <= 0`

### 2.4 Case Index Computation

The case index is a combination of:
- **X coordinate**: `vertex_id (0-3)` + `4 * (simplex[0].w > 0)`
- **Y coordinate**: Bitmask of `simplex[1..3].w > 0`

This creates an 8x8 lookup table (64 cases for 4 vertices with 16 sign combinations, times 4 output vertices per cross-section).

---

## 3. Lookup Table Structure

### 3.1 Table Dimensions

**4D LUT (8x8 texture):**
- X: 0-7 (3 bits for vertex_id 0-3, 1 bit for vertex 0 sign)
- Y: 0-7 (3 bits for vertices 1-3 signs)
- Output: RG channels encode two vertex indices (0-3)

**5D LUT (128x64 texture):**
- X: 7 bits (triangle_id 0-4 + edge crossing signs)
- Y: 6 bits (remaining edge crossing signs)
- Output: RGB channels encode three vertex indices (0-4)

### 3.2 Edge Numbering

**From GenerateSliceLUT.cs (lines 131-146):**
```csharp
bool[] lines = new bool[] {
    a != b, // Edge 0: A-B
    a != c, // Edge 1: A-C
    a != d, // Edge 2: A-D
    b != c, // Edge 3: B-C
    b != d, // Edge 4: B-D
    c != d, // Edge 5: C-D
};
int[] mask = new int[] {
    A | B, // Edge 0 connects vertices A,B
    A | C, // Edge 1 connects vertices A,C
    A | D, // Edge 2 connects vertices A,D
    B | C, // Edge 3 connects vertices B,C
    B | D, // Edge 4 connects vertices B,D
    C | D, // Edge 5 connects vertices C,D
};
```

**Edge indices:**
| Edge | Vertices | Bitmask |
|------|----------|---------|
| 0 | A-B | 0x03 |
| 1 | A-C | 0x05 |
| 2 | A-D | 0x09 |
| 3 | B-C | 0x06 |
| 4 | B-D | 0x0A |
| 5 | C-D | 0x0C |

### 3.3 Output Encoding

**From GenerateSliceLUT.cs (lines 35-36):**
```csharp
Color outColor = new Color((0.5f + vix[0]) / 4.0f, (0.5f + vix[1]) / 4.0f, 0.0f, 1.0f);
texture.SetPixel(x, y, outColor);
```

The LUT encodes vertex indices as normalized floats:
```
vertex_index = floor(channel_value * 4.0)
```

The `+ 0.5f` centers the value to avoid precision issues.

### 3.4 Winding Convention

Winding is determined during LUT generation using a **3D cross product test**.

**From GenerateSliceLUT.cs (lines 148-173):**
```csharp
Vector3[] v = new Vector3[] {
    new Vector3(-1, -1, -1),
    new Vector3(-1, 1, 1),
    new Vector3(1, -1, 1),
    new Vector3(1, 1, -1),
};

// ... vertex pair selection ...

Vector3 va = (v[ia[0]] - v[ia[1]]) * (verts[ia[0]] ? 1.0f : -1.0f);
Vector3 vb = (v[ib[0]] - v[ib[1]]) * (verts[ib[0]] ? 1.0f : -1.0f);
Vector3 vc = (v[ic[0]] - v[ic[1]]) * (verts[ic[0]] ? 1.0f : -1.0f);
bool needsFlip = Vector3.Dot(va, Vector3.Cross(vb, vc)) < 0.0f;
```

The algorithm:
1. Uses a reference tetrahedron with known vertex positions
2. Computes edge vectors based on which endpoint is "above"
3. Tests orientation using triple scalar product (determinant)
4. Flips vertex order if `needsFlip` is true

**If winding needs correction:**
```csharp
if (LUT4D(a, b, c, d, t, vix, out bool slice)) {
    int flipT = (t > 0 ? 4 - t : 0);
    LUT4D(a, b, c, d, flipT, vix, out slice);
}
```

The `flipT = 4 - t` reverses the triangle winding by iterating vertices in reverse order.

---

## 4. Normal Calculation

### 4.1 4D Normal Formula (MakeNormal)

**From Transform4D.cs (lines 437-447):**
```csharp
public static Vector4 MakeNormal(Vector4 a, Vector4 b, Vector4 c) {
    return new Vector4(
       -Vector3.Dot(YZW(a), Vector3.Cross(YZW(b), YZW(c))),
        Vector3.Dot(ZWX(a), Vector3.Cross(ZWX(b), ZWX(c))),
       -Vector3.Dot(WXY(a), Vector3.Cross(WXY(b), WXY(c))),
        Vector3.Dot(XYZ(a), Vector3.Cross(XYZ(b), XYZ(c))));
}
public static Vector3 YZW(Vector4 v) { return new Vector3(v.y, v.z, v.w); }
public static Vector3 ZWX(Vector4 v) { return new Vector3(v.z, v.w, v.x); }
public static Vector3 WXY(Vector4 v) { return new Vector3(v.w, v.x, v.y); }
public static Vector3 XYZ(Vector4 v) { return new Vector3(v.x, v.y, v.z); }
```

**Mathematical interpretation:**

This computes the 4D cross product of three vectors (a, b, c) which produces a vector normal to the hyperplane spanned by those vectors:

```
N = a x b x c (4D cross product)
```

Each component is a 3x3 determinant:
```
N.x = -det(a.yzw, b.yzw, c.yzw)
N.y = +det(a.zwx, b.zwx, c.zwx)
N.z = -det(a.wxy, b.wxy, c.wxy)
N.w = +det(a.xyz, b.xyz, c.xyz)
```

The alternating signs follow the generalized cross product formula:
```
N_i = (-1)^i * det(M_i)
```
where M_i is the 3x3 minor with row i removed.

### 4.2 Flat Shading Normals

**From Mesh4D.cs (lines 60-63):**
```csharp
public static PackedNormal Flat(Vector4 a, Vector4 b, Vector4 c, Vector4 d) {
    Vector4 n = Transform4D.MakeNormal(a - d, b - d, c - d);
    uint p = (n.magnitude >= 1e-12f ? PackNormal(-n) : 0);
    return new PackedNormal(p, p, p, p);
}
```

For flat shading, the normal is computed from three edge vectors emanating from vertex D:
```
N = MakeNormal(A-D, B-D, C-D)
```

The normal is negated (`-n`) before packing, indicating outward-facing normals.

### 4.3 Normal Packing/Unpacking

**From Mesh4D.cs (lines 503-518):**
```csharp
public static uint PackNormal(Vector4 n) {
    n /= n.magnitude;
    uint x = (uint)Mathf.FloorToInt(n.x * 127f + 128.0f);
    uint y = (uint)Mathf.FloorToInt(n.y * 127f + 128.0f);
    uint z = (uint)Mathf.FloorToInt(n.z * 127f + 128.0f);
    uint w = (uint)Mathf.FloorToInt(n.w * 127f + 128.0f);
    return (x & 0xFF) | ((y & 0xFF) << 8) | ((z & 0xFF) << 16) | ((w & 0xFF) << 24);
}

public static Vector4 UnpackNormal(uint p) {
    float x = (float)(p & 0xFF) - 128.0f;
    float y = (float)((p >> 8) & 0xFF) - 128.0f;
    float z = (float)((p >> 16) & 0xFF) - 128.0f;
    float w = (float)((p >> 24) & 0xFF) - 128.0f;
    return new Vector4(x, y, z, w).normalized;
}
```

**Encoding:** `packed = floor(component * 127 + 128)` maps [-1, 1] to [1, 255]

**Decoding:** `component = (packed - 128) / magnitude` recovers normalized vector

### 4.4 Normal Interpolation in Shader

**From Core4D.cginc (lines 154-159):**
```hlsl
uint n1 = v.normal[ix1];
uint n2 = v.normal[ix2];
float4 n1f = float4(uint4(n1, n1 >> 8, n1 >> 16, n1 >> 24) & 0xFF) - 128.0;
float4 n2f = float4(uint4(n2, n2 >> 8, n2 >> 16, n2 >> 24) & 0xFF) - 128.0;
float4 n = n1f * v2.w + n2f * v1.w;
o.normal = normalize(mul(UNITY_ACCESS_INSTANCED_PROP(Props, _ModelMatrixIT), n));
```

Normals are:
1. Unpacked from each vertex
2. Linearly interpolated using the same weights as position
3. Transformed by the inverse-transpose model matrix
4. Normalized

---

## 5. Orientation Determination

### 5.1 Runtime vs. Precomputed

Engine4D uses a **hybrid approach**:
- **Winding order is baked into the LUT** during generation
- **Sign determination happens at runtime** based on vertex W coordinates

### 5.2 Orientation Algorithm

**From GenerateSliceLUT.cs (lines 168-177):**

```csharp
// Get edge vectors oriented by which vertex is "above"
Vector3 va = (v[ia[0]] - v[ia[1]]) * (verts[ia[0]] ? 1.0f : -1.0f);
Vector3 vb = (v[ib[0]] - v[ib[1]]) * (verts[ib[0]] ? 1.0f : -1.0f);
Vector3 vc = (v[ic[0]] - v[ic[1]]) * (verts[ic[0]] ? 1.0f : -1.0f);

// Test orientation using triple product (determinant)
bool needsFlip = Vector3.Dot(va, Vector3.Cross(vb, vc)) < 0.0f;
```

**Algorithm:**
1. Select three edges that cross the slicing plane
2. Compute edge vectors, directing each from "below" to "above"
3. Project to 3D reference positions (x,y,z without w)
4. Compute scalar triple product: `va . (vb x vc)`
5. If negative, triangle has wrong winding - flip vertex order

### 5.3 AddTetrahedronNormal for Consistent Orientation

**From Mesh4D.cs (lines 167-175):**
```csharp
public void AddTetrahedronNormal(Vector4 n, Vector4 a, Vector4 b, Vector4 c, Vector4 d) {
    float nsign = Vector4.Dot(n, Transform4D.MakeNormal(a - d, b - d, c - d));
    Debug.Assert(Mathf.Abs(nsign) > 1e-12f);
    if (nsign > 0) {
        AddTetrahedron(a, b, c, d, Twiddle(0x3065));
    } else {
        AddTetrahedron(b, a, c, d, Twiddle(0x3065));
    }
}
```

When adding tetrahedra with a known outward normal:
1. Compute the tetrahedron's natural normal
2. Compare with expected normal using dot product
3. If signs differ, swap vertices A and B to reverse orientation

---

## 6. Coordinate System

### 6.1 Axes Convention

**W is the 4th spatial dimension.** Coordinates are `(X, Y, Z, W)`.

From README.md:
> Setting an object's position and scale in 4D is done using Unity's Transform component for the x, y, and z components plus the extra fields `PositionW` and `scaleW` from `Object4D`.

### 6.2 Handedness

The system appears to use **left-handed** coordinates in the 3D projection (Z points into screen after negation).

**From Core4D.cginc (line 164):**
```hlsl
o.vertex = mul(UNITY_MATRIX_V, float4(o.vertex.x, o.vertex.y, -o.vertex.z, 1.0));
```

The Z negation converts from the internal right-handed 4D system to Unity's left-handed rendering.

### 6.3 Origin Placement

The origin is at the camera position. Objects are transformed relative to camera:
```
V_cam = CamMatrix * (V_world + CamPosition)
```

---

## 7. Key Differences from Rust4D

Based on the analysis, here are potential areas where Rust4D may differ:

| Aspect | Engine4D | Rust4D (to verify) |
|--------|----------|-------------------|
| Primitive | Tetrahedra | Tetrahedra |
| Slicing plane | W=0 in camera space | ? |
| LUT encoding | Vertex indices in texture | Triangle config |
| Winding | Baked in LUT + runtime flip | ? |
| Normal formula | 4D cross product | ? |
| Handedness | Left-handed (Unity) | ? |

---

## 8. Code References

### Key Files Analyzed:
- `/home/lemoneater/Projects/Personal/Engine4D/Assets/Editor/GenerateSliceLUT.cs` - LUT generation
- `/home/lemoneater/Projects/Personal/Engine4D/Assets/ShadersND/Core4D.cginc` - 4D vertex shader
- `/home/lemoneater/Projects/Personal/Engine4D/Assets/ShadersND/Core5D.cginc` - 5D vertex shader
- `/home/lemoneater/Projects/Personal/Engine4D/Assets/Scripts/Mesh4D.cs` - Mesh structure
- `/home/lemoneater/Projects/Personal/Engine4D/Assets/Scripts/Transform4D.cs` - Math utilities
- `/home/lemoneater/Projects/Personal/Engine4D/Assets/Scripts/Object4D.cs` - 4D game object

---

## 9. Summary

Engine4D's 4D rendering pipeline is mathematically sound and uses:

1. **Standard tetrahedra** decomposition for 4D volumes
2. **W=0 hyperplane** slicing in camera-relative coordinates
3. **Linear interpolation** along edges: `t = w_A / (w_A - w_B)`
4. **Precomputed LUT** with baked winding + runtime sign-based lookup
5. **4D cross product** for normal calculation using 3x3 minors
6. **Orientation test** via scalar triple product in reference coordinates

The winding determination is the most sophisticated part - it precomputes correct vertex ordering in the LUT by testing orientation in a reference tetrahedron, then applies corrections based on the runtime sign pattern.
