//! Deterministic ballistics for COLLATERAL.
//!
//! The plan requires every shot to be resolved before it is shown, and to
//! resolve identically every time. That rules out frame-rate-dependent
//! integration and any randomness in the solver, so this crate steps the
//! trajectory at a fixed 1 kHz with fixed-size arithmetic in `f64` and never
//! consults a clock.
//!
//! The model is deliberately stylised, not a G7 table: quadratic drag from a
//! single coefficient per weapon, constant gravity, a uniform wind field and
//! the Coriolis term for the local latitude. Constants such as the drag scale
//! are tuning values chosen to give plausible long-range behaviour, not
//! measured ballistic data.
//!
//! Frame of reference is ENU: `+x` east, `+y` north, `+z` up, in metres.

pub mod atmosphere;
pub mod trajectory;
pub mod vector;
pub mod weapon;
pub mod zero;

pub use atmosphere::Atmosphere;
pub use trajectory::{Launch, Sample, Trajectory, STEP_SECONDS};
pub use vector::Vec3;
pub use weapon::WeaponSpec;
pub use zero::{STANDING_HEIGHT, ZeroSolution, solve_zero};
