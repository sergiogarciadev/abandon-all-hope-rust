use bevy::asset::RenderAssetUsages;
use bevy::image::{ImageAddressMode, ImageFilterMode, ImageSampler, ImageSamplerDescriptor};
use bevy::light::NotShadowCaster;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use rand::RngExt;
use std::f32::consts::{FRAC_PI_2, FRAC_PI_4, PI, TAU};

use crate::core::blocks::bevel_box_raw;
use crate::core::math::{damp, ease, wrap_a};
use crate::core::state::{AimState, GameStats};
use crate::effects::debris::{Debris, DebrisAssets, spawn_smoke};
use crate::world::world::HALF;

use super::animation::{bone_transform, foot_cycle, ik2};

pub const THIGH: f32 = 0.44;
pub const SHIN: f32 = 0.42;
pub const ANKLE_H: f32 = 0.15;
pub const HIP_H: f32 = 0.97;
pub const UPPER_ARM: f32 = 0.42;
pub const FOREARM: f32 = 0.42;

const BETA: f32 = 0.38;
const HIP_MAX: f32 = 1.0;
const IDLE_HIP: f32 = -0.35;
const BLADE: f32 = -0.62;
const POCKET: Vec3 = Vec3::new(-0.13, 0.6, 0.22);
const POUCH: Vec3 = Vec3::new(0.17, 0.33, 0.36);
const KNEE_POLE: Vec3 = Vec3::Z;
const PLAYER_SPEED: f32 = 6.2;

const GRIP_R: Vec3 = Vec3::new(0.0, -0.1, 0.3);
const GRIP_L: Vec3 = Vec3::new(0.0, -0.07, 0.74);
const MAG_Y: f32 = -0.14;
const MAG_BASE: Vec3 = Vec3::new(0.0, -0.29, 0.53);
const MAG_OUT: Vec3 = Vec3::new(0.0, -0.53, 0.53);
const MAG_LINEUP: Vec3 = Vec3::new(0.0, -0.49, 0.53);
const EJECT: Vec3 = Vec3::new(-0.08, 0.06, 0.48);
const FLASH_COLOR: u32 = 0xffa64d;
const FLASH_HOT: u32 = 0xff6a2a;

#[derive(Resource)]
#[allow(dead_code)]
pub struct PlayerRigState {
    pub phase: f32,
    pub gait: f32,
    pub turn_w: f32,
    pub turning: bool,
    pub hip_yaw: f32,
    pub back: bool,
    pub stride: Vec3,
    pub lean_f: f32,
    pub lean_s: f32,
    pub kick: f32,
    pub flash: f32,
    pub flash_peak: f32,
    pub reload: f32,
    pub mag_dropped: bool,
    pub t: f32,
    pub vel: Vec2,
    pub speed: f32,
    pub reloading: bool,
    pub reload_t: f32,
    pub reload_time: f32,
}

impl Default for PlayerRigState {
    fn default() -> Self {
        Self {
            phase: 0.0,
            gait: 0.0,
            turn_w: 0.0,
            turning: false,
            hip_yaw: 0.0,
            back: false,
            stride: Vec3::Z,
            lean_f: 0.0,
            lean_s: 0.0,
            kick: 0.0,
            flash: 0.0,
            flash_peak: 0.0,
            reload: 0.0,
            mag_dropped: false,
            t: 0.0,
            vel: Vec2::ZERO,
            speed: PLAYER_SPEED,
            reloading: false,
            reload_t: 0.0,
            reload_time: 1.2,
        }
    }
}

#[allow(dead_code)]
pub struct LegRig {
    pub side: f32,
    pub thigh: Entity,
    pub shin: Entity,
    pub boot: Entity,
    pub hip: Vec3,
}

#[allow(dead_code)]
pub struct ArmRig {
    pub side: f32,
    pub upper: Entity,
    pub forearm: Entity,
    pub hand: Entity,
    pub shoulder: Vec3,
    pub pole: Vec3,
}

#[derive(Resource)]
#[allow(dead_code)]
pub struct PlayerRig {
    pub root: Entity,
    pub lower: Entity,
    pub pelvis: Entity,
    pub legs: [LegRig; 2],
    pub upper: Entity,
    pub head: Entity,
    pub gun: Entity,
    pub mag: Entity,
    pub mag_mesh: Handle<Mesh>,
    pub metal_mat: Handle<StandardMaterial>,
    pub spare_mag: Entity,
    pub muzzle: Entity,
    pub arms: [ArmRig; 2],
    pub muzzle_light: Entity,
    pub flash: Entity,
    pub flash_core: Entity,
    pub flash_spikes: Vec<Entity>,
    pub flash_glow: Entity,
    pub flash_core_mat: Handle<StandardMaterial>,
    pub flash_fire_mat: Handle<StandardMaterial>,
    pub flash_glow_mat: Handle<StandardMaterial>,
}

fn bevel_box_mesh(w: f32, h: f32, d: f32, r: f32) -> Mesh {
    let (pos, nrm) = bevel_box_raw(w, h, d, r);
    let mut uvs = Vec::with_capacity(pos.len());
    for (&p, &n) in pos.iter().zip(nrm.iter()) {
        let ax = n.x.abs();
        let ay = n.y.abs();
        let az = n.z.abs();
        let (u, v) = if ax >= ay && ax >= az {
            (p.z / 0.32, p.y / 0.32)
        } else if ay >= az {
            (p.x / 0.32, p.z / 0.32)
        } else {
            (p.x / 0.32, p.y / 0.32)
        };
        uvs.push([u, v]);
    }
    let indices: Vec<u32> = (0..pos.len() as u32).collect();
    let p_arr: Vec<[f32; 3]> = pos.into_iter().map(|v| v.to_array()).collect();
    let n_arr: Vec<[f32; 3]> = nrm.into_iter().map(|v| v.to_array()).collect();
    let mut mesh = Mesh::new(
        bevy::mesh::PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, p_arr);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, n_arr);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_indices(bevy::mesh::Indices::U32(indices));
    mesh
}

fn plaid_texture() -> Image {
    let mut data = vec![0_u8; 8 * 8 * 4];
    for j in 0..8 {
        for i in 0..8 {
            let warp = i < 4;
            let weft = j < 4;
            let col = if warp && weft {
                [0x19, 0x13, 0x14, 0xff]
            } else if warp || weft {
                [0x5c, 0x1b, 0x18, 0xff]
            } else {
                [0xa8, 0x33, 0x2b, 0xff]
            };
            let idx = (j * 8 + i) * 4;
            data[idx..idx + 4].copy_from_slice(&col);
        }
    }
    let mut image = Image::new(
        Extent3d {
            width: 8,
            height: 8,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    );
    image.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
        address_mode_u: ImageAddressMode::Repeat,
        address_mode_v: ImageAddressMode::Repeat,
        mag_filter: ImageFilterMode::Nearest,
        min_filter: ImageFilterMode::Nearest,
        ..default()
    });
    image
}

fn glow_texture() -> Image {
    let size = 64_usize;
    let mut data = vec![0_u8; size * size * 4];
    for y in 0..size {
        for x in 0..size {
            let dx = x as f32 - 32.0;
            let dy = y as f32 - 32.0;
            let t = ((dx * dx + dy * dy).sqrt() / 32.0).clamp(0.0, 1.0);
            let a = if t < 0.25 {
                1.0 + (0.45 - 1.0) * (t / 0.25)
            } else {
                0.45 * (1.0 - (t - 0.25) / 0.75)
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

fn blk(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    mat: &Handle<StandardMaterial>,
    w: f32,
    h: f32,
    d: f32,
    x: f32,
    y: f32,
    z: f32,
    r: f32,
    parent: Entity,
) -> Entity {
    let e = commands
        .spawn((
            Mesh3d(meshes.add(bevel_box_mesh(w, h, d, r))),
            MeshMaterial3d(mat.clone()),
            Transform::from_xyz(x, y, z),
        ))
        .id();
    commands.entity(parent).add_child(e);
    e
}

pub fn setup_player(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
) {
    let mat = |materials: &mut Assets<StandardMaterial>, hex: u32, rough: f32, metal: f32| {
        materials.add(StandardMaterial {
            base_color: Color::srgb_u8(
                ((hex >> 16) & 0xff) as u8,
                ((hex >> 8) & 0xff) as u8,
                (hex & 0xff) as u8,
            ),
            perceptual_roughness: rough,
            metallic: metal,
            ..default()
        })
    };

    let plaid_tex = images.add(plaid_texture());
    let flannel = materials.add(StandardMaterial {
        base_color_texture: Some(plaid_tex),
        perceptual_roughness: 0.9,
        ..default()
    });
    let lining = mat(&mut materials, 0xa88d68, 1.0, 0.0);
    let vest = mat(&mut materials, 0x515737, 0.9, 0.0);
    let vest_dk = mat(&mut materials, 0x3a3e27, 0.9, 0.0);
    let pants = mat(&mut materials, 0x39404d, 0.9, 0.0);
    let leather = mat(&mut materials, 0x5e3f27, 0.7, 0.0);
    let sole = mat(&mut materials, 0x17130f, 0.9, 0.0);
    let skin = mat(&mut materials, 0xe0915c, 0.9, 0.0);
    let hair = mat(&mut materials, 0x3a2517, 1.0, 0.0);
    let beard = mat(&mut materials, 0x5b3920, 1.0, 0.0);
    let beanie = mat(&mut materials, 0xe0662a, 1.0, 0.0);
    let cuff = mat(&mut materials, 0xb9501f, 1.0, 0.0);
    let eye = mat(&mut materials, 0x16100d, 0.4, 0.0);
    let canvas = mat(&mut materials, 0x6f5d3f, 0.9, 0.0);
    let bedroll = mat(&mut materials, 0x3e4a36, 0.9, 0.0);
    let brass = mat(&mut materials, 0xc9a14a, 0.35, 0.7);
    let metal = mat(&mut materials, 0x2b2e34, 0.42, 0.45);
    let wood = mat(&mut materials, 0x7b4a29, 0.65, 0.0);
    let rubber = mat(&mut materials, 0x151518, 0.9, 0.0);
    let lens = materials.add(StandardMaterial {
        base_color: Color::srgb_u8(0x10, 0x16, 0x1e),
        perceptual_roughness: 0.15,
        emissive: LinearRgba::rgb(0.16, 0.43, 0.72) * 0.2,
        ..default()
    });

    let root = commands
        .spawn((Transform::default(), Visibility::default()))
        .id();

    // Lower body
    let lower = commands
        .spawn((Transform::default(), Visibility::default()))
        .id();
    let pelvis = commands
        .spawn((Transform::from_xyz(0.0, 0.97, 0.0), Visibility::default()))
        .id();
    commands.entity(root).add_child(lower);
    commands.entity(lower).add_child(pelvis);

    blk(&mut commands, &mut meshes, &pants, 0.62, 0.3, 0.4, 0.0, 0.02, 0.0, 0.03, pelvis);
    blk(&mut commands, &mut meshes, &leather, 0.66, 0.08, 0.43, 0.0, 0.12, 0.0, 0.02, pelvis);
    blk(&mut commands, &mut meshes, &brass, 0.1, 0.07, 0.03, 0.0, 0.12, 0.22, 0.01, pelvis);
    blk(&mut commands, &mut meshes, &vest_dk, 0.12, 0.17, 0.17, 0.36, 0.02, 0.04, 0.03, pelvis);
    blk(&mut commands, &mut meshes, &leather, 0.07, 0.28, 0.1, -0.35, -0.04, -0.03, 0.03, pelvis);

    let mut build_leg = |side: f32| -> LegRig {
        let thigh = blk(&mut commands, &mut meshes, &pants, 0.26, THIGH + 0.08, 0.29, side * 0.16, 0.75, 0.0, 0.03, lower);
        let shin = blk(&mut commands, &mut meshes, &pants, 0.24, SHIN + 0.06, 0.27, side * 0.16, 0.35, 0.0, 0.03, lower);
        blk(&mut commands, &mut meshes, &vest_dk, 0.2, 0.17, 0.07, 0.0, SHIN * 0.5 - 0.03, 0.15, 0.03, shin);

        let boot = commands
            .spawn((Transform::from_xyz(side * 0.16, 0.15, 0.0), Visibility::default()))
            .id();
        commands.entity(lower).add_child(boot);
        blk(&mut commands, &mut meshes, &leather, 0.27, 0.17, 0.29, 0.0, 0.0, -0.01, 0.03, boot);
        blk(&mut commands, &mut meshes, &leather, 0.27, 0.1, 0.42, 0.0, -0.08, 0.06, 0.03, boot);
        blk(&mut commands, &mut meshes, &sole, 0.29, 0.05, 0.45, 0.0, -0.13, 0.06, 0.015, boot);

        LegRig {
            side,
            thigh,
            shin,
            boot,
            hip: Vec3::new(side * 0.16, 0.0, 0.0),
        }
    };
    let legs = [build_leg(1.0), build_leg(-1.0)];

    // Upper body
    let upper = commands
        .spawn((Transform::from_xyz(0.0, 0.97, 0.0), Visibility::default()))
        .id();
    commands.entity(root).add_child(upper);
    blk(&mut commands, &mut meshes, &flannel, 0.74, 0.6, 0.42, 0.0, 0.36, 0.0, 0.03, upper);
    blk(&mut commands, &mut meshes, &flannel, 0.84, 0.26, 0.46, 0.0, 0.62, 0.0, 0.03, upper);
    blk(&mut commands, &mut meshes, &lining, 0.5, 0.1, 0.4, 0.0, 0.78, -0.02, 0.03, upper);
    blk(&mut commands, &mut meshes, &vest, 0.6, 0.4, 0.07, 0.0, 0.38, 0.22, 0.03, upper);
    for x in [-0.17, 0.0, 0.17] {
        blk(&mut commands, &mut meshes, &vest_dk, 0.15, 0.17, 0.08, x, 0.33, 0.28, 0.03, upper);
    }
    blk(&mut commands, &mut meshes, &vest, 0.6, 0.48, 0.06, 0.0, 0.42, -0.23, 0.03, upper);
    for x in [-0.21, 0.21] {
        blk(&mut commands, &mut meshes, &vest, 0.12, 0.05, 0.5, x, 0.77, 0.0, 0.03, upper);
    }
    blk(&mut commands, &mut meshes, &canvas, 0.48, 0.42, 0.18, 0.0, 0.36, -0.33, 0.03, upper);
    blk(&mut commands, &mut meshes, &vest_dk, 0.3, 0.14, 0.06, 0.0, 0.27, -0.44, 0.03, upper);
    blk(&mut commands, &mut meshes, &bedroll, 0.68, 0.17, 0.17, 0.0, 0.66, -0.36, 0.03, upper);
    for x in [-0.2, 0.2] {
        blk(&mut commands, &mut meshes, &leather, 0.05, 0.19, 0.19, x, 0.66, -0.36, 0.01, upper);
    }

    // Head
    let head = commands
        .spawn((Transform::from_xyz(0.0, 0.8, 0.02), Visibility::default()))
        .id();
    commands.entity(upper).add_child(head);
    blk(&mut commands, &mut meshes, &skin, 0.2, 0.14, 0.2, 0.0, 0.04, 0.0, 0.03, head);
    blk(&mut commands, &mut meshes, &skin, 0.52, 0.5, 0.5, 0.0, 0.32, 0.0, 0.03, head);
    blk(&mut commands, &mut meshes, &hair, 0.54, 0.2, 0.2, 0.0, 0.36, -0.17, 0.03, head);
    for x in [-0.25, 0.25] {
        blk(&mut commands, &mut meshes, &beard, 0.05, 0.2, 0.3, x, 0.19, 0.08, 0.015, head);
    }
    blk(&mut commands, &mut meshes, &beard, 0.46, 0.12, 0.06, 0.0, 0.12, 0.24, 0.02, head);
    blk(&mut commands, &mut meshes, &beard, 0.2, 0.04, 0.03, 0.0, 0.21, 0.26, 0.01, head);
    blk(&mut commands, &mut meshes, &skin, 0.08, 0.1, 0.07, 0.0, 0.27, 0.27, 0.015, head);
    for x in [-0.12, 0.12] {
        blk(&mut commands, &mut meshes, &eye, 0.07, 0.07, 0.02, x, 0.34, 0.255, 0.005, head);
        blk(&mut commands, &mut meshes, &hair, 0.13, 0.04, 0.03, x, 0.41, 0.255, 0.008, head);
        blk(&mut commands, &mut meshes, &skin, 0.05, 0.11, 0.1, x * 2.25, 0.32, 0.0, 0.012, head);
    }
    blk(&mut commands, &mut meshes, &cuff, 0.58, 0.1, 0.56, 0.0, 0.49, -0.01, 0.03, head);
    blk(&mut commands, &mut meshes, &beanie, 0.55, 0.2, 0.53, 0.0, 0.6, -0.01, 0.03, head);
    blk(&mut commands, &mut meshes, &beanie, 0.18, 0.06, 0.18, 0.0, 0.72, -0.01, 0.02, head);

    // Rifle
    let gun = commands
        .spawn((Transform::from_xyz(-0.22, 1.38, 0.16), Visibility::default()))
        .id();
    commands.entity(root).add_child(gun);
    blk(&mut commands, &mut meshes, &rubber, 0.13, 0.19, 0.04, 0.0, -0.02, 0.02, 0.01, gun);
    blk(&mut commands, &mut meshes, &wood, 0.12, 0.17, 0.24, 0.0, -0.02, 0.15, 0.03, gun);
    let grip = blk(&mut commands, &mut meshes, &wood, 0.1, 0.2, 0.1, 0.0, -0.11, 0.29, 0.03, gun);
    commands.entity(grip).insert(Transform::from_xyz(0.0, -0.11, 0.29).with_rotation(Quat::from_rotation_x(0.35)));
    blk(&mut commands, &mut meshes, &metal, 0.14, 0.15, 0.4, 0.0, 0.03, 0.44, 0.03, gun);
    blk(&mut commands, &mut meshes, &metal, 0.04, 0.03, 0.12, 0.0, -0.07, 0.37, 0.01, gun);
    let mag_mesh = meshes.add(bevel_box_mesh(0.09, 0.24, 0.12, 0.03));
    let mag = commands
        .spawn((
            Mesh3d(mag_mesh.clone()),
            MeshMaterial3d(metal.clone()),
            Transform::from_xyz(0.0, -0.14, 0.52).with_rotation(Quat::from_rotation_x(-0.12)),
        ))
        .id();
    commands.entity(gun).add_child(mag);
    blk(&mut commands, &mut meshes, &wood, 0.13, 0.13, 0.36, 0.0, 0.01, 0.8, 0.03, gun);
    blk(&mut commands, &mut meshes, &metal, 0.06, 0.06, 0.46, 0.0, 0.03, 1.18, 0.015, gun);
    blk(&mut commands, &mut meshes, &metal, 0.085, 0.085, 0.08, 0.0, 0.03, 1.42, 0.015, gun);
    blk(&mut commands, &mut meshes, &metal, 0.03, 0.07, 0.03, 0.0, 0.09, 1.34, 0.008, gun);
    blk(&mut commands, &mut meshes, &rubber, 0.075, 0.075, 0.4, 0.0, 0.17, 0.46, 0.02, gun);
    blk(&mut commands, &mut meshes, &rubber, 0.1, 0.1, 0.08, 0.0, 0.17, 0.28, 0.02, gun);
    blk(&mut commands, &mut meshes, &rubber, 0.11, 0.11, 0.09, 0.0, 0.17, 0.66, 0.02, gun);
    blk(&mut commands, &mut meshes, &lens, 0.08, 0.08, 0.01, 0.0, 0.17, 0.706, 0.004, gun);
    for z in [0.38, 0.56] {
        blk(&mut commands, &mut meshes, &metal, 0.04, 0.07, 0.04, 0.0, 0.11, z, 0.01, gun);
    }

    let spare_mag = commands
        .spawn((
            Mesh3d(mag_mesh.clone()),
            MeshMaterial3d(metal.clone()),
            Visibility::Hidden,
            Transform::default(),
        ))
        .id();
    commands.entity(root).add_child(spare_mag);

    let muzzle = commands
        .spawn((Transform::from_xyz(0.0, 0.03, 1.47), Visibility::default()))
        .id();
    commands.entity(gun).add_child(muzzle);

    // Arms
    let mut build_arm = |side: f32| -> ArmRig {
        let up = blk(&mut commands, &mut meshes, &flannel, 0.2, UPPER_ARM + 0.1, 0.21, side * 0.41, 1.45, 0.08, 0.03, root);
        blk(&mut commands, &mut meshes, &flannel, 0.25, 0.24, 0.25, 0.0, UPPER_ARM * 0.5, 0.0, 0.03, up);
        let fore = blk(&mut commands, &mut meshes, &flannel, 0.18, FOREARM + 0.08, 0.19, side * 0.22, 1.25, 0.35, 0.03, root);
        blk(&mut commands, &mut meshes, &canvas, 0.21, 0.08, 0.22, 0.0, -FOREARM * 0.5 + 0.05, 0.0, 0.03, fore);
        let hand = blk(&mut commands, &mut meshes, &skin, 0.14, 0.15, 0.17, side * 0.15, 1.25, 0.55, 0.03, root);
        ArmRig {
            side,
            upper: up,
            forearm: fore,
            hand,
            shoulder: Vec3::new(side * 0.41, 0.7, 0.0),
            pole: if side > 0.0 {
                Vec3::new(0.5, -1.0, -0.1)
            } else {
                Vec3::new(-0.8, -1.0, -0.5)
            },
        }
    };
    let arms = [build_arm(1.0), build_arm(-1.0)];

    // Muzzle flash
    let flash_core_mat = materials.add(StandardMaterial {
        base_color: Color::srgba_u8(0xff, 0xf3, 0xd0, 0),
        emissive: LinearRgba::rgb(1.0, 0.95, 0.82) * 4.0,
        alpha_mode: AlphaMode::Add,
        unlit: true,
        ..default()
    });
    let flash_fire_mat = materials.add(StandardMaterial {
        base_color: Color::srgba_u8(0xff, 0xa6, 0x4d, 0),
        emissive: LinearRgba::rgb(1.0, 0.65, 0.30) * 3.5,
        alpha_mode: AlphaMode::Add,
        unlit: true,
        ..default()
    });
    let glow_tex = images.add(glow_texture());
    let flash_glow_mat = materials.add(StandardMaterial {
        base_color: Color::srgba_u8(0xff, 0xa6, 0x4d, 0),
        base_color_texture: Some(glow_tex),
        emissive: LinearRgba::rgb(1.0, 0.65, 0.30) * 2.5,
        alpha_mode: AlphaMode::Add,
        unlit: true,
        ..default()
    });

    let muzzle_light = commands
        .spawn((
            PointLight {
                color: Color::srgb_u8(0xff, 0xa6, 0x4d),
                intensity: 0.0,
                range: 14.0,
                shadow_maps_enabled: true,
                ..default()
            },
            Transform::default(),
        ))
        .id();

    let flash = commands
        .spawn((Transform::default(), Visibility::Hidden))
        .id();
    commands.entity(muzzle).add_child(flash);

    let flash_core = commands
        .spawn((
            Mesh3d(meshes.add(Sphere::new(0.5).mesh().ico(0).unwrap())),
            MeshMaterial3d(flash_core_mat.clone()),
            NotShadowCaster,
        ))
        .id();
    commands.entity(flash).add_child(flash_core);

    let flame_mesh = meshes.add(Cone::new(0.5, 1.0));
    let mut flash_spikes = Vec::with_capacity(6);
    for i in 0..6 {
        let pivot = commands
            .spawn((
                Transform::from_rotation(Quat::from_rotation_z(i as f32 * FRAC_PI_2 + FRAC_PI_4)),
                Visibility::default(),
            ))
            .id();
        let tilt = commands
            .spawn((
                Transform::from_rotation(Quat::from_rotation_x(if i > 1 { -1.05 } else { 0.0 })),
                Visibility::default(),
            ))
            .id();
        let spike_mat = if i == 0 {
            flash_core_mat.clone()
        } else {
            flash_fire_mat.clone()
        };
        let spike = commands
            .spawn((
                Mesh3d(flame_mesh.clone()),
                MeshMaterial3d(spike_mat),
                Transform::from_xyz(0.0, 0.0, 0.5).with_rotation(Quat::from_rotation_x(FRAC_PI_2)),
                NotShadowCaster,
            ))
            .id();
        commands.entity(flash).add_child(pivot);
        commands.entity(pivot).add_child(tilt);
        commands.entity(tilt).add_child(spike);
        flash_spikes.push(spike);
    }

    let flash_glow = commands
        .spawn((
            Mesh3d(meshes.add(Rectangle::new(1.0, 1.0))),
            MeshMaterial3d(flash_glow_mat.clone()),
            Transform::from_xyz(0.0, 0.0, 0.15),
            NotShadowCaster,
        ))
        .id();
    commands.entity(flash).add_child(flash_glow);

    commands.insert_resource(PlayerRig {
        root,
        lower,
        pelvis,
        legs,
        upper,
        head,
        gun,
        mag,
        mag_mesh,
        metal_mat: metal,
        spare_mag,
        muzzle,
        arms,
        muzzle_light,
        flash,
        flash_core,
        flash_spikes,
        flash_glow,
        flash_core_mat,
        flash_fire_mat,
        flash_glow_mat,
    });
}

enum ReloadSpace {
    Gun,
    Upper,
}

const RELOAD_PATH: [(f32, Vec3, ReloadSpace); 7] = [
    (0.0, GRIP_L, ReloadSpace::Gun),
    (0.12, MAG_BASE, ReloadSpace::Gun),
    (0.22, MAG_OUT, ReloadSpace::Gun),
    (0.45, POUCH, ReloadSpace::Upper),
    (0.55, POUCH, ReloadSpace::Upper),
    (0.78, MAG_LINEUP, ReloadSpace::Gun),
    (0.95, MAG_BASE, ReloadSpace::Gun),
];

fn reload_hand(u: f32, gun_m: Mat4, upper_m: Mat4) -> Vec3 {
    let at = |k: usize| -> Vec3 {
        let (_, pt, ref space) = RELOAD_PATH[k];
        match space {
            ReloadSpace::Gun => gun_m.transform_point3(pt),
            ReloadSpace::Upper => upper_m.transform_point3(pt),
        }
    };
    let mut i = RELOAD_PATH.len() - 1;
    while i > 0 && u < RELOAD_PATH[i].0 {
        i -= 1;
    }
    if i == RELOAD_PATH.len() - 1 {
        return at(i);
    }
    let t = (u - RELOAD_PATH[i].0) / (RELOAD_PATH[i + 1].0 - RELOAD_PATH[i].0);
    at(i).lerp(at(i + 1), ease(t))
}

pub fn trigger_muzzle_flash(
    commands: &mut Commands,
    rig: &PlayerRig,
    st: &mut PlayerRigState,
    debris_assets: &DebrisAssets,
    materials: &mut Assets<StandardMaterial>,
    global_transforms: &Query<&GlobalTransform>,
    hot: bool,
) {
    let mut rng = rand::rng();
    st.kick = 1.0;
    st.flash = 1.0;
    st.flash_peak = 40.0 + rng.random::<f32>() * 20.0;

    let hex = if hot { FLASH_HOT } else { FLASH_COLOR };
    let r = ((hex >> 16) & 0xff) as f32 / 255.0;
    let g = ((hex >> 8) & 0xff) as f32 / 255.0;
    let b = (hex & 0xff) as f32 / 255.0;
    let col = Color::srgb(r, g, b);

    if let Some(mut mat) = materials.get_mut(&rig.flash_fire_mat) {
        mat.base_color = col;
        mat.emissive = LinearRgba::rgb(r, g, b) * 3.5;
    }
    if let Some(mut mat) = materials.get_mut(&rig.flash_glow_mat) {
        mat.base_color = col;
        mat.emissive = LinearRgba::rgb(r, g, b) * 2.5;
    }

    let gun_gt = global_transforms
        .get(rig.gun)
        .copied()
        .unwrap_or(GlobalTransform::IDENTITY);
    let gun_rot = gun_gt.to_scale_rotation_translation().1;
    let eject_pos = gun_gt.transform_point(EJECT);
    let right = gun_rot * Vec3::NEG_X;
    let back = gun_rot * Vec3::NEG_Z;
    let vel = right * (2.4 + rng.random::<f32>() * 1.2)
        + back * 0.6
        + Vec3::new(0.0, 3.0 + rng.random::<f32>(), 0.0);

    commands.spawn((
        Mesh3d(debris_assets.casing_mesh.clone()),
        MeshMaterial3d(debris_assets.brass_mat.clone()),
        Transform::from_translation(eject_pos).with_rotation(gun_rot),
        Debris {
            vel,
            spin: Vec3::new(
                rng.random::<f32>() * 20.0 - 10.0,
                rng.random::<f32>() * 20.0 - 10.0,
                rng.random::<f32>() * 20.0 - 10.0,
            ),
            life: 2.5,
            max_life: 2.5,
            bounce: 0.025,
            smoke_size: None,
            base_scale: Vec3::ONE,
        },
    ));

    let muzzle_pos = global_transforms
        .get(rig.muzzle)
        .map(|gt| gt.translation())
        .unwrap_or(eject_pos);
    let smoke_rot = Quat::from_euler(
        EulerRot::YXZ,
        rng.random::<f32>() * 3.0,
        rng.random::<f32>() * 3.0,
        0.0,
    );
    let fwd = gun_rot * Vec3::Z;
    spawn_smoke(
        commands,
        debris_assets,
        materials,
        muzzle_pos,
        smoke_rot,
        fwd * 1.6,
        0.14 + rng.random::<f32>() * 0.06,
    );
}

pub fn player_movement_and_aim(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    windows: Query<&Window>,
    camera_q: Query<(&Camera, &GlobalTransform)>,
    rig: Res<PlayerRig>,
    stats: Res<GameStats>,
    mut rig_state: ResMut<PlayerRigState>,
    mut aim_state: ResMut<AimState>,
    mut transforms: Query<&mut Transform>,
) {
    let dt = time.delta_secs();
    let mut dir = Vec2::ZERO;
    if keys.pressed(KeyCode::KeyW) || keys.pressed(KeyCode::ArrowUp) {
        dir.y -= 1.0;
    }
    if keys.pressed(KeyCode::KeyS) || keys.pressed(KeyCode::ArrowDown) {
        dir.y += 1.0;
    }
    if keys.pressed(KeyCode::KeyA) || keys.pressed(KeyCode::ArrowLeft) {
        dir.x -= 1.0;
    }
    if keys.pressed(KeyCode::KeyD) || keys.pressed(KeyCode::ArrowRight) {
        dir.x += 1.0;
    }
    let move_dir = dir.normalize_or_zero();
    rig_state.vel = move_dir * stats.speed;

    let clamp_lim = HALF - 0.7;
    if let Ok(mut p_tf) = transforms.get_mut(rig.root) {
        p_tf.translation.x = (p_tf.translation.x + rig_state.vel.x * dt).clamp(-clamp_lim, clamp_lim);
        p_tf.translation.z = (p_tf.translation.z + rig_state.vel.y * dt).clamp(-clamp_lim, clamp_lim);

        if let (Ok(window), Ok((camera, cam_gt))) = (windows.single(), camera_q.single()) {
            if let Some(cursor_pos) = window.cursor_position() {
                if let Ok(ray) = camera.viewport_to_world(cam_gt, cursor_pos) {
                    if let Some(dist) =
                        ray.intersect_plane(Vec3::new(0.0, 0.9, 0.0), InfinitePlane3d::new(Vec3::Y))
                    {
                        let hit = ray.get_point(dist);
                        aim_state.aim_point = hit;
                        let yaw = (hit.x - p_tf.translation.x).atan2(hit.z - p_tf.translation.z);
                        p_tf.rotation = Quat::from_rotation_y(yaw);
                    }
                }
            }
        }
    }
}

/// Poses the survivor's hierarchical rig every frame using 2-bone IK and gait cycles.
pub fn animate_player(
    mut commands: Commands,
    time: Res<Time>,
    rig: Res<PlayerRig>,
    mut st: ResMut<PlayerRigState>,
    mut transforms: Query<&mut Transform>,
    global_transforms: Query<&GlobalTransform>,
    mut visibilities: Query<&mut Visibility>,
    mut lights: Query<&mut PointLight>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let dt = time.delta_secs();
    let mx = st.vel.x;
    let mz = st.vel.y;
    let pace = (mx.hypot(mz) / st.speed).min(1.0);
    let moving = pace > 0.05;

    let aim = transforms
        .get(rig.root)
        .map(|t| t.rotation.to_euler(EulerRot::YXZ).1)
        .unwrap_or(0.0);

    st.t += dt;
    st.gait = damp(st.gait, pace, if moving { 9.0 } else { 7.0 }, dt);

    let move_yaw = mx.atan2(mz);
    let mut hip_target = st.hip_yaw;
    if moving {
        let rel = wrap_a(move_yaw - aim);
        if if st.back { rel.abs() < 1.25 } else { rel.abs() > 1.9 } {
            st.back = !st.back;
        }
        let offset = if st.back { wrap_a(rel - PI) } else { rel }.clamp(-HIP_MAX, HIP_MAX);
        hip_target = aim + offset;
        st.turning = false;
    } else {
        let idle = aim + IDLE_HIP;
        if wrap_a(idle - st.hip_yaw).abs() > 0.8 {
            st.turning = true;
        }
        if st.turning {
            hip_target = idle;
            if wrap_a(idle - st.hip_yaw).abs() < 0.06 {
                st.turning = false;
            }
        }
    }

    st.hip_yaw = wrap_a(st.hip_yaw + wrap_a(hip_target - st.hip_yaw) * (1.0 - (-10.0 * dt).exp()));
    let hip_rel = wrap_a(st.hip_yaw - aim);
    if let Ok(mut tf) = transforms.get_mut(rig.lower) {
        tf.rotation = Quat::from_rotation_y(hip_rel);
    }
    if moving {
        let s = move_yaw - st.hip_yaw;
        st.stride = Vec3::new(s.sin(), 0.0, s.cos());
    }

    st.turn_w = damp(st.turn_w, if st.turning { 1.0 } else { 0.0 }, 12.0, dt);
    let drive = st.gait.max(st.turn_w * 0.7);
    let freq = 2.45 * (st.speed / PLAYER_SPEED).sqrt();
    if drive > 0.02 {
        st.phase = (st.phase + freq * dt) % 1.0;
    }
    let ph = st.phase * TAU;
    let stride_len = (st.speed * BETA / freq).min(0.86) * st.gait;
    let lift_h = 0.2 * st.gait + 0.1 * st.turn_w;

    let bob = -0.04 * st.gait * (2.0 * (ph - PI * BETA)).cos();
    let pelvis_y = HIP_H - 0.11 * st.gait + bob;
    let pelvis_rot = Quat::from_euler(
        EulerRot::YXZ,
        0.14 * st.gait * ph.sin() * st.stride.z,
        0.0,
        0.05 * st.gait * ph.sin(),
    );
    let pelvis_m = Mat4::from_rotation_translation(pelvis_rot, Vec3::new(0.0, pelvis_y, 0.0));
    if let Ok(mut tf) = transforms.get_mut(rig.pelvis) {
        tf.translation.y = pelvis_y;
        tf.rotation = pelvis_rot;
    }

    // Legs
    for (i, leg) in rig.legs.iter().enumerate() {
        let p = (st.phase + i as f32 * 0.5) % 1.0;
        let fc = foot_cycle(p, BETA);
        let rest_z = if leg.side > 0.0 { 0.12 } else { -0.1 };
        let foot = Vec3::new(
            leg.side * (0.17 + 0.03 * (1.0 - st.gait)),
            ANKLE_H + fc.lift * lift_h,
            rest_z * (1.0 - st.gait),
        ) + st.stride * (fc.s * stride_len);

        let hip_p = pelvis_m.transform_point3(leg.hip);
        let knee = ik2(hip_p, foot, THIGH, SHIN, KNEE_POLE);
        let ankle = knee + (foot - knee).normalize_or_zero() * SHIN;

        if let Ok(mut tf) = transforms.get_mut(leg.thigh) {
            *tf = bone_transform(hip_p, knee, KNEE_POLE);
        }
        if let Ok(mut tf) = transforms.get_mut(leg.shin) {
            *tf = bone_transform(knee, ankle, KNEE_POLE);
        }
        if let Ok(mut tf) = transforms.get_mut(leg.boot) {
            tf.translation = ankle;
            let sgn = if st.stride.z == 0.0 { 1.0 } else { st.stride.z.signum() };
            tf.rotation = Quat::from_euler(EulerRot::YXZ, leg.side * 0.08, fc.pitch * st.gait * sgn, 0.0);
        }
    }

    // Upper body
    let breathe = (st.t * 2.1).sin() * (1.0 - st.gait);
    let tw = BLADE + 0.3 * (hip_rel - BLADE) - 0.07 * st.gait * ph.sin() * st.stride.z;
    let travel = move_yaw - (aim + tw);
    st.lean_f = damp(st.lean_f, if moving { travel.cos() } else { 0.0 }, 8.0, dt);
    st.lean_s = damp(st.lean_s, if moving { travel.sin() } else { 0.0 }, 8.0, dt);
    st.kick *= (-14.0 * dt).exp();

    let upper_pos = Vec3::new(0.0, pelvis_y + 0.1 + 0.006 * breathe, 0.0);
    let upper_rot = Quat::from_euler(
        EulerRot::YXZ,
        tw,
        0.05 + 0.1 * st.lean_f * st.gait - 0.08 * st.kick,
        -0.07 * st.lean_s * st.gait,
    );
    let upper_m = Mat4::from_rotation_translation(upper_rot, upper_pos);
    if let Ok(mut tf) = transforms.get_mut(rig.upper) {
        tf.translation = upper_pos;
        tf.rotation = upper_rot;
    }

    // Reload
    st.reload = damp(st.reload, if st.reloading { 1.0 } else { 0.0 }, if st.reloading { 14.0 } else { 9.0 }, dt);
    let rl = ease(st.reload);
    let u = if st.reloading { st.reload_t / st.reload_time } else { 1.0 };

    if let Ok(mut tf) = transforms.get_mut(rig.head) {
        tf.rotation = Quat::from_euler(EulerRot::YXZ, -tw * 0.85 + 0.25 * rl, 0.1 + 0.32 * rl, 0.0);
    }

    // Rifle
    let pocket_w = upper_m.transform_point3(POCKET);
    let gun_pos = Vec3::new(
        pocket_w.x + 0.1 * rl,
        pocket_w.y - 0.1 * rl + 0.012 * breathe,
        pocket_w.z - 0.11 * st.kick - 0.08 * rl,
    );
    let gun_rot = Quat::from_euler(
        EulerRot::YXZ,
        0.15 * rl,
        -0.16 * st.kick - 0.35 * rl + 0.01 * (st.t * 1.3).sin(),
        0.65 * rl,
    );
    let gun_m = Mat4::from_rotation_translation(gun_rot, gun_pos);
    if let Ok(mut tf) = transforms.get_mut(rig.gun) {
        tf.translation = gun_pos;
        tf.rotation = gun_rot;
    }

    // Magazine
    if !st.reloading || u < 0.12 {
        st.mag_dropped = false;
    }
    let mut mag_drop = 0.0;
    if st.reloading && u >= 0.12 && u < 0.22 {
        mag_drop = 0.24 * ease((u - 0.12) / 0.1);
    }
    if st.reloading && u >= 0.78 {
        mag_drop = 0.2 * (1.0 - ease(((u - 0.78) / 0.17).min(1.0)));
    }
    if let Ok(mut tf) = transforms.get_mut(rig.mag) {
        tf.translation.y = MAG_Y - mag_drop;
    }
    let mag_vis = !st.reloading || u < 0.22 || u >= 0.78;
    if let Ok(mut vis) = visibilities.get_mut(rig.mag) {
        *vis = if mag_vis { Visibility::Inherited } else { Visibility::Hidden };
    }
    if st.reloading && u >= 0.22 && !st.mag_dropped {
        st.mag_dropped = true;
        if let Ok(mag_gt) = global_transforms.get(rig.mag) {
            let (_, rot, pos) = mag_gt.to_scale_rotation_translation();
            commands.spawn((
                Mesh3d(rig.mag_mesh.clone()),
                MeshMaterial3d(rig.metal_mat.clone()),
                Transform::from_translation(pos).with_rotation(rot),
                Debris {
                    vel: Vec3::new(0.0, -1.0, 0.0),
                    spin: Vec3::new(3.0, 0.0, 2.0),
                    life: 4.0,
                    max_life: 4.0,
                    bounce: 0.05,
                    smoke_size: None,
                    base_scale: Vec3::ONE,
                },
            ));
        }
    }

    // Arms & Hands
    let g_r = gun_m.transform_point3(GRIP_R);
    let mut g_l = gun_m.transform_point3(GRIP_L);
    if st.reload > 0.001 {
        g_l = g_l.lerp(reload_hand(u, gun_m, upper_m), rl);
    }

    let mut left_wrist = Vec3::ZERO;
    for (i, arm) in rig.arms.iter().enumerate() {
        let sh = upper_m.transform_point3(arm.shoulder);
        let target = if arm.side < 0.0 { g_r } else { g_l };
        let elbow = ik2(sh, target, UPPER_ARM, FOREARM, arm.pole);
        let wrist = elbow + (target - elbow).normalize_or_zero() * FOREARM;
        if i == 0 {
            left_wrist = wrist;
        }
        if let Ok(mut tf) = transforms.get_mut(arm.upper) {
            *tf = bone_transform(sh, elbow, arm.pole);
        }
        if let Ok(mut tf) = transforms.get_mut(arm.forearm) {
            *tf = bone_transform(elbow, wrist, arm.pole);
        }
        if let Ok(mut tf) = transforms.get_mut(arm.hand) {
            tf.translation = wrist;
            tf.rotation = gun_rot;
        }
    }

    let spare_vis = st.reloading && u >= 0.45 && u < 0.78;
    if let Ok(mut vis) = visibilities.get_mut(rig.spare_mag) {
        *vis = if spare_vis { Visibility::Inherited } else { Visibility::Hidden };
    }
    if spare_vis {
        if let Ok(mut tf) = transforms.get_mut(rig.spare_mag) {
            tf.rotation = gun_rot;
            tf.translation = left_wrist + gun_rot * Vec3::new(0.0, 0.13, 0.0);
        }
    }

    // Muzzle flash fadeout
    st.flash = (st.flash - dt / 0.07).max(0.0);
    let f = st.flash;
    if let Ok(mut vis) = visibilities.get_mut(rig.flash) {
        *vis = if f > 0.3 { Visibility::Inherited } else { Visibility::Hidden };
    }
    if let Some(mut mat) = materials.get_mut(&rig.flash_core_mat) {
        mat.base_color.set_alpha((f * 1.4).min(1.0));
    }
    if let Some(mut mat) = materials.get_mut(&rig.flash_fire_mat) {
        mat.base_color.set_alpha((f * 1.2).min(1.0));
    }
    if let Some(mut mat) = materials.get_mut(&rig.flash_glow_mat) {
        mat.base_color.set_alpha(0.85 * f);
    }
    if let Ok(mut light) = lights.get_mut(rig.muzzle_light) {
        light.intensity = st.flash_peak * 1000.0 * f * f;
    }
    if f > 0.0 {
        if let Ok(muzzle_gt) = global_transforms.get(rig.muzzle) {
            let mp = muzzle_gt.translation();
            if let Ok(mut tf) = transforms.get_mut(rig.muzzle_light) {
                tf.translation = Vec3::new(mp.x + aim.sin() * 0.4, mp.y, mp.z + aim.cos() * 0.4);
            }
        }
    }
}
