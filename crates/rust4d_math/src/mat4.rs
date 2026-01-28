//! 4x4 Matrix utilities for 4D transformations
//!
//! This module provides matrix operations needed for Engine4D-style camera control,
//! including the critical SkipY transformation that remaps 3D rotations to 4D
//! while keeping the Y axis unchanged.

use crate::Vec4;

/// 4x4 matrix type (column-major)
pub type Mat4 = [[f32; 4]; 4];

/// Identity matrix
pub const IDENTITY: Mat4 = [
    [1.0, 0.0, 0.0, 0.0],
    [0.0, 1.0, 0.0, 0.0],
    [0.0, 0.0, 1.0, 0.0],
    [0.0, 0.0, 0.0, 1.0],
];

/// Create a rotation matrix in a specific 2D plane within 4D space.
///
/// This is equivalent to Engine4D's `Transform4D.PlaneRotation`.
///
/// # Arguments
/// * `angle` - Rotation angle in radians
/// * `p1`, `p2` - Indices of the axes forming the rotation plane (0=X, 1=Y, 2=Z, 3=W)
///
/// # Example
/// ```
/// use rust4d_math::mat4::plane_rotation;
/// // Create a YZ plane rotation (pitch)
/// let pitch_matrix = plane_rotation(0.5, 1, 2);
/// ```
pub fn plane_rotation(angle: f32, p1: usize, p2: usize) -> Mat4 {
    let cs = angle.cos();
    let sn = angle.sin();

    let mut m = IDENTITY;

    // Rotation in plane p1-p2
    m[p1][p1] = cs;
    m[p2][p2] = cs;
    m[p1][p2] = sn;
    m[p2][p1] = -sn;

    m
}

/// Remap a 4D rotation matrix so it operates in the XZW hyperplane,
/// leaving the Y axis unchanged.
///
/// This is the critical transformation from Engine4D (`Transform4D.SkipY`).
/// It maps:
/// - X axis → X axis (unchanged)
/// - Y axis → Z axis (in 4D)
/// - Z axis → W axis (in 4D)
///
/// The Y axis of the *output* remains identity, preserving gravity alignment.
///
/// # Why this matters
/// When you apply 4D rotations with SkipY, the Y axis (gravity direction) is
/// never affected. This means walking forward always stays horizontal relative
/// to world up, regardless of what 4D rotation state you're in.
///
/// # Implementation
/// This is equivalent to Engine4D's `XYZTo(matrix, 0, 2, 3)`:
/// - Takes a 3x3 rotation embedded in 4x4 (top-left 3x3)
/// - Remaps columns: 0→0, 1→2, 2→3
/// - Remaps rows: 0→0, 1→2, 2→3
/// - Column/row 1 (Y) is left as identity
pub fn skip_y(m: Mat4) -> Mat4 {
    // The input matrix is a 3D rotation embedded in 4x4 (top-left 3x3 is rotation).
    // We need to remap it so that the rotation affects XZW instead of XYZ.
    //
    // Engine4D's XYZTo does:
    // 1. Create a column-remapped matrix: columns 0,1,2 → columns sendX,sendY,sendZ
    // 2. Create a row-remapped matrix from that
    //
    // For SkipY: sendX=0, sendY=2, sendZ=3 (skip position 1)

    let mut result = IDENTITY;

    // The rotation in the input affects indices 0,1,2 (XYZ in 3D)
    // We want it to affect indices 0,2,3 (XZW in 4D)

    // Remap: input col 0 (X) → output col 0 (X)
    //        input col 1 (Y) → output col 2 (Z)
    //        input col 2 (Z) → output col 3 (W)
    // Output col 1 (Y) stays identity

    // Copy the 3x3 rotation with remapping
    // Input indices [0,1,2] map to output indices [0,2,3]
    let src_idx = [0usize, 1, 2];
    let dst_idx = [0usize, 2, 3];

    for i in 0..3 {
        for j in 0..3 {
            result[dst_idx[j]][dst_idx[i]] = m[src_idx[j]][src_idx[i]];
        }
    }

    // Y column/row stays identity (already set)
    result[1][1] = 1.0;

    result
}

/// Multiply two 4x4 matrices: result = a * b
///
/// In column-major convention, this applies b first, then a.
#[allow(clippy::needless_range_loop)]
pub fn mul(a: Mat4, b: Mat4) -> Mat4 {
    let mut result = [[0.0f32; 4]; 4];

    for i in 0..4 {
        for j in 0..4 {
            for k in 0..4 {
                result[i][j] += a[k][j] * b[i][k];
            }
        }
    }

    result
}

/// Transform a Vec4 by a 4x4 matrix (column-major)
///
/// result = M * v
pub fn transform(m: Mat4, v: Vec4) -> Vec4 {
    Vec4::new(
        m[0][0] * v.x + m[1][0] * v.y + m[2][0] * v.z + m[3][0] * v.w,
        m[0][1] * v.x + m[1][1] * v.y + m[2][1] * v.z + m[3][1] * v.w,
        m[0][2] * v.x + m[1][2] * v.y + m[2][2] * v.z + m[3][2] * v.w,
        m[0][3] * v.x + m[1][3] * v.y + m[2][3] * v.z + m[3][3] * v.w,
    )
}

/// Get a column vector from a matrix
pub fn get_column(m: Mat4, col: usize) -> Vec4 {
    Vec4::new(m[col][0], m[col][1], m[col][2], m[col][3])
}

/// Transpose a matrix
pub fn transpose(m: Mat4) -> Mat4 {
    [
        [m[0][0], m[1][0], m[2][0], m[3][0]],
        [m[0][1], m[1][1], m[2][1], m[3][1]],
        [m[0][2], m[1][2], m[2][2], m[3][2]],
        [m[0][3], m[1][3], m[2][3], m[3][3]],
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    const EPSILON: f32 = 0.0001;

    fn approx_eq(a: f32, b: f32) -> bool {
        (a - b).abs() < EPSILON
    }

    fn vec_approx_eq(a: Vec4, b: Vec4) -> bool {
        approx_eq(a.x, b.x) && approx_eq(a.y, b.y) && approx_eq(a.z, b.z) && approx_eq(a.w, b.w)
    }

    fn mat_approx_eq(a: Mat4, b: Mat4) -> bool {
        for i in 0..4 {
            for j in 0..4 {
                if !approx_eq(a[i][j], b[i][j]) {
                    return false;
                }
            }
        }
        true
    }

    #[test]
    fn test_identity() {
        let v = Vec4::new(1.0, 2.0, 3.0, 4.0);
        let result = transform(IDENTITY, v);
        assert!(vec_approx_eq(v, result));
    }

    #[test]
    fn test_plane_rotation_yz() {
        use std::f32::consts::FRAC_PI_2;

        // 90° rotation in YZ plane (pitch)
        let m = plane_rotation(FRAC_PI_2, 1, 2);

        // Y should go to Z
        let y = Vec4::new(0.0, 1.0, 0.0, 0.0);
        let result = transform(m, y);
        assert!(vec_approx_eq(result, Vec4::new(0.0, 0.0, 1.0, 0.0)),
            "Y should become Z, got {:?}", result);

        // Z should go to -Y
        let z = Vec4::new(0.0, 0.0, 1.0, 0.0);
        let result = transform(m, z);
        assert!(vec_approx_eq(result, Vec4::new(0.0, -1.0, 0.0, 0.0)),
            "Z should become -Y, got {:?}", result);

        // X should be unchanged
        let x = Vec4::new(1.0, 0.0, 0.0, 0.0);
        let result = transform(m, x);
        assert!(vec_approx_eq(result, x),
            "X should be unchanged, got {:?}", result);
    }

    #[test]
    fn test_skip_y_preserves_y_axis() {
        use std::f32::consts::FRAC_PI_4;
        use crate::Rotor4;
        use crate::RotationPlane;

        // Create a 3D rotation (using YZ plane which affects Y and Z)
        let r = Rotor4::from_plane_angle(RotationPlane::YZ, FRAC_PI_4);
        let m = r.to_matrix();

        // Apply SkipY
        let skip_m = skip_y(m);

        // Now transform Y axis - should be unchanged!
        let y = Vec4::new(0.0, 1.0, 0.0, 0.0);
        let result = transform(skip_m, y);

        assert!(vec_approx_eq(result, y),
            "Y axis should be preserved after skip_y, got {:?}", result);
    }

    #[test]
    fn test_skip_y_remaps_rotation() {
        use std::f32::consts::FRAC_PI_2;
        use crate::Rotor4;
        use crate::RotationPlane;

        // Create a 90° rotation in XY plane (affects X and Y)
        let r = Rotor4::from_plane_angle(RotationPlane::XY, FRAC_PI_2);
        let m = r.to_matrix();

        // Original: X→Y, Y→-X
        let x = Vec4::new(1.0, 0.0, 0.0, 0.0);
        let original_result = transform(m, x);
        assert!(vec_approx_eq(original_result, Vec4::new(0.0, 1.0, 0.0, 0.0)),
            "Original: X should become Y, got {:?}", original_result);

        // After SkipY: rotation is now in XZ plane (indices 0,2)
        let skip_m = skip_y(m);

        // X should now go to Z (not Y)
        let result = transform(skip_m, x);
        assert!(vec_approx_eq(result, Vec4::new(0.0, 0.0, 1.0, 0.0)),
            "After skip_y: X should become Z, got {:?}", result);

        // Y should be unchanged
        let y = Vec4::new(0.0, 1.0, 0.0, 0.0);
        let result = transform(skip_m, y);
        assert!(vec_approx_eq(result, y),
            "After skip_y: Y should be unchanged, got {:?}", result);
    }

    #[test]
    fn test_skip_y_xz_becomes_xw() {
        use std::f32::consts::FRAC_PI_2;
        use crate::Rotor4;
        use crate::RotationPlane;

        // Create a 90° rotation in XZ plane
        let r = Rotor4::from_plane_angle(RotationPlane::XZ, FRAC_PI_2);
        let m = r.to_matrix();

        // After SkipY: XZ rotation becomes XW rotation
        // (Z axis in input becomes W axis in output)
        let skip_m = skip_y(m);

        // X should go to W
        let x = Vec4::new(1.0, 0.0, 0.0, 0.0);
        let result = transform(skip_m, x);
        assert!(vec_approx_eq(result, Vec4::new(0.0, 0.0, 0.0, 1.0)),
            "After skip_y(XZ rotation): X should become W, got {:?}", result);
    }

    #[test]
    fn test_mul_identity() {
        let a = plane_rotation(0.5, 0, 1);
        let result = mul(IDENTITY, a);
        assert!(mat_approx_eq(a, result));

        let result = mul(a, IDENTITY);
        assert!(mat_approx_eq(a, result));
    }

    #[test]
    fn test_mul_composition() {
        use std::f32::consts::FRAC_PI_4;

        // Two 45° rotations should equal one 90° rotation
        let r45 = plane_rotation(FRAC_PI_4, 0, 1);
        let r90 = plane_rotation(FRAC_PI_4 * 2.0, 0, 1);

        let composed = mul(r45, r45);

        let v = Vec4::new(1.0, 0.0, 0.0, 0.0);
        let result1 = transform(composed, v);
        let result2 = transform(r90, v);

        assert!(vec_approx_eq(result1, result2),
            "Composed: {:?}, Direct: {:?}", result1, result2);
    }

    #[test]
    fn test_get_column() {
        let m = plane_rotation(0.5, 1, 2);

        let col0 = get_column(m, 0);
        assert!(vec_approx_eq(col0, Vec4::new(1.0, 0.0, 0.0, 0.0)),
            "Column 0 should be X axis for YZ rotation");
    }
}
