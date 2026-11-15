mod audio;
mod core;
mod effects;
mod entities;
mod game;
mod ui;
mod world;

use bevy::prelude::*;

use audio::{setup_audio, sync_audio_state};
use core::state::{
    AimState, BannerState, CameraTransition, DamageFlash, GameState, GameStats, ToastState,
};
use effects::blood::{setup_blood_assets, update_blood_particles};
use effects::debris::{setup_debris_assets, update_debris};
use entities::drops::{setup_drop_assets, update_drops};
use entities::player::{
    PlayerRigState, animate_player, player_movement_and_aim, setup_player,
};
use entities::weapon::{setup_bullet_assets, update_bullets, weapon_input_and_fire};
use entities::zombies::{chase_zombies, separate_zombies, setup_zombie_assets, update_corpses};
use game::{check_player_death, pause_toggle, wave_spawner};
use ui::hud::{setup_hud, update_hud};
use ui::storage::load_storage;
use ui::title::{
    TitleUiState, setup_title_ui, spawn_title_zombies, update_camera_with_title, update_overlays,
    update_title_ui, wander_title_zombies,
};
use ui::upgrades::{
    UpgradePicker, despawn_upgrade_picker, setup_upgrade_icons, spawn_upgrade_picker,
    update_upgrade_picker_timer,
};
use world::openings::{Openings, update_openings};
use world::solids::SolidsNav;
use world::weather::{setup_weather, update_weather};
use world::world::{WallBoxes, WorldWeatherLevel, setup_world, update_world};

fn main() {
    let loaded_settings = load_storage();

    App::new()
        .insert_resource(ClearColor(Color::srgb_u8(0x05, 0x06, 0x0a)))
        .insert_resource(loaded_settings)
        .init_resource::<SolidsNav>()
        .init_resource::<WallBoxes>()
        .init_resource::<Openings>()
        .init_resource::<WorldWeatherLevel>()
        .init_resource::<PlayerRigState>()
        .init_resource::<GameStats>()
        .init_resource::<AimState>()
        .init_resource::<ToastState>()
        .init_resource::<DamageFlash>()
        .init_resource::<BannerState>()
        .init_resource::<CameraTransition>()
        .init_resource::<UpgradePicker>()
        .init_resource::<TitleUiState>()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "ABANDON ALL HOPE".into(),
                resolution: (1280, 720).into(),
                ..default()
            }),
            ..default()
        }))
        .init_state::<GameState>()
        .add_systems(OnEnter(GameState::Title), spawn_title_zombies)
        .add_systems(OnEnter(GameState::ChoosingUpgrade), spawn_upgrade_picker)
        .add_systems(OnExit(GameState::ChoosingUpgrade), despawn_upgrade_picker)
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
                setup_hud,
                setup_upgrade_icons,
                setup_title_ui,
                spawn_title_zombies.after(setup_zombie_assets),
            ),
        )
        .add_systems(
            Update,
            (
                pause_toggle,
                sync_audio_state,
                update_world,
                update_weather,
                update_camera_with_title,
                update_hud,
                update_title_ui,
                update_overlays,
                wander_title_zombies.run_if(in_state(GameState::Title)),
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

fn setup_basic_camera(mut commands: Commands) {
    commands.spawn((
        Camera3d::default(),
        Projection::Perspective(PerspectiveProjection {
            fov: 42.0_f32.to_radians(),
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
        Transform::from_xyz(2.0, 2.0, 24.0).looking_at(Vec3::new(-1.5, 2.2, 8.0), Vec3::Y),
    ));
}
