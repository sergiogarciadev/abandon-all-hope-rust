use bevy::prelude::*;
use rand::RngExt;
use std::f32::consts::{PI, TAU};

use crate::audio::AudioEngine;
use crate::core::blocks::{BlockSpec, bake_blocks};
use crate::core::math::{SeededRng, damp, ease, wrap_a};
use crate::core::state::GameStats;
use crate::effects::debris::DebrisAssets;
use crate::world::openings::{
    CLIMB_TIME, OpeningKind, Openings, hit_opening, nearest_opening, shut_at,
};
use crate::world::solids::{SolidsNav, inside_cabin};
use crate::world::world::{HALF, WallBoxes};

use super::animation::{bone_transform, foot_cycle, ik2};
use super::player::PlayerRig;

pub const Z_THIGH: f32 = 0.42;
pub const Z_SHIN: f32 = 0.4;
pub const Z_ANKLE: f32 = 0.11;
pub const Z_HIP: f32 = 0.9;
pub const Z_UPPER_ARM: f32 = 0.4;
pub const Z_FOREARM: f32 = 0.39;
pub const Z_SCALE: f32 = 1.08;
pub const Z_UPPER: f32 = 1.1;
const Z_SWING: f32 = 0.85;
const Z_STRIKE: f32 = 0.2;
const Z_REACH: f32 = 1.2;
const Z_LET_GO: f32 = 1.5;
const Z_CLOSE: f32 = 0.95;
const Z_SPACE: f32 = 0.8;
const Z_HIT_RANGE: f32 = 1.65;
const Z_TRACK: f32 = 7.0;
pub const HURT_IFRAMES: f32 = 1.5;
const CORPSE_LIFE: f32 = 20.0;
const MAX_CORPSES: usize = 120;
const BODY_THICK: f32 = 0.3;
const Z_POLE_KNEE: Vec3 = Vec3::Z;

pub struct ZombieLimb {
    pub side: f32,
    pub upper_arm: Entity,
    pub forearm: Entity,
    pub thigh: Entity,
    pub shin: Entity,
    pub foot: Entity,
    pub shoulder: Vec3,
    pub hip: Vec3,
}

#[derive(Component)]
pub struct Zombie {
    pub hp: f32,
    pub hurt: f32,
    pub body: Vec<Entity>,
    pub eyes: Entity,
    #[allow(dead_code)]
    pub pace: f32,
    pub swing_cd: f32,
    pub struck: bool,
    pub standing: bool,
    pub swing_at: Option<usize>,
    pub via: Option<usize>,
    pub stuck_t: f32,
    pub stuck_x: f32,
    pub stuck_z: f32,
    pub opening: Option<usize>,
    pub stage: u8,
    pub heading: f32,
    pub turn: f32,
    pub climb_o: Option<usize>,
    pub climb_from: Vec3,
}

#[derive(Component)]
pub struct Corpse {
    pub dead_t: f32,
    pub rest_y: f32,
    pub points: [(f32, f32); 3],
    pub under: Vec<Entity>,
    pub sinking: bool,
    pub sink_t: f32,
    pub pool: Entity,
    pub pool_size: f32,
    pub fall_dir: f32,
}

#[derive(Component)]
#[allow(dead_code)]
pub struct ZombieAnim {
    pub pelvis: Entity,
    pub upper: Entity,
    pub head: Entity,
    pub jaw: Entity,
    pub limbs: [ZombieLimb; 2],
    pub phase: f32,
    pub seed: f32,
    pub limp: f32,
    pub limp_side: f32,
    pub hunch: f32,
    pub tilt: f32,
    pub arms: u8,
    pub flinch: f32,
    pub attack: f32,
    pub groan: f32,
    pub gait: f32,
    pub speed: f32,
    pub swing: f32,
    pub climb: f32,
    pub fall_dir: f32,
}

pub const GORE: u32 = 0x5c1010;
pub const BLOOD: u32 = 0x3a0807;
pub const BONE: u32 = 0xcfc4a2;
pub const SOCKET: u32 = 0x120a08;

pub struct ZombieLook {
    pub shirt: u32,
    pub pants: u32,
    pub skin: u32,
    pub hair: u32,
    pub shoe: u32,
    pub bald: bool,
    pub tank: bool,
    pub ribs: bool,
    pub barefoot: bool,
}

pub const ZOMBIE_LOOKS: [ZombieLook; 4] = [
    ZombieLook {
        shirt: 0x4d6680,
        pants: 0x5a4632,
        skin: 0x7f9a66,
        hair: 0x2a2620,
        shoe: 0x2b2420,
        bald: false,
        tank: false,
        ribs: false,
        barefoot: false,
    },
    ZombieLook {
        shirt: 0x5b6239,
        pants: 0x6b5f43,
        skin: 0x8aa67a,
        hair: 0x4a3a28,
        shoe: 0x3a2c1f,
        bald: false,
        tank: false,
        ribs: true,
        barefoot: false,
    },
    ZombieLook {
        shirt: 0x9a9584,
        pants: 0x3a4a63,
        skin: 0x6f8a52,
        hair: 0x000000,
        shoe: 0x1c1c1e,
        bald: true,
        tank: true,
        ribs: false,
        barefoot: true,
    },
    ZombieLook {
        shirt: 0x86703a,
        pants: 0x38383b,
        skin: 0x82a07c,
        hair: 0x1b1714,
        shoe: 0x242226,
        bald: false,
        tank: false,
        ribs: true,
        barefoot: false,
    },
];

fn darker(hex: u32, k: f32) -> u32 {
    let r = (((hex >> 16) & 0xff) as f32 * k).round().clamp(0.0, 255.0) as u32;
    let g = (((hex >> 8) & 0xff) as f32 * k).round().clamp(0.0, 255.0) as u32;
    let b = ((hex & 0xff) as f32 * k).round().clamp(0.0, 255.0) as u32;
    (r << 16) | (g << 8) | b
}

pub struct ZombieKit {
    pub head: Handle<Mesh>,
    pub jaw: Handle<Mesh>,
    pub eyes: Handle<Mesh>,
    pub torso: Handle<Mesh>,
    pub pelvis: Handle<Mesh>,
    pub upper_arm: Handle<Mesh>,
    pub forearm: Handle<Mesh>,
    pub thigh: Handle<Mesh>,
    pub shin: Handle<Mesh>,
    pub feet: [Handle<Mesh>; 2],
}

#[derive(Resource)]
pub struct ZombieAssets {
    pub kits: Vec<ZombieKit>,
    pub skin_mat: Handle<StandardMaterial>,
    pub hurt_mat: Handle<StandardMaterial>,
    pub eyes_mat: Handle<StandardMaterial>,
    pub blood_pool_mat: Handle<StandardMaterial>,
    pub pool_mesh: Handle<Mesh>,
}

fn bake_zombie(l: &ZombieLook, seed: u32, meshes: &mut Assets<Mesh>) -> ZombieKit {
    let mut rng = SeededRng::new(seed);
    let skin_dk = darker(l.skin, 0.72);
    let shirt_dk = darker(l.shirt, 0.65);
    let pants_dk = darker(l.pants, 0.65);
    let sleeve = if l.tank { l.skin } else { l.shirt };

    let mut head = vec![
        BlockSpec::new(0.46, 0.44, 0.44, 0.0, 0.24, 0.0, l.skin),
        BlockSpec::new(0.47, 0.06, 0.07, 0.0, 0.33, 0.2, skin_dk),
        BlockSpec::new(0.11, 0.08, 0.03, -0.11, 0.27, 0.215, SOCKET),
        BlockSpec::new(0.11, 0.08, 0.03, 0.11, 0.27, 0.215, SOCKET),
        BlockSpec::new(0.06, 0.07, 0.04, 0.0, 0.2, 0.23, skin_dk),
        BlockSpec::new(0.25, 0.03, 0.03, 0.0, 0.06, 0.225, BONE).with_bevel(0.01),
        BlockSpec::new(0.05, 0.1, 0.08, 0.235, 0.24, 0.0, l.skin),
        BlockSpec::new(0.03, 0.1, 0.13, -0.232, 0.17, 0.07, GORE).with_bevel(0.01),
    ];
    if l.bald {
        head.push(BlockSpec::new(0.2, 0.03, 0.16, 0.06, 0.465, 0.03, GORE).with_bevel(0.01));
        head.push(BlockSpec::new(0.09, 0.03, 0.08, 0.07, 0.475, 0.03, BONE).with_bevel(0.01));
    } else {
        head.push(BlockSpec::new(0.44, 0.16, 0.08, 0.0, 0.32, -0.2, l.hair));
        for _ in 0..3 {
            head.push(BlockSpec::new(
                0.12 + rng.next_f32() * 0.08,
                0.06,
                0.12 + rng.next_f32() * 0.08,
                (rng.next_f32() - 0.5) * 0.28,
                0.48,
                (rng.next_f32() - 0.6) * 0.3,
                l.hair,
            ));
        }
    }

    let mut torso = vec![
        BlockSpec::new(0.64, 0.6, 0.36, 0.0, 0.32, 0.0, l.shirt),
        BlockSpec::new(0.7, 0.18, 0.38, 0.0, 0.56, -0.01, if l.tank { l.skin } else { l.shirt }),
        BlockSpec::new(0.4, 0.2, 0.12, 0.0, 0.5, -0.2, if l.tank { l.skin } else { l.shirt }),
        BlockSpec::new(0.2, 0.12, 0.2, 0.0, 0.67, 0.04, l.skin),
        BlockSpec::new(0.15, 0.13, 0.39, -0.21, 0.0, 0.0, shirt_dk).with_rot(0.0, 0.0, 0.25),
        BlockSpec::new(0.12, 0.1, 0.39, 0.19, -0.01, 0.0, shirt_dk).with_rot(0.0, 0.0, -0.2),
        BlockSpec::new(0.18, 0.24, 0.02, -0.03, 0.47, 0.185, BLOOD),
        BlockSpec::new(0.2, 0.17, 0.04, 0.15, 0.3, 0.17, l.skin),
        BlockSpec::new(0.12, 0.1, 0.03, 0.16, 0.3, 0.19, GORE),
    ];
    if l.tank {
        torso.push(BlockSpec::new(0.1, 0.18, 0.37, -0.17, 0.56, 0.0, l.shirt));
        torso.push(BlockSpec::new(0.1, 0.18, 0.37, 0.17, 0.56, 0.0, l.shirt));
    }
    if l.ribs {
        torso.push(BlockSpec::new(0.26, 0.26, 0.04, -0.15, 0.33, 0.175, GORE));
        for k in 0..3 {
            torso.push(
                BlockSpec::new(0.24, 0.035, 0.03, -0.15, 0.25 + k as f32 * 0.08, 0.2, BONE)
                    .with_bevel(0.01),
            );
        }
    }

    let pelvis = vec![
        BlockSpec::new(0.52, 0.26, 0.32, 0.0, 0.02, 0.0, l.pants),
        BlockSpec::new(0.54, 0.06, 0.34, 0.0, 0.13, 0.0, 0x2a221a).with_bevel(0.015),
        BlockSpec::new(0.18, 0.16, 0.04, 0.12, 0.02, 0.17, if l.tank { l.shirt } else { shirt_dk })
            .with_rot(0.0, 0.0, 0.12),
    ];

    let mut upper_arm = vec![BlockSpec::new(0.17, Z_UPPER_ARM + 0.06, 0.18, 0.0, 0.0, 0.0, sleeve)];
    if !l.tank {
        upper_arm.push(BlockSpec::new(0.19, 0.07, 0.2, 0.0, -Z_UPPER_ARM * 0.5 + 0.03, 0.0, shirt_dk));
    }

    let hand = -Z_FOREARM * 0.5 - 0.06;
    let mut forearm = vec![
        BlockSpec::new(0.15, Z_FOREARM + 0.04, 0.16, 0.0, 0.0, 0.0, l.skin),
        BlockSpec::new(0.03, 0.12, 0.1, 0.075, 0.05, 0.0, GORE).with_bevel(0.01),
        BlockSpec::new(0.16, 0.14, 0.1, 0.0, hand, 0.01, skin_dk),
    ];
    for x in [-0.05, 0.0, 0.05] {
        forearm.push(
            BlockSpec::new(
                0.035,
                0.12,
                0.035,
                x,
                hand - 0.12,
                0.02 + rng.next_f32() * 0.02,
                skin_dk,
            )
            .with_rot(0.2 + rng.next_f32() * 0.3, 0.0, 0.0)
            .with_bevel(0.008),
        );
    }

    let thigh = vec![BlockSpec::new(0.21, Z_THIGH + 0.06, 0.23, 0.0, 0.0, 0.0, l.pants)];
    let shin = vec![
        BlockSpec::new(0.19, Z_SHIN + 0.05, 0.21, 0.0, 0.0, 0.0, l.pants),
        BlockSpec::new(0.21, 0.07, 0.23, 0.0, -Z_SHIN * 0.5 + 0.02, 0.0, pants_dk),
    ];
    let shoe = vec![
        BlockSpec::new(0.22, 0.12, 0.34, 0.0, -0.05, 0.05, l.shoe),
        BlockSpec::new(0.23, 0.03, 0.35, 0.0, -0.1, 0.05, 0x141210).with_bevel(0.01),
    ];
    let bare = vec![
        BlockSpec::new(0.18, 0.08, 0.3, 0.0, -0.07, 0.06, skin_dk),
        BlockSpec::new(0.16, 0.05, 0.06, 0.0, -0.09, 0.22, skin_dk),
    ];
    let eyes = vec![
        BlockSpec::new(0.06, 0.045, 0.02, -0.11, 0.27, 0.225, 0xffffff).with_bevel(0.005),
        BlockSpec::new(0.06, 0.045, 0.02, 0.11, 0.27, 0.225, 0xffffff).with_bevel(0.005),
    ];
    let jaw = vec![
        BlockSpec::new(0.36, 0.09, 0.3, 0.0, -0.015, 0.15, l.skin),
        BlockSpec::new(0.22, 0.025, 0.03, 0.0, 0.035, 0.29, BONE).with_bevel(0.01),
        BlockSpec::new(0.2, 0.05, 0.02, 0.0, -0.035, 0.305, BLOOD).with_bevel(0.01),
    ];

    let mut b = |parts: &[BlockSpec]| meshes.add(bake_blocks(parts, &mut rng, false));
    let shoe_mesh = b(&shoe);
    let foot_r = if l.barefoot { b(&bare) } else { shoe_mesh.clone() };

    ZombieKit {
        head: b(&head),
        jaw: b(&jaw),
        eyes: b(&eyes),
        torso: b(&torso),
        pelvis: b(&pelvis),
        upper_arm: b(&upper_arm),
        forearm: b(&forearm),
        thigh: b(&thigh),
        shin: b(&shin),
        feet: [shoe_mesh, foot_r],
    }
}

pub fn spawn_zombie(
    commands: &mut Commands,
    assets: &ZombieAssets,
    kind: usize,
    pos: Vec3,
    hp: f32,
    base_speed: f32,
) -> Entity {
    let k = &assets.kits[kind % assets.kits.len()];
    let mut rng = rand::rng();
    let heading = (-pos.x).atan2(-pos.z);
    let scale = Z_SCALE * (0.94 + rng.random::<f32>() * 0.12);
    let pace = 0.85 + rng.random::<f32>() * 0.3;

    let root = commands
        .spawn((
            Transform::from_translation(pos)
                .with_rotation(Quat::from_rotation_y(heading))
                .with_scale(Vec3::splat(scale)),
            Visibility::default(),
        ))
        .id();

    let pelvis = commands
        .spawn((
            Mesh3d(k.pelvis.clone()),
            MeshMaterial3d(assets.skin_mat.clone()),
            Transform::from_xyz(0.0, Z_HIP, 0.0),
        ))
        .id();
    commands.entity(root).add_child(pelvis);

    let upper = commands
        .spawn((
            Transform::from_xyz(0.0, Z_HIP, 0.0).with_scale(Vec3::splat(Z_UPPER)),
            Visibility::default(),
        ))
        .id();
    let torso = commands
        .spawn((
            Mesh3d(k.torso.clone()),
            MeshMaterial3d(assets.skin_mat.clone()),
        ))
        .id();
    let head = commands
        .spawn((
            Mesh3d(k.head.clone()),
            MeshMaterial3d(assets.skin_mat.clone()),
            Transform::from_xyz(0.0, 0.68, 0.06),
        ))
        .id();
    let eyes = commands
        .spawn((
            Mesh3d(k.eyes.clone()),
            MeshMaterial3d(assets.eyes_mat.clone()),
        ))
        .id();
    let jaw = commands
        .spawn((
            Mesh3d(k.jaw.clone()),
            MeshMaterial3d(assets.skin_mat.clone()),
            Transform::from_xyz(0.0, 0.0, -0.06),
        ))
        .id();

    commands.entity(root).add_child(upper);
    commands.entity(upper).add_children(&[torso, head]);
    commands.entity(head).add_children(&[eyes, jaw]);

    let mut make_limb = |i: usize, side: f32| -> ZombieLimb {
        let thigh = commands
            .spawn((
                Mesh3d(k.thigh.clone()),
                MeshMaterial3d(assets.skin_mat.clone()),
                Transform::default(),
            ))
            .id();
        let shin = commands
            .spawn((
                Mesh3d(k.shin.clone()),
                MeshMaterial3d(assets.skin_mat.clone()),
                Transform::default(),
            ))
            .id();
        let foot = commands
            .spawn((
                Mesh3d(k.feet[i].clone()),
                MeshMaterial3d(assets.skin_mat.clone()),
                Transform::default(),
            ))
            .id();
        let upper_arm = commands
            .spawn((
                Mesh3d(k.upper_arm.clone()),
                MeshMaterial3d(assets.skin_mat.clone()),
                Transform::default(),
            ))
            .id();
        let forearm = commands
            .spawn((
                Mesh3d(k.forearm.clone()),
                MeshMaterial3d(assets.skin_mat.clone()),
                Transform::default(),
            ))
            .id();
        commands
            .entity(root)
            .add_children(&[thigh, shin, foot, upper_arm, forearm]);
        ZombieLimb {
            side,
            upper_arm,
            forearm,
            thigh,
            shin,
            foot,
            shoulder: Vec3::new(side * 0.36, 0.58, 0.0),
            hip: Vec3::new(side * 0.14, 0.0, 0.0),
        }
    };
    let limbs = [make_limb(0, 1.0), make_limb(1, -1.0)];

    let mut body = vec![pelvis, torso, head, jaw];
    for l in &limbs {
        body.extend_from_slice(&[l.thigh, l.shin, l.foot, l.upper_arm, l.forearm]);
    }

    let mut rng = rand::rng();
    commands.entity(root).insert((
        Zombie {
            hp,
            hurt: 0.0,
            body,
            eyes,
            pace,
            swing_cd: 0.15,
            struck: false,
            standing: false,
            swing_at: None,
            via: None,
            stuck_t: 0.0,
            stuck_x: pos.x,
            stuck_z: pos.z,
            opening: None,
            stage: 0,
            heading,
            turn: 3.0 + rng.random::<f32>() * 2.0,
            climb_o: None,
            climb_from: Vec3::ZERO,
        },
        ZombieAnim {
            pelvis,
            upper,
            head,
            jaw,
            limbs,
            phase: rng.random::<f32>(),
            seed: rng.random::<f32>() * 100.0,
            limp: if rng.random::<f32>() < 0.45 {
                0.5 + rng.random::<f32>() * 0.5
            } else {
                0.0
            },
            limp_side: if rng.random::<f32>() < 0.5 { 1.0 } else { -1.0 },
            hunch: 0.25 + rng.random::<f32>() * 0.3,
            tilt: (rng.random::<f32>() - 0.5) * 0.5,
            arms: if rng.random::<f32>() < 0.7 {
                2
            } else if rng.random::<f32>() < 0.85 {
                1
            } else {
                0
            },
            flinch: 0.0,
            attack: 0.0,
            groan: 0.0,
            gait: 1.0,
            speed: base_speed * pace,
            swing: -1.0,
            climb: -1.0,
            fall_dir: 1.0,
        },
    ));
    root
}

pub fn pose_zombie_rig(
    u: &mut ZombieAnim,
    dt: f32,
    elapsed: f32,
    near: f32,
    dead: f32,
    sag: f32,
    transforms: &mut Query<&mut Transform>,
) {
    let fall = if dead > 0.0 { u.fall_dir } else { 0.0 };
    let live = 1.0 - dead;
    let gait = u.gait * live;
    let sp = u.speed * gait;
    let run = ((u.speed - 3.0) / 2.0).clamp(0.0, 1.0);
    let freq = 1.1 + sp * 0.22;
    let beta = 0.6 - 0.18 * run;
    u.phase = (u.phase + freq * dt * live) % 1.0;
    let ph = u.phase * TAU;
    let stride_len = (sp * beta / freq).min(0.7);
    let t = elapsed + u.seed;

    u.flinch *= (-7.0 * dt).exp();
    u.groan *= (-2.5 * dt).exp();
    u.attack = damp(
        u.attack,
        if near < 2.4 && u.swing < 0.0 { live } else { 0.0 },
        6.0,
        dt,
    );

    let sw = u.swing;
    let rec = Z_SWING - Z_STRIKE - 0.1;
    let blow = if sw < 0.0 || dead > 0.0 {
        0.0
    } else if sw < Z_STRIKE {
        -ease(sw / Z_STRIKE)
    } else if sw < Z_STRIKE + 0.1 {
        -1.0 + 2.0 * ease((sw - Z_STRIKE) / 0.1)
    } else {
        1.0 - ease(((sw - Z_STRIKE - 0.1) / rec).min(1.0))
    };
    let rear = (-blow).max(0.0);
    let slam = blow.max(0.0);
    let clamber = if u.climb >= 0.0 && dead == 0.0 {
        (PI * u.climb).sin()
    } else {
        0.0
    };

    let bad_stance = if u.limp > 0.0 {
        (ph + if u.limp_side > 0.0 { 0.0 } else { PI }).sin().max(0.0)
    } else {
        0.0
    };
    let pelvis_y = Z_HIP
        - 0.05
        - 0.06 * run
        - 0.05 * ph.sin().abs() * gait
        - 0.06 * slam
        - 0.08 * u.limp * bad_stance
        - 0.3 * sag;
    let pelvis_rot = Quat::from_euler(
        EulerRot::YXZ,
        0.12 * ph.sin() * live,
        0.0,
        (0.1 * ph.sin() + 0.12 * u.limp * bad_stance * u.limp_side) * live,
    );
    let pelvis_m = Mat4::from_rotation_translation(pelvis_rot, Vec3::new(0.0, pelvis_y, 0.0));
    if let Ok(mut tf) = transforms.get_mut(u.pelvis) {
        tf.translation = Vec3::new(0.0, pelvis_y, 0.0);
        tf.rotation = pelvis_rot;
    }

    let upper_pos = Vec3::new(0.0, pelvis_y + 0.08, 0.0);
    let upper_rot = Quat::from_euler(
        EulerRot::YXZ,
        -0.15 * ph.sin() * live,
        (u.hunch + 0.12 * run + 0.25 * u.attack - 0.55 * rear + 0.5 * slam) * live
            - 0.6 * u.flinch
            + 0.35 * fall * dead,
        (-0.14 * ph.sin() - 0.1 * u.limp * bad_stance * u.limp_side) * live + 0.04 * (t * 0.7).sin(),
    );
    let upper_m = Mat4::from_scale_rotation_translation(Vec3::splat(Z_UPPER), upper_rot, upper_pos);
    if let Ok(mut tf) = transforms.get_mut(u.upper) {
        tf.translation = upper_pos;
        tf.rotation = upper_rot;
    }

    if let Ok(mut tf) = transforms.get_mut(u.head) {
        tf.rotation = Quat::from_euler(
            EulerRot::YXZ,
            0.25 * (t * 0.9).sin() * live,
            (-u.hunch * 0.7 + 0.12 * ph.sin().abs() - 0.25 * u.attack - 0.2 * rear) * live
                - 0.7 * u.flinch
                + (if fall > 0.0 { 0.3 } else { -0.6 }) * dead,
            u.tilt + 0.18 * (t * 1.3 + 1.0).sin() * live,
        );
    }
    if let Ok(mut tf) = transforms.get_mut(u.jaw) {
        let jx = (0.12
            + 0.12 * (t * 2.3).sin().abs()
            + 0.45 * u.groan
            + 0.35 * u.attack * (t * 9.0).sin().abs()
            + 0.5 * rear)
            * live
            + 0.5 * dead;
        tf.rotation = Quat::from_rotation_x(jx);
    }

    for l in &u.limbs {
        let bad = u.limp > 0.0 && l.side == u.limp_side;
        let p = (u.phase + if l.side > 0.0 { 0.0 } else { 0.5 + 0.06 * u.limp }) % 1.0;
        let fc = foot_cycle(p, beta);
        let foot3 = Vec3::new(
            l.side * (0.15 + 0.06 * dead),
            Z_ANKLE
                + fc.lift * (if bad { 0.03 } else { 0.12 + 0.1 * run }) * gait
                + 0.3 * clamber,
            fc.s * stride_len - 0.2 * clamber,
        );
        let hip_p = pelvis_m.transform_point3(l.hip);
        let knee = ik2(hip_p, foot3, Z_THIGH, Z_SHIN, Z_POLE_KNEE);
        let ankle = knee + (foot3 - knee).normalize_or_zero() * Z_SHIN;

        if let Ok(mut tf) = transforms.get_mut(l.thigh) {
            *tf = bone_transform(hip_p, knee, Z_POLE_KNEE);
        }
        if let Ok(mut tf) = transforms.get_mut(l.shin) {
            *tf = bone_transform(knee, ankle, Z_POLE_KNEE);
        }
        if let Ok(mut tf) = transforms.get_mut(l.foot) {
            tf.translation = ankle;
            tf.rotation = Quat::from_euler(
                EulerRot::YXZ,
                l.side * if bad { 0.35 } else { 0.12 },
                if bad { 0.35 } else { fc.pitch * live },
                0.0,
            );
        }

        let sh = upper_m.transform_point3(l.shoulder);
        let reaches = (if l.side > 0.0 { u.arms >= 1 } else { u.arms >= 2 }) && dead == 0.0;
        let k = l.side * 1.7;
        let mut reach = if reaches {
            let claw = u.attack * (t * 9.0 + k).sin();
            Vec3::new(
                -l.side * 0.08,
                -0.08 + 0.06 * (ph + k).sin() + 0.18 * u.attack + 0.12 * claw - 0.3 * u.flinch,
                0.72 + 0.07 * (ph * 2.0 + k).sin() + 0.2 * claw - 0.35 * u.flinch,
            )
        } else if dead > 0.0 {
            Vec3::new(l.side * 0.45 * dead, -0.7, -0.15 * fall * dead)
        } else {
            Vec3::new(
                l.side * 0.1,
                -0.84,
                0.1 + 0.16 * (ph + if l.side > 0.0 { PI } else { 0.0 }).sin() * gait + 0.25 * u.flinch,
            )
        };

        if rear > 0.0 {
            reach = reach.lerp(Vec3::new(l.side * 0.22, 0.5, 0.18), rear);
        }
        if slam > 0.0 {
            reach = reach.lerp(Vec3::new(-l.side * 0.06, -0.3, 0.78), slam);
        }
        if clamber > 0.0 {
            reach = reach.lerp(Vec3::new(l.side * 0.24, -0.6, 0.45), clamber);
        }
        reach += sh;

        let arm_pole = Vec3::new(l.side * 0.6, -1.0, -0.3);
        let elbow = ik2(sh, reach, Z_UPPER_ARM, Z_FOREARM, arm_pole);
        let wrist = elbow + (reach - elbow).normalize_or_zero() * Z_FOREARM;
        if let Ok(mut tf) = transforms.get_mut(l.upper_arm) {
            *tf = bone_transform(sh, elbow, arm_pole);
        }
        if let Ok(mut tf) = transforms.get_mut(l.forearm) {
            *tf = bone_transform(elbow, wrist, arm_pole);
        }
    }
}

pub fn setup_zombie_assets(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let kits: Vec<ZombieKit> = ZOMBIE_LOOKS
        .iter()
        .enumerate()
        .map(|(i, look)| bake_zombie(look, 7 + i as u32 * 31, &mut meshes))
        .collect();

    let skin_mat = materials.add(StandardMaterial {
        perceptual_roughness: 0.9,
        ..default()
    });
    let hurt_mat = materials.add(StandardMaterial {
        perceptual_roughness: 0.9,
        emissive: LinearRgba::rgb(0.85, 0.91, 0.69) * 0.45,
        ..default()
    });
    let eyes_mat = materials.add(StandardMaterial {
        base_color: Color::srgb_u8(0xd9, 0xe0, 0x6a),
        emissive: LinearRgba::rgb(0.85, 0.88, 0.42),
        unlit: true,
        ..default()
    });
    let blood_pool_mat = materials.add(StandardMaterial {
        base_color: Color::srgb_u8(0x3a, 0x06, 0x06),
        perceptual_roughness: 0.25,
        ..default()
    });

    let mut pool_rng = SeededRng::new(99);
    let mut slabs = vec![
        BlockSpec::new(0.8, 0.02, 0.8, 0.0, 0.0, 0.0, 0xffffff)
            .with_rot(0.0, 0.3, 0.0)
            .with_bevel(0.008),
    ];
    for i in 0..6 {
        let a = i as f32 * 1.1 + pool_rng.next_f32();
        let r = 0.3 + pool_rng.next_f32() * 0.25;
        let s = 0.25 + pool_rng.next_f32() * 0.3;
        slabs.push(
            BlockSpec::new(
                s,
                0.02,
                s * (0.6 + pool_rng.next_f32() * 0.6),
                a.cos() * r,
                0.003 * i as f32,
                a.sin() * r,
                0xffffff,
            )
            .with_rot(0.0, pool_rng.next_f32() * 3.0, 0.0)
            .with_bevel(0.008),
        );
    }
    let pool_mesh = meshes.add(bake_blocks(&slabs, &mut pool_rng, false));

    let assets = ZombieAssets {
        kits,
        skin_mat,
        hurt_mat,
        eyes_mat,
        blood_pool_mat,
        pool_mesh,
    };

    commands.insert_resource(assets);
}

pub fn zombie_dies(
    z_ent: Entity,
    z_tf: &Transform,
    z: &mut Zombie,
    anim: &mut ZombieAnim,
    dir: Option<Vec3>,
    player_pos: Vec3,
    commands: &mut Commands,
    assets: &ZombieAssets,
    openings: &mut Openings,
    corpses_q: &Query<(Entity, &Corpse)>,
    mats: &mut Query<&mut MeshMaterial3d<StandardMaterial>>,
    visibilities: &mut Query<&mut Visibility>,
) {
    if anim.climb >= 0.0 {
        if let Some(o_idx) = z.climb_o {
            let o = &mut openings.list[o_idx];
            if o.climber == Some(z_ent) {
                o.climber = None;
            }
        }
        anim.climb = -1.0;
        z.climb_o = None;
    }

    for &m in &z.body {
        if let Ok(mut mat) = mats.get_mut(m) {
            mat.0 = assets.skin_mat.clone();
        }
    }
    if let Ok(mut vis) = visibilities.get_mut(z.eyes) {
        *vis = Visibility::Hidden;
    }

    let (fx, fz) = if let Some(d) = dir {
        (d.x, d.z)
    } else {
        (
            z_tf.translation.x - player_pos.x,
            z_tf.translation.z - player_pos.z,
        )
    };
    let fall_dir = if fx * z.heading.sin() + fz * z.heading.cos() >= 0.0 {
        1.0
    } else {
        -1.0
    };
    anim.fall_dir = fall_dir;

    let dx = z.heading.sin() * fall_dir;
    let dz = z.heading.cos() * fall_dir;
    let s = z_tf.scale.x;
    let points = [
        (z_tf.translation.x + dx * 0.5 * s, z_tf.translation.z + dz * 0.5 * s),
        (z_tf.translation.x + dx * 1.1 * s, z_tf.translation.z + dz * 1.1 * s),
        (z_tf.translation.x + dx * 1.7 * s, z_tf.translation.z + dz * 1.7 * s),
    ];

    let mut rest_y = 0.0_f32;
    let mut under = Vec::new();
    for (c_ent, c) in corpses_q.iter() {
        if c.sinking {
            continue;
        }
        let overlaps = c.points.iter().any(|&(ax, az)| {
            points
                .iter()
                .any(|&(bx, bz)| (ax - bx).powi(2) + (az - bz).powi(2) < 0.36)
        });
        if overlaps {
            under.push(c_ent);
            rest_y = rest_y.max(c.rest_y + BODY_THICK);
        }
    }
    rest_y = rest_y.min(1.5);

    let mut rng = rand::rng();
    let pool_size = 1.1 + rng.random::<f32>() * 0.6;
    let pool = commands
        .spawn((
            Mesh3d(assets.pool_mesh.clone()),
            MeshMaterial3d(assets.blood_pool_mat.clone()),
            Transform::from_xyz(
                z_tf.translation.x + dx * 0.9,
                0.045,
                z_tf.translation.z + dz * 0.9,
            )
            .with_rotation(Quat::from_rotation_y(rng.random::<f32>() * PI))
            .with_scale(Vec3::splat(0.05)),
        ))
        .id();

    commands.entity(z_ent).remove::<Zombie>().insert(Corpse {
        dead_t: 0.0,
        rest_y,
        points,
        under,
        sinking: false,
        sink_t: 0.0,
        pool,
        pool_size,
        fall_dir,
    });
}

pub fn update_corpses(
    mut commands: Commands,
    time: Res<Time>,
    mut corpses: Query<(Entity, &mut Corpse, &mut ZombieAnim)>,
    mut transforms: Query<&mut Transform>,
) {
    let dt = time.delta_secs();
    let elapsed = time.elapsed_secs();
    let total = corpses.iter().count();

    let mut to_sink = Vec::new();
    for (i, (ent, c, _)) in corpses.iter().enumerate() {
        if !c.sinking && (c.dead_t > CORPSE_LIFE || total.saturating_sub(i) > MAX_CORPSES) {
            to_sink.push(ent);
        }
    }
    if !to_sink.is_empty() {
        let mut changed = true;
        while changed {
            changed = false;
            for (ent, c, _) in corpses.iter() {
                if !to_sink.contains(&ent) && c.under.iter().any(|u| to_sink.contains(u)) {
                    to_sink.push(ent);
                    changed = true;
                }
            }
        }
    }

    for (ent, mut c, mut anim) in &mut corpses {
        c.dead_t += dt;
        if to_sink.contains(&ent) {
            c.sinking = true;
        }
        if c.sinking {
            c.sink_t += dt;
            let dy = -c.sink_t * (0.6 + c.rest_y * 0.5);
            if let Ok(mut tf) = transforms.get_mut(ent) {
                tf.translation.y = (0.12 + c.rest_y) + dy;
            }
            if let Ok(mut p_tf) = transforms.get_mut(c.pool) {
                p_tf.translation.y = 0.045 - c.sink_t * 0.04;
            }
            if dy < -1.1 - c.rest_y {
                commands.entity(c.pool).despawn();
                commands.entity(ent).despawn();
            }
            continue;
        }

        let crumple = (c.dead_t / 0.25).min(1.0);
        let fall = ((c.dead_t - 0.1) / 0.45).clamp(0.0, 1.0);
        pose_zombie_rig(
            &mut anim,
            dt,
            elapsed,
            99.0,
            crumple,
            crumple * (1.0 - 0.85 * fall),
            &mut transforms,
        );

        let f = fall * fall;
        let bounce = if fall >= 1.0 {
            0.06 * (-(c.dead_t - 0.55) * 9.0).exp() * ((c.dead_t - 0.55) * 30.0).sin()
        } else {
            0.0
        };
        if let Ok(mut tf) = transforms.get_mut(ent) {
            let (y, _, z_rot) = tf.rotation.to_euler(EulerRot::YXZ);
            tf.rotation = Quat::from_euler(EulerRot::YXZ, y, c.fall_dir * (f * 1.42 - bounce), z_rot);
            tf.translation.y = (0.12 + c.rest_y) * f;
        }
        if let Ok(mut p_tf) = transforms.get_mut(c.pool) {
            let ps = ((c.dead_t - 0.4) / 1.4).clamp(0.05, 1.0) * c.pool_size;
            p_tf.scale = Vec3::splat(ps);
        }
    }
}

fn wall_between(a: Vec3, b: Vec3, wall_boxes: &WallBoxes, openings: &Openings) -> bool {
    for i in 1..4 {
        let t = i as f32 / 4.0;
        let x = a.x + (b.x - a.x) * t;
        let z = a.z + (b.z - a.z) * t;
        for w in &wall_boxes.boxes {
            if x >= w.min_x && x <= w.max_x && z >= w.min_z && z <= w.max_z {
                return true;
            }
        }
        if shut_at(openings, x, z).is_some() {
            return true;
        }
    }
    false
}

fn face_toward(z: &mut Zombie, tf: &mut Transform, dx: f32, dz: f32, rate: f32, dt: f32) -> f32 {
    let d_a = wrap_a(dx.atan2(dz) - z.heading);
    let step = rate * dt;
    z.heading += d_a.clamp(-step, step);
    let (_, x_rot, z_rot) = tf.rotation.to_euler(EulerRot::YXZ);
    tf.rotation = Quat::from_euler(EulerRot::YXZ, z.heading, x_rot, z_rot);
    d_a.abs() - d_a.abs().min(step)
}

fn walk_toward(z: &mut Zombie, tf: &mut Transform, dx: f32, dz: f32, speed: f32, turn: f32, dt: f32) {
    let off = face_toward(z, tf, dx, dz, turn, dt);
    let v = speed * off.min(PI * 0.5).cos().max(0.15);
    tf.translation.x += z.heading.sin() * v * dt;
    tf.translation.z += z.heading.cos() * v * dt;
}

pub fn chase_zombies(
    mut commands: Commands,
    time: Res<Time>,
    rig: Res<PlayerRig>,
    mut stats: ResMut<GameStats>,
    solids: Res<SolidsNav>,
    wall_boxes: Res<WallBoxes>,
    mut openings: ResMut<Openings>,
    debris_assets: Res<DebrisAssets>,
    zombie_assets: Res<ZombieAssets>,
    audio: Res<AudioEngine>,
    mut zombies: Query<(Entity, &mut Zombie, &mut ZombieAnim)>,
    mut transforms: Query<&mut Transform>,
    mut mats: Query<&mut MeshMaterial3d<StandardMaterial>>,
    mut visibilities: Query<&mut Visibility>,
) {
    let dt = time.delta_secs();
    let elapsed = time.elapsed_secs();
    stats.dmg_cd = (stats.dmg_cd - dt).max(0.0);

    let p_pos = transforms
        .get(rig.root)
        .map(|t| t.translation)
        .unwrap_or(Vec3::ZERO);

    let mut rng = rand::rng();

    for (z_ent, mut z, mut anim) in &mut zombies {
        if z.hurt > 0.0 {
            z.hurt -= dt;
            if z.hurt <= 0.0 {
                for &m in &z.body {
                    if let Ok(mut mat) = mats.get_mut(m) {
                        mat.0 = zombie_assets.skin_mat.clone();
                    }
                }
            }
        }

        let Ok(mut z_tf) = transforms.get_mut(z_ent) else {
            continue;
        };
        if anim.groan < 0.05 && rng.random::<f32>() < dt * 0.08 {
            anim.groan = 1.0;
            audio.groan(z_tf.translation);
        }
        let pdx = p_pos.x - z_tf.translation.x;
        let pdz = p_pos.z - z_tf.translation.z;
        let pd = pdx.hypot(pdz);
        let mut walking = false;

        if anim.climb >= 0.0 {
            if let Some(o_idx) = z.climb_o {
                anim.climb = (anim.climb + dt / CLIMB_TIME).min(1.0);
                let k = anim.climb;
                let e = ease(k);
                let o = &openings.list[o_idx];
                z_tf.translation.x = z.climb_from.x + (o.lx - z.climb_from.x) * e;
                z_tf.translation.z = z.climb_from.z + (o.lz - z.climb_from.z) * e;
                z_tf.translation.y = (o.sill - 0.15) * ((PI * k).sin() * 1.4).min(1.0);
                let (y, _, z_rot) = z_tf.rotation.to_euler(EulerRot::YXZ);
                z_tf.rotation = Quat::from_euler(EulerRot::YXZ, y, 0.75 * (PI * k).sin(), z_rot);
                if k >= 1.0 {
                    let o = &mut openings.list[o_idx];
                    if o.climber == Some(z_ent) {
                        o.climber = None;
                    }
                    z_tf.translation = Vec3::new(o.lx, 0.0, o.lz);
                    z_tf.rotation = Quat::from_rotation_y(z.heading);
                    anim.climb = -1.0;
                    z.climb_o = None;
                    z.opening = None;
                    z.stage = 0;
                }
            }
        } else if anim.swing >= 0.0 {
            anim.swing += dt;
            if anim.swing < Z_STRIKE {
                let (tx, tz) = if let Some(o_idx) = z.swing_at {
                    let o = &openings.list[o_idx];
                    (o.x - z_tf.translation.x, o.z - z_tf.translation.z)
                } else {
                    (pdx, pdz)
                };
                face_toward(&mut z, &mut z_tf, tx, tz, Z_TRACK, dt);
            }
            if !z.struck && anim.swing >= Z_STRIKE {
                z.struck = true;
                if let Some(o_idx) = z.swing_at {
                    hit_opening(
                        o_idx,
                        &mut openings,
                        &mut commands,
                        &debris_assets,
                        &audio,
                        &mut mats,
                        &mut visibilities,
                    );
                } else {
                    audio.swipe(z_tf.translation);
                    if pd < Z_HIT_RANGE
                        && !wall_between(z_tf.translation, p_pos, &wall_boxes, &openings)
                        && stats.dmg_cd <= 0.0
                    {
                        stats.hp -= 1;
                        stats.dmg_cd = HURT_IFRAMES;
                        stats.shake = stats.shake.max(0.45);
                        audio.hurt();
                    }
                }
            }
            if anim.swing >= Z_SWING {
                anim.swing = -1.0;
                z.swing_cd = 0.25 + rng.random::<f32>() * 0.35;
            }
        } else if pd < (if z.standing { Z_LET_GO } else { Z_REACH })
            && !wall_between(z_tf.translation, p_pos, &wall_boxes, &openings)
        {
            if !z.standing {
                z.swing_cd = 0.0;
            }
            z.standing = true;
            let off = face_toward(&mut z, &mut z_tf, pdx, pdz, Z_TRACK, dt);
            z.swing_cd -= dt;
            if z.swing_cd <= 0.0 && off < 0.5 {
                anim.swing = 0.0;
                z.struck = false;
                z.swing_at = None;
            }
        } else {
            z.standing = false;
            let mut target = Some((p_pos.x, p_pos.z));
            if inside_cabin(z_tf.translation, HALF) {
                z.opening = None;
                z.stage = 0;
            } else {
                if z.opening.is_none() {
                    z.opening = nearest_opening(&openings, z_tf.translation, None);
                    z.stage = 0;
                }
                if let Some(o_idx) = z.opening {
                    let (ox, oz, ix, iz, fx, fz, lx, lz, ox_w, oz_w, is_door, hp, has_climber) = {
                        let o = &openings.list[o_idx];
                        (
                            o.ox,
                            o.oz,
                            o.ix,
                            o.iz,
                            o.fx,
                            o.fz,
                            o.lx,
                            o.lz,
                            o.x,
                            o.z,
                            o.kind == OpeningKind::Door,
                            o.hp,
                            o.climber.is_some(),
                        )
                    };
                    if z.stage == 0 {
                        target = Some((ox, oz));
                        if (ox - z_tf.translation.x).powi(2) + (oz - z_tf.translation.z).powi(2)
                            < 1.4
                        {
                            z.stage = 1;
                        }
                    }
                    if z.stage == 1 {
                        if is_door && hp <= 0 {
                            target = Some((ix, iz));
                        } else if (ox_w - z_tf.translation.x).powi(2)
                            + (oz_w - z_tf.translation.z).powi(2)
                            > 1.45 * 1.45
                        {
                            target = Some((fx, fz));
                        } else {
                            target = None;
                            let dx = ox_w - z_tf.translation.x;
                            let dz = oz_w - z_tf.translation.z;
                            let off = face_toward(&mut z, &mut z_tf, dx, dz, Z_TRACK, dt);
                            z.swing_cd -= dt;
                            if hp > 0 {
                                if z.swing_cd <= 0.0 && off < 0.5 {
                                    anim.swing = 0.0;
                                    z.struck = false;
                                    z.swing_at = Some(o_idx);
                                }
                            } else if !has_climber {
                                openings.list[o_idx].climber = Some(z_ent);
                                anim.climb = 0.0;
                                z.climb_o = Some(o_idx);
                                z.climb_from = z_tf.translation;
                                z.heading =
                                    (lx - z_tf.translation.x).atan2(lz - z_tf.translation.z);
                                z_tf.rotation = Quat::from_rotation_y(z.heading);
                            }
                        }
                    }
                }
            }

            if let Some((tx, tz)) = target {
                let g = solids.route_around(
                    Vec2::new(z_tf.translation.x, z_tf.translation.z),
                    tx,
                    tz,
                    &mut z.via,
                );
                let turn_rate = z.turn * if pd < 4.0 { 2.5 } else { 1.0 };
                let dx = g.x - z_tf.translation.x;
                let dz = g.y - z_tf.translation.z;
                walk_toward(&mut z, &mut z_tf, dx, dz, anim.speed, turn_rate, dt);
                walking = true;
            }
        }

        if walking {
            z.stuck_t += dt;
            if z.stuck_t > 2.0 {
                if (z_tf.translation.x - z.stuck_x).hypot(z_tf.translation.z - z.stuck_z) < 0.6
                    && z.opening.is_some()
                    && !inside_cabin(z_tf.translation, HALF)
                {
                    z.opening = nearest_opening(&openings, z_tf.translation, z.opening);
                    z.stage = 0;
                    z.via = None;
                }
                z.stuck_t = 0.0;
            }
        } else {
            z.stuck_t = 0.0;
        }
        if !walking || z.stuck_t == 0.0 {
            z.stuck_x = z_tf.translation.x;
            z.stuck_z = z_tf.translation.z;
        }

        anim.gait = damp(anim.gait, if walking { 1.0 } else { 0.0 }, 8.0, dt);
        pose_zombie_rig(&mut anim, dt, elapsed, pd, 0.0, 0.0, &mut transforms);
    }
}

pub fn separate_zombies(
    rig: Res<PlayerRig>,
    solids: Res<SolidsNav>,
    player_q: Query<&Transform, Without<ZombieAnim>>,
    mut zombies: Query<(Entity, &ZombieAnim, &mut Transform)>,
) {
    let p_pos = player_q
        .get(rig.root)
        .map(|t| t.translation)
        .unwrap_or(Vec3::ZERO);

    let mut positions: Vec<(Entity, Vec3, bool)> = zombies
        .iter()
        .map(|(e, anim, tf)| (e, tf.translation, anim.climb >= 0.0))
        .collect();

    let n = positions.len();
    for i in 0..n {
        for k in (i + 1)..n {
            let dx = positions[k].1.x - positions[i].1.x;
            let dz = positions[k].1.z - positions[i].1.z;
            let d2 = dx * dx + dz * dz;
            if d2 >= Z_SPACE * Z_SPACE || d2 < 1e-6 {
                continue;
            }
            let d = d2.sqrt();
            let push = (Z_SPACE - d) * 0.5 / d;
            positions[i].1.x -= dx * push;
            positions[i].1.z -= dz * push;
            positions[k].1.x += dx * push;
            positions[k].1.z += dz * push;
        }
    }

    for (e, mut pos, climbing) in positions {
        if climbing {
            continue;
        }
        let dx = pos.x - p_pos.x;
        let dz = pos.z - p_pos.z;
        let d = dx.hypot(dz);
        if d < Z_CLOSE && d > 1e-6 {
            pos.x += dx / d * (Z_CLOSE - d);
            pos.z += dz / d * (Z_CLOSE - d);
        }
        let mut v2 = Vec2::new(pos.x, pos.z);
        solids.push_out(&mut v2);
        pos.x = v2.x;
        pos.z = v2.y;
        if let Ok((_, _, mut tf)) = zombies.get_mut(e) {
            tf.translation.x = pos.x;
            tf.translation.z = pos.z;
        }
    }
}
