use bevy::prelude::*;
use std::f32::consts::{PI, TAU};

use crate::core::blocks::{BlockSpec, bake_blocks};
use crate::core::math::{SeededRng, damp, ease};

use super::animation::{bone_transform, foot_cycle, ik2};

pub const Z_THIGH: f32 = 0.42;
pub const Z_SHIN: f32 = 0.4;
pub const Z_ANKLE: f32 = 0.11;
pub const Z_HIP: f32 = 0.9;
pub const Z_UPPER_ARM: f32 = 0.4;
pub const Z_FOREARM: f32 = 0.39;
pub const Z_SCALE: f32 = 1.08;
pub const Z_UPPER: f32 = 1.1;
const Z_SWING: f32 = 0.65;
const Z_STRIKE: f32 = 0.25;
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
#[allow(dead_code)]
pub struct ZombieAssets {
    pub kits: Vec<ZombieKit>,
    pub skin_mat: Handle<StandardMaterial>,
    pub hurt_mat: Handle<StandardMaterial>,
    pub eyes_mat: Handle<StandardMaterial>,
    pub blood_pool_mat: Handle<StandardMaterial>,
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

pub fn spawn_zombie_preview(commands: &mut Commands, assets: &ZombieAssets, kind: usize, pos: Vec3) {
    let k = &assets.kits[kind % assets.kits.len()];
    let mut rng = SeededRng::new(100 + kind as u32 * 37);

    let root = commands
        .spawn((
            Transform::from_translation(pos).with_scale(Vec3::splat(Z_SCALE)),
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

    commands.entity(root).insert(ZombieAnim {
        pelvis,
        upper,
        head,
        jaw,
        limbs,
        phase: rng.next_f32(),
        seed: rng.next_f32() * 100.0,
        limp: if rng.next_f32() < 0.45 { 0.5 + rng.next_f32() * 0.5 } else { 0.0 },
        limp_side: if rng.next_f32() < 0.5 { 1.0 } else { -1.0 },
        hunch: 0.25 + rng.next_f32() * 0.3,
        tilt: (rng.next_f32() - 0.5) * 0.5,
        arms: if rng.next_f32() < 0.7 { 2 } else if rng.next_f32() < 0.85 { 1 } else { 0 },
        flinch: 0.0,
        attack: 0.0,
        groan: 0.0,
        gait: 1.0,
        speed: 2.2 * (0.85 + rng.next_f32() * 0.3),
        swing: -1.0,
        climb: -1.0,
        fall_dir: 1.0,
    });
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

pub fn animate_preview_zombies(
    time: Res<Time>,
    mut zombies: Query<&mut ZombieAnim>,
    mut transforms: Query<&mut Transform>,
) {
    let dt = time.delta_secs();
    let elapsed = time.elapsed_secs();
    for mut z in &mut zombies {
        pose_zombie_rig(&mut z, dt, elapsed, 5.0, 0.0, 0.0, &mut transforms);
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

    let assets = ZombieAssets {
        kits,
        skin_mat,
        hurt_mat,
        eyes_mat,
        blood_pool_mat,
    };

    // Preview the 4 zombie archetypes outside the south door so we can inspect them in Step 3
    for i in 0..4 {
        spawn_zombie_preview(
            &mut commands,
            &assets,
            i,
            Vec3::new(-3.0 + i as f32 * 2.0, 0.0, 4.5),
        );
    }

    commands.insert_resource(assets);
}
