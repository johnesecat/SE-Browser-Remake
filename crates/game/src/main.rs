//! COLLATERAL — the range.
//!
//! Every shot is resolved by `collateral-sim` before it is drawn; this crate
//! only presents the resolved flight. Nothing here changes where the round goes.
use bevy::{
    prelude::*,
    render::view::{Msaa, Tonemapping},
};
#[cfg(target_arch = "wasm32")]
use bevy::{
    render::error_handler::{RenderErrorHandler, RenderErrorPolicy},
    time::Real,
};

use collateral_sim::{Atmosphere, STANDING_HEIGHT, Trajectory, Vec3 as SimVec3, WeaponSpec, solve_zero};

/// The range is zeroed here, in metres.
const ZERO_RANGE: f64 = 300.0;
/// Firing point latitude, degrees north. Drives the Coriolis deflection.
const LATITUDE: f64 = 51.5;
/// Real flight time is under a second; slow it down only for watching.
const SLOW_MOTION: f64 = 0.18;
/// Pause between tracer passes, seconds of wall time.
const TRACER_PAUSE: f64 = 0.8;

#[derive(Resource)]
struct Shot {
    flight: Trajectory,
    /// First sample the on-screen tracer follows. A marker a few centimetres from
    /// the camera would cover the whole view, so the pass starts downrange.
    start_index: usize,
    zero_range: f64,
    come_up: f64,
    impact_speed: f64,
    impact_energy: f64,
}

impl Shot {
    /// The range card, shared by the startup log and the on-screen diagnostics.
    fn summary(&self) -> String {
        format!(
            "ZERO {:.0} m\nCOME-UP {:.2} mrad\nFLIGHT {:.3} s\nIMPACT {:.0} m/s / {:.0} J\nSAMPLES {}",
            self.zero_range,
            self.come_up * 1000.0,
            self.flight.duration(),
            self.impact_speed,
            self.impact_energy,
            self.flight.samples.len()
        )
    }
}

#[derive(Component)]
struct Tracer;

#[derive(Component)]
struct MuzzleFlash;

/// Simulation frame (east, north, up) to Bevy's (right, up, towards viewer).
/// Downrange is Bevy `-Z`, so the camera looks the way the round travels.
fn to_world(point: SimVec3) -> Vec3 {
    Vec3::new(point.y as f32, point.z as f32, -(point.x as f32))
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "COLLATERAL — range".into(),
                canvas: Some("#game".into()),
                fit_canvas_to_parent: true,
                prevent_default_event_handling: false,
                ..default()
            }),
            ..default()
        }))
        .add_plugins(PlatformStatusPlugin)
        .insert_resource(ClearColor(Color::srgb(0.52, 0.64, 0.76)))
        .add_systems(Startup, setup)
        .add_systems(Update, (draw_flight, advance_tracer, breathe_camera))
        .add_systems(Update, update_diagnostics)
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let weapon = WeaponSpec::long_range_rifle();
    let air = Atmosphere::still(LATITUDE);
    let origin = SimVec3::new(0.0, 0.0, STANDING_HEIGHT);
    let zero = solve_zero(weapon, air, origin, ZERO_RANGE).expect("a 300 m zero is solvable");
    let impact = *zero.trajectory.samples.last().expect("the flight reaches the target");

    let flight = zero.trajectory.clone();
    let start_index = flight
        .samples
        .iter()
        .position(|sample| sample.position.horizontal_length() >= 25.0)
        .unwrap_or(0);
    let shot = Shot {
        flight,
        start_index,
        zero_range: ZERO_RANGE,
        come_up: zero.come_up,
        impact_speed: impact.speed(),
        impact_energy: impact.energy,
    };
    println!("{}", shot.summary());
    commands.insert_resource(shot);

    let ground = materials.add(StandardMaterial {
        base_color: Color::srgb(0.22, 0.28, 0.17),
        perceptual_roughness: 0.95,
        ..default()
    });
    let marker = materials.add(StandardMaterial {
        base_color: Color::srgb(0.85, 0.84, 0.78),
        perceptual_roughness: 0.8,
        ..default()
    });
    let target = materials.add(StandardMaterial {
        base_color: Color::srgb(0.88, 0.87, 0.82),
        perceptual_roughness: 0.7,
        ..default()
    });
    let tracer = materials.add(StandardMaterial {
        base_color: Color::srgb(1.0, 0.72, 0.18),
        emissive: LinearRgba::rgb(6.0, 3.0, 0.4),
        ..default()
    });

    // Ground plane, long enough to reach past the target and wide enough to read
    // as a field rather than a strip.
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(60.0, 0.2, 700.0))),
        MeshMaterial3d(ground.clone()),
        Transform::from_xyz(0.0, -0.1, -320.0),
    ));

    // Lane markers every 50 m give the range a sense of depth and speed.
    for step in 1..=6 {
        let distance = 50.0 * step as f32;
        for side in [-1.0, 1.0] {
            commands.spawn((
                Mesh3d(meshes.add(Cuboid::new(0.12, 1.2, 0.12))),
                MeshMaterial3d(marker.clone()),
                Transform::from_xyz(side * 4.0, 0.6, -distance),
            ));
        }
    }

    // A berm behind the target so the plate reads against a solid backdrop.
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(40.0, 8.0, 6.0))),
        MeshMaterial3d(ground.clone()),
        Transform::from_xyz(0.0, 4.0, -330.0),
    ));

    // Scattered cover gives the field depth. Positions come from a fixed hash so
    // the layout is identical on every load and every screenshot.
    let scrub = materials.add(StandardMaterial {
        base_color: Color::srgb(0.26, 0.34, 0.19),
        perceptual_roughness: 0.95,
        ..default()
    });
    let dry = materials.add(StandardMaterial {
        base_color: Color::srgb(0.42, 0.38, 0.22),
        perceptual_roughness: 0.95,
        ..default()
    });
    let scrub_mesh = meshes.add(Cuboid::new(1.0, 1.0, 1.0));
    for index in 0..140u32 {
        let hash = |salt: u32| {
            let mut value = index.wrapping_mul(2_654_435_761).wrapping_add(salt.wrapping_mul(40_503));
            value ^= value >> 13;
            value = value.wrapping_mul(1_274_126_177);
            ((value ^ (value >> 16)) % 10_000) as f32 / 10_000.0
        };
        let side = if hash(1) < 0.5 { -1.0 } else { 1.0 };
        let across = side * (6.0 + hash(2) * 22.0);
        let downrange = -(12.0 + hash(3) * 300.0);
        let height = 0.4 + hash(4) * 1.6;
        commands.spawn((
            Mesh3d(scrub_mesh.clone()),
            MeshMaterial3d(if hash(5) < 0.35 { dry.clone() } else { scrub.clone() }),
            Transform::from_xyz(across, height / 2.0, downrange)
                .with_scale(Vec3::new(0.5 + hash(6), height, 0.5 + hash(7)))
                .with_rotation(Quat::from_rotation_y(hash(8) * 6.2)),
        ));
    }

    // The target: a plate standing on the ground at the zero range. The shot is
    // zeroed to ground level here, so it strikes the plate at its base.
    let plate_height = 2.4;
    let plate_width = 2.0;
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(plate_width, plate_height, 0.08))),
        MeshMaterial3d(target.clone()),
        Transform::from_xyz(0.0, plate_height / 2.0, -(ZERO_RANGE as f32)),
    ));
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(plate_width, 0.12, 0.10))),
        MeshMaterial3d(tracer.clone()),
        Transform::from_xyz(0.0, 0.06, -(ZERO_RANGE as f32)),
    ));
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(0.5, 0.5, 0.10))),
        MeshMaterial3d(marker.clone()),
        Transform::from_xyz(0.0, plate_height * 0.7, -(ZERO_RANGE as f32 - 0.01)),
    ));

    // The tracer is a marker, not a bullet at true size: a real round is a
    // fraction of a pixel at this distance.
    commands.spawn((
        Mesh3d(meshes.add(Sphere::new(0.12))),
        MeshMaterial3d(tracer.clone()),
        Transform::from_xyz(0.0, STANDING_HEIGHT as f32, 0.0),
        Tracer,
    ));
    // Muzzle flash: presentation only, and a close, large signal that the shot
    // is running. It carries no simulation meaning. White-hot, so the amber
    // pixel check measures the trajectory rather than the flash.
    let flash = materials.add(StandardMaterial {
        base_color: Color::srgb(1.0, 0.97, 0.92),
        emissive: LinearRgba::rgb(10.0, 9.0, 7.0),
        ..default()
    });
    commands.spawn((
        Mesh3d(meshes.add(Sphere::new(0.1))),
        MeshMaterial3d(flash),
        Transform::from_xyz(0.0, STANDING_HEIGHT as f32, 0.0),
        MuzzleFlash,
    ));

    commands.spawn((
        DirectionalLight {
            illuminance: 12_000.0,
            ..default()
        },
        Transform::from_xyz(-40.0, 60.0, 30.0).looking_at(Vec3::new(0.0, 0.0, -150.0), Vec3::Y),
    ));
    commands.spawn((
        Camera3d::default(),
        Tonemapping::AcesFitted,
        Msaa::Off,
        AmbientLight {
            color: Color::srgb(0.6, 0.7, 0.85),
            brightness: 400.0,
            ..default()
        },
        // Standing at the firing point, looking downrange. The field of view is
        // narrow like a scope, which is what makes a 2.4 m plate read at 300 m.
        Projection::Perspective(PerspectiveProjection {
            fov: 20f32.to_radians(),
            ..default()
        }),
        Transform::from_xyz(0.0, 1.7, 1.5).looking_at(Vec3::new(0.0, 1.5, -(ZERO_RANGE as f32)), Vec3::Y),
    ));
}

/// Breathing sway. Presentation only: the aim wobbles, the resolved shot does
/// not. It also keeps the frame changing every second, which is what a viewer
/// and the pixel checks can see.
fn breathe_camera(time: Res<Time>, mut camera: Query<&mut Transform, With<Camera3d>>) {
    let now = time.elapsed_secs();
    let sway = Vec3::new((now * 1.7).sin() * 0.9, (now * 2.3).sin() * 0.35, 0.0);
    let target = Vec3::new(0.0, 1.5, -(ZERO_RANGE as f32)) + sway;
    for mut transform in &mut camera {
        *transform = Transform::from_xyz(0.0, 1.7, 1.5).looking_at(target, Vec3::Y);
    }
}

/// Draw the resolved flight. The path is the simulation's own samples.
fn draw_flight(mut gizmos: Gizmos, shot: Option<Res<Shot>>) {
    let Some(shot) = shot else { return };
    let points: Vec<Vec3> = shot
        .flight
        .samples
        .iter()
        .step_by(4)
        .map(|sample| to_world(sample.position))
        .collect();
    gizmos.linestrip(points, Color::srgb(1.0, 0.72, 0.18));
}

fn advance_tracer(
    time: Res<Time>,
    shot: Option<Res<Shot>>,
    mut tracer: Query<&mut Transform, (With<Tracer>, Without<MuzzleFlash>)>,
    mut flash: Query<&mut Transform, With<MuzzleFlash>>,
) {
    let Some(shot) = shot else { return };
    let flight = shot.flight.duration();
    if flight <= 0.0 {
        return;
    }
    // Wall-clock phase, so the pause is real time rather than slow-motion time.
    let wall_flight = flight / SLOW_MOTION;
    let phase = time.elapsed_secs_f64() % (wall_flight + TRACER_PAUSE);
    for mut transform in &mut tracer {
        if phase > wall_flight {
            // Between passes the marker waits out of sight behind the shooter.
            transform.translation = to_world(SimVec3::new(-5.0, 0.0, 0.0));
            continue;
        }
        let simulated = phase * SLOW_MOTION;
        let span = (shot.flight.samples.len() - 1 - shot.start_index) as f64;
        let index = shot.start_index + ((simulated / flight) * span) as usize;
        transform.translation = to_world(shot.flight.samples[index].position);
    }
    let burning = phase <= wall_flight && phase * SLOW_MOTION < 0.12;
    for mut transform in &mut flash {
        // Park it under the ground when the shot has left the barrel.
        transform.translation = if burning {
            to_world(SimVec3::new(0.0, 0.0, STANDING_HEIGHT))
        } else {
            Vec3::new(0.0, -50.0, 0.0)
        };
    }
}

struct PlatformStatusPlugin;

impl Plugin for PlatformStatusPlugin {
    fn build(&self, app: &mut App) {
        #[cfg(target_arch = "wasm32")]
        {
            app.insert_resource(RenderErrorHandler(|error, _, _| {
                set_status(
                    "data-render-error",
                    &format!("{:?}: {}", error.ty, error.description),
                );
                RenderErrorPolicy::StopRendering
            }))
            .add_systems(Update, signal_running);
        }
        #[cfg(not(target_arch = "wasm32"))]
        let _ = app;
    }
}

#[cfg(target_arch = "wasm32")]
fn set_status(name: &str, value: &str) {
    if let Some(document) = web_sys::window().and_then(|window| window.document())
        && let Some(root) = document.document_element()
        && let Err(error) = root.set_attribute(name, value)
    {
        bevy::log::error!("Could not report renderer status: {error:?}");
    }
}

// This only confirms the app loop is progressing, not that pixels reached the display.
#[cfg(target_arch = "wasm32")]
fn signal_running(mut updates: Local<u8>) {
    if *updates < 3 {
        *updates += 1;
        if *updates == 3 {
            set_status("data-bevy-running", "true");
        }
    }
}

#[cfg(target_arch = "wasm32")]
fn update_diagnostics(time: Res<Time<Real>>, shot: Option<Res<Shot>>, mut stats: Local<(f32, u32)>) {
    stats.0 += time.delta_secs();
    stats.1 += 1;
    if stats.0 < 1.0 {
        return;
    }
    let fps = stats.1 as f32 / stats.0;
    if let Some(document) = web_sys::window().and_then(|window| window.document())
        && let (Some(root), Some(display)) = (
            document.document_element(),
            document.get_element_by_id("diag"),
        )
        && root.get_attribute("data-diagnostics").as_deref() == Some("true")
    {
        let renderer = root
            .get_attribute("data-renderer")
            .unwrap_or_else(|| "unknown".into());
        let flight = shot.as_ref().map(|shot| shot.summary());
        display.set_text_content(Some(&format!(
            "ARTIFACT {renderer}\nBEVY FPS {fps:.1}\nFRAME {:.1} ms (wall)\n{}\nGPU / draw calls / memory: unavailable",
            1000.0 / fps,
            flight.unwrap_or_default()
        )));
    }
    *stats = (0.0, 0);
}

#[cfg(not(target_arch = "wasm32"))]
fn update_diagnostics() {}
