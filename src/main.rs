mod core;
mod effects;
mod entities;
mod world;

use bevy::prelude::*;
use rand::RngExt;

use core::state::{AimState, GameStats};
use effects::blood::{setup_blood_assets, update_blood_particles};
use effects::debris::{setup_debris_assets, update_debris};
use entities::player::{
    PlayerRig, PlayerRigState, animate_player, player_movement_and_aim, setup_player,
};
use entities::weapon::{setup_bullet_assets, update_bullets, weapon_input_and_fire};
use entities::zombies::{animate_preview_zombies, setup_zombie_assets};
use world::openings::{Openings, update_openings};
use world::solids::SolidsNav;
use world::world::{WallBoxes, WorldWeatherLevel, setup_world, update_world};

const CAM_OFFSET: Vec3 = Vec3::new(0.0, 30.0, 13.0);

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

#[derive(Component)]
struct MainCamera;

fn main() {
    App::new()
        .insert_resource(ClearColor(Color::srgb_u8(0x05, 0x06, 0x0a)))
        .init_resource::<SolidsNav>()
        .init_resource::<WallBoxes>()
        .init_resource::<Openings>()
        .init_resource::<WorldWeatherLevel>()
        .init_resource::<PlayerRigState>()
        .init_resource::<GameStats>()
        .init_resource::<AimState>()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "ABANDON ALL HOPE".into(),
                resolution: (1280, 720).into(),
                ..default()
            }),
            ..default()
        }))
        .init_state::<GameState>()
        .add_systems(
            Startup,
            (
                setup_basic_camera,
                setup_world,
                setup_player,
                setup_zombie_assets,
                setup_bullet_assets,
                setup_blood_assets,
                setup_debris_assets,
            ),
        )
        .add_systems(
            Update,
            (
                update_world,
                (
                    player_movement_and_aim,
                    weapon_input_and_fire,
                    animate_player,
                    update_bullets,
                )
                    .chain(),
                animate_preview_zombies,
                update_openings,
                update_debris,
                update_blood_particles,
                update_camera.after(player_movement_and_aim),
            ),
        )
        .run();
}

/// Spawns the initial overhead 3D camera at the `CAM_OFFSET` from `js/src/core/scene.js`.
fn setup_basic_camera(mut commands: Commands) {
    commands.spawn((
        Camera3d::default(),
        MainCamera,
        Projection::Perspective(PerspectiveProjection {
            fov: 55.0_f32.to_radians(),
            near: 0.1,
            far: 200.0,
            ..default()
        }),
        DistanceFog {
            color: Color::srgb_u8(0x05, 0x06, 0x0a),
            falloff: FogFalloff::Linear {
                start: 32.0,
                end: 62.0,
            },
            ..default()
        },
        Transform::from_translation(CAM_OFFSET).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

fn update_camera(
    time: Res<Time>,
    rig: Res<PlayerRig>,
    mut stats: ResMut<GameStats>,
    mut cam_q: Query<&mut Transform, With<MainCamera>>,
    player_q: Query<&Transform, Without<MainCamera>>,
) {
    let dt = time.delta_secs();
    stats.shake = (stats.shake - dt * 1.4).max(0.0);
    let p_pos = player_q
        .get(rig.root)
        .map(|t| t.translation)
        .unwrap_or(Vec3::ZERO);
    let mut rng = rand::rng();
    let s = stats.shake * stats.shake * 0.45;
    let shake_off = Vec3::new(
        (rng.random::<f32>() - 0.5) * 2.0 * s,
        0.0,
        (rng.random::<f32>() - 0.5) * 2.0 * s,
    );
    if let Ok(mut cam_tf) = cam_q.single_mut() {
        let target = Vec3::new(p_pos.x, 0.0, p_pos.z);
        *cam_tf = Transform::from_translation(target + CAM_OFFSET + shake_off)
            .looking_at(target, Vec3::Y);
    }
}
