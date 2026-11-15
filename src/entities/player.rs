use bevy::asset::RenderAssetUsages;
use bevy::image::{ImageAddressMode, ImageFilterMode, ImageSampler, ImageSamplerDescriptor};
use bevy::light::NotShadowCaster;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use std::f32::consts::{FRAC_PI_2, FRAC_PI_4};

use crate::core::blocks::bevel_box_raw;

pub const THIGH: f32 = 0.44;
pub const SHIN: f32 = 0.42;
#[allow(dead_code)]
pub const ANKLE_H: f32 = 0.15;
#[allow(dead_code)]
pub const HIP_H: f32 = 0.97;
pub const UPPER_ARM: f32 = 0.42;
pub const FOREARM: f32 = 0.42;

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
            Mesh3d(mag_mesh),
            MeshMaterial3d(metal),
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
