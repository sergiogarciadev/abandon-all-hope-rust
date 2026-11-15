use bevy::asset::RenderAssetUsages;
use bevy::light::NotShadowCaster;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use rand::RngExt;

use crate::audio::AudioEngine;
use crate::core::blocks::{BlockSpec, bake_blocks};
use crate::core::math::SeededRng;
use crate::core::state::{GameStats, ToastState};
use crate::world::world::HALF;

use super::player::PlayerRig;

pub const DROP_CHANCE: f32 = 0.05;
pub const DROP_PITY: u32 = 30;
pub const DROP_LIFE: f32 = 12.0;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum DropKind {
    Med,
    Dmg,
    Ammo,
}

impl DropKind {
    pub fn name(self) -> &'static str {
        match self {
            DropKind::Med => "+1 HEALTH",
            DropKind::Dmg => "DOUBLE DAMAGE",
            DropKind::Ammo => "BOTTOMLESS MAG",
        }
    }

    pub fn color(self) -> Color {
        match self {
            DropKind::Med => Color::srgb_u8(0xd6, 0x45, 0x3c),
            DropKind::Dmg => Color::srgb_u8(0xff, 0x7a, 0x2e),
            DropKind::Ammo => Color::srgb_u8(0xe8, 0xd2, 0x8a),
        }
    }

    pub fn weight(self) -> f32 {
        match self {
            DropKind::Med => 40.0,
            DropKind::Dmg => 25.0,
            DropKind::Ammo => 25.0,
        }
    }
}

#[derive(Component)]
pub struct DropItem {
    pub kind: DropKind,
    pub life: f32,
    pub t: f32,
}

#[derive(Resource)]
pub struct DropAssets {
    pub meshes: [Handle<Mesh>; 3],
    pub glow_mats: [Handle<StandardMaterial>; 3],
    pub drop_mat: Handle<StandardMaterial>,
    pub glow_quad: Handle<Mesh>,
}

fn drop_glow_texture() -> Image {
    let size = 128_usize;
    let mut data = vec![0_u8; size * size * 4];
    for y in 0..size {
        for x in 0..size {
            let dx = x as f32 - 64.0;
            let dy = y as f32 - 64.0;
            let t = ((dx * dx + dy * dy).sqrt() / 64.0).clamp(0.0, 1.0);
            let a = if t < 0.2 {
                1.0 + (0.7 - 1.0) * (t / 0.2)
            } else if t < 0.5 {
                0.7 + (0.15 - 0.7) * ((t - 0.2) / 0.3)
            } else {
                0.15 * (1.0 - (t - 0.5) / 0.5)
            };
            let idx = (y * size + x) * 4;
            data[idx] = 255;
            data[idx + 1] = 255;
            data[idx + 2] = 255;
            data[idx + 3] = (a * 255.0).round().clamp(0.0, 255.0) as u8;
        }
    }
    Image::new(
        Extent3d {
            width: size as u32,
            height: size as u32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    )
}

pub fn setup_drop_assets(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
) {
    let med = [
        BlockSpec::new(0.52, 0.34, 0.36, 0.0, 0.0, 0.0, 0xe6e2da).with_bevel(0.05),
        BlockSpec::new(0.3, 0.02, 0.1, 0.0, 0.175, 0.0, 0xc8302a).with_bevel(0.006),
        BlockSpec::new(0.1, 0.022, 0.3, 0.0, 0.176, 0.0, 0xc8302a).with_bevel(0.006),
        BlockSpec::new(0.24, 0.08, 0.02, 0.0, 0.0, 0.185, 0xc8302a).with_bevel(0.006),
        BlockSpec::new(0.08, 0.22, 0.022, 0.0, 0.0, 0.186, 0xc8302a).with_bevel(0.006),
    ];
    let dmg = [
        BlockSpec::new(0.13, 0.3, 0.13, -0.09, -0.06, 0.0, 0xc9a14a).with_bevel(0.03),
        BlockSpec::new(0.13, 0.3, 0.13, 0.09, -0.06, 0.0, 0xc9a14a).with_bevel(0.03),
        BlockSpec::new(0.11, 0.16, 0.11, -0.09, 0.17, 0.0, 0xff5a1e).with_bevel(0.04),
        BlockSpec::new(0.11, 0.16, 0.11, 0.09, 0.17, 0.0, 0xff5a1e).with_bevel(0.04),
        BlockSpec::new(0.06, 0.07, 0.06, -0.09, 0.27, 0.0, 0xff9a5a).with_bevel(0.02),
        BlockSpec::new(0.06, 0.07, 0.06, 0.09, 0.27, 0.0, 0xff9a5a).with_bevel(0.02),
    ];
    let ammo = [
        BlockSpec::new(0.52, 0.3, 0.28, 0.0, 0.0, 0.0, 0x4f5634).with_bevel(0.04),
        BlockSpec::new(0.54, 0.06, 0.3, 0.0, 0.17, 0.0, 0x3d4228).with_bevel(0.02),
        BlockSpec::new(0.24, 0.04, 0.06, 0.0, 0.215, 0.0, 0x2a2c24).with_bevel(0.01),
        BlockSpec::new(0.3, 0.06, 0.006, 0.0, 0.0, 0.143, 0xd8b84a).with_bevel(0.002),
    ];

    let mut rng = SeededRng::new(3);
    let drop_meshes = [
        meshes.add(bake_blocks(&med, &mut rng, false)),
        meshes.add(bake_blocks(&dmg, &mut rng, false)),
        meshes.add(bake_blocks(&ammo, &mut rng, false)),
    ];

    let glow_tex = images.add(drop_glow_texture());
    let make_glow = |materials: &mut Assets<StandardMaterial>, kind: DropKind| {
        let lin = kind.color().to_linear();
        materials.add(StandardMaterial {
            base_color: kind.color().with_alpha(0.95),
            base_color_texture: Some(glow_tex.clone()),
            emissive: lin * 1.5,
            alpha_mode: AlphaMode::Add,
            unlit: true,
            ..default()
        })
    };

    let glow_mats = [
        make_glow(&mut materials, DropKind::Med),
        make_glow(&mut materials, DropKind::Dmg),
        make_glow(&mut materials, DropKind::Ammo),
    ];

    let drop_mat = materials.add(StandardMaterial {
        perceptual_roughness: 0.5,
        emissive: LinearRgba::rgb(0.11, 0.11, 0.11),
        ..default()
    });

    let glow_quad = meshes.add(Rectangle::new(2.6, 2.6));

    commands.insert_resource(DropAssets {
        meshes: drop_meshes,
        glow_mats,
        drop_mat,
        glow_quad,
    });
}

pub fn maybe_drop(commands: &mut Commands, stats: &mut GameStats, assets: &DropAssets, pos: Vec3) {
    stats.since_drop += 1;
    let mut rng = rand::rng();
    if rng.random::<f32>() > DROP_CHANCE + stats.drop_bonus && stats.since_drop < DROP_PITY {
        return;
    }
    stats.since_drop = 0;

    let mut pool = Vec::with_capacity(3);
    if stats.hp < stats.max_hp {
        pool.push(DropKind::Med);
    }
    pool.push(DropKind::Dmg);
    pool.push(DropKind::Ammo);

    let total: f32 = pool.iter().map(|k| k.weight()).sum();
    let mut r = rng.random::<f32>() * total;
    let mut pick = pool[0];
    for k in pool {
        r -= k.weight();
        if r <= 0.0 {
            pick = k;
            break;
        }
    }
    spawn_drop(commands, assets, pick, pos);
}

pub fn spawn_drop(commands: &mut Commands, assets: &DropAssets, kind: DropKind, pos: Vec3) {
    let idx = match kind {
        DropKind::Med => 0,
        DropKind::Dmg => 1,
        DropKind::Ammo => 2,
    };
    let mut rng = rand::rng();
    let root = commands
        .spawn((
            Transform::from_xyz(pos.x, 0.7, pos.z),
            Visibility::default(),
            DropItem {
                kind,
                life: DROP_LIFE,
                t: rng.random::<f32>() * 6.0,
            },
        ))
        .id();
    let item = commands
        .spawn((
            Mesh3d(assets.meshes[idx].clone()),
            MeshMaterial3d(assets.drop_mat.clone()),
            Transform::from_scale(Vec3::splat(1.25)),
        ))
        .id();
    let glow = commands
        .spawn((
            Mesh3d(assets.glow_quad.clone()),
            MeshMaterial3d(assets.glow_mats[idx].clone()),
            Transform::from_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_4)),
            NotShadowCaster,
        ))
        .id();
    commands.entity(root).add_children(&[item, glow]);
}

pub fn update_drops(
    mut commands: Commands,
    time: Res<Time>,
    rig: Res<PlayerRig>,
    mut stats: ResMut<GameStats>,
    mut toast: ResMut<ToastState>,
    audio: Res<AudioEngine>,
    player_q: Query<&Transform, Without<DropItem>>,
    mut drops: Query<(Entity, &mut Transform, &mut Visibility, &mut DropItem)>,
) {
    let dt = time.delta_secs();
    let lim = HALF - 0.9;
    let p_pos = player_q
        .get(rig.root)
        .map(|t| t.translation)
        .unwrap_or(Vec3::ZERO);

    for (e, mut tf, mut vis, mut d) in &mut drops {
        d.life -= dt;
        d.t += dt;
        let dx = p_pos.x - tf.translation.x;
        let dz = p_pos.z - tf.translation.z;
        let dist = dx.hypot(dz);

        if dist < 0.9 {
            audio.pickup(d.kind);
            toast.text = d.kind.name().to_string();
            toast.color = d.kind.color();
            toast.timer = 2.0;
            match d.kind {
                DropKind::Med => stats.hp = (stats.hp + 1).min(stats.max_hp),
                DropKind::Dmg => stats.p_damage = 10.0,
                DropKind::Ammo => {
                    stats.p_ammo = 8.0;
                    stats.reloading = false;
                    stats.mag = stats.mag_size;
                }
            }
            commands.entity(e).despawn();
            continue;
        }

        if dist < 2.5 {
            tf.translation.x += (dx / dist) * 10.0 * dt;
            tf.translation.z += (dz / dist) * 10.0 * dt;
        } else {
            let cx = tf.translation.x.clamp(-lim, lim);
            let cz = tf.translation.z.clamp(-lim, lim);
            let ox = cx - tf.translation.x;
            let oz = cz - tf.translation.z;
            let od = ox.hypot(oz);
            if od > 0.01 {
                let st = od.min(6.0 * dt);
                tf.translation.x += (ox / od) * st;
                tf.translation.z += (oz / od) * st;
            }
        }

        tf.translation.y = 0.7 + (d.t * 3.0).sin() * 0.15;
        tf.rotate_y(dt * 2.0);
        *vis = if d.life > 3.0 || ((d.t * 8.0) as i32) % 2 == 0 {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        if d.life <= 0.0 {
            commands.entity(e).despawn();
        }
    }
}
