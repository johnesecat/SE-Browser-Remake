use bevy::{
    prelude::*,
    render::view::{Msaa, Tonemapping},
};
#[cfg(target_arch = "wasm32")]
use bevy::{
    render::error_handler::{RenderErrorHandler, RenderErrorPolicy},
    time::Real,
};

#[derive(Component)]
struct SpinningCube;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "COLLATERAL — platform spike".into(),
                canvas: Some("#game".into()),
                fit_canvas_to_parent: true,
                prevent_default_event_handling: false,
                ..default()
            }),
            ..default()
        }))
        .add_plugins(PlatformStatusPlugin)
        .add_systems(Startup, setup)
        .add_systems(Update, spin)
        .add_systems(Update, update_diagnostics)
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let cube = meshes.add(Cuboid::new(0.65, 0.65, 0.65));
    let material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.32, 0.76, 0.72),
        perceptual_roughness: 0.6,
        ..default()
    });

    for x in 0..10 {
        for y in 0..10 {
            for z in 0..10 {
                commands.spawn((
                    Mesh3d(cube.clone()),
                    MeshMaterial3d(material.clone()),
                    Transform::from_xyz(
                        (x as f32 - 4.5) * 1.2,
                        (y as f32 - 4.5) * 1.2,
                        (z as f32 - 4.5) * 1.2,
                    ),
                    SpinningCube,
                ));
            }
        }
    }

    commands.spawn((
        DirectionalLight {
            illuminance: 10_000.0,
            ..default()
        },
        Transform::from_xyz(5.0, 8.0, 10.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
    commands.spawn((
        Camera3d::default(),
        Tonemapping::AcesFitted,
        Msaa::Off,
        Transform::from_xyz(18.0, 15.0, 22.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
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
fn update_diagnostics(time: Res<Time<Real>>, mut stats: Local<(f32, u32)>) {
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
        display.set_text_content(Some(&format!(
            "ARTIFACT {renderer}\nBEVY FPS {fps:.1}\nFRAME {:.1} ms (wall)\nCUBES 1000\nGPU / draw calls / memory: unavailable",
            1000.0 / fps
        )));
    }
    *stats = (0.0, 0);
}

#[cfg(not(target_arch = "wasm32"))]
fn update_diagnostics() {}

fn spin(time: Res<Time>, mut cubes: Query<&mut Transform, With<SpinningCube>>) {
    for mut transform in &mut cubes {
        transform.rotate_y(0.7 * time.delta_secs());
        transform.rotate_x(0.4 * time.delta_secs());
    }
}
