//! Minimal 3D vector in `f64`, large enough for the simulation and nothing more.

use std::ops::{Add, Mul, Sub};

/// World frame: `+x` east, `+y` north, `+z` up.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Vec3 {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl Vec3 {
    pub const ZERO: Self = Self::new(0.0, 0.0, 0.0);

    pub const fn new(x: f64, y: f64, z: f64) -> Self {
        Self { x, y, z }
    }

    pub fn dot(self, other: Self) -> f64 {
        self.x * other.x + self.y * other.y + self.z * other.z
    }

    pub fn cross(self, other: Self) -> Self {
        Self::new(
            self.y * other.z - self.z * other.y,
            self.z * other.x - self.x * other.z,
            self.x * other.y - self.y * other.x,
        )
    }

    pub fn length(self) -> f64 {
        self.dot(self).sqrt()
    }

    /// Distance in the horizontal plane, which is what range means for a shot.
    pub fn horizontal_length(self) -> f64 {
        (self.x * self.x + self.y * self.y).sqrt()
    }

    pub fn is_finite(self) -> bool {
        self.x.is_finite() && self.y.is_finite() && self.z.is_finite()
    }

    /// Direction of the vector, or `None` when it has no length.
    pub fn direction(self) -> Option<Self> {
        let length = self.length();
        (length > 0.0 && length.is_finite()).then(|| self * (1.0 / length))
    }

    /// Bit pattern of every component, for exact replay comparison.
    pub fn bits(self) -> [u64; 3] {
        [self.x.to_bits(), self.y.to_bits(), self.z.to_bits()]
    }
}

impl Add for Vec3 {
    type Output = Self;
    fn add(self, other: Self) -> Self {
        Self::new(self.x + other.x, self.y + other.y, self.z + other.z)
    }
}

impl Sub for Vec3 {
    type Output = Self;
    fn sub(self, other: Self) -> Self {
        Self::new(self.x - other.x, self.y - other.y, self.z - other.z)
    }
}

impl Mul<f64> for Vec3 {
    type Output = Self;
    fn mul(self, scale: f64) -> Self {
        Self::new(self.x * scale, self.y * scale, self.z * scale)
    }
}

#[cfg(test)]
mod tests {
    use super::Vec3;

    #[test]
    fn cross_product_follows_the_right_hand_rule() {
        let east = Vec3::new(1.0, 0.0, 0.0);
        let north = Vec3::new(0.0, 1.0, 0.0);
        assert_eq!(east.cross(north), Vec3::new(0.0, 0.0, 1.0));
        assert_eq!(north.cross(east), Vec3::new(0.0, 0.0, -1.0));
    }

    #[test]
    fn direction_is_unit_length_and_absent_for_zero() {
        let unit = Vec3::new(3.0, 4.0, 0.0).direction().expect("nonzero");
        assert!((unit.length() - 1.0).abs() < 1e-15);
        assert_eq!(Vec3::ZERO.direction(), None);
    }
}
