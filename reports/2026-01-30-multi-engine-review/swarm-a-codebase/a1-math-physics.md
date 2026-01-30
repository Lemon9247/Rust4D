# Agent A1: Math & Physics Review
**Date**: 2026-01-30
**Crates**: `rust4d_math`, `rust4d_physics`

---

## rust4d_math

### Types and Operations

#### Vec4 (`src/vec4.rs`, 371 lines)
The foundational 4D vector type with `x, y, z, w` components. Uses `#[repr(C)]` for GPU compatibility and derives `Pod`, `Zeroable` (bytemuck), `Serialize`, `Deserialize` (serde).

**Constants**: `ZERO`, `X`, `Y`, `Z`, `W` (basis vectors)

**Operations supported**:
- `new`, `dot`, `length`, `length_squared`, `normalized`
- `lerp`, `xyz()` (extract 3D for rendering)
- `clamp_components`, `min_components`, `max_components`
- `abs`, `sign`, `component_mul` (Hadamard product)
- Operator overloads: `Add`, `AddAssign`, `Sub`, `SubAssign`, `Mul<f32>`, `MulAssign<f32>`, `Neg`, `Div<f32>`

**Missing operations** (relevant for FPS):
- No `cross` product (4D has no single cross product; would need a wedge product or the 3D-subset cross)
- No `distance(a, b)` convenience method
- No `reflect(normal)` (needed for projectile bouncing)
- No `project_onto(other)` (needed for movement along surfaces)
- No `f32 * Vec4` (only `Vec4 * f32` is implemented)

#### Rotor4 (`src/rotor4.rs`, 713 lines)
The 4D rotation type using geometric algebra. This is the most mathematically sophisticated part of the codebase.

**Structure**: 8 components -- 1 scalar, 6 bivectors (XY, XZ, XW, YZ, YW, ZW), 1 pseudoscalar (e1234).

**Rotation planes** (enum `RotationPlane`): All 6 planes in 4D space -- XY, XZ, YZ, XW, YW, ZW. Well-documented with comments mapping to 3D analogues (yaw, pitch, roll) and 4D-specific planes (ana/kata).

**Key methods**:
- `from_plane_angle(plane, angle)` -- single-plane rotation
- `from_euler_xyz(x, y, z)` -- 3D Euler angle compatibility (matches Unity conventions)
- `from_plane_vectors(a, b, angle)` -- rotation in arbitrary plane from two spanning vectors
- `rotate(v)` -- full sandwich product `R v R~` computed directly (lines 237-299)
- `compose(other)` -- full geometric product of two rotors (lines 303-372)
- `to_matrix()` -- converts to 4x4 matrix by rotating basis vectors
- `normalize()`, `reverse()`, `magnitude()`

**Assessment**: The rotor implementation is mathematically correct and thorough. The sandwich product is implemented as a direct computation (not decomposed into matrix ops), which is the right approach for correctness. The composition handles all 8 components correctly including the pseudoscalar term. The code comments reference Engine4D conventions, suggesting careful porting from a reference implementation.

**Missing for FPS**:
- No `slerp` (spherical linear interpolation for smooth rotation transitions)
- No `look_at` or `from_to` rotation (needed for enemies/turrets facing player)
- No angular velocity or torque types

#### Mat4 (`src/mat4.rs`, 321 lines)
A 4x4 matrix type alias (`[[f32; 4]; 4]`, column-major).

**Functions**:
- `plane_rotation(angle, p1, p2)` -- rotation matrix for a 2D plane in 4D
- `skip_y(m)` -- the critical Engine4D `SkipY` transformation that remaps XYZ rotations to XZW, preserving the Y axis (gravity direction). Extensively documented at lines 49-70
- `mul(a, b)` -- matrix multiplication
- `transform(m, v)` -- matrix-vector multiplication
- `get_column(m, col)` -- extract column vector
- `transpose(m)`

**Missing for FPS**:
- No `inverse()` (needed for unprojection, camera transforms)
- No translation/scale matrix constructors
- No perspective/orthographic projection matrices (currently likely handled by wgpu/glam elsewhere)
- No `determinant()`

#### Shapes (`src/shape.rs`, 92 lines)
**`ConvexShape4D` trait**: Core abstraction for sliceable 4D shapes. Requires `vertices()` and `tetrahedra()`. Sends trait: `Send + Sync`.

**`Tetrahedron`**: A 3-simplex represented as 4 vertex indices. Has `new`, `new_canonical` (sorted), and `canonical()` methods. Used as the fundamental unit for 4D-to-3D cross-section slicing.

#### Tesseract4D (`src/tesseract.rs`, 255 lines)
A 4D hypercube. 16 vertices, decomposed into tetrahedra using Kuhn triangulation.

**Implementation**: Binary indexing for vertices (line 32). 24 permutations of [0,1,2,3] generate 5-cells, which are decomposed into tetrahedra with HashSet deduplication.

#### Hyperplane4D (`src/hyperplane.rs`, 263 lines)
A 4D floor/ground plane, modeled as a grid of "pillars" (mini-tesseracts) in X, Z with W extent. Each cell has 16 vertices. Uses the same Kuhn triangulation as Tesseract4D for tetrahedra decomposition.

**Note**: The Kuhn triangulation code is duplicated between `tesseract.rs` and `hyperplane.rs` (lines 144-189 and 89-139 respectively). This is a refactoring opportunity.

### Testing (59 unit tests + 1 doc test, all passing)

| Module | Test Count | Coverage Assessment |
|--------|-----------|-------------------|
| vec4 | 15 | Good. Tests all operations including edge cases (zero-length normalize) |
| rotor4 | 19 | Excellent. Tests identity, all cardinal plane rotations, composition, inverse, normalization, orthogonality, matrix conversion, sequential vs composed verification |
| mat4 | 9 | Good. Tests SkipY extensively (preserves Y, remaps rotations, XZ->XW) |
| shape | 3 | Basic. Only tests Tetrahedron struct, not ConvexShape4D trait |
| tesseract | 7 | Good. Tests vertex count/positions, tetrahedra validity, edge coverage |
| hyperplane | 6 | Adequate. Tests creation, vertex positions, accessors, trait impl |

**What's not tested**:
- `from_plane_vectors` (arbitrary rotation plane construction)
- `from_euler_xyz` (only tested indirectly via SkipY tests)
- Edge cases: NaN/infinity inputs, very large/small values
- Performance/allocation (Hyperplane creates many Vec allocations)

### Dependencies
- `bytemuck` (workspace) -- zero-copy casting, Pod/Zeroable derives
- `serde` 1.0 with `derive` -- serialization

---

## rust4d_physics

### Collision Shapes (`src/shapes.rs`, 272 lines)

Three primitive collision shapes:

1. **Sphere4D**: center + radius. Methods: `new`, `unit`, `contains`, `closest_point`
2. **AABB4D**: min/max corners. Methods: `new`, `from_center_half_extents`, `unit`, `center`, `half_extents`, `size`, `contains`, `closest_point`, `translated`
3. **Plane4D**: normal + distance. Methods: `new`, `from_point_normal`, `floor`, `signed_distance`, `project_point`, `is_above`

**Collider enum** wraps all three with `center()` and `translated()` methods.

### Collision Detection (`src/collision.rs`, 617 lines)

Four collision routines:
- `sphere_vs_plane` -- signed distance test (line 168)
- `aabb_vs_plane` -- vertex-based (closest vertex to plane, line 193)
- `sphere_vs_aabb` -- closest point on AABB to sphere center (line 218). Handles degenerate case where sphere center is inside AABB by finding shortest escape axis (lines 234-271)
- `aabb_vs_aabb` -- SAT (separating axis theorem) on all 4 axes (line 284). Returns minimum overlap axis as contact normal

**Missing collision pairs**:
- `sphere_vs_sphere` exists only as private method on `PhysicsWorld` (line 265 of `world.rs`), not as a public standalone function

**Collision Filtering** (lines 11-134):
- `CollisionLayer` bitflags: DEFAULT, PLAYER, ENEMY, STATIC, TRIGGER, PROJECTILE, PICKUP, ALL
- `CollisionFilter` with layer + mask. Symmetric check: `(A.layer & B.mask) != 0 AND (B.layer & A.mask) != 0`
- Preset filters: `player()`, `enemy()`, `static_world()`, `trigger(detects)`, `player_projectile()`

**Contact struct**: point, normal, penetration depth.

### Physics Materials (`src/material.rs`, 152 lines)

**PhysicsMaterial**: friction (0-1) + restitution (0-1).

**Presets**: ICE (0.05, 0.1), RUBBER (0.9, 0.8), METAL (0.3, 0.3), WOOD (0.5, 0.2), CONCRETE (0.7, 0.1)

**Combination**: Geometric mean for friction, maximum for restitution. This is a reasonable physical model.

### Rigid Bodies (`src/body.rs`, 724 lines)

**RigidBody4D** fields: position, velocity, mass, material, collider, body_type, grounded, filter.

**BodyType enum**: Dynamic (full physics), Static (never moves), Kinematic (user-controlled, no gravity).

**Builder pattern**: `new_sphere()`, `new_aabb()`, `new_static_aabb()`, plus `.with_velocity()`, `.with_mass()`, `.with_material()`, `.with_restitution()`, `.with_body_type()`, `.with_gravity()`, `.with_static()`, `.with_filter()`, `.with_layer()`, `.with_mask()`.

**Position management**: `set_position()` and `apply_correction()` both update position AND sync the collider shape. This is important for keeping physics state consistent.

**StaticCollider**: separate type for floors/walls/platforms. Notable methods:
- `floor_bounded()` -- creates AABB floor with minimum 5.0 unit thickness (anti-tunneling, line 270)
- `is_position_over()` -- XZW bounds check for edge-falling detection (line 308)

**BodyKey**: Uses slotmap generational indexing for safe handle-based access.

### Physics World (`src/world.rs`, 1476 lines)

**PhysicsConfig**: gravity (-20.0 default), jump_velocity (8.0 default). Serializable.

**PhysicsWorld** contains:
- `SlotMap<BodyKey, RigidBody4D>` for bodies
- `Vec<StaticCollider>` for static geometry
- Player body tracking (optional `BodyKey`)

**Simulation step** (`step(dt)`, line 193):
1. Reset player grounded state
2. Apply gravity to Dynamic bodies and player (Kinematic but has gravity for jump/fall)
3. Integrate velocity -> position
4. Resolve static collisions
5. Resolve body-body collisions

**Static collision resolution** (line 282):
- Iterates all non-static bodies against all static colliders
- Checks collision filter compatibility
- **Edge falling detection** (line 303): if player is off a bounded floor's XZW bounds, skip collision with that floor. This prevents oscillation at platform edges.
- Applies positional correction (push out along contact normal)
- Ground detection: `contact.normal.y > 0.7` threshold
- Velocity response with restitution and friction
- Friction model: removes tangent velocity proportional to combined friction coefficient

**Body-body collision resolution** (line 354):
- O(n^2) pair-wise check
- Respects collision filters
- Mass-based position correction splitting
- Kinematic bodies: pushed by static geometry but NOT by dynamic bodies (line 440)
- Kinematic velocity is never modified by collisions (line 483)

**Player-specific features**:
- `apply_player_movement(movement)` -- sets XZ+W velocity, preserves Y for gravity
- `player_jump()` -- only succeeds if grounded
- Grounded state reset each frame before collision detection

### Testing (97 unit tests, all passing)

| Module | Test Count | Coverage Assessment |
|--------|-----------|-------------------|
| shapes | 7 | Good. Tests Sphere4D contains/closest, AABB4D construction/contains/closest, Plane4D distance/project/above |
| collision | 22 | Excellent. Tests all collision pairs (above/touching/colliding), all filter presets, filter interactions (player vs player, player vs enemy, trigger behavior, projectile filtering), bounded floor scenarios |
| material | 7 | Good. Tests default, clamping, presets, combine (geometric mean, max restitution, commutativity, identity) |
| body | 24 | Excellent. Tests constructors, builder methods, position sync, correction, body types, collision filters, bounded floor creation/edge detection/collision, is_position_over for AABB/plane/Y-independence |
| world | 37 | Excellent. Tests gravity, velocity integration, static body immobility, floor collision with bounce/no-bounce, body-body collisions (sphere-sphere, sphere-AABB), collider sync, friction (rubber vs ice), player movement/grounding/jumping, collision filter integration (trigger passthrough, player-player skip, player-enemy collide, projectile filter), kinematic push behavior, edge falling (W edge, no oscillation, void fall, jumping still works, center floor grounding) |

**Outstanding test quality**: The edge-falling tests (lines 1242-1474) are particularly thorough, testing the oscillation bug fix, void falling, and ensuring normal jumping still works after the fix.

### Dependencies
- `rust4d_math` (local) -- Vec4 type
- `slotmap` (workspace) -- generational arena for bodies
- `bitflags` (workspace) -- collision layer flags
- `serde` 1.0 with `derive` -- PhysicsConfig serialization

---

## Overall Assessment

### Ratings (1-5 scale)

| Criteria | rust4d_math | rust4d_physics |
|----------|------------|---------------|
| Feature completeness | 3 | 3.5 |
| Code quality | 4.5 | 4 |
| Test coverage | 4 | 4.5 |
| FPS readiness | 2 | 2.5 |

### rust4d_math -- Top 3 Strengths

1. **Rotor4 is mathematically rigorous**: Full geometric algebra rotor with correct sandwich product, composition, and all 6 rotation planes. This is the hardest part of 4D math and it's done right.

2. **GPU-friendly types**: Vec4 uses `#[repr(C)]`, `Pod`, `Zeroable` for zero-copy GPU upload. Column-major matrix convention matches typical graphics APIs.

3. **Strong test coverage of rotation correctness**: 19 rotor tests verify orthogonality, length preservation, composition, matrix equivalence, and sequential-vs-composed consistency.

### rust4d_math -- Top 3 Gaps

1. **No raycasting primitives**: No `Ray4D` struct, no ray-shape intersection functions. This is the single biggest FPS gap -- needed for shooting, line-of-sight, picking.

2. **No matrix inverse or full transform type**: No `inverse()`, no combined translation+rotation+scale `Transform4D` struct. FPS cameras and character controllers need these.

3. **Missing vector utility operations**: No `reflect`, `project_onto`, `distance`, `f32 * Vec4`, `cross` (3D subset). These come up constantly in gameplay code.

### rust4d_physics -- Top 3 Strengths

1. **Complete collision layer system**: 7 named layers with bitflag masks, preset filters for player/enemy/trigger/projectile, symmetric collision check. This is FPS-ready architecture.

2. **Robust player physics**: Kinematic body type with gravity for jumping, grounded detection, edge-falling with oscillation prevention, bounded floors. The player controller foundation is solid.

3. **Material system with realistic combining**: Geometric mean friction + max restitution is physically motivated. 5 preset materials (ICE, RUBBER, METAL, WOOD, CONCRETE) cover common FPS surfaces.

### rust4d_physics -- Top 3 Gaps

1. **No raycasting / spatial queries**: No ray-shape intersection, no overlap queries, no "what am I looking at" functionality. Critical for shooting, line-of-sight, item pickup detection.

2. **No trigger volume system**: CollisionLayer::TRIGGER exists and CollisionFilter::trigger() is defined, but there's no event/callback system for "player entered trigger zone". The current asymmetric filter design means triggers can detect but there's no mechanism to report the detection.

3. **No projectile physics**: No continuous collision detection (CCD), no swept tests for fast-moving objects. The minimum 5.0 unit floor thickness (line 270 of body.rs) is a workaround for tunneling but not a real solution. Bullets at high speed will pass through thin geometry.

### Additional FPS Gaps (both crates)

- **No character controller**: No capsule collider, no step-up logic, no slope handling
- **No explosion/radial force**: No "apply force from point with falloff"
- **No ragdoll/articulated bodies**: Only single rigid bodies
- **No spatial partitioning**: O(n^2) collision detection will not scale
- **No angular dynamics**: No torque, angular velocity, or moment of inertia
- **No damage zones**: Would need trigger volumes + event system
- **No Bezier/spline curves**: Useful for projectile arcs, path following
- **No AABB tree or BVH**: Broadphase collision detection missing

### Architecture Notes

1. **Clean separation of concerns**: Math crate has no physics knowledge, physics crate depends on math but not rendering. Good layering.

2. **Dual shape systems**: `rust4d_math` has renderable shapes (`ConvexShape4D`, `Tesseract4D`, `Hyperplane4D`) while `rust4d_physics` has collision shapes (`Sphere4D`, `AABB4D`, `Plane4D`). These are intentionally separate -- renderable shapes are tetrahedra-based for 4D slicing, collision shapes are analytical primitives. This is the right design.

3. **Code duplication**: Kuhn triangulation is duplicated between `tesseract.rs` and `hyperplane.rs`. Should be extracted to a shared utility.

4. **PhysicsWorld has player-specific logic**: The world has `player_body`, `player_jump_velocity`, `apply_player_movement`, `player_jump`. This couples the physics engine to a specific gameplay pattern. For a boomer shooter, this is acceptable short-term but should eventually be extracted into a character controller system.

5. **No ECS integration point**: Physics world manages its own body storage. Integration with the core entity system (if it exists) would require syncing positions between two storage systems.
