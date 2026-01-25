//! 4D Rotor for representing rotations in 4D space
//!
//! In 4D, rotations happen in planes rather than around axes.
//! There are 6 rotation planes: XY, XZ, XW, YZ, YW, ZW.
//!
//! A rotor has 8 components:
//! - 1 scalar
//! - 6 bivectors (one for each plane)
//! - 1 pseudoscalar (4-vector)

use bytemuck::{Pod, Zeroable};
use crate::Vec4;

/// The 6 rotation planes in 4D space
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RotationPlane {
    /// XY plane - standard yaw (rotation around Z axis in 3D)
    XY,
    /// XZ plane - standard pitch (rotation around Y axis in 3D)
    XZ,
    /// YZ plane - standard roll (rotation around X axis in 3D)
    YZ,
    /// XW plane - ana-kata rotation affecting X
    XW,
    /// YW plane - ana-kata rotation affecting Y
    YW,
    /// ZW plane - ana-kata rotation affecting Z (W-roll in 4D Golf)
    ZW,
}

/// 4D Rotor for representing rotations
///
/// Rotor = scalar + bivectors + pseudoscalar
/// R = s + b_xy*e12 + b_xz*e13 + b_xw*e14 + b_yz*e23 + b_yw*e24 + b_zw*e34 + p*e1234
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct Rotor4 {
    /// Scalar component
    pub s: f32,
    /// Bivector component for XY plane (e12)
    pub b_xy: f32,
    /// Bivector component for XZ plane (e13)
    pub b_xz: f32,
    /// Bivector component for XW plane (e14)
    pub b_xw: f32,
    /// Bivector component for YZ plane (e23)
    pub b_yz: f32,
    /// Bivector component for YW plane (e24)
    pub b_yw: f32,
    /// Bivector component for ZW plane (e34)
    pub b_zw: f32,
    /// Pseudoscalar component (e1234)
    pub p: f32,
}

impl Default for Rotor4 {
    fn default() -> Self {
        Self::IDENTITY
    }
}

impl Rotor4 {
    /// Identity rotor (no rotation)
    pub const IDENTITY: Self = Self {
        s: 1.0,
        b_xy: 0.0,
        b_xz: 0.0,
        b_xw: 0.0,
        b_yz: 0.0,
        b_yw: 0.0,
        b_zw: 0.0,
        p: 0.0,
    };

    /// Create a rotor for rotation in a single plane
    ///
    /// For a rotation by angle θ in a plane, the rotor is:
    /// R = cos(θ/2) - sin(θ/2) * B
    /// where B is the unit bivector for that plane
    pub fn from_plane_angle(plane: RotationPlane, angle: f32) -> Self {
        let half = angle * 0.5;
        let cos_h = half.cos();
        let sin_h = half.sin();

        let mut r = Self::IDENTITY;
        r.s = cos_h;

        // The bivector component is -sin(θ/2) for the rotation plane
        match plane {
            RotationPlane::XY => r.b_xy = -sin_h,
            RotationPlane::XZ => r.b_xz = -sin_h,
            RotationPlane::XW => r.b_xw = -sin_h,
            RotationPlane::YZ => r.b_yz = -sin_h,
            RotationPlane::YW => r.b_yw = -sin_h,
            RotationPlane::ZW => r.b_zw = -sin_h,
        }

        r
    }

    /// Compute the squared magnitude of the rotor
    #[inline]
    pub fn magnitude_squared(&self) -> f32 {
        self.s * self.s
            + self.b_xy * self.b_xy
            + self.b_xz * self.b_xz
            + self.b_xw * self.b_xw
            + self.b_yz * self.b_yz
            + self.b_yw * self.b_yw
            + self.b_zw * self.b_zw
            + self.p * self.p
    }

    /// Compute the magnitude of the rotor
    #[inline]
    pub fn magnitude(&self) -> f32 {
        self.magnitude_squared().sqrt()
    }

    /// Normalize the rotor to unit magnitude
    pub fn normalize(&self) -> Self {
        let mag = self.magnitude();
        if mag > 0.0 {
            let inv_mag = 1.0 / mag;
            Self {
                s: self.s * inv_mag,
                b_xy: self.b_xy * inv_mag,
                b_xz: self.b_xz * inv_mag,
                b_xw: self.b_xw * inv_mag,
                b_yz: self.b_yz * inv_mag,
                b_yw: self.b_yw * inv_mag,
                b_zw: self.b_zw * inv_mag,
                p: self.p * inv_mag,
            }
        } else {
            Self::IDENTITY
        }
    }

    /// Compute the reverse (conjugate) of the rotor
    /// For unit rotors, this is the inverse rotation
    /// Reverse negates all bivector components
    pub fn reverse(&self) -> Self {
        Self {
            s: self.s,
            b_xy: -self.b_xy,
            b_xz: -self.b_xz,
            b_xw: -self.b_xw,
            b_yz: -self.b_yz,
            b_yw: -self.b_yw,
            b_zw: -self.b_zw,
            p: self.p, // Pseudoscalar doesn't change sign under reverse
        }
    }

    /// Rotate a 4D vector using the sandwich product: v' = R * v * R†
    ///
    /// This is the core operation for applying rotations.
    pub fn rotate(&self, v: Vec4) -> Vec4 {
        // For simple rotors (pure bivector rotations), we can use explicit formulas
        // For general rotors, we compute the sandwich product step by step

        let s = self.s;
        let b12 = self.b_xy;
        let b13 = self.b_xz;
        let b14 = self.b_xw;
        let b23 = self.b_yz;
        let b24 = self.b_yw;
        let b34 = self.b_zw;
        let ps = self.p;

        let x = v.x;
        let y = v.y;
        let z = v.z;
        let w = v.w;

        // Pre-compute squared terms
        let s2 = s * s;
        let b12_2 = b12 * b12;
        let b13_2 = b13 * b13;
        let b14_2 = b14 * b14;
        let b23_2 = b23 * b23;
        let b24_2 = b24 * b24;
        let b34_2 = b34 * b34;
        let ps2 = ps * ps;

        // The rotation matrix for R * v * R†
        // Derived from the geometric algebra sandwich product
        // For each basis vector, the diagonal term is (s² - b²) for bivectors
        // that include that basis, and cross terms come from the sandwich product

        // For x (e1): involved in b12, b13, b14
        // Diagonal: s² - b12² - b13² - b14² + other terms from normalization
        // For a unit rotor (s² + sum of b² = 1), this simplifies

        // x' component
        let new_x = x * (s2 - b12_2 - b13_2 - b14_2 + b23_2 + b24_2 + b34_2 - ps2)
            + 2.0 * y * (s * b12 + b13 * b23 + b14 * b24 + b34 * ps)
            + 2.0 * z * (s * b13 - b12 * b23 + b14 * b34 - b24 * ps)
            + 2.0 * w * (s * b14 - b12 * b24 - b13 * b34 + b23 * ps);

        // y' component
        let new_y = 2.0 * x * (-s * b12 + b13 * b23 + b14 * b24 - b34 * ps)
            + y * (s2 - b12_2 + b13_2 + b14_2 - b23_2 - b24_2 + b34_2 - ps2)
            + 2.0 * z * (s * b23 + b12 * b13 - b24 * b34 + b14 * ps)
            + 2.0 * w * (s * b24 + b12 * b14 + b23 * b34 - b13 * ps);

        // z' component
        let new_z = 2.0 * x * (-s * b13 - b12 * b23 + b14 * b34 + b24 * ps)
            + 2.0 * y * (-s * b23 + b12 * b13 - b24 * b34 - b14 * ps)
            + z * (s2 + b12_2 - b13_2 + b14_2 - b23_2 + b24_2 - b34_2 - ps2)
            + 2.0 * w * (s * b34 + b13 * b14 + b23 * b24 + b12 * ps);

        // w' component
        let new_w = 2.0 * x * (-s * b14 - b12 * b24 - b13 * b34 - b23 * ps)
            + 2.0 * y * (-s * b24 + b12 * b14 + b23 * b34 + b13 * ps)
            + 2.0 * z * (-s * b34 + b13 * b14 + b23 * b24 - b12 * ps)
            + w * (s2 + b12_2 + b13_2 - b14_2 + b23_2 - b24_2 - b34_2 - ps2);

        Vec4::new(new_x, new_y, new_z, new_w)
    }

    /// Compose two rotations: result = self * other
    /// The composed rotation applies `other` first, then `self`
    pub fn compose(&self, other: &Self) -> Self {
        // Geometric product of two rotors
        // This is a lengthy computation involving all 8 components

        let a = self;
        let b = other;

        // Scalar part
        let s = a.s * b.s
            - a.b_xy * b.b_xy
            - a.b_xz * b.b_xz
            - a.b_xw * b.b_xw
            - a.b_yz * b.b_yz
            - a.b_yw * b.b_yw
            - a.b_zw * b.b_zw
            + a.p * b.p;

        // XY bivector
        let b_xy = a.s * b.b_xy + a.b_xy * b.s
            - a.b_xz * b.b_yz + a.b_yz * b.b_xz
            - a.b_xw * b.b_yw + a.b_yw * b.b_xw
            - a.b_zw * b.p - a.p * b.b_zw;

        // XZ bivector
        let b_xz = a.s * b.b_xz + a.b_xz * b.s
            + a.b_xy * b.b_yz - a.b_yz * b.b_xy
            - a.b_xw * b.b_zw + a.b_zw * b.b_xw
            + a.b_yw * b.p + a.p * b.b_yw;

        // XW bivector
        let b_xw = a.s * b.b_xw + a.b_xw * b.s
            + a.b_xy * b.b_yw - a.b_yw * b.b_xy
            + a.b_xz * b.b_zw - a.b_zw * b.b_xz
            - a.b_yz * b.p - a.p * b.b_yz;

        // YZ bivector
        let b_yz = a.s * b.b_yz + a.b_yz * b.s
            - a.b_xy * b.b_xz + a.b_xz * b.b_xy
            - a.b_yw * b.b_zw + a.b_zw * b.b_yw
            - a.b_xw * b.p - a.p * b.b_xw;

        // YW bivector
        let b_yw = a.s * b.b_yw + a.b_yw * b.s
            - a.b_xy * b.b_xw + a.b_xw * b.b_xy
            + a.b_yz * b.b_zw - a.b_zw * b.b_yz
            + a.b_xz * b.p + a.p * b.b_xz;

        // ZW bivector
        let b_zw = a.s * b.b_zw + a.b_zw * b.s
            - a.b_xz * b.b_xw + a.b_xw * b.b_xz
            - a.b_yz * b.b_yw + a.b_yw * b.b_yz
            - a.b_xy * b.p - a.p * b.b_xy;

        // Pseudoscalar
        let p = a.s * b.p + a.p * b.s
            + a.b_xy * b.b_zw + a.b_zw * b.b_xy
            - a.b_xz * b.b_yw - a.b_yw * b.b_xz
            + a.b_xw * b.b_yz + a.b_yz * b.b_xw;

        Self {
            s,
            b_xy,
            b_xz,
            b_xw,
            b_yz,
            b_yw,
            b_zw,
            p,
        }
    }

    /// Convert rotor to a 4x4 rotation matrix
    /// Useful for sending to GPU
    pub fn to_matrix(&self) -> [[f32; 4]; 4] {
        // We compute the matrix by rotating each basis vector
        let x_col = self.rotate(Vec4::X);
        let y_col = self.rotate(Vec4::Y);
        let z_col = self.rotate(Vec4::Z);
        let w_col = self.rotate(Vec4::W);

        // Column-major order
        [
            [x_col.x, x_col.y, x_col.z, x_col.w],
            [y_col.x, y_col.y, y_col.z, y_col.w],
            [z_col.x, z_col.y, z_col.z, z_col.w],
            [w_col.x, w_col.y, w_col.z, w_col.w],
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::PI;

    const EPSILON: f32 = 0.0001;

    fn approx_eq(a: f32, b: f32) -> bool {
        (a - b).abs() < EPSILON
    }

    fn vec_approx_eq(a: Vec4, b: Vec4) -> bool {
        approx_eq(a.x, b.x) && approx_eq(a.y, b.y) && approx_eq(a.z, b.z) && approx_eq(a.w, b.w)
    }

    #[test]
    fn test_identity_rotation() {
        let r = Rotor4::IDENTITY;
        let v = Vec4::new(1.0, 2.0, 3.0, 4.0);
        let rotated = r.rotate(v);
        assert!(vec_approx_eq(v, rotated));
    }

    #[test]
    fn test_xy_rotation_90() {
        let r = Rotor4::from_plane_angle(RotationPlane::XY, PI / 2.0);

        // Rotating X by 90° in XY plane should give Y
        let v = Vec4::X;
        let rotated = r.rotate(v);
        assert!(vec_approx_eq(rotated, Vec4::Y), "Expected Y, got {:?}", rotated);

        // Rotating Y by 90° in XY plane should give -X
        let v = Vec4::Y;
        let rotated = r.rotate(v);
        assert!(vec_approx_eq(rotated, -Vec4::X), "Expected -X, got {:?}", rotated);
    }

    #[test]
    fn test_xz_rotation_90() {
        let r = Rotor4::from_plane_angle(RotationPlane::XZ, PI / 2.0);

        // Rotating X by 90° in XZ plane should give Z
        let v = Vec4::X;
        let rotated = r.rotate(v);
        assert!(vec_approx_eq(rotated, Vec4::Z), "Expected Z, got {:?}", rotated);
    }

    #[test]
    fn test_zw_rotation_90() {
        let r = Rotor4::from_plane_angle(RotationPlane::ZW, PI / 2.0);

        // Rotating Z by 90° in ZW plane should give W
        let v = Vec4::Z;
        let rotated = r.rotate(v);
        assert!(vec_approx_eq(rotated, Vec4::W), "Expected W, got {:?}", rotated);
    }

    #[test]
    fn test_rotation_preserves_length() {
        let r = Rotor4::from_plane_angle(RotationPlane::XY, 1.23);
        let v = Vec4::new(1.0, 2.0, 3.0, 4.0);
        let rotated = r.rotate(v);
        assert!(approx_eq(v.length(), rotated.length()));
    }

    #[test]
    fn test_compose_identity() {
        let r = Rotor4::from_plane_angle(RotationPlane::XY, PI / 4.0);
        let identity = Rotor4::IDENTITY;

        let composed = r.compose(&identity);
        assert!(approx_eq(composed.s, r.s));
        assert!(approx_eq(composed.b_xy, r.b_xy));
    }

    #[test]
    fn test_compose_inverse() {
        let r = Rotor4::from_plane_angle(RotationPlane::XY, PI / 3.0);
        let r_inv = r.reverse();

        let composed = r.compose(&r_inv);
        // Should be close to identity
        assert!(approx_eq(composed.normalize().s, 1.0), "Expected identity, got {:?}", composed);
    }

    #[test]
    fn test_full_rotation() {
        // Two 180° rotations should give identity
        let r = Rotor4::from_plane_angle(RotationPlane::XY, PI);
        let composed = r.compose(&r);

        let v = Vec4::new(1.0, 2.0, 3.0, 4.0);
        let rotated = composed.normalize().rotate(v);
        assert!(vec_approx_eq(v, rotated), "Expected original, got {:?}", rotated);
    }

    #[test]
    fn test_normalize() {
        let mut r = Rotor4::from_plane_angle(RotationPlane::XY, PI / 4.0);
        // Artificially scale
        r.s *= 2.0;
        r.b_xy *= 2.0;

        let normalized = r.normalize();
        assert!(approx_eq(normalized.magnitude(), 1.0));
    }

    #[test]
    fn test_to_matrix_identity() {
        let r = Rotor4::IDENTITY;
        let m = r.to_matrix();

        // Should be identity matrix
        assert!(approx_eq(m[0][0], 1.0) && approx_eq(m[0][1], 0.0) && approx_eq(m[0][2], 0.0) && approx_eq(m[0][3], 0.0));
        assert!(approx_eq(m[1][0], 0.0) && approx_eq(m[1][1], 1.0) && approx_eq(m[1][2], 0.0) && approx_eq(m[1][3], 0.0));
        assert!(approx_eq(m[2][0], 0.0) && approx_eq(m[2][1], 0.0) && approx_eq(m[2][2], 1.0) && approx_eq(m[2][3], 0.0));
        assert!(approx_eq(m[3][0], 0.0) && approx_eq(m[3][1], 0.0) && approx_eq(m[3][2], 0.0) && approx_eq(m[3][3], 1.0));
    }
}
