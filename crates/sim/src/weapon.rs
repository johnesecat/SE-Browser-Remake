//! The weapon's ballistic description.

/// One rifle's tuning values. All figures are SI.
///
/// `drag_per_meter` is the coefficient `k` in `a = -k * density_ratio * |v| * v`,
/// so drag grows with the square of speed and the atmosphere scales it. It is a
/// single-number stand-in for a published drag table, chosen so that long-range
/// shots lose speed at a plausible rate; it is not measured data.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WeaponSpec {
    pub name: &'static str,
    /// Speed at the muzzle, metres per second.
    pub muzzle_velocity: f64,
    /// Bullet mass, kilograms, used for kinetic energy.
    pub mass_kg: f64,
    /// Quadratic drag scale in 1/m at sea-level density.
    pub drag_per_meter: f64,
    /// Scope height above the bore, metres. Sets the line of sight.
    pub sight_height: f64,
}

impl WeaponSpec {
    /// A long-range anti-materiel-style rifle, the M0–M2 reference weapon.
    pub const fn long_range_rifle() -> Self {
        Self {
            name: "long-range rifle",
            muzzle_velocity: 900.0,
            mass_kg: 0.0162,
            drag_per_meter: 0.0011,
            sight_height: 0.07,
        }
    }

    /// Muzzle kinetic energy in joules.
    pub fn muzzle_energy(&self) -> f64 {
        0.5 * self.mass_kg * self.muzzle_velocity * self.muzzle_velocity
    }
}

#[cfg(test)]
mod tests {
    use super::WeaponSpec;

    #[test]
    fn muzzle_energy_matches_the_kinetic_energy_formula() {
        let weapon = WeaponSpec::long_range_rifle();
        let expected = 0.5 * weapon.mass_kg * weapon.muzzle_velocity.powi(2);
        assert!((weapon.muzzle_energy() - expected).abs() < 1e-9);
        // A .338-class round at 900 m/s carries roughly this much energy.
        assert!((6500.0..7000.0).contains(&weapon.muzzle_energy()));
    }
}
