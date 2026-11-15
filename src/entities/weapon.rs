use bevy::light::NotShadowCaster;
use bevy::prelude::*;
use std::collections::HashSet;

use crate::core::state::{AimState, GameStats};
use crate::effects::blood::{BloodAssets, spawn_blood};
use crate::effects::debris::DebrisAssets;
use crate::world::openings::{OpeningKind, Openings, break_opening, burst, shut_at};
use crate::world::world::{WALL_T, WallBoxes};

use super::player::{PlayerRig, PlayerRigState, trigger_muzzle_flash};
use super::zombies::{Corpse, Zombie, ZombieAnim, ZombieAssets, zombie_dies};

#[derive(Component)]
pub struct Bullet {
    pub dir: Vec3,
    pub life: f32,
    pub pierce: i32,
    pub hits: HashSet<Entity>,
}

#[derive(Resource)]
pub struct BulletAssets {
    pub core_mesh: Handle<Mesh>,
    pub glow_mesh: Handle<Mesh>,
    pub core_mat: Handle<StandardMaterial>,
    pub glow_mat: Handle<StandardMaterial>,
    pub glow_hot_mat: Handle<StandardMaterial>,
}

pub fn setup_bullet_assets(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let glow_mat = |materials: &mut Assets<StandardMaterial>, hex: u32, alpha: f32| {
        let r = ((hex >> 16) & 0xff) as f32 / 255.0;
        let g = ((hex >> 8) & 0xff) as f32 / 255.0;
        let b = (hex & 0xff) as f32 / 255.0;
        materials.add(StandardMaterial {
            base_color: Color::srgba(r, g, b, alpha),
            emissive: LinearRgba::rgb(r, g, b) * 4.0,
            alpha_mode: AlphaMode::Add,
            unlit: true,
            ..default()
        })
    };

    commands.insert_resource(BulletAssets {
        core_mesh: meshes.add(Cuboid::new(0.05, 0.05, 0.9)),
        glow_mesh: meshes.add(Cuboid::new(0.15, 0.15, 1.05)),
        core_mat: glow_mat(&mut materials, 0xfff4d6, 1.0),
        glow_mat: glow_mat(&mut materials, 0xffb04a, 0.45),
        glow_hot_mat: glow_mat(&mut materials, 0xff5a1e, 0.6),
    });
}

fn spawn_tracer(commands: &mut Commands, assets: &BulletAssets, pos: Vec3, dir: Vec3, hot: bool, pierce: i32) {
    let rot = Transform::from_translation(pos)
        .looking_at(pos + dir, Vec3::Y)
        .rotation;
    let root = commands
        .spawn((
            Transform::from_translation(pos).with_rotation(rot),
            Visibility::default(),
            Bullet {
                dir,
                life: 0.9,
                pierce,
                hits: HashSet::new(),
            },
        ))
        .id();
    let core = commands
        .spawn((
            Mesh3d(assets.core_mesh.clone()),
            MeshMaterial3d(assets.core_mat.clone()),
            Transform::from_xyz(0.0, 0.0, 0.35),
            NotShadowCaster,
        ))
        .id();
    let glow = commands
        .spawn((
            Mesh3d(assets.glow_mesh.clone()),
            MeshMaterial3d(if hot {
                assets.glow_hot_mat.clone()
            } else {
                assets.glow_mat.clone()
            }),
            Transform::from_xyz(0.0, 0.0, 0.42),
            NotShadowCaster,
        ))
        .id();
    commands.entity(root).add_children(&[core, glow]);
}

pub fn weapon_input_and_fire(
    mut commands: Commands,
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    rig: Res<PlayerRig>,
    mut rig_state: ResMut<PlayerRigState>,
    mut stats: ResMut<GameStats>,
    aim_state: Res<AimState>,
    bullet_assets: Res<BulletAssets>,
    debris_assets: Res<DebrisAssets>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    global_transforms: Query<&GlobalTransform>,
    mut transforms: Query<&mut Transform>,
) {
    let dt = time.delta_secs();
    stats.fire_cd -= dt;
    stats.p_damage = (stats.p_damage - dt).max(0.0);
    stats.p_ammo = (stats.p_ammo - dt).max(0.0);

    if keys.just_pressed(KeyCode::KeyR) && !stats.reloading && stats.mag < stats.mag_size {
        stats.reloading = true;
        stats.reload_t = 0.0;
    }

    if stats.reloading {
        stats.reload_t += dt;
        if stats.reload_t >= stats.reload_time {
            stats.reloading = false;
            stats.reload_t = 0.0;
            stats.mag = stats.mag_size;
        }
    }

    rig_state.reloading = stats.reloading;
    rig_state.reload_t = stats.reload_t;
    rig_state.reload_time = stats.reload_time;
    rig_state.speed = stats.speed;

    if mouse.pressed(MouseButton::Left) && stats.fire_cd <= 0.0 && !stats.reloading && stats.mag > 0 {
        if stats.p_ammo <= 0.0 {
            stats.mag -= 1;
            if stats.mag <= 0 {
                stats.reloading = true;
                stats.reload_t = 0.0;
            }
        }
        stats.shots += 1;
        stats.fire_cd = stats.fire_rate;

        let p_pos = transforms
            .get(rig.root)
            .map(|t| t.translation)
            .unwrap_or(Vec3::ZERO);
        let aim = Vec3::new(
            aim_state.aim_point.x - p_pos.x,
            0.0,
            aim_state.aim_point.z - p_pos.z,
        );
        if aim.length_squared() >= 1e-4 {
            let aim = aim.normalize();
            let mp = global_transforms
                .get(rig.muzzle)
                .map(|gt| gt.translation())
                .unwrap_or(p_pos + Vec3::new(0.0, 1.38, 0.0));
            let count = 1 + stats.spread;
            let hot = stats.p_damage > 0.0;
            for k in 0..count {
                let a = (k as f32 - (count - 1) as f32 * 0.5) * 0.13;
                let (s, c) = a.sin_cos();
                let dir = Vec3::new(aim.x * c - aim.z * s, 0.0, aim.x * s + aim.z * c);
                spawn_tracer(&mut commands, &bullet_assets, mp, dir, hot, stats.pierce);
            }
            trigger_muzzle_flash(
                &mut commands,
                &rig,
                &mut rig_state,
                &debris_assets,
                &mut materials,
                &global_transforms,
                &mut transforms,
                hot,
            );
            stats.shake = stats.shake.max(0.12);
        }
    }
}

pub fn update_bullets(
    mut commands: Commands,
    time: Res<Time>,
    rig: Res<PlayerRig>,
    mut stats: ResMut<GameStats>,
    wall_boxes: Res<WallBoxes>,
    mut openings: ResMut<Openings>,
    debris_assets: Res<DebrisAssets>,
    blood_assets: Res<BloodAssets>,
    zombie_assets: Res<ZombieAssets>,
    mut bullets: Query<(Entity, &mut Transform, &mut Bullet)>,
    mut zombies: Query<(Entity, &mut Transform, &mut Zombie, &mut ZombieAnim), Without<Bullet>>,
    corpses_q: Query<(Entity, &Corpse)>,
    player_q: Query<&Transform, (Without<Bullet>, Without<Zombie>)>,
    mut mats: Query<&mut MeshMaterial3d<StandardMaterial>>,
    mut visibilities: Query<&mut Visibility>,
) {
    let dt = time.delta_secs();
    let player_pos = player_q
        .get(rig.root)
        .map(|t| t.translation)
        .unwrap_or(Vec3::ZERO);
    for (b_ent, mut b_tf, mut b) in &mut bullets {
        let bpx = b_tf.translation.x;
        let bpz = b_tf.translation.z;
        b_tf.translation += b.dir * (38.0 * dt);
        b.life -= dt;
        let mut hit = false;

        let travel = (b_tf.translation.x - bpx).hypot(b_tf.translation.z - bpz);
        let steps = ((travel / (WALL_T * 0.5)).ceil() as usize).max(1);
        for s in 1..=steps {
            if hit {
                break;
            }
            let t = s as f32 / steps as f32;
            let sx = bpx + (b_tf.translation.x - bpx) * t;
            let sz = bpz + (b_tf.translation.z - bpz) * t;
            for w in &wall_boxes.boxes {
                if sx >= w.min_x && sx <= w.max_x && sz >= w.min_z && sz <= w.max_z {
                    hit = true;
                    break;
                }
            }
            if !hit {
                if let Some(idx) = shut_at(&openings, sx, sz) {
                    let o = &mut openings.list[idx];
                    if o.kind == OpeningKind::Door {
                        hit = true;
                        o.jolt = 0.4;
                        burst(o, 1, false, 0.08, 1.5, &mut commands, &debris_assets);
                    } else {
                        break_opening(
                            o,
                            &mut commands,
                            &debris_assets,
                            &mut visibilities,
                        );
                    }
                }
            }
        }

        if !hit {
            let sx = b_tf.translation.x - bpx;
            let sz = b_tf.translation.z - bpz;
            let len_sq = (sx * sx + sz * sz).max(1e-6);
            for (z_ent, mut z_tf, mut z_state, mut z_anim) in &mut zombies {
                if b.hits.contains(&z_ent) {
                    continue;
                }
                let t = (((z_tf.translation.x - bpx) * sx + (z_tf.translation.z - bpz) * sz) / len_sq)
                    .clamp(0.0, 1.0);
                let dx = z_tf.translation.x - (bpx + sx * t);
                let dz = z_tf.translation.z - (bpz + sz * t);
                if dx * dx + dz * dz < 0.55 {
                    let dmg = stats.damage * if stats.p_damage > 0.0 { 2.0 } else { 1.0 };
                    z_state.hp -= dmg;
                    z_state.hurt = 0.12;
                    z_anim.flinch = 1.0;
                    for &m in &z_state.body {
                        if let Ok(mut mat) = mats.get_mut(m) {
                            mat.0 = zombie_assets.hurt_mat.clone();
                        }
                    }
                    z_tf.translation += b.dir * 0.25;
                    spawn_blood(&mut commands, &blood_assets, z_tf.translation);

                    if z_state.hp <= 0.0 {
                        stats.kills += 1;
                        zombie_dies(
                            z_ent,
                            &z_tf,
                            &mut z_state,
                            &mut z_anim,
                            Some(b.dir),
                            player_pos,
                            &mut commands,
                            &zombie_assets,
                            &mut openings,
                            &corpses_q,
                            &mut mats,
                            &mut visibilities,
                        );
                    }
                    b.hits.insert(z_ent);
                    if b.pierce > 0 {
                        b.pierce -= 1;
                    } else {
                        hit = true;
                        break;
                    }
                }
            }
        }

        if hit || b.life <= 0.0 {
            commands.entity(b_ent).despawn();
        }
    }
}
