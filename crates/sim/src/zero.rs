//! Zeroing: find the elevation that puts the round on the line of sight.

use crate::atmosphere::Atmosphere;
use crate::trajectory::{Launch, Sample, Trajectory, crossing_height};
use crate::vector::Vec3;
use crate::weapon::WeaponSpec;

/// Bore height for a standing shooter, metres. The range scene shoots from here.
pub const STANDING_HEIGHT: f64 = 1.5;

/// A rifle zeroed at a range, ready to be fired.
#[derive(Clone, Debug, PartialEq)]
pub struct ZeroSolution {
    /// Horizontal distance where the sight and the trajectory agree, metres.
    pub range: f64,
    /// Elevation above the line of sight, radians. This is the "come-up".
    pub come_up: f64,
    /// Pitch of the line of sight itself, radians (negative for a downward aim).
    pub line_of_sight: f64,
    /// Launch that makes the round cross the line of sight at `range`.
    pub launch: Launch,
    /// The resolved flight for that launch.
    pub trajectory: Trajectory,
}

/// Zero at `range` metres from a bore at `origin`, aiming east at a target on
/// the ground plane.
///
/// The sight sits `weapon.sight_height` above the bore, so the line of sight
/// starts slightly above it and ends at the target; the solver returns the angle
/// added on top of that line. Bisection is used because the crossing height
/// rises monotonically with elevation, which makes the result reproducible and
/// independent of any starting guess.
pub fn solve_zero(weapon: WeaponSpec, air: Atmosphere, origin: Vec3, range: f64) -> Option<ZeroSolution> {
    if !(range.is_finite() && range > 0.0) || !origin.is_finite() {
        return None;
    }
    let sight_origin = origin.z + weapon.sight_height;
    let line_of_sight = (-sight_origin / range).atan();
    let launch_at = |come_up: f64| {
        let pitch = line_of_sight + come_up;
        Launch::new(
            origin,
            Vec3::new(pitch.cos(), 0.0, pitch.sin()),
            weapon.muzzle_velocity,
        )
    };
    let height_at = |come_up: f64| crossing_height(weapon, air, launch_at(come_up), range);

    // Bracket the root. The lower bound aims at the ground, the upper one well
    // above any practical come-up; the bracket is widened until it straddles the
    // target height, which is the ground plane in absolute coordinates.
    let mut low = -0.01;
    let mut high = 0.01;
    while height_at(high)? < 0.0 {
        high *= 2.0;
        if high > 1.0 {
            return None;
        }
    }
    while height_at(low)? > 0.0 {
        low *= 2.0;
        if low < -1.0 {
            return None;
        }
    }

    // 80 halvings take the interval below f64 resolution; a fixed count keeps
    // the answer identical on every run rather than depending on a tolerance.
    for _ in 0..80 {
        let middle = 0.5 * (low + high);
        if height_at(middle)? > 0.0 {
            high = middle;
        } else {
            low = middle;
        }
    }
    let come_up = 0.5 * (low + high);
    let launch = launch_at(come_up);
    let trajectory = Trajectory::resolve(weapon, air, launch);
    Some(ZeroSolution {
        range,
        come_up,
        line_of_sight,
        launch,
        trajectory,
    })
}

impl ZeroSolution {
    /// Height of the sight line at a distance, metres above the ground.
    fn sight_line(&self, distance: f64) -> f64 {
        let start = self.launch.origin.z + self.trajectory.weapon.sight_height;
        start + (0.0 - start) * (distance / self.range)
    }

    /// Sample at a horizontal distance.
    ///
    /// A zeroed shot reaches the ground at the zero range, so its flight ends a
    /// few micrometres short of it after the impact interpolation. That landing
    /// sample is the crossing a caller asking for the zero range wants.
    fn sample_at(&self, distance: f64) -> Option<Sample> {
        if let Some(sample) = self.trajectory.at_horizontal_distance(distance) {
            return Some(sample);
        }
        let impact = *self.trajectory.samples.last()?;
        let reach = (impact.position - self.launch.origin).horizontal_length();
        (distance - reach).abs().lt(&0.01).then_some(impact)
    }

    /// Height above the line of sight at a horizontal distance, metres.
    /// Negative values shoot low.
    pub fn height_above_sight(&self, distance: f64) -> Option<f64> {
        let sample = self.sample_at(distance)?;
        Some(sample.height() - self.sight_line(distance))
    }

    /// The distance where the round is highest above the line of sight, the
    /// "maximum ordinate" a shooter uses to clear cover.
    pub fn peak_above_sight(&self) -> Option<(f64, f64)> {
        let mut best: Option<(f64, f64)> = None;
        for step in 0..=200 {
            let distance = self.range * step as f64 / 200.0;
            if let Some(height) = self.height_above_sight(distance)
                && best.is_none_or(|(_, highest)| height > highest)
            {
                best = Some((distance, height));
            }
        }
        best
    }

    /// Time of flight at a distance, for the range card.
    pub fn flight_time(&self, distance: f64) -> Option<f64> {
        self.sample_at(distance).map(|sample| sample.time)
    }
}

#[cfg(test)]
mod tests {
    use super::{STANDING_HEIGHT, solve_zero};
    use crate::atmosphere::Atmosphere;
    use crate::vector::Vec3;
    use crate::weapon::WeaponSpec;

    fn rifle() -> WeaponSpec {
        WeaponSpec::long_range_rifle()
    }

    fn origin() -> Vec3 {
        Vec3::new(0.0, 0.0, STANDING_HEIGHT)
    }

    #[test]
    fn a_zeroed_shot_crosses_the_line_of_sight_at_the_zero_range() {
        for range in [100.0, 300.0, 600.0, 1000.0] {
            let zero = solve_zero(rifle(), Atmosphere::still(45.0), origin(), range).expect("a solution");
            let height = zero
                .height_above_sight(range)
                .expect("a sample at the zero range");
            assert!(
                height.abs() < 1e-3,
                "at {range} m the round was {height} m off the sight"
            );
        }
    }

    #[test]
    fn a_longer_zero_needs_more_come_up() {
        let air = Atmosphere::still(45.0);
        let near = solve_zero(rifle(), air, origin(), 200.0).expect("solution").come_up;
        let far = solve_zero(rifle(), air, origin(), 800.0).expect("solution").come_up;
        assert!(far > near, "come-up {far} should exceed {near}");
        // A 900 m/s round needs single-digit milliradians at 200 m and tens at 800 m.
        assert!((0.0005..0.02).contains(&near), "near come-up {near}");
        assert!((0.005..0.06).contains(&far), "far come-up {far}");
    }

    #[test]
    fn the_line_of_sight_points_slightly_down_at_a_ground_target() {
        let zero = solve_zero(rifle(), Atmosphere::still(45.0), origin(), 100.0).expect("solution");
        assert!(zero.line_of_sight < 0.0);
        // 1.57 m of sight over 100 m is about 15.7 mrad below horizontal.
        assert!((zero.line_of_sight + 0.0157).abs() < 0.0005, "{}", zero.line_of_sight);
    }

    #[test]
    fn the_round_rises_above_the_sight_before_it_comes_back_down() {
        let zero = solve_zero(rifle(), Atmosphere::still(45.0), origin(), 600.0).expect("solution");
        let (distance, height) = zero.peak_above_sight().expect("a peak");
        assert!(height > 0.0, "peak height {height}");
        assert!(distance > 0.0 && distance < zero.range);
        // Mid-range rise for a 600 m zero is metres, not centimetres.
        assert!((0.5..5.0).contains(&height), "peak height {height}");
        assert!(zero.flight_time(600.0).expect("time") > 0.4);
    }

    #[test]
    fn a_round_inside_the_zero_range_shoots_high() {
        let zero = solve_zero(rifle(), Atmosphere::still(45.0), origin(), 600.0).expect("solution");
        let close = zero.height_above_sight(300.0).expect("a sample at 300 m");
        assert!(close > 0.0, "a shot inside the zero should land high, was {close}");
    }

    #[test]
    fn zeroing_is_reproducible() {
        let air = Atmosphere::still(51.5).with_wind(Vec3::new(4.0, 0.0, 0.0));
        let first = solve_zero(rifle(), air, origin(), 500.0).expect("solution");
        let second = solve_zero(rifle(), air, origin(), 500.0).expect("solution");
        assert_eq!(first.come_up.to_bits(), second.come_up.to_bits());
        assert_eq!(first.trajectory.samples.len(), second.trajectory.samples.len());
        assert_eq!(
            first.launch, second.launch,
            "the solver must not depend on iteration order"
        );
    }

    #[test]
    fn impossible_ranges_have_no_solution() {
        let air = Atmosphere::still(45.0);
        assert!(solve_zero(rifle(), air, origin(), 0.0).is_none());
        assert!(solve_zero(rifle(), air, origin(), -5.0).is_none());
        assert!(solve_zero(rifle(), air, origin(), f64::NAN).is_none());
    }
}
