use bevy::prelude::*;
use rand::RngExt;
use std::f32::consts::TAU;

use crate::audio::AudioEngine;
use crate::core::state::{BannerState, GameState, GameStats};
use crate::entities::player::PlayerRig;
use crate::entities::zombies::{Corpse, Zombie, ZombieAssets, spawn_zombie};
use crate::ui::upgrades::UpgradePicker;
use crate::world::weather::{WeatherState, storm_for};
use crate::world::world::EDGE;

const MAX_ALIVE: usize = 30;

pub fn start_wave(
    stats: &mut GameStats,
    weather: &mut WeatherState,
    audio: &AudioEngine,
    banner: &mut BannerState,
) {
    stats.wave += 1;
    let n = stats.wave;
    stats.zombie_speed = (2.2 + (n - 1) as f32 * 0.2).min(5.2);
    stats.zombie_hp = 2.0 + ((n - 1) / 2) as f32;
    stats.to_spawn = 6 + (n - 1) * 4;
    stats.spawn_timer = 0.0;
    stats.intermission = 0.0;
    weather.target = storm_for(n);
    audio.wave_start(n);
    banner.title = format!("NIGHT {n}");
    banner.sub = if n == 1 {
        "HOLD THE CABIN".to_string()
    } else {
        format!("{} INCOMING", stats.to_spawn)
    };
    banner.timer = 1.6;
}

fn spawn_point(player_pos: Vec3) -> Vec3 {
    let mut rng = rand::rng();
    let ang = rng.random::<f32>() * TAU;
    let cx = ang.sin();
    let cz = ang.cos();
    let mut r = 18.0_f32;
    while r < 45.0 {
        let x = player_pos.x + cx * r;
        let z = player_pos.z + cz * r;
        if x.abs() > EDGE + 1.5 || z.abs() > EDGE + 1.5 {
            return Vec3::new(x, 0.0, z);
        }
        r += 1.0;
    }
    Vec3::new(player_pos.x + cx * 24.0, 0.0, player_pos.z + cz * 24.0)
}

pub fn wave_spawner(
    mut commands: Commands,
    time: Res<Time>,
    rig: Res<PlayerRig>,
    mut stats: ResMut<GameStats>,
    mut weather: ResMut<WeatherState>,
    mut picker: ResMut<UpgradePicker>,
    mut banner: ResMut<BannerState>,
    mut next_state: ResMut<NextState<GameState>>,
    audio: Res<AudioEngine>,
    zombie_assets: Res<ZombieAssets>,
    player_q: Query<&Transform>,
    live_zombies: Query<Entity, (With<Zombie>, Without<Corpse>)>,
) {
    let dt = time.delta_secs();
    let alive = live_zombies.iter().count();
    let p_pos = player_q
        .get(rig.root)
        .map(|t| t.translation)
        .unwrap_or(Vec3::ZERO);

    if stats.intermission > 0.0 {
        stats.intermission -= dt;
        if stats.intermission <= 0.0 {
            stats.hp = (stats.hp + 1).min(stats.max_hp);
            picker.open(&stats);
            if picker.choices.is_empty() {
                start_wave(&mut stats, &mut weather, &audio, &mut banner);
            } else {
                next_state.set(GameState::ChoosingUpgrade);
            }
        }
        return;
    }

    if stats.to_spawn > 0 {
        stats.spawn_timer -= dt;
        if stats.spawn_timer <= 0.0 && alive < MAX_ALIVE {
            let mut rng = rand::rng();
            let burst = 1 + if rng.random::<f32>() < (stats.wave as f32 * 0.08).min(0.6) {
                1
            } else {
                0
            };
            for _ in 0..burst {
                if stats.to_spawn == 0 {
                    break;
                }
                let kind = rng.random_range(0..zombie_assets.kits.len());
                let pos = spawn_point(p_pos);
                spawn_zombie(
                    &mut commands,
                    &zombie_assets,
                    kind,
                    pos,
                    stats.zombie_hp,
                    stats.zombie_speed,
                );
                stats.to_spawn -= 1;
            }
            stats.spawn_timer = (1.1 - stats.wave as f32 * 0.05).max(0.32);
        }
    } else if alive == 0 {
        stats.intermission = 1.6;
        audio.wave_clear();
    }
}

pub fn pause_toggle(
    keys: Res<ButtonInput<KeyCode>>,
    state: Res<State<GameState>>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    if keys.just_pressed(KeyCode::Escape) || keys.just_pressed(KeyCode::KeyP) {
        match state.get() {
            GameState::Playing => next_state.set(GameState::Paused),
            GameState::Paused => next_state.set(GameState::Playing),
            _ => {}
        }
    }
}

pub fn check_player_death(
    stats: Res<GameStats>,
    audio: Res<AudioEngine>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    if stats.hp <= 0 {
        audio.death();
        next_state.set(GameState::Dead);
    }
}
