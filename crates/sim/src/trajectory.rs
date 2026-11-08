//! Fixed-step trajectory integration.

use crate::atmosphere::Atmosphere;
use crate::vector::Vec3;
use crate::weapon::WeaponSpec;

/// Integration step: exactly 1 kHz, as the plan specifies.
pub const STEP_SECONDS: f64 = 0.001;

/// How long a single shot may be integrated. A long-range round that has not
/// landed in this time has been fired into the sky or off the map.
const MAX_FLIGHT_SECONDS: f64 = 12.0;

/// Ground plane. The range scene and the zeroing solver share it.
pub const GROUND_HEIGHT: f64 = 0.0;

/// A shot's starting state. `direction` need not be normalised.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Launch {
    pub origin: Vec3,
    pub direction: Vec3,
    pub speed: f64,
}

impl Launch {
    pub fn new(origin: Vec3, direction: Vec3, speed: f64) -> Self {
        Self {
            origin,
            direction,
            speed,
        }
    }
}

/// One integration step's result. Positions are absolute; heights are taken
/// relative to the ground plane at `z = 0` by the range scene.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Sample {
    pub time: f64,
    pub position: Vec3,
    pub velocity: Vec3,
    /// Kinetic energy in joules.
    pub energy: f64,
}

impl Sample {
    pub fn speed(&self) -> f64 {
        self.velocity.length()
    }

    /// Height above the ground plane, metres.
    pub fn height(&self) -> f64 {
        self.position.z
    }
}

/// A resolved flight: one sample per millisecond from the muzzle until the
/// round reaches the ground or the flight limit.
#[derive(Clone, Debug, PartialEq)]
pub struct Trajectory {
    pub weapon: WeaponSpec,
    pub samples: Vec<Sample>,
    /// True when the round landed rather than being cut off by the time limit.
    pub landed: bool,
}

impl Trajectory {
    /// Resolve a shot. The same inputs always produce bit-identical samples.
    pub fn resolve(weapon: WeaponSpec, air: Atmosphere, launch: Launch) -> Self {
        let Some(direction) = launch.direction.direction() else {
            return Self {
                weapon,
                samples: Vec::new(),
                landed: false,
            };
        };
        let flight = Flight { weapon, air };
        let mut samples = Vec::with_capacity(4096);
        let mut position = launch.origin;
        let mut velocity = direction * launch.speed;
        samples.push(flight.sample(0.0, position, velocity));

        let steps = (MAX_FLIGHT_SECONDS / STEP_SECONDS) as usize;
        let mut landed = false;
        for step in 1..=steps {
            let (next_position, next_velocity) = flight.step(position, velocity);
            let time = step as f64 * STEP_SECONDS;
            // Interpolate the ground crossing so the impact lands exactly on the
            // ground plane instead of up to one step below it.
            if next_position.z <= GROUND_HEIGHT && position.z >= GROUND_HEIGHT {
                let fraction = position.z / (position.z - next_position.z);
                position = position + (next_position - position) * fraction;
                velocity = velocity + (next_velocity - velocity) * fraction;
                samples.push(flight.sample(time - STEP_SECONDS * (1.0 - fraction), position, velocity));
                landed = true;
                break;
            }
            position = next_position;
            velocity = next_velocity;
            samples.push(flight.sample(time, position, velocity));
        }

        Self {
            weapon,
            samples,
            landed,
        }
    }

    /// The sample at a horizontal distance from the origin, interpolated
    /// between steps. Used by the zeroing solver and the range readout.
    pub fn at_horizontal_distance(&self, distance: f64) -> Option<Sample> {
        let first = self.samples.first()?;
        let origin = first.position;
        let reach = |sample: &Sample| (sample.position - origin).horizontal_length();
        let last = self.samples.last()?;
        if distance < 0.0 || distance > reach(last) {
            return None;
        }
        // Samples are ordered by time, and a shot only ever moves downrange.
        let index = self.samples.partition_point(|sample| reach(sample) < distance);
        if index == 0 {
            return Some(*first);
        }
        let before = &self.samples[index - 1];
        let before_reach = reach(before);
        let after = &self.samples[index];
        let span = reach(after) - before_reach;
        let fraction = if span > 0.0 {
            (distance - before_reach) / span
        } else {
            0.0
        };
        Some(Sample {
            time: before.time + (after.time - before.time) * fraction,
            position: before.position + (after.position - before.position) * fraction,
            velocity: before.velocity + (after.velocity - before.velocity) * fraction,
            energy: before.energy + (after.energy - before.energy) * fraction,
        })
    }

    /// Time of flight in seconds.
    pub fn duration(&self) -> f64 {
        self.samples.last().map_or(0.0, |sample| sample.time)
    }
}

struct Flight {
    weapon: WeaponSpec,
    air: Atmosphere,
}

impl Flight {
    fn sample(&self, time: f64, position: Vec3, velocity: Vec3) -> Sample {
        Sample {
            time,
            position,
            velocity,
            energy: 0.5 * self.weapon.mass_kg * velocity.dot(velocity),
        }
    }

    fn acceleration(&self, velocity: Vec3) -> Vec3 {
        // Drag acts on the bullet's motion through the air, so the wind changes
        // the relative velocity rather than adding a force of its own.
        let relative = velocity - self.air.wind;
        let drag = relative * (-self.weapon.drag_per_meter * self.air.density_ratio * relative.length());
        let gravity = Vec3::new(0.0, 0.0, -self.air.gravity);
        // Coriolis: -2 Ω × v, using the velocity through the rotating frame.
        let coriolis = self.air.earth_rotation().cross(relative) * -2.0;
        drag + gravity + coriolis
    }

    /// Fourth-order Runge-Kutta over one step. Acceleration depends only on
    /// velocity here, which keeps the stages cheap.
    fn step(&self, position: Vec3, velocity: Vec3) -> (Vec3, Vec3) {
        let half = STEP_SECONDS * 0.5;
        let a1 = self.acceleration(velocity);
        let v2 = velocity + a1 * half;
        let a2 = self.acceleration(v2);
        let v3 = velocity + a2 * half;
        let a3 = self.acceleration(v3);
        let v4 = velocity + a3 * STEP_SECONDS;
        let a4 = self.acceleration(v4);

        let delta_velocity = (a1 + a2 * 2.0 + a3 * 2.0 + a4) * (STEP_SECONDS / 6.0);
        let delta_position = (velocity + v2 * 2.0 + v3 * 2.0 + v4) * (STEP_SECONDS / 6.0);
        (position + delta_position, velocity + delta_velocity)
    }
}

/// Absolute height at a horizontal distance from the origin, integrating without
/// keeping samples. The zeroing solver calls this repeatedly.
///
/// `None` means the launch is degenerate or never covers the distance; a round
/// that lands short returns `Some(f64::NEG_INFINITY)` so the solver can keep
/// treating the shot as low instead of failing.
pub(crate) fn crossing_height(weapon: WeaponSpec, air: Atmosphere, launch: Launch, distance: f64) -> Option<f64> {
    let direction = launch.direction.direction()?;
    let flight = Flight { weapon, air };
    let mut position = launch.origin;
    let mut velocity = direction * launch.speed;
    let previous_height = position.z;
    let steps = (MAX_FLIGHT_SECONDS / STEP_SECONDS) as usize;

    for _ in 0..steps {
        let (next_position, next_velocity) = flight.step(position, velocity);
        let reached = (next_position - launch.origin).horizontal_length() >= distance;
        if reached {
            let before = (position - launch.origin).horizontal_length();
            let span = (next_position - launch.origin).horizontal_length() - before;
            let fraction = if span > 0.0 { (distance - before) / span } else { 0.0 };
            return Some(position.z + (next_position.z - position.z) * fraction);
        }
        if next_position.z <= GROUND_HEIGHT && previous_height >= GROUND_HEIGHT {
            return Some(f64::NEG_INFINITY);
        }
        position = next_position;
        velocity = next_velocity;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::{Launch, STEP_SECONDS, Trajectory};
    use crate::atmosphere::Atmosphere;
    use crate::vector::Vec3;
    use crate::weapon::WeaponSpec;

    fn rifle() -> WeaponSpec {
        WeaponSpec::long_range_rifle()
    }

    /// Standing height of the bore, so a level shot has room to fall.
    const FIRING_HEIGHT: f64 = 1.5;

    fn level_shot(air: Atmosphere) -> Trajectory {
        Trajectory::resolve(
            rifle(),
            air,
            Launch::new(
                Vec3::new(0.0, 0.0, FIRING_HEIGHT),
                Vec3::new(1.0, 0.0, 0.0),
                rifle().muzzle_velocity,
            ),
        )
    }

    #[test]
    fn samples_are_one_millisecond_apart() {
        let trajectory = level_shot(Atmosphere::still(45.0));
        assert!(trajectory.samples.len() > 100);
        let pairs = trajectory.samples.windows(2);
        let last = pairs.len() - 1;
        for (index, pair) in pairs.enumerate() {
            let spacing = pair[1].time - pair[0].time;
            if index == last {
                // The impact sample is interpolated inside the final step.
                assert!(spacing > 0.0 && spacing <= STEP_SECONDS + 1e-12);
            } else {
                assert!((spacing - STEP_SECONDS).abs() < 1e-12, "spacing was {spacing}");
            }
        }
    }

    #[test]
    fn without_air_the_flight_is_the_exact_parabola() {
        // At the pole the rotation axis is vertical, so the only acceleration
        // left is gravity: the Eötvös term would otherwise tilt the parabola.
        let air = Atmosphere::vacuum(90.0);
        let trajectory = level_shot(air);
        for sample in &trajectory.samples {
            let fall = -0.5 * air.gravity * sample.time * sample.time;
            let expected = FIRING_HEIGHT + fall;
            assert!(
                (sample.height() - expected).abs() < 1e-6,
                "at {} s height was {} not {expected}",
                sample.time,
                sample.height()
            );
        }
    }

    #[test]
    fn drag_costs_speed_and_range() {
        let vacuum = level_shot(Atmosphere::vacuum(45.0));
        let real = level_shot(Atmosphere::still(45.0));
        // Drag costs range, not flight time: the fall from the same height takes
        // about as long, while the round covers less ground doing it.
        let range_of = |trajectory: &Trajectory| {
            let last = trajectory.samples.last().expect("samples");
            last.position.horizontal_length()
        };
        assert!(
            range_of(&real) < range_of(&vacuum) * 0.9,
            "drag must shorten the range: {} vs {}",
            range_of(&real),
            range_of(&vacuum)
        );
        let vacuum_end = vacuum.samples.last().expect("samples").speed();
        let real_end = real.samples.last().expect("samples").speed();
        assert!(real_end < vacuum_end);
        // Roughly a fifth to half of the muzzle speed should be gone by impact.
        assert!(
            (0.4..0.85).contains(&(real_end / rifle().muzzle_velocity)),
            "impact speed ratio was {}",
            real_end / rifle().muzzle_velocity
        );
    }

    #[test]
    fn energy_only_falls_when_the_air_is_still() {
        let trajectory = level_shot(Atmosphere::still(45.0));
        for pair in trajectory.samples.windows(2) {
            assert!(
                pair[1].energy <= pair[0].energy,
                "energy rose from {} to {} at {} s",
                pair[0].energy,
                pair[1].energy,
                pair[1].time
            );
        }
    }

    #[test]
    fn crosswind_pushes_the_round_downwind() {
        let wind = Vec3::new(0.0, 8.0, 0.0);
        let still = level_shot(Atmosphere::still(45.0));
        let blown = level_shot(Atmosphere::still(45.0).with_wind(wind));

        // The no-wind shot still drifts from Coriolis, so compare the two.
        let sideways = |trajectory: &Trajectory, distance: f64| {
            trajectory
                .at_horizontal_distance(distance)
                .expect("a sample at that range")
                .position
                .y
        };
        let wind_drift = |distance: f64| sideways(&blown, distance) - sideways(&still, distance);
        assert!(wind_drift(300.0) > 0.01, "wind from the north pushes south");
        // Wind drift grows with distance, so it is larger further out.
        assert!(wind_drift(300.0) > wind_drift(150.0));
        assert!(wind_drift(150.0) > 0.0);
    }

    #[test]
    fn coriolis_deflects_right_in_the_northern_hemisphere() {
        // Facing east, the right hand points south (-y).
        let north = level_shot(Atmosphere::vacuum(45.0).with_wind(Vec3::ZERO));
        let equator = level_shot(Atmosphere::vacuum(0.0));
        let south = level_shot(Atmosphere::vacuum(-45.0));

        let lateral = |trajectory: &Trajectory| {
            trajectory
                .at_horizontal_distance(500.0)
                .expect("a sample at 500 m")
                .position
                .y
        };
        assert!(lateral(&north) < -0.01, "northern shot must drift south");
        assert!(lateral(&south) > 0.01, "southern shot must drift north");
        assert!(
            lateral(&equator).abs() < lateral(&north).abs() * 0.01,
            "a level shot at the equator has no lateral Coriolis drift"
        );
    }

    #[test]
    fn eastward_shots_lose_apparent_weight_at_the_equator() {
        // Eötvös: firing east lifts the round, firing west drops it.
        let air = Atmosphere::vacuum(0.0);
        let east = level_shot(air);
        let west = Trajectory::resolve(
            rifle(),
            air,
            Launch::new(
                Vec3::new(0.0, 0.0, FIRING_HEIGHT),
                Vec3::new(-1.0, 0.0, 0.0),
                rifle().muzzle_velocity,
            ),
        );
        let at_quarter_second = |trajectory: &Trajectory| {
            trajectory
                .samples
                .iter()
                .find(|sample| (sample.time - 0.4).abs() < 1e-9)
                .expect("sample at 0.4 s")
                .position
                .z
        };
        assert!(at_quarter_second(&east) > at_quarter_second(&west));
    }

    #[test]
    fn a_thousand_replays_are_bit_identical() {
        let air = Atmosphere::still(51.5).with_wind(Vec3::new(-3.5, 6.0, 0.25));
        let launch = Launch::new(
            Vec3::new(2.0, -1.0, 1.6),
            Vec3::new(0.999, 0.02, 0.04),
            rifle().muzzle_velocity,
        );
        let reference = Trajectory::resolve(rifle(), air, launch);
        assert!(reference.samples.len() > 500);
        let fingerprint: Vec<[u64; 3]> = reference
            .samples
            .iter()
            .map(|sample| sample.position.bits())
            .collect();
        for replay in 1..1000 {
            let again = Trajectory::resolve(rifle(), air, launch);
            let again_fingerprint: Vec<[u64; 3]> =
                again.samples.iter().map(|sample| sample.position.bits()).collect();
            assert_eq!(again_fingerprint, fingerprint, "replay {replay} diverged");
        }
    }

    #[test]
    fn the_round_lands_on_the_ground_plane() {
        let trajectory = level_shot(Atmosphere::still(45.0));
        assert!(trajectory.landed, "a level shot must come down");
        let impact = trajectory.samples.last().expect("samples");
        assert!(impact.height().abs() < 1e-9, "impact height was {}", impact.height());
        assert!(trajectory.samples.iter().all(|sample| sample.position.is_finite()));
    }

    #[test]
    fn a_degenerate_launch_resolves_to_nothing() {
        let trajectory = Trajectory::resolve(
            rifle(),
            Atmosphere::still(45.0),
            Launch::new(Vec3::ZERO, Vec3::ZERO, 900.0),
        );
        assert!(trajectory.samples.is_empty());
        assert_eq!(trajectory.duration(), 0.0);
        assert_eq!(trajectory.at_horizontal_distance(10.0), None);
    }
}
