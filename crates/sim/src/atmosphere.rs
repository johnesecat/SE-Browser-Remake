//! Air, wind, gravity and the Earth's rotation.

use crate::vector::Vec3;

/// Earth's sidereal rotation rate, radians per second.
const EARTH_OMEGA: f64 = 7.292_115e-5;

/// Conditions a shot is fired in. Uniform over the whole flight, which is all a
/// first-hit prediction needs; layered wind is a later refinement.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Atmosphere {
    /// Air density as a multiple of sea-level density (1.0 = 1.225 kg/m³).
    pub density_ratio: f64,
    /// Wind velocity in ENU metres per second. Downrange is `+x` for a level shot.
    pub wind: Vec3,
    /// Gravity, metres per second squared.
    pub gravity: f64,
    /// Latitude in degrees, north positive. Drives the Coriolis deflection.
    pub latitude_deg: f64,
}

impl Atmosphere {
    /// Sea-level density, no wind, standard gravity, at the given latitude.
    pub const fn still(latitude_deg: f64) -> Self {
        Self {
            density_ratio: 1.0,
            wind: Vec3::ZERO,
            gravity: 9.806_65,
            latitude_deg,
        }
    }

    /// Air with no drag at all, for checking the integrator against an exact
    /// parabola. Wind and rotation stay in the model, so this is not simply
    /// "everything switched off".
    pub const fn vacuum(latitude_deg: f64) -> Self {
        Self {
            density_ratio: 0.0,
            ..Self::still(latitude_deg)
        }
    }

    pub fn with_wind(mut self, wind: Vec3) -> Self {
        self.wind = wind;
        self
    }

    /// Earth's angular velocity in the local ENU frame.
    ///
    /// The axis points at the celestial pole, so in local coordinates it is
    /// `(0, cos φ, sin φ)` scaled by the rotation rate.
    pub fn earth_rotation(&self) -> Vec3 {
        let latitude = self.latitude_deg.to_radians();
        Vec3::new(0.0, EARTH_OMEGA * latitude.cos(), EARTH_OMEGA * latitude.sin())
    }
}

#[cfg(test)]
mod tests {
    use super::Atmosphere;

    #[test]
    fn the_rotation_axis_is_horizontal_at_the_equator_and_vertical_at_the_pole() {
        let equator = Atmosphere::still(0.0).earth_rotation();
        assert!(equator.z.abs() < 1e-20);
        assert!((equator.y - 7.292_115e-5).abs() < 1e-18);

        let pole = Atmosphere::still(90.0).earth_rotation();
        assert!(pole.y.abs() < 1e-20);
        assert!((pole.z - 7.292_115e-5).abs() < 1e-18);
    }

    #[test]
    fn vacuum_has_no_air_but_keeps_gravity() {
        let air = Atmosphere::vacuum(45.0);
        assert_eq!(air.density_ratio, 0.0);
        assert!(air.gravity > 9.0);
    }
}
