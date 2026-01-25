//! 4D Vector type

use bytemuck::{Pod, Zeroable};

/// 4D Vector with x, y, z, w components
/// The w component represents the 4th spatial dimension (ana/kata)
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Pod, Zeroable)]
pub struct Vec4 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,
}

impl Vec4 {
    pub const ZERO: Self = Self { x: 0.0, y: 0.0, z: 0.0, w: 0.0 };
    pub const X: Self = Self { x: 1.0, y: 0.0, z: 0.0, w: 0.0 };
    pub const Y: Self = Self { x: 0.0, y: 1.0, z: 0.0, w: 0.0 };
    pub const Z: Self = Self { x: 0.0, y: 0.0, z: 1.0, w: 0.0 };
    pub const W: Self = Self { x: 0.0, y: 0.0, z: 0.0, w: 1.0 };

    /// Create a new Vec4
    #[inline]
    pub const fn new(x: f32, y: f32, z: f32, w: f32) -> Self {
        Self { x, y, z, w }
    }

    /// Dot product
    #[inline]
    pub fn dot(self, other: Self) -> f32 {
        self.x * other.x + self.y * other.y + self.z * other.z + self.w * other.w
    }

    /// Length squared (faster than length)
    #[inline]
    pub fn length_squared(self) -> f32 {
        self.dot(self)
    }

    /// Length (magnitude)
    #[inline]
    pub fn length(self) -> f32 {
        self.length_squared().sqrt()
    }

    /// Normalize to unit length
    #[inline]
    pub fn normalized(self) -> Self {
        let len = self.length();
        if len > 0.0 {
            self * (1.0 / len)
        } else {
            Self::ZERO
        }
    }

    /// Extract the xyz components as an array (for 3D rendering)
    #[inline]
    pub fn xyz(&self) -> [f32; 3] {
        [self.x, self.y, self.z]
    }

    /// Linear interpolation between two vectors
    #[inline]
    pub fn lerp(self, other: Self, t: f32) -> Self {
        self * (1.0 - t) + other * t
    }
}

// Operator overloads

impl std::ops::Add for Vec4 {
    type Output = Self;
    #[inline]
    fn add(self, other: Self) -> Self {
        Self::new(
            self.x + other.x,
            self.y + other.y,
            self.z + other.z,
            self.w + other.w,
        )
    }
}

impl std::ops::AddAssign for Vec4 {
    #[inline]
    fn add_assign(&mut self, other: Self) {
        self.x += other.x;
        self.y += other.y;
        self.z += other.z;
        self.w += other.w;
    }
}

impl std::ops::Sub for Vec4 {
    type Output = Self;
    #[inline]
    fn sub(self, other: Self) -> Self {
        Self::new(
            self.x - other.x,
            self.y - other.y,
            self.z - other.z,
            self.w - other.w,
        )
    }
}

impl std::ops::SubAssign for Vec4 {
    #[inline]
    fn sub_assign(&mut self, other: Self) {
        self.x -= other.x;
        self.y -= other.y;
        self.z -= other.z;
        self.w -= other.w;
    }
}

impl std::ops::Mul<f32> for Vec4 {
    type Output = Self;
    #[inline]
    fn mul(self, scalar: f32) -> Self {
        Self::new(
            self.x * scalar,
            self.y * scalar,
            self.z * scalar,
            self.w * scalar,
        )
    }
}

impl std::ops::MulAssign<f32> for Vec4 {
    #[inline]
    fn mul_assign(&mut self, scalar: f32) {
        self.x *= scalar;
        self.y *= scalar;
        self.z *= scalar;
        self.w *= scalar;
    }
}

impl std::ops::Neg for Vec4 {
    type Output = Self;
    #[inline]
    fn neg(self) -> Self {
        Self::new(-self.x, -self.y, -self.z, -self.w)
    }
}

impl std::ops::Div<f32> for Vec4 {
    type Output = Self;
    #[inline]
    fn div(self, scalar: f32) -> Self {
        Self::new(
            self.x / scalar,
            self.y / scalar,
            self.z / scalar,
            self.w / scalar,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new() {
        let v = Vec4::new(1.0, 2.0, 3.0, 4.0);
        assert_eq!(v.x, 1.0);
        assert_eq!(v.y, 2.0);
        assert_eq!(v.z, 3.0);
        assert_eq!(v.w, 4.0);
    }

    #[test]
    fn test_dot() {
        let a = Vec4::new(1.0, 2.0, 3.0, 4.0);
        let b = Vec4::new(5.0, 6.0, 7.0, 8.0);
        // 1*5 + 2*6 + 3*7 + 4*8 = 5 + 12 + 21 + 32 = 70
        assert_eq!(a.dot(b), 70.0);
    }

    #[test]
    fn test_length() {
        let v = Vec4::new(1.0, 0.0, 0.0, 0.0);
        assert_eq!(v.length(), 1.0);

        let v2 = Vec4::new(1.0, 1.0, 1.0, 1.0);
        assert!((v2.length() - 2.0).abs() < 0.0001);
    }

    #[test]
    fn test_normalized() {
        let v = Vec4::new(3.0, 0.0, 0.0, 0.0);
        let n = v.normalized();
        assert!((n.x - 1.0).abs() < 0.0001);
        assert_eq!(n.y, 0.0);
        assert_eq!(n.z, 0.0);
        assert_eq!(n.w, 0.0);
    }

    #[test]
    fn test_add() {
        let a = Vec4::new(1.0, 2.0, 3.0, 4.0);
        let b = Vec4::new(5.0, 6.0, 7.0, 8.0);
        let c = a + b;
        assert_eq!(c.x, 6.0);
        assert_eq!(c.y, 8.0);
        assert_eq!(c.z, 10.0);
        assert_eq!(c.w, 12.0);
    }

    #[test]
    fn test_sub() {
        let a = Vec4::new(5.0, 6.0, 7.0, 8.0);
        let b = Vec4::new(1.0, 2.0, 3.0, 4.0);
        let c = a - b;
        assert_eq!(c.x, 4.0);
        assert_eq!(c.y, 4.0);
        assert_eq!(c.z, 4.0);
        assert_eq!(c.w, 4.0);
    }

    #[test]
    fn test_mul_scalar() {
        let v = Vec4::new(1.0, 2.0, 3.0, 4.0);
        let scaled = v * 2.0;
        assert_eq!(scaled.x, 2.0);
        assert_eq!(scaled.y, 4.0);
        assert_eq!(scaled.z, 6.0);
        assert_eq!(scaled.w, 8.0);
    }

    #[test]
    fn test_neg() {
        let v = Vec4::new(1.0, -2.0, 3.0, -4.0);
        let neg = -v;
        assert_eq!(neg.x, -1.0);
        assert_eq!(neg.y, 2.0);
        assert_eq!(neg.z, -3.0);
        assert_eq!(neg.w, 4.0);
    }

    #[test]
    fn test_xyz() {
        let v = Vec4::new(1.0, 2.0, 3.0, 4.0);
        let xyz = v.xyz();
        assert_eq!(xyz, [1.0, 2.0, 3.0]);
    }

    #[test]
    fn test_lerp() {
        let a = Vec4::new(0.0, 0.0, 0.0, 0.0);
        let b = Vec4::new(10.0, 10.0, 10.0, 10.0);
        let mid = a.lerp(b, 0.5);
        assert_eq!(mid.x, 5.0);
        assert_eq!(mid.y, 5.0);
        assert_eq!(mid.z, 5.0);
        assert_eq!(mid.w, 5.0);
    }
}
