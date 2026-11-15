use bevy::prelude::*;
use rand::RngExt;
use std::f32::consts::{FRAC_PI_2, PI};

use crate::audio::AudioEngine;
use crate::core::blocks::{BlockBatch, BlockSpec, bake_blocks};
use crate::core::math::SeededRng;
use crate::effects::debris::{Debris, DebrisAssets};

pub const OPENING_INSET: f32 = 1.5;
pub const SILL_X: f32 = 0.85;
pub const SILL_Z: f32 = 1.05;
pub const HEAD_X: f32 = 1.63;
pub const HEAD_Z: f32 = 1.83;
pub const DOOR_TOP: f32 = 2.03;
pub const JAMB: f32 = 0.14;
pub const DOOR_HP: i32 = 12;
pub const WINDOW_HP: i32 = 2;
pub const CLIMB_TIME: f32 = 1.1;

pub const LOG_D: f32 = 0.62;
pub const POST: u32 = 0x3a2b1d;
pub const WEATHERED: u32 = 0x6d6352;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum OpeningKind {
    Door,
    Window,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum WallAxis {
    X,
    Z,
}

#[derive(Clone, Copy)]
#[allow(dead_code)]
pub struct Aabb2d {
    pub min_x: f32,
    pub max_x: f32,
    pub min_z: f32,
    pub max_z: f32,
}

#[allow(dead_code)]
pub struct Opening {
    pub kind: OpeningKind,
    pub axis: WallAxis,
    pub u0: f32,
    pub u1: f32,
    pub w: f32,
    pub x: f32,
    pub z: f32,
    pub ox: f32,
    pub oz: f32,
    pub ix: f32,
    pub iz: f32,
    pub fx: f32,
    pub fz: f32,
    pub lx: f32,
    pub lz: f32,
    pub sill: f32,
    pub head: f32,
    pub rot_y: f32,
    pub bbox: Aabb2d,
    pub max_hp: i32,
    pub hp: i32,
    pub climber: Option<Entity>,
    pub jolt: f32,
    pub fall_t: f32,
    // Entity handles for moving/breaking parts
    pub fall_entity: Option<Entity>,
    pub leaf_entity: Option<Entity>,
    pub pane_entity: Option<Entity>,
    pub glass_entity: Option<Entity>,
    pub bars_entity: Option<Entity>,
    pub shards_entity: Option<Entity>,
}

#[derive(Resource, Default)]
pub struct Openings {
    pub list: Vec<Opening>,
    pub glass_mat: Handle<StandardMaterial>,
    pub glass_cracked: Handle<StandardMaterial>,
}

impl Openings {
    pub fn reset(
        &mut self,
        mats: &mut Query<&mut MeshMaterial3d<StandardMaterial>>,
        visibilities: &mut Query<&mut Visibility>,
    ) {
        let clean = self.glass_mat.clone();
        for o in &mut self.list {
            o.hp = o.max_hp;
            o.climber = None;
            o.jolt = 0.0;
            o.fall_t = -1.0;
            if let Some(e) = o.fall_entity {
                if let Ok(mut v) = visibilities.get_mut(e) {
                    *v = Visibility::Inherited;
                }
            }
            if let Some(e) = o.pane_entity {
                if let Ok(mut v) = visibilities.get_mut(e) {
                    *v = Visibility::Inherited;
                }
            }
            if let Some(e) = o.bars_entity {
                if let Ok(mut v) = visibilities.get_mut(e) {
                    *v = Visibility::Inherited;
                }
            }
            if let Some(e) = o.shards_entity {
                if let Ok(mut v) = visibilities.get_mut(e) {
                    *v = Visibility::Hidden;
                }
            }
            if let Some(glass) = o.glass_entity {
                if let Ok(mut m) = mats.get_mut(glass) {
                    m.0 = clean.clone();
                }
            }
        }
    }
}

pub fn make_opening(axis: WallAxis, fixed: f32, u0: f32, u1: f32, kind: OpeningKind, wall_t: f32) -> Opening {
    let c = (u0 + u1) * 0.5;
    let s = fixed.signum();
    let ht = wall_t * 0.5;
    let p = |out: f32| -> (f32, f32) {
        match axis {
            WallAxis::X => (c, fixed + s * out),
            WallAxis::Z => (fixed + s * out, c),
        }
    };
    let (x, z) = p(0.0);
    let (ox, oz) = p(OPENING_INSET);
    let (ix, iz) = p(-OPENING_INSET);
    let (fx, fz) = p(0.85);
    let (lx, lz) = p(-0.95);
    let (sill, head) = match axis {
        WallAxis::X => (SILL_X, HEAD_X),
        WallAxis::Z => (SILL_Z, HEAD_Z),
    };
    let rot_y = match axis {
        WallAxis::X => {
            if s > 0.0 {
                0.0
            } else {
                PI
            }
        }
        WallAxis::Z => s * PI * 0.5,
    };
    let bbox = match axis {
        WallAxis::X => Aabb2d {
            min_x: u0,
            max_x: u1,
            min_z: fixed - ht,
            max_z: fixed + ht,
        },
        WallAxis::Z => Aabb2d {
            min_x: fixed - ht,
            max_x: fixed + ht,
            min_z: u0,
            max_z: u1,
        },
    };
    let max_hp = match kind {
        OpeningKind::Door => DOOR_HP,
        OpeningKind::Window => WINDOW_HP,
    };

    Opening {
        kind,
        axis,
        u0,
        u1,
        w: u1 - u0,
        x,
        z,
        ox,
        oz,
        ix,
        iz,
        fx,
        fz,
        lx,
        lz,
        sill,
        head,
        rot_y,
        bbox,
        max_hp,
        hp: max_hp,
        climber: None,
        jolt: 0.0,
        fall_t: -1.0,
        fall_entity: None,
        leaf_entity: None,
        pane_entity: None,
        glass_entity: None,
        bars_entity: None,
        shards_entity: None,
    }
}

pub fn fit_opening(
    o: &mut Opening,
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    wall_batch: &mut BlockBatch,
    world_mat: &Handle<StandardMaterial>,
    glass_mat: &Handle<StandardMaterial>,
    wrng: &mut SeededRng,
) {
    let place = Mat4::from_rotation_translation(
        Quat::from_rotation_y(o.rot_y),
        Vec3::new(o.x, 0.0, o.z),
    );
    let hw = o.w * 0.5;
    let seed = ((o.x * 7.0 + o.z * 13.0).round() as i32 + 99) as u32;
    let mut frng = SeededRng::new(seed);

    let root = commands
        .spawn((
            Transform::from_xyz(o.x, 0.0, o.z).with_rotation(Quat::from_rotation_y(o.rot_y)),
            Visibility::default(),
        ))
        .id();

    if o.kind == OpeningKind::Door {
        wall_batch.add(
            &[
                BlockSpec::new(JAMB, DOOR_TOP + 0.1, LOG_D + 0.1, -hw + JAMB * 0.5, (DOOR_TOP + 0.1) * 0.5, 0.0, POST).with_bevel(0.03),
                BlockSpec::new(JAMB, DOOR_TOP + 0.1, LOG_D + 0.1, hw - JAMB * 0.5, (DOOR_TOP + 0.1) * 0.5, 0.0, POST).with_bevel(0.03),
                BlockSpec::new(o.w + 0.1, 0.14, LOG_D + 0.12, 0.0, DOOR_TOP + 0.02, 0.0, POST).with_bevel(0.03),
                BlockSpec::new(o.w - 0.1, 0.05, LOG_D, 0.0, 0.0, 0.0, POST).with_bevel(0.02),
            ],
            wrng,
            Some(place),
            false,
        );

        let lw = o.w - 2.0 * JAMB - 0.02;
        let lh = 1.92;
        let n = 4;
        let pw = lw / n as f32;
        let mut blocks = Vec::new();
        for i in 0..n {
            let col = if i % 2 == 1 { WEATHERED } else { 0x5f5646 };
            blocks.push(
                BlockSpec::new(pw - 0.012, lh, 0.06, pw * (i as f32 + 0.5), lh * 0.5, 0.0, col)
                    .with_bevel(0.012),
            );
        }
        blocks.push(BlockSpec::new(lw - 0.04, 0.14, 0.04, lw * 0.5, 0.35, -0.05, POST).with_bevel(0.012));
        blocks.push(BlockSpec::new(lw - 0.04, 0.14, 0.04, lw * 0.5, lh - 0.35, -0.05, POST).with_bevel(0.012));
        let bl = (lw - 0.1_f32).hypot(lh - 0.7);
        let brace_ang = -(lw - 0.1).atan2(lh - 0.7);
        blocks.push(
            BlockSpec::new(0.12, bl, 0.04, lw * 0.5, lh * 0.5, -0.05, POST)
                .with_rot(0.0, 0.0, brace_ang)
                .with_bevel(0.012),
        );
        blocks.push(BlockSpec::new(0.05, 0.05, 0.08, lw - 0.15, 1.0, 0.06, 0x2a2a2a).with_bevel(0.01));
        for y in [0.35, lh - 0.35] {
            blocks.push(BlockSpec::new(0.32, 0.05, 0.015, 0.16, y, 0.035, 0x2a2a2a).with_bevel(0.005));
        }

        let leaf_mesh = meshes.add(bake_blocks(&blocks, &mut frng, false));
        let fall = commands
            .spawn((
                Transform::from_xyz(0.0, 0.0, -0.02),
                Visibility::default(),
            ))
            .id();
        let leaf = commands
            .spawn((
                Transform::from_xyz(-hw + JAMB + 0.01, 0.02, 0.0),
                Visibility::default(),
            ))
            .id();
        let leaf_mesh_e = commands
            .spawn((
                Mesh3d(leaf_mesh),
                MeshMaterial3d(world_mat.clone()),
            ))
            .id();

        commands.entity(root).add_child(fall);
        commands.entity(fall).add_child(leaf);
        commands.entity(leaf).add_child(leaf_mesh_e);

        o.fall_entity = Some(fall);
        o.leaf_entity = Some(leaf);
    } else {
        let h = o.head - o.sill;
        let mid = (o.sill + o.head) * 0.5;
        let iw = o.w - 0.2;

        wall_batch.add(
            &[
                BlockSpec::new(0.1, h + 0.1, LOG_D + 0.06, -hw + 0.05, mid, 0.0, POST).with_bevel(0.02),
                BlockSpec::new(0.1, h + 0.1, LOG_D + 0.06, hw - 0.05, mid, 0.0, POST).with_bevel(0.02),
                BlockSpec::new(o.w + 0.12, 0.08, LOG_D + 0.08, 0.0, o.head, 0.0, POST).with_bevel(0.02),
                BlockSpec::new(o.w + 0.3, 0.07, LOG_D + 0.24, 0.0, o.sill, 0.05, WEATHERED).with_bevel(0.02),
            ],
            wrng,
            Some(place),
            false,
        );

        let pane = commands
            .spawn((
                Transform::from_xyz(0.0, mid, 0.0),
                Visibility::default(),
            ))
            .id();
        let glass = commands
            .spawn((
                Mesh3d(meshes.add(Cuboid::new(iw, h - 0.08, 0.02))),
                MeshMaterial3d(glass_mat.clone()),
            ))
            .id();
        let bars_mesh = meshes.add(bake_blocks(
            &[
                BlockSpec::new(0.05, h - 0.08, 0.05, 0.0, 0.0, 0.0, POST),
                BlockSpec::new(iw, 0.05, 0.05, 0.0, 0.0, 0.0, POST),
            ],
            &mut frng,
            false,
        ));
        let bars = commands
            .spawn((
                Mesh3d(bars_mesh),
                MeshMaterial3d(world_mat.clone()),
            ))
            .id();

        let mut teeth = Vec::new();
        for i in 0..9 {
            let side = i % 3;
            let t = frng.next_f32() - 0.5;
            let len = 0.12 + frng.next_f32() * 0.2;
            let (px, py) = match side {
                0 => (t * iw, -(h - 0.1) * 0.5 + 0.05),
                1 => (-iw * 0.5 + 0.05, t * (h - 0.1)),
                _ => (iw * 0.5 - 0.05, t * (h - 0.1)),
            };
            teeth.push(
                BlockSpec::new(0.03 + frng.next_f32() * 0.06, len, 0.015, px, py, 0.0, 0xffffff)
                    .with_rot(0.0, 0.0, (frng.next_f32() - 0.5) * 1.6)
                    .with_bevel(0.0),
            );
        }
        for _ in 0..3 {
            teeth.push(
                BlockSpec::new(
                    0.04 + frng.next_f32() * 0.05,
                    0.1 + frng.next_f32() * 0.12,
                    0.015,
                    (frng.next_f32() - 0.5) * iw,
                    (h - 0.1) * 0.5 - 0.04,
                    0.0,
                    0xffffff,
                )
                .with_rot(0.0, 0.0, PI + (frng.next_f32() - 0.5) * 1.2)
                .with_bevel(0.0),
            );
        }
        let shards_mesh = meshes.add(bake_blocks(&teeth, &mut frng, true));
        let shards = commands
            .spawn((
                Mesh3d(shards_mesh),
                MeshMaterial3d(glass_mat.clone()),
                Visibility::Hidden,
            ))
            .id();

        commands.entity(root).add_child(pane);
        commands.entity(pane).add_children(&[glass, bars, shards]);

        o.pane_entity = Some(pane);
        o.glass_entity = Some(glass);
        o.bars_entity = Some(bars);
        o.shards_entity = Some(shards);
    }
}

pub fn burst(
    o: &Opening,
    n: usize,
    is_glass: bool,
    size: f32,
    speed: f32,
    commands: &mut Commands,
    debris_assets: &DebrisAssets,
) {
    let sx = o.ix - o.x;
    let sz = o.iz - o.z;
    let l = sx.hypot(sz).max(1e-4);
    let mut rng = rand::rng();
    let mat = if is_glass {
        debris_assets.glass_chip_mat.clone()
    } else {
        debris_assets.wood_chip_mat.clone()
    };

    for _ in 0..n {
        let scale = Vec3::new(
            size * (0.4 + rng.random::<f32>() * 0.6),
            size * (0.2 + rng.random::<f32>() * 0.4),
            size * (1.0 + rng.random::<f32>() * 1.5),
        );
        let along = (rng.random::<f32>() - 0.5) * (o.w - 0.3);
        let px = o.x + if o.axis == WallAxis::X { along } else { 0.0 };
        let py = if o.kind == OpeningKind::Door {
            0.3 + rng.random::<f32>() * 1.5
        } else {
            o.sill + 0.05 + rng.random::<f32>() * (o.head - o.sill - 0.1)
        };
        let pz = o.z + if o.axis == WallAxis::Z { along } else { 0.0 };
        let rot = Quat::from_euler(
            EulerRot::YXZ,
            rng.random::<f32>() * 3.0,
            rng.random::<f32>() * 3.0,
            rng.random::<f32>() * 3.0,
        );
        let v = speed * (0.5 + rng.random::<f32>() * 0.7);
        let vel = Vec3::new(
            sx / l * v + (rng.random::<f32>() - 0.5) * 2.0,
            0.5 + rng.random::<f32>() * 2.0,
            sz / l * v + (rng.random::<f32>() - 0.5) * 2.0,
        );
        let life = 2.0 + rng.random::<f32>() * 1.5;
        commands.spawn((
            Mesh3d(debris_assets.cube_mesh.clone()),
            MeshMaterial3d(mat.clone()),
            Transform::from_xyz(px, py, pz)
                .with_rotation(rot)
                .with_scale(scale),
            Debris {
                vel,
                spin: Vec3::new(
                    (rng.random::<f32>() - 0.5) * 24.0,
                    (rng.random::<f32>() - 0.5) * 24.0,
                    (rng.random::<f32>() - 0.5) * 24.0,
                ),
                life,
                max_life: life,
                bounce: 0.02,
                smoke_size: None,
                base_scale: scale,
            },
        ));
    }
}

pub fn break_opening(
    o: &mut Opening,
    commands: &mut Commands,
    debris_assets: &DebrisAssets,
    visibilities: &mut Query<&mut Visibility>,
) {
    o.hp = 0;
    if o.kind == OpeningKind::Door {
        o.fall_t = 0.0;
        burst(o, 9, false, 0.16, 3.5, commands, debris_assets);
    } else {
        for e in [o.glass_entity, o.bars_entity].into_iter().flatten() {
            if let Ok(mut vis) = visibilities.get_mut(e) {
                *vis = Visibility::Hidden;
            }
        }
        if let Some(shards) = o.shards_entity {
            if let Ok(mut vis) = visibilities.get_mut(shards) {
                *vis = Visibility::Inherited;
            }
        }
        burst(o, 16, true, 0.09, 4.0, commands, debris_assets);
    }
}

pub fn shut_at(openings: &Openings, x: f32, z: f32) -> Option<usize> {
    openings.list.iter().position(|o| {
        o.hp > 0
            && x >= o.bbox.min_x
            && x <= o.bbox.max_x
            && z >= o.bbox.min_z
            && z <= o.bbox.max_z
    })
}

pub fn update_openings(
    time: Res<Time>,
    mut openings: ResMut<Openings>,
    mut transforms: Query<&mut Transform>,
) {
    let dt = time.delta_secs();
    for o in &mut openings.list {
        o.jolt *= (-9.0 * dt).exp();
        if o.kind == OpeningKind::Window {
            if let Some(pane) = o.pane_entity {
                if let Ok(mut tf) = transforms.get_mut(pane) {
                    tf.translation.z = -0.03 * o.jolt;
                }
            }
            continue;
        }
        if o.fall_t >= 0.0 {
            o.fall_t += dt;
            let k = (o.fall_t / 0.5).min(1.0);
            let slap = if k >= 1.0 {
                0.05 * (-(o.fall_t - 0.5) * 10.0).exp() * ((o.fall_t - 0.5) * 30.0).sin()
            } else {
                0.0
            };
            if let Some(leaf) = o.leaf_entity {
                if let Ok(mut tf) = transforms.get_mut(leaf) {
                    tf.rotation = Quat::IDENTITY;
                }
            }
            if let Some(fall) = o.fall_entity {
                if let Ok(mut tf) = transforms.get_mut(fall) {
                    tf.rotation = Quat::from_rotation_x(-(FRAC_PI_2 - 0.02) * k * k + slap);
                    tf.translation = Vec3::new(0.0, 0.08 * k, -0.02 - 0.35 * k);
                }
            }
            continue;
        }
        if o.hp <= 0 {
            continue;
        }
        if let Some(leaf) = o.leaf_entity {
            if let Ok(mut tf) = transforms.get_mut(leaf) {
                tf.rotation = Quat::from_rotation_y(o.jolt * 0.07);
            }
        }
    }
}

pub fn hit_opening(
    idx: usize,
    openings: &mut Openings,
    commands: &mut Commands,
    debris_assets: &DebrisAssets,
    audio: &AudioEngine,
    mats: &mut Query<&mut MeshMaterial3d<StandardMaterial>>,
    visibilities: &mut Query<&mut Visibility>,
) {
    let cracked = openings.glass_cracked.clone();
    let o = &mut openings.list[idx];
    if o.hp <= 0 {
        return;
    }
    let pos = Vec3::new(o.x, 0.0, o.z);
    o.hp -= 1;
    o.jolt = 1.0;
    if o.kind == OpeningKind::Door {
        burst(o, 2, false, 0.12, 2.0, commands, debris_assets);
        if o.hp > 0 {
            audio.bash(pos, 1.0);
        }
    } else if o.hp > 0 {
        audio.crack(pos);
        if let Some(glass) = o.glass_entity {
            if let Ok(mut mat) = mats.get_mut(glass) {
                mat.0 = cracked;
            }
        }
    }
    if o.hp <= 0 {
        if o.kind == OpeningKind::Door {
            audio.door_break(pos);
        } else {
            audio.shatter(pos);
        }
        break_opening(o, commands, debris_assets, visibilities);
    }
}

pub fn nearest_opening(openings: &Openings, pos: Vec3, skip: Option<usize>) -> Option<usize> {
    let mut best = None;
    let mut bd = f32::INFINITY;
    for (i, o) in openings.list.iter().enumerate() {
        if skip == Some(i) {
            continue;
        }
        let d = (o.x - pos.x).powi(2) + (o.z - pos.z).powi(2);
        if d < bd {
            bd = d;
            best = Some(i);
        }
    }
    best
}
