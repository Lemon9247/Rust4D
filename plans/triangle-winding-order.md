╭────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────╮
│ Plan to implement                                                                                                                                          │
│                                                                                                                                                            │
│ Fix Triangle Winding Order for Back-Face Culling                                                                                                           │
│                                                                                                                                                            │
│ Problem                                                                                                                                                    │
│                                                                                                                                                            │
│ The 4D slicing compute shader (slice_tetra.wgsl) flips all triangle normals/winding to face the camera, defeating back-face culling. Additionally, it uses │
│  camera_eye in world space while vertices are in camera space (camera at origin), causing incorrect flips when the camera rotates — making the hyperplane  │
│ floor disappear.                                                                                                                                           │
│                                                                                                                                                            │
│ Approach: Determinant + Case Parity                                                                                                                        │
│                                                                                                                                                            │
│ Replace the camera-facing heuristic with a deterministic geometric computation:                                                                            │
│                                                                                                                                                            │
│ 1. Compute the 3D signed volume (determinant) of each tetrahedron in camera space                                                                          │
│ 2. XOR the determinant sign with case parity (case_idx >= 8) to decide winding flip                                                                        │
│ 3. Remove the camera-facing normal flip entirely                                                                                                           │
│ 4. Re-enable back-face culling                                                                                                                             │
│                                                                                                                                                            │
│ Why this works: The lookup table produces the same triangle indices for complement cases (e.g., case 1 and case 14 both use edges 0,1,2 with indices       │
│ 0,1,2). The cross product gives the same normal for both, but they need opposite outward directions. XOR-ing the tetrahedron handedness (determinant sign) │
│  with the case parity (which half of the table) consistently produces outward-facing normals regardless of tetrahedron orientation or camera position.     │
│                                                                                                                                                            │
│ Files to Modify                                                                                                                                            │
│                                                                                                                                                            │
│ crates/rust4d_render/src/shaders/slice_tetra.wgsl                                                                                                          │
│                                                                                                                                                            │
│ 1. After line 254 (the early return for no-intersection), add signed volume computation:                                                                   │
│ let e1 = pos[1].xyz - pos[0].xyz;                                                                                                                          │
│ let e2 = pos[2].xyz - pos[0].xyz;                                                                                                                          │
│ let e3 = pos[3].xyz - pos[0].xyz;                                                                                                                          │
│ let signed_vol = dot(e1, cross(e2, e3));                                                                                                                   │
│ let should_flip = (signed_vol > 0.0) != (case_idx >= 8u);                                                                                                  │
│ 2. Remove the camera-facing flip block (lines 297-307) and the camera_eye variable (line 279). Replace with:                                               │
│ if (should_flip) {                                                                                                                                         │
│     let tmp = tv1;                                                                                                                                         │
│     tv1 = tv2;                                                                                                                                             │
│     tv2 = tmp;                                                                                                                                             │
│     normal = -normal;                                                                                                                                      │
│ }                                                                                                                                                          │
│                                                                                                                                                            │
│ crates/rust4d_render/src/pipeline/render_pipeline.rs                                                                                                       │
│                                                                                                                                                            │
│ - Line 97: Change cull_mode: None to cull_mode: Some(wgpu::Face::Back)                                                                                     │
│                                                                                                                                                            │
│ Files NOT changed                                                                                                                                          │
│                                                                                                                                                            │
│ - SliceParams struct — camera_eye field stays for API compat (shader just won't use it)                                                                    │
│ - Render shader — max(dot(normal, light_dir), 0.0) works correctly with outward normals                                                                    │
│ - Examples — they pass camera_eye but shader ignores it                                                                                                    │
│ - Geometry generators — mixed-orientation tetrahedra handled by the determinant computation                                                                │
│                                                                                                                                                            │
│ Sign Uncertainty                                                                                                                                           │
│                                                                                                                                                            │
│ The formula may be inverted (all normals inward → everything culled). If geometry disappears after the change: negate to (signed_vol > 0.0) == (case_idx   │
│ >= 8u). Visual test determines the correct sign.                                                                                                           │
│                                                                                                                                                            │
│ Verification                                                                                                                                               │
│                                                                                                                                                            │
│ 1. cargo build — shader compiles without errors                                                                                                            │
│ 2. cargo test --workspace — all existing tests pass                                                                                                        │
│ 3. Visual test (run the engine): hyperplane floor visible from above, culled from below; tesseract faces visible from outside, culled from inside; no      │
│ disappearing geometry when rotating camera                                                                                                                 │
╰────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────╯