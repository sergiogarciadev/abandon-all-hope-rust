use bevy::prelude::*;

/// High-level game flow states, matching the JS state machine.
#[derive(States, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[allow(dead_code)]
enum GameState {
    #[default]
    Title,
    Playing,
    ChoosingUpgrade,
    Paused,
    Dead,
}

fn main() {
    App::new()
        .insert_resource(ClearColor(Color::srgb_u8(0x05, 0x06, 0x0a)))
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "ABANDON ALL HOPE".into(),
                resolution: (1280, 720).into(),
                ..default()
            }),
            ..default()
        }))
        .init_state::<GameState>()
        .add_systems(Startup, setup_basic_camera)
        .run();
}

/// Spawns the initial overhead 3D camera at the `CAM_OFFSET` from `js/src/core/scene.js`.
fn setup_basic_camera(mut commands: Commands) {
    commands.spawn((
        Camera3d::default(),
        Projection::Perspective(PerspectiveProjection {
            fov: 55.0_f32.to_radians(),
            near: 0.1,
            far: 200.0,
            ..default()
        }),
        Transform::from_xyz(0.0, 30.0, 13.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}
