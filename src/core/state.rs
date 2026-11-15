use bevy::prelude::*;

#[derive(Resource)]
#[allow(dead_code)]
pub struct GameStats {
    pub hp: i32,
    pub max_hp: i32,
    pub kills: u32,
    pub shots: u32,
    pub wave: u32,
    pub mag: i32,
    pub mag_size: i32,
    pub reloading: bool,
    pub reload_t: f32,
    pub reload_time: f32,
    pub fire_cd: f32,
    pub fire_rate: f32,
    pub dmg_cd: f32,
    pub damage: f32,
    pub pierce: i32,
    pub spread: i32,
    pub speed: f32,
    pub drop_bonus: f32,
    pub p_damage: f32,
    pub p_ammo: f32,
    pub since_drop: u32,
    pub zombie_speed: f32,
    pub zombie_hp: f32,
    pub shake: f32,
}

impl Default for GameStats {
    fn default() -> Self {
        Self {
            hp: 3,
            max_hp: 3,
            kills: 0,
            shots: 0,
            wave: 0,
            mag: 8,
            mag_size: 8,
            reloading: false,
            reload_t: 0.0,
            reload_time: 1.2,
            fire_cd: 0.0,
            fire_rate: 0.13,
            dmg_cd: 0.0,
            damage: 1.0,
            pierce: 0,
            spread: 0,
            speed: 6.2,
            drop_bonus: 0.0,
            p_damage: 0.0,
            p_ammo: 0.0,
            since_drop: 0,
            zombie_speed: 2.2,
            zombie_hp: 2.0,
            shake: 0.0,
        }
    }
}

#[derive(Resource, Default)]
pub struct AimState {
    pub aim_point: Vec3,
}
