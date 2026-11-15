mod audio;
mod core;
mod effects;
mod entities;
mod game;
mod ui;
mod world;

use bevy::prelude::*;
use rand::RngExt;

use audio::{setup_audio, sync_audio_state};

use core::state::{AimState, GameState, GameStats, ToastState};
use effects::blood::{setup_blood_assets, update_blood_particles};
use effects::debris::{setup_debris_assets, update_debris};
use entities::drops::{setup_drop_assets, update_drops};
use entities::player::{
    PlayerRig, PlayerRigState, animate_player, player_movement_and_aim, setup_player,
};
use entities::weapon::{setup_bullet_assets, update_bullets, weapon_input_and_fire};
use entities::zombies::{chase_zombies, separate_zombies, setup_zombie_assets, update_corpses};
use game::{check_player_death, pause_toggle, start_first_wave, wave_spawner};
use ui::upgrades::{
    UpgradePicker, despawn_temp_upgrade_ui, spawn_temp_upgrade_ui, update_upgrade_picker_timer,
};
use world::openings::{Openings, update_openings};
use world::solids::SolidsNav;
use world::weather::{setup_weather, update_weather};
use world::world::{WallBoxes, WorldWeatherLevel, setup_world, update_world};

const CAM_OFFSET: Vec3 = Vec3::new(0.0, 30.0, 13.0);

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
        .init_resource::<ToastState>()
        .init_resource::<UpgradePicker>()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "ABANDON ALL HOPE".into(),
                resolution: (1280, 720).into(),
                ..default()
            }),
            ..default()
        }))
        .init_state::<GameState>()
        .add_systems(OnEnter(GameState::ChoosingUpgrade), spawn_temp_upgrade_ui)
        .add_systems(OnExit(GameState::ChoosingUpgrade), despawn_temp_upgrade_ui)
        .add_systems(
            Startup,
            (
                setup_audio,
                setup_basic_camera,
                setup_world,
                setup_player,
                setup_zombie_assets,
                setup_bullet_assets,
                setup_blood_assets,
                setup_debris_assets,
                setup_drop_assets,
                setup_weather,
                start_first_wave.after(setup_weather).after(setup_audio),
            ),
        )
        .add_systems(
            Update,
            (
                pause_toggle,
                sync_audio_state,
                update_world,
                update_weather,
                update_camera,
                update_upgrade_picker_timer.run_if(in_state(GameState::ChoosingUpgrade)),
                (
                    (
                        player_movement_and_aim,
                        weapon_input_and_fire,
                        animate_player,
                        update_bullets,
                    )
                        .chain(),
                    (chase_zombies, separate_zombies, update_corpses).chain(),
                    wave_spawner,
                    update_drops,
                    update_openings,
                    update_debris,
                    update_blood_particles,
                    check_player_death,
                )
                    .run_if(in_state(GameState::Playing)),
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
