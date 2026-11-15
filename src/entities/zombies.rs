use bevy::prelude::*;

use crate::core::blocks::{BlockSpec, bake_blocks};
use crate::core::math::SeededRng;

pub const Z_THIGH: f32 = 0.42;
pub const Z_SHIN: f32 = 0.4;
#[allow(dead_code)]
pub const Z_ANKLE: f32 = 0.11;
pub const Z_HIP: f32 = 0.9;
pub const Z_UPPER_ARM: f32 = 0.4;
pub const Z_FOREARM: f32 = 0.39;
pub const Z_SCALE: f32 = 1.08;
pub const Z_UPPER: f32 = 1.1;

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
    let root = commands
        .spawn((
            Transform::from_translation(pos).with_scale(Vec3::splat(Z_SCALE)),
            Visibility::default(),
        ))
        .id();

    let lower = commands.spawn((Transform::default(), Visibility::default())).id();
    let pelvis = commands
        .spawn((
            Mesh3d(k.pelvis.clone()),
            MeshMaterial3d(assets.skin_mat.clone()),
            Transform::from_xyz(0.0, Z_HIP, 0.0),
        ))
        .id();
    commands.entity(root).add_child(lower);
    commands.entity(lower).add_child(pelvis);

    for (i, side) in [1.0_f32, -1.0].into_iter().enumerate() {
        let thigh = commands
            .spawn((
                Mesh3d(k.thigh.clone()),
                MeshMaterial3d(assets.skin_mat.clone()),
                Transform::from_xyz(side * 0.14, Z_HIP - Z_THIGH * 0.5, 0.0),
            ))
            .id();
        let shin = commands
            .spawn((
                Mesh3d(k.shin.clone()),
                MeshMaterial3d(assets.skin_mat.clone()),
                Transform::from_xyz(side * 0.14, Z_HIP - Z_THIGH - Z_SHIN * 0.5, 0.0),
            ))
            .id();
        let foot = commands
            .spawn((
                Mesh3d(k.feet[i].clone()),
                MeshMaterial3d(assets.skin_mat.clone()),
                Transform::from_xyz(side * 0.14, 0.11, 0.0),
            ))
            .id();
        commands.entity(lower).add_children(&[thigh, shin, foot]);
    }

    let upper = commands
        .spawn((
            Transform::from_xyz(0.0, Z_HIP, 0.0).with_scale(Vec3::new(Z_UPPER, 1.0, Z_UPPER)),
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
            Transform::from_xyz(0.0, 0.72, 0.03),
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
            Transform::from_xyz(0.0, 0.06, 0.03).with_rotation(Quat::from_rotation_x(0.35)),
        ))
        .id();

    commands.entity(root).add_child(upper);
    commands.entity(upper).add_children(&[torso, head]);
    commands.entity(head).add_children(&[eyes, jaw]);

    for side in [1.0_f32, -1.0] {
        let ua = commands
            .spawn((
                Mesh3d(k.upper_arm.clone()),
                MeshMaterial3d(assets.skin_mat.clone()),
                Transform::from_xyz(side * 0.42, 1.48, 0.2)
                    .with_rotation(Quat::from_rotation_x(-1.1)),
            ))
            .id();
        let fa = commands
            .spawn((
                Mesh3d(k.forearm.clone()),
                MeshMaterial3d(assets.skin_mat.clone()),
                Transform::from_xyz(side * 0.42, 1.32, 0.56)
                    .with_rotation(Quat::from_rotation_x(-1.3)),
            ))
            .id();
        commands.entity(root).add_children(&[ua, fa]);
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
