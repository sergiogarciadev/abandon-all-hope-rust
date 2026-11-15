use bevy::asset::RenderAssetUsages;
use bevy::image::{ImageAddressMode, ImageFilterMode, ImageSampler, ImageSamplerDescriptor};
use bevy::light::{CascadeShadowConfigBuilder, NotShadowCaster};
use bevy::math::Affine2;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use rand::RngExt;
use std::f32::consts::{FRAC_PI_4, TAU};

use crate::core::blocks::{BlockBatch, BlockSpec, bake_blocks};
use crate::core::math::SeededRng;

use super::openings::{LOG_D, OpeningKind, Openings, POST, WEATHERED, WallAxis, fit_opening, make_opening};
use super::solids::SolidsNav;

pub const HALF: f32 = 10.0;
#[allow(dead_code)]
pub const WALL_H: f32 = 2.4;
pub const WALL_T: f32 = 0.7;
pub const EDGE: f32 = HALF + WALL_T * 0.5;
pub const GROUND_Y: f32 = -0.06;

const LOG_STEP: f32 = 0.4;
const LOG_H: f32 = 0.42;
const OVERHANG: f32 = 0.42;
const LOG_TONES: [u32; 4] = [0x5a4430, 0x4d3926, 0x664e36, 0x45331f];
const END_GRAIN: u32 = 0x8d7150;
const CHINK: u32 = 0x9a9486;
const STONE_TONES: [u32; 3] = [0x3d3e40, 0x46464a, 0x353638];
const DOOR_TOP: f32 = 2.03;

const FOREST_COUNT: usize = 900;
const R_MIN: f32 = 13.0;
const R_MAX: f32 = 70.0;
const SHADOW_R: f32 = 24.0;
const TRUNK: u32 = 0x2b2018;
const NEEDLES: u32 = 0x1a2b1a;
const NEEDLE_TIPS: u32 = 0x2a4229;

#[derive(Clone, Copy)]
pub struct WallBox {
    pub min_x: f32,
    pub max_x: f32,
    pub min_z: f32,
    pub max_z: f32,
}

#[derive(Resource, Default)]
pub struct WallBoxes {
    pub boxes: Vec<WallBox>,
}

#[derive(Component)]
pub struct LanternLight;

#[derive(Component)]
pub struct LanternGlass;

#[derive(Resource, Default)]
pub struct WorldWeatherLevel(pub f32);

#[derive(Resource)]
pub struct WorldMaterials {
    pub world_mat: Handle<StandardMaterial>,
    pub ground_mat: Handle<StandardMaterial>,
}

#[inline]
fn at(x: f32, z: f32, ry: f32) -> Mat4 {
    Mat4::from_rotation_translation(Quat::from_rotation_y(ry), Vec3::new(x, 0.0, z))
}

fn hex_rgba(hex: u32) -> [u8; 4] {
    [
        ((hex >> 16) & 0xff) as u8,
        ((hex >> 8) & 0xff) as u8,
        (hex & 0xff) as u8,
        255,
    ]
}

fn ground_texture() -> Image {
    let w = 128_usize;
    let h = 128_usize;
    let mut data = vec![0_u8; w * h * 4];
    let bg = hex_rgba(0x1b1e15);
    for px in data.chunks_exact_mut(4) {
        px.copy_from_slice(&bg);
    }

    let tones = [
        hex_rgba(0x232b19),
        hex_rgba(0x161812),
        hex_rgba(0x2a2418),
        hex_rgba(0x1e2716),
        hex_rgba(0x121410),
        hex_rgba(0x2c3320),
    ];
    let mut rng = SeededRng::new(5);
    for _ in 0..2600 {
        let col = tones[(rng.next_f32() * tones.len() as f32) as usize % tones.len()];
        let s = if rng.next_f32() < 0.85 { 2 } else { 4 };
        let x0 = ((rng.next_f32() * 64.0) as usize) * 2;
        let y0 = ((rng.next_f32() * 64.0) as usize) * 2;
        for dy in 0..s {
            for dx in 0..s {
                let x = (x0 + dx) % w;
                let y = (y0 + dy) % h;
                let idx = (y * w + x) * 4;
                data[idx..idx + 4].copy_from_slice(&col);
            }
        }
    }

    let mut image = Image::new(
        Extent3d {
            width: w as u32,
            height: h as u32,
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

fn moon_texture() -> Image {
    let size = 256_usize;
    let mut data = vec![0_u8; size * size * 4];
    let cx = 128.0_f32;
    let cy = 128.0_f32;

    for y in 0..size {
        for x in 0..size {
            let dx = x as f32 - cx;
            let dy = y as f32 - cy;
            let dist = (dx * dx + dy * dy).sqrt();
            let t = (dist / 128.0).clamp(0.0, 1.0);

            // Radial halo
            let (hr, hg, hb, ha) = if t < 0.2 {
                let k = t / 0.2;
                (
                    200.0 + (160.0 - 200.0) * k,
                    214.0 + (180.0 - 214.0) * k,
                    240.0 + (220.0 - 240.0) * k,
                    0.55 + (0.16 - 0.55) * k,
                )
            } else {
                let k = (t - 0.2) / 0.8;
                (160.0, 180.0, 220.0, 0.16 * (1.0 - k))
            };

            let mut r = hr;
            let mut g = hg;
            let mut b = hb;
            let mut a = ha;

            if dist <= 22.0 {
                r = 0xe4 as f32;
                g = 0xe9 as f32;
                b = 0xf2 as f32;
                a = 1.0;

                for &(mx, my, mr) in &[(121.0, 122.0, 7.0), (134.0, 131.0, 5.0), (126.0, 137.0, 4.0)] {
                    let md = ((x as f32 - mx).powi(2) + (y as f32 - my).powi(2)).sqrt();
                    if md <= mr {
                        r = r * 0.65 + 150.0 * 0.35;
                        g = g * 0.65 + 160.0 * 0.35;
                        b = b * 0.65 + 180.0 * 0.35;
                    }
                }
            }

            let idx = (y * size + x) * 4;
            data[idx] = r.round().clamp(0.0, 255.0) as u8;
            data[idx + 1] = g.round().clamp(0.0, 255.0) as u8;
            data[idx + 2] = b.round().clamp(0.0, 255.0) as u8;
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

fn wall_block(
    axis: WallAxis,
    fixed: f32,
    u0: f32,
    u1: f32,
    y: f32,
    h: f32,
    d: f32,
    color: u32,
    bevel: f32,
    n: f32,
    tilt: f32,
) -> BlockSpec {
    let len = (u1 - u0).abs();
    let c = (u0 + u1) * 0.5;
    match axis {
        WallAxis::X => BlockSpec::new(len, h, d, c, y, fixed + n, color)
            .with_rot(0.0, 0.0, tilt)
            .with_bevel(bevel),
        WallAxis::Z => BlockSpec::new(d, h, len, fixed + n, y, c, color)
            .with_rot(-tilt, 0.0, 0.0)
            .with_bevel(bevel),
    }
}

fn log_run(
    wall_b: &mut BlockBatch,
    wrng: &mut SeededRng,
    axis: WallAxis,
    fixed: f32,
    a: f32,
    b: f32,
    layer0: f32,
    keep: impl Fn(f32) -> bool,
    cut_a: bool,
    cut_b: bool,
    footing: bool,
) {
    for k in 0..6 {
        let y = layer0 + k as f32 * LOG_STEP;
        if !keep(y) {
            continue;
        }
        let u0 = if cut_a { a } else { a - OVERHANG };
        let u1 = if cut_b { b } else { b + OVERHANG };
        let tone = LOG_TONES[(wrng.next_f32() * 4.0) as usize % 4];
        wall_b.add(
            &[wall_block(axis, fixed, u0, u1, y, LOG_H, LOG_D, tone, 0.19, 0.0, 0.0)],
            wrng,
            None,
            false,
        );
        if k < 5 && keep(y + LOG_STEP) {
            wall_b.add(
                &[wall_block(axis, fixed, a, b, y + LOG_STEP * 0.5, 0.08, LOG_D * 0.72, CHINK, 0.02, 0.0, 0.0)],
                wrng,
                None,
                false,
            );
        }
        for (cut, u) in [(cut_a, u0), (cut_b, u1)] {
            if !cut {
                wall_b.add(
                    &[wall_block(
                        axis,
                        fixed,
                        u - 0.012,
                        u + 0.012,
                        y,
                        LOG_H * 0.8,
                        LOG_D * 0.8,
                        END_GRAIN,
                        0.1,
                        0.0,
                        0.0,
                    )],
                    wrng,
                    None,
                    false,
                );
            }
        }
    }
    if !footing {
        return;
    }
    if axis == WallAxis::Z {
        wall_b.add(
            &[wall_block(axis, fixed, a, b, 0.13, 0.24, LOG_D * 0.9, LOG_TONES[1], 0.06, 0.0, 0.0)],
            wrng,
            None,
            false,
        );
    }
    let mut u = a - if cut_a { 0.0 } else { 0.3 };
    let end = b + if cut_b { 0.0 } else { 0.3 };
    while u < end {
        let len = 0.45 + wrng.next_f32() * 0.6;
        let tone = STONE_TONES[(wrng.next_f32() * 3.0) as usize % 3];
        wall_b.add(
            &[wall_block(
                axis,
                fixed,
                u,
                (u + len).min(end) - 0.04,
                0.02,
                0.18,
                LOG_D + 0.16,
                tone,
                0.05,
                0.0,
                0.0,
            )],
            wrng,
            None,
            false,
        );
        u += len;
    }
}

fn pine_tree(tiers: usize, w0: f32, tier_h: f32, trunk_h: f32) -> Vec<BlockSpec> {
    let mut b = vec![BlockSpec::new(0.32, trunk_h, 0.32, 0.0, trunk_h * 0.5, 0.0, TRUNK).with_bevel(0.06)];
    let mut y = trunk_h * 0.6;
    for i in 0..tiers {
        let w = w0 * (1.0 - (i as f32 / tiers as f32) * 0.75);
        let ry = (i % 2) as f32 * FRAC_PI_4;
        b.push(
            BlockSpec::new(w, tier_h * 0.62, w, 0.0, y, 0.0, NEEDLES)
                .with_rot(0.0, ry, 0.0)
                .with_bevel(0.14),
        );
        b.push(
            BlockSpec::new(w * 0.6, tier_h * 0.42, w * 0.6, 0.0, y + tier_h * 0.36, 0.0, NEEDLE_TIPS)
                .with_rot(0.0, ry, 0.0)
                .with_bevel(0.09),
        );
        y += tier_h * 0.78;
    }
    b.push(BlockSpec::new(0.3, tier_h * 0.7, 0.3, 0.0, y, 0.0, NEEDLE_TIPS).with_bevel(0.08));
    b
}

fn oak_tree(rng: &mut SeededRng) -> Vec<BlockSpec> {
    let mut b = vec![BlockSpec::new(0.46, 2.4, 0.46, 0.0, 1.2, 0.0, TRUNK).with_bevel(0.08)];
    for i in 0..6 {
        let s = 1.3 + rng.next_f32() * 0.9;
        let a = i as f32 * 1.05;
        let m = if i == 0 { 0.0 } else { 1.0 };
        let col = if i % 2 == 1 { 0x24311c } else { 0x2e3d22 };
        b.push(
            BlockSpec::new(
                s,
                s * 0.85,
                s,
                a.cos() * 0.75 * m,
                2.6 + rng.next_f32() * 1.1 + if i == 0 { 0.8 } else { 0.0 },
                a.sin() * 0.75 * m,
                col,
            )
            .with_rot(0.0, rng.next_f32(), 0.0)
            .with_bevel(0.2),
        );
    }
    b
}

fn dead_tree(rng: &mut SeededRng) -> Vec<BlockSpec> {
    let mut b = vec![BlockSpec::new(0.34, 4.4, 0.34, 0.0, 2.2, 0.0, 0x3d362e).with_bevel(0.06)];
    for i in 0..6 {
        let len = 0.9 + rng.next_f32() * 0.8;
        let y0 = 1.8 + i as f32 * 0.42;
        let phi = 0.7 + rng.next_f32() * 0.4;
        let side = if i % 2 == 1 { 1.0 } else { -1.0 };
        if i % 3 == 2 {
            b.push(
                BlockSpec::new(
                    0.11,
                    len,
                    0.11,
                    0.0,
                    y0 + phi.cos() * len * 0.5,
                    side * phi.sin() * len * 0.5,
                    0x3d362e,
                )
                .with_rot(side * phi, 0.0, 0.0)
                .with_bevel(0.03),
            );
        } else {
            b.push(
                BlockSpec::new(
                    0.11,
                    len,
                    0.11,
                    -side * phi.sin() * len * 0.5,
                    y0 + phi.cos() * len * 0.5,
                    0.0,
                    0x3d362e,
                )
                .with_rot(0.0, 0.0, side * phi)
                .with_bevel(0.03),
            );
        }
    }
    b
}

pub fn setup_world(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
    mut solids: ResMut<SolidsNav>,
    mut wall_boxes: ResMut<WallBoxes>,
    mut openings: ResMut<Openings>,
) {
    let world_mat = materials.add(StandardMaterial {
        perceptual_roughness: 0.92,
        ..default()
    });
    let glass_mat = materials.add(StandardMaterial {
        base_color: Color::srgba_u8(0x9f, 0xb6, 0xc9, 77),
        perceptual_roughness: 0.08,
        metallic: 0.4,
        alpha_mode: AlphaMode::Blend,
        ..default()
    });
    let glass_cracked = materials.add(StandardMaterial {
        base_color: Color::srgba_u8(0xd8, 0xe4, 0xee, 140),
        perceptual_roughness: 0.45,
        metallic: 0.2,
        alpha_mode: AlphaMode::Blend,
        ..default()
    });
    openings.glass_mat = glass_mat.clone();
    openings.glass_cracked = glass_cracked;

    // 1. Forest floor
    let ground_tex = images.add(ground_texture());
    let ground_mat = materials.add(StandardMaterial {
        base_color_texture: Some(ground_tex),
        uv_transform: Affine2::from_scale(Vec2::splat(25.0)),
        perceptual_roughness: 1.0,
        ..default()
    });
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(200.0, 200.0))),
        MeshMaterial3d(ground_mat.clone()),
        Transform::from_xyz(0.0, GROUND_Y, 0.0),
    ));

    // 2. Cabin floor base + staggered oak floorboards + rug + clutter
    let floor_base_mat = materials.add(StandardMaterial {
        base_color: Color::srgb_u8(0x15, 0x0f, 0x0a),
        perceptual_roughness: 1.0,
        ..default()
    });
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(HALF * 2.0 + WALL_T, 0.4, HALF * 2.0 + WALL_T))),
        MeshMaterial3d(floor_base_mat),
        Transform::from_xyz(0.0, -0.28, 0.0),
    ));

    let board = 0.42;
    let mut floor_b = BlockBatch::new();
    let mut frng = SeededRng::new(11);
    let board_tones = [0x4d3825, 0x46321f, 0x533d28, 0x41301e];
    let mut z = -EDGE + board * 0.5;
    while z < EDGE {
        let mut x = -EDGE - frng.next_f32() * 2.0;
        while x < EDGE {
            let len = 1.6 + frng.next_f32() * 2.6;
            let a = x.max(-EDGE);
            let b = (x + len).min(EDGE);
            let r = frng.next_f32();
            let tone = if r < 0.05 {
                0x2e2115
            } else if r < 0.1 {
                0x5e4730
            } else {
                board_tones[(frng.next_f32() * 4.0) as usize % 4]
            };
            floor_b.add(
                &[BlockSpec::new(b - a - 0.015, 0.08, board - 0.015, (a + b) * 0.5, -0.04, z, tone).with_bevel(0.018)],
                &mut frng,
                None,
                false,
            );
            x += len;
        }
        z += board;
    }

    let rug_m = at(3.0, -2.6, 0.08);
    for (i, (w1, d1, w2, d2, c)) in [
        (3.8, 2.2, 3.2, 2.7, 0x55201c),
        (3.2, 1.6, 2.6, 2.1, 0x6a5432),
        (2.6, 1.0, 2.0, 1.5, 0x55201c),
        (1.7, 0.45, 1.3, 0.75, 0x3e2618),
    ]
    .into_iter()
    .enumerate()
    {
        let fi = i as f32;
        floor_b.add(
            &[
                BlockSpec::new(w1, 0.012, d1, 0.0, 0.006 + fi * 0.008, 0.0, c).with_bevel(0.004),
                BlockSpec::new(w2, 0.012, d2, 0.0, 0.009 + fi * 0.008, 0.0, c).with_bevel(0.004),
            ],
            &mut frng,
            Some(rug_m),
            false,
        );
    }

    for (x, pz, ry) in [
        (-6.5, -7.8, 0.4),
        (-6.1, -7.4, 1.3),
        (7.2, 6.4, 2.2),
        (-8.2, 3.2, 0.9),
        (1.2, 7.9, 2.8),
    ] {
        floor_b.add(
            &[BlockSpec::new(0.22, 0.006, 0.3, 0.0, 0.004, 0.0, 0xb5ab90).with_bevel(0.002)],
            &mut frng,
            Some(at(x, pz, ry)),
            false,
        );
    }
    floor_b.add(
        &[
            BlockSpec::new(0.13, 0.13, 0.17, 0.0, 0.065, 0.0, 0x8a8f96).with_bevel(0.02),
            BlockSpec::new(0.135, 0.07, 0.1, 0.0, 0.065, 0.02, 0x9a3a2a).with_bevel(0.01),
        ],
        &mut frng,
        Some(at(-3.5, 6.2, 0.7)),
        false,
    );
    for (x, pz, w, ry) in [(0.4, 8.9, 0.9, 0.2), (-0.5, 8.1, 0.6, -0.4), (0.9, 7.3, 0.5, 0.6)] {
        floor_b.add(
            &[BlockSpec::new(w, 0.006, w * 0.45, 0.0, 0.004, 0.0, 0x2a0b09).with_bevel(0.002)],
            &mut frng,
            Some(at(x, pz, ry)),
            false,
        );
    }
    commands.spawn((
        Mesh3d(meshes.add(floor_b.build())),
        MeshMaterial3d(world_mat.clone()),
        NotShadowCaster,
    ));

    // 3. Log walls, openings, and interior props
    let mut wall_b = BlockBatch::new();
    let mut wrng = SeededRng::new(23);

    let mut build_wall = |axis: WallAxis, fixed: f32, mut gaps: Vec<(f32, f32, OpeningKind)>| {
        gaps.sort_by(|a, b| a.0.total_cmp(&b.0));
        let min = -EDGE;
        let max = EDGE;
        let mut segs = Vec::new();
        let mut mine = Vec::new();
        let mut cursor = min;
        for (gs, ge, kind) in gaps {
            if gs > cursor {
                segs.push((cursor, gs));
            }
            cursor = cursor.max(ge);
            mine.push(make_opening(axis, fixed, gs, ge, kind, WALL_T));
        }
        if cursor < max {
            segs.push((cursor, max));
        }
        let layer0 = if axis == WallAxis::X { 0.24 } else { 0.44 };
        for (a, b) in segs {
            let (hw, hd, cx, cz) = match axis {
                WallAxis::X => ((b - a) * 0.5, WALL_T * 0.5, (a + b) * 0.5, fixed),
                WallAxis::Z => (WALL_T * 0.5, (b - a) * 0.5, fixed, (a + b) * 0.5),
            };
            wall_boxes.boxes.push(WallBox {
                min_x: cx - hw,
                max_x: cx + hw,
                min_z: cz - hd,
                max_z: cz + hd,
            });
            log_run(
                &mut wall_b,
                &mut wrng,
                axis,
                fixed,
                a,
                b,
                layer0,
                |_| true,
                a > min + 0.01,
                b < max - 0.01,
                true,
            );
        }
        for mut o in mine {
            let sill = o.sill;
            let head = o.head;
            let is_door = o.kind == OpeningKind::Door;
            log_run(
                &mut wall_b,
                &mut wrng,
                axis,
                fixed,
                o.u0,
                o.u1,
                layer0,
                |y| {
                    if is_door {
                        y - LOG_H * 0.5 >= DOOR_TOP - 0.01
                    } else {
                        y + LOG_H * 0.5 <= sill + 0.01 || y - LOG_H * 0.5 >= head - 0.01
                    }
                },
                true,
                true,
                !is_door,
            );
            fit_opening(
                &mut o,
                &mut commands,
                &mut meshes,
                &mut wall_b,
                &world_mat,
                &glass_mat,
                &mut wrng,
            );
            openings.list.push(o);
        }
    };

    build_wall(WallAxis::X, HALF, vec![(-0.8, 0.8, OpeningKind::Door)]);
    build_wall(WallAxis::X, -HALF, vec![(-5.8, -4.2, OpeningKind::Door), (4.0, 6.0, OpeningKind::Window)]);
    build_wall(WallAxis::Z, -HALF, vec![(-1.5, 0.5, OpeningKind::Window)]);
    build_wall(WallAxis::Z, HALF, vec![(1.5, 3.5, OpeningKind::Window)]);

    let in_n = -HALF + LOG_D * 0.5;
    let in_w = -HALF + LOG_D * 0.5;
    let in_e = HALF - LOG_D * 0.5;
    wall_b.add(
        &[
            BlockSpec::new(1.9, 0.06, 0.3, -1.7, 1.6, in_n + 0.15, 0x5e4730).with_bevel(0.015),
            BlockSpec::new(0.05, 0.22, 0.26, -2.5, 1.48, in_n + 0.13, POST),
            BlockSpec::new(0.05, 0.22, 0.26, -0.9, 1.48, in_n + 0.13, POST),
            BlockSpec::new(0.14, 0.2, 0.14, -2.45, 1.73, in_n + 0.15, 0x6e7f4a).with_bevel(0.03),
            BlockSpec::new(0.14, 0.22, 0.14, -2.22, 1.74, in_n + 0.15, 0xa05a2a).with_bevel(0.03),
            BlockSpec::new(0.12, 0.16, 0.12, -1.95, 1.71, in_n + 0.15, 0x8a8f96).with_bevel(0.02),
            BlockSpec::new(0.12, 0.16, 0.12, -1.78, 1.71, in_n + 0.13, 0x8a8f96).with_bevel(0.02),
            BlockSpec::new(0.32, 0.08, 0.22, -1.25, 1.67, in_n + 0.14, 0x5a2a20)
                .with_rot(0.0, 0.1, 0.0)
                .with_bevel(0.015),
            BlockSpec::new(0.3, 0.07, 0.2, -1.24, 1.745, in_n + 0.14, 0x2a3a4a)
                .with_rot(0.0, -0.15, 0.0)
                .with_bevel(0.015),
            BlockSpec::new(0.36, 0.44, 0.05, 1.4, 1.9, in_n + 0.025, POST).with_bevel(0.02),
            BlockSpec::new(0.17, 0.2, 0.16, 1.4, 1.9, in_n + 0.1, 0xcfc4a2).with_bevel(0.03),
            BlockSpec::new(0.04, 0.42, 0.04, 1.27, 2.13, in_n + 0.1, 0xb8ab88)
                .with_rot(0.0, 0.0, 0.55)
                .with_bevel(0.01),
            BlockSpec::new(0.04, 0.42, 0.04, 1.53, 2.13, in_n + 0.1, 0xb8ab88)
                .with_rot(0.0, 0.0, -0.55)
                .with_bevel(0.01),
            BlockSpec::new(0.03, 0.18, 0.03, 1.15, 2.25, in_n + 0.1, 0xb8ab88)
                .with_rot(0.0, 0.0, -0.3)
                .with_bevel(0.008),
            BlockSpec::new(0.03, 0.18, 0.03, 1.65, 2.25, in_n + 0.1, 0xb8ab88)
                .with_rot(0.0, 0.0, 0.3)
                .with_bevel(0.008),
            BlockSpec::new(0.48, 0.62, 0.02, 2.75, 1.4, in_n + 0.012, 0xb8a888)
                .with_rot(0.0, 0.0, 0.04)
                .with_bevel(0.005),
            BlockSpec::new(0.4, 0.2, 0.025, 2.75, 1.58, in_n + 0.015, 0x7a2a22)
                .with_rot(0.0, 0.0, 0.04)
                .with_bevel(0.005),
            BlockSpec::new(0.06, 0.06, 1.1, in_w + 0.04, 1.75, 4.6, POST).with_bevel(0.015),
            BlockSpec::new(0.16, 0.85, 0.5, in_w + 0.1, 1.3, 4.3, 0x3e4a33)
                .with_rot(0.0, 0.0, 0.04)
                .with_bevel(0.04),
            BlockSpec::new(0.24, 0.12, 0.24, in_w + 0.14, 1.76, 4.95, 0x6b5232).with_bevel(0.04),
            BlockSpec::new(0.03, 0.5, 0.42, in_e - 0.015, 1.6, -5.0, POST).with_bevel(0.01),
            BlockSpec::new(0.035, 0.42, 0.34, in_e - 0.02, 1.6, -5.0, 0x55606a).with_bevel(0.005),
        ],
        &mut wrng,
        None,
        false,
    );
    commands.spawn((
        Mesh3d(meshes.add(wall_b.build())),
        MeshMaterial3d(world_mat.clone()),
    ));

    // 4. The yard
    let mut yard = BlockBatch::new();
    let mut yrng = SeededRng::new(41);
    let mut clearings: Vec<(f32, f32, f32)> = Vec::new();

    for k in 0..4 {
        for i in 0..10 {
            let x = 3.75 + i as f32 * 0.37 + (k % 2) as f32 * 0.18;
            let y = 0.17 + k as f32 * 0.3;
            if x > 7.5 {
                continue;
            }
            let tone = LOG_TONES[(yrng.next_f32() * 4.0) as usize % 4];
            let rz = (yrng.next_f32() - 0.5) * 0.2;
            yard.add(
                &[
                    BlockSpec::new(0.33, 0.3, 0.62, x, y, 10.9, tone)
                        .with_rot(0.0, 0.0, rz)
                        .with_bevel(0.1),
                    BlockSpec::new(0.27, 0.24, 0.02, x, y, 11.22, END_GRAIN).with_bevel(0.06),
                ],
                &mut yrng,
                None,
                false,
            );
        }
    }
    yard.add(
        &[
            BlockSpec::new(0.12, 1.4, 0.12, 3.45, 0.7, 10.9, POST),
            BlockSpec::new(0.12, 1.4, 0.12, 7.75, 0.7, 10.9, POST),
        ],
        &mut yrng,
        None,
        false,
    );
    clearings.push((5.6, 11.0, 2.4));

    yard.add(
        &[
            BlockSpec::new(0.62, 0.5, 0.62, 0.0, 0.25, 0.0, LOG_TONES[2]).with_bevel(0.18),
            BlockSpec::new(0.5, 0.02, 0.5, 0.0, 0.505, 0.0, END_GRAIN).with_bevel(0.12),
            BlockSpec::new(0.06, 0.06, 0.78, 0.06, 0.82, -0.2, 0x6b4a2a)
                .with_rot(-0.75, 0.0, 0.0)
                .with_bevel(0.02),
            BlockSpec::new(0.06, 0.2, 0.28, 0.06, 0.56, 0.06, 0x5a5e66)
                .with_rot(-0.75, 0.0, 0.0)
                .with_bevel(0.02),
        ],
        &mut yrng,
        Some(at(-4.3, 12.5, 0.6)),
        false,
    );
    clearings.push((-4.3, 12.5, 1.5));

    const RUST: u32 = 0x46524c;
    const RUST2: u32 = 0x6a4a2c;
    const CHROME: u32 = 0x7a7a72;
    const GLASS: u32 = 0x182027;
    yard.add(
        &[
            BlockSpec::new(1.92, 0.62, 4.6, 0.0, 0.78, 0.0, RUST).with_bevel(0.12),
            BlockSpec::new(1.8, 0.74, 1.5, 0.0, 1.45, 0.45, RUST).with_bevel(0.1),
            BlockSpec::new(1.6, 0.4, 0.05, 0.0, 1.52, 1.21, GLASS)
                .with_rot(-0.25, 0.0, 0.0)
                .with_bevel(0.01),
            BlockSpec::new(1.82, 0.36, 1.2, 0.0, 1.55, 0.4, GLASS).with_bevel(0.02),
            BlockSpec::new(1.86, 0.2, 1.4, 0.0, 1.18, 1.55, RUST).with_bevel(0.08),
            BlockSpec::new(0.1, 0.42, 2.0, 0.92, 1.28, -1.2, RUST).with_bevel(0.03),
            BlockSpec::new(0.1, 0.42, 2.0, -0.92, 1.28, -1.2, RUST).with_bevel(0.03),
            BlockSpec::new(1.94, 0.42, 0.1, 0.0, 1.28, -2.25, RUST)
                .with_rot(0.0, 0.0, 0.06)
                .with_bevel(0.03),
            BlockSpec::new(1.7, 0.06, 1.9, 0.0, 1.1, -1.2, 0x2a2a28).with_bevel(0.01),
            BlockSpec::new(2.02, 0.2, 0.16, 0.0, 0.6, 2.33, CHROME).with_bevel(0.05),
            BlockSpec::new(2.02, 0.2, 0.16, 0.0, 0.6, -2.33, CHROME).with_bevel(0.05),
            BlockSpec::new(0.32, 0.2, 0.05, 0.62, 0.95, 2.31, 0xe8e2b0).with_bevel(0.02),
            BlockSpec::new(0.32, 0.2, 0.05, -0.62, 0.95, 2.31, 0x1a1a18).with_bevel(0.02),
            BlockSpec::new(0.6, 0.34, 0.03, 0.97, 0.85, 0.9, RUST2).with_bevel(0.01),
            BlockSpec::new(0.5, 0.3, 0.03, -0.97, 0.75, -1.6, RUST2).with_bevel(0.01),
            BlockSpec::new(0.9, 0.04, 0.7, 0.2, 1.33, -1.5, 0x4a3a2a)
                .with_rot(0.0, 0.3, 0.0)
                .with_bevel(0.01),
            BlockSpec::new(0.36, 0.7, 0.7, 0.98, 0.35, 1.45, 0x141414).with_bevel(0.2),
            BlockSpec::new(0.36, 0.7, 0.7, -0.98, 0.35, 1.45, 0x141414).with_bevel(0.2),
            BlockSpec::new(0.36, 0.7, 0.7, 0.98, 0.35, -1.45, 0x141414).with_bevel(0.2),
            BlockSpec::new(0.36, 0.56, 0.78, -0.98, 0.26, -1.45, 0x141414).with_bevel(0.16),
        ],
        &mut yrng,
        Some(at(-7.6, 15.2, 0.55)),
        false,
    );
    clearings.push((-7.6, 15.2, 3.4));
    solids.add_solid(-7.6, 15.2, 0.55, 1.05, 2.45);
    clearings.push((2.0, 23.5, 4.5));
    clearings.push((1.0, 17.5, 3.5));
    clearings.push((7.5, 20.0, 3.5));

    // Porch lantern post
    yard.add(
        &[
            BlockSpec::new(0.16, 2.3, 0.16, 2.7, 1.15, 11.3, POST),
            BlockSpec::new(0.56, 0.08, 0.08, 2.48, 2.22, 11.3, POST),
            BlockSpec::new(0.24, 0.06, 0.24, 2.26, 2.0, 11.3, 0x2a2a2a),
        ],
        &mut yrng,
        None,
        false,
    );

    // Broken fence
    let fence_segs: [(f32, f32, f32, f32, bool); 3] = [
        (8.6, 12.6, 10.8, 12.0, false),
        (10.8, 12.0, 12.6, 10.6, true),
        (-12.5, -9.5, -12.2, -12.0, false),
    ];
    for (x0, z0, x1, z1, broken) in fence_segs {
        let len = (x1 - x0).hypot(z1 - z0);
        let ry = (x1 - x0).atan2(z1 - z0);
        yard.add(&[BlockSpec::new(0.14, 1.2, 0.14, 0.0, 0.6, 0.0, POST)], &mut yrng, Some(at(x0, z0, ry)), false);
        yard.add(&[BlockSpec::new(0.14, 1.2, 0.14, 0.0, 0.6, 0.0, POST)], &mut yrng, Some(at(x1, z1, ry)), false);
        if broken {
            yard.add(
                &[BlockSpec::new(0.08, 0.14, len, 0.0, 0.5, len * 0.5, WEATHERED)
                    .with_rot(0.35, 0.0, 0.0)
                    .with_bevel(0.02)],
                &mut yrng,
                Some(at(x0, z0, ry)),
                false,
            );
        } else {
            yard.add(
                &[
                    BlockSpec::new(0.08, 0.14, len, 0.0, 0.95, len * 0.5, WEATHERED).with_bevel(0.02),
                    BlockSpec::new(0.08, 0.14, len, 0.0, 0.5, len * 0.5, WEATHERED).with_bevel(0.02),
                ],
                &mut yrng,
                Some(at(x0, z0, ry)),
                false,
            );
        }
    }

    // Stumps and fallen trunk
    for (x, pz, s) in [
        (11.6, -4.0, 1.0),
        (-11.4, 7.5, 0.8),
        (6.0, -12.0, 1.1),
        (-12.6, -2.5, 0.9),
        (12.8, 7.5, 0.85),
        (-2.5, -12.4, 1.0),
    ] {
        let ry1 = yrng.next_f32();
        let ry2 = yrng.next_f32();
        yard.add(
            &[
                BlockSpec::new(0.55 * s, 0.4 * s, 0.55 * s, 0.0, 0.2 * s, 0.0, 0x3a2b1d)
                    .with_rot(0.0, ry1, 0.0)
                    .with_bevel(0.15),
                BlockSpec::new(0.46 * s, 0.02, 0.46 * s, 0.0, 0.4 * s + 0.01, 0.0, END_GRAIN)
                    .with_rot(0.0, ry2, 0.0)
                    .with_bevel(0.12),
            ],
            &mut yrng,
            Some(at(x, pz, 0.0)),
            false,
        );
    }
    yard.add(
        &[
            BlockSpec::new(0.6, 0.55, 5.5, 0.0, 0.27, 0.0, 0x3a2f24).with_bevel(0.18),
            BlockSpec::new(0.5, 0.45, 0.02, 0.0, 0.27, 2.76, END_GRAIN).with_bevel(0.12),
            BlockSpec::new(0.14, 0.9, 0.14, 0.3, 0.6, -1.2, 0x3a2f24)
                .with_rot(0.0, 0.0, -0.7)
                .with_bevel(0.03),
        ],
        &mut yrng,
        Some(at(-13.5, 4.0, 0.3)),
        false,
    );
    clearings.push((-13.5, 4.0, 3.0));
    commands.spawn((
        Mesh3d(meshes.add(yard.build())),
        MeshMaterial3d(world_mat.clone()),
    ));

    // Porch lantern glass & PointLight
    let lantern_mat = materials.add(StandardMaterial {
        base_color: Color::srgb_u8(0x2a, 0x1a, 0x08),
        emissive: LinearRgba::rgb(1.0, 0.69, 0.31) * 1.6,
        ..default()
    });
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(0.2, 0.26, 0.2))),
        MeshMaterial3d(lantern_mat),
        Transform::from_xyz(2.26, 1.84, 11.3),
        LanternGlass,
    ));
    commands.spawn((
        PointLight {
            color: Color::srgb_u8(0xff, 0xa6, 0x4a),
            intensity: 200_000.0,
            range: 9.0,
            ..default()
        },
        Transform::from_xyz(2.26, 1.7, 11.3),
        LanternLight,
    ));

    // 5. Forest & brush
    let mut trng = SeededRng::new(77);
    let tree_defs = [
        (pine_tree(4, 3.0, 1.05, 1.6), 38.0),
        (pine_tree(3, 2.6, 0.95, 1.2), 18.0),
        (pine_tree(6, 1.8, 0.75, 1.3), 22.0),
        (oak_tree(&mut trng), 13.0),
        (dead_tree(&mut trng), 9.0),
    ];
    let tree_kinds: Vec<(Handle<Mesh>, Handle<Mesh>, f32)> = tree_defs
        .into_iter()
        .map(|(blocks, weight)| {
            let near = meshes.add(bake_blocks(&blocks, &mut SeededRng::new(1), false));
            let far = meshes.add(bake_blocks(&blocks, &mut SeededRng::new(1), true));
            (near, far, weight)
        })
        .collect();

    let in_clearing = |x: f32, z: f32, pad: f32| -> bool {
        (x.abs() < EDGE + 0.6 + pad && z.abs() < EDGE + 0.6 + pad)
            || clearings
                .iter()
                .any(|&(cx, cz, r)| (x - cx).powi(2) + (z - cz).powi(2) < (r + pad).powi(2))
    };

    let mut rng = rand::rng();
    let total_tree_w: f32 = tree_kinds.iter().map(|k| k.2).sum();
    for _ in 0..FOREST_COUNT {
        let rad = (R_MIN * R_MIN + rng.random::<f32>() * (R_MAX * R_MAX - R_MIN * R_MIN)).sqrt();
        let ang = rng.random::<f32>() * TAU;
        let x = ang.cos() * rad;
        let z = ang.sin() * rad;
        if in_clearing(x, z, 0.8) {
            continue;
        }
        let mut r = rng.random::<f32>() * total_tree_w;
        let mut k = 0;
        while k + 1 < tree_kinds.len() {
            r -= tree_kinds[k].2;
            if r <= 0.0 {
                break;
            }
            k += 1;
        }
        let s = 0.85 + rng.random::<f32>() * 0.45;
        let sy = s * (0.9 + rng.random::<f32>() * 0.3);
        let far = rad >= SHADOW_R;
        let mesh = if far {
            tree_kinds[k].1.clone()
        } else {
            tree_kinds[k].0.clone()
        };
        let mut entity = commands.spawn((
            Mesh3d(mesh),
            MeshMaterial3d(world_mat.clone()),
            Transform::from_xyz(x, GROUND_Y, z)
                .with_rotation(Quat::from_rotation_y(rng.random::<f32>() * TAU))
                .with_scale(Vec3::new(s, sy, s)),
        ));
        if far {
            entity.insert(NotShadowCaster);
        }
    }

    let mut brng = SeededRng::new(91);
    let mut tuft = Vec::new();
    for i in 0..6 {
        let a = i as f32 * 1.05 + brng.next_f32();
        let h = 0.25 + brng.next_f32() * 0.25;
        let col = if brng.next_f32() < 0.5 { 0x2d3b1e } else { 0x3a4826 };
        tuft.push(
            BlockSpec::new(0.05, h, 0.05, a.cos() * 0.07, h * 0.5, a.sin() * 0.07, col)
                .with_rot(a.sin() * 0.35, 0.0, -a.cos() * 0.35)
                .with_bevel(0.0),
        );
    }
    let mut fern = Vec::new();
    for i in 0..7 {
        let a = i as f32 * 0.9;
        fern.push(
            BlockSpec::new(0.11, 0.03, 0.6, a.sin() * 0.28, 0.12, a.cos() * 0.28, 0x2c4a22)
                .with_rot(-0.35, a, 0.0)
                .with_bevel(0.0),
        );
    }
    let rock = vec![
        BlockSpec::new(0.5, 0.32, 0.42, 0.0, 0.12, 0.0, 0x3a3834)
            .with_rot(0.0, 0.3, 0.1)
            .with_bevel(0.1),
        BlockSpec::new(0.3, 0.22, 0.3, 0.24, 0.08, 0.14, 0x33312d)
            .with_rot(0.0, 0.9, 0.0)
            .with_bevel(0.08),
    ];
    let brush_kinds = [
        (meshes.add(bake_blocks(&tuft, &mut brng, false)), 60.0, false),
        (meshes.add(bake_blocks(&fern, &mut brng, false)), 25.0, false),
        (meshes.add(bake_blocks(&rock, &mut brng, false)), 15.0, true),
    ];
    let total_brush_w: f32 = brush_kinds.iter().map(|k| k.1).sum();
    for _ in 0..1000 {
        let rad = (10.6_f32 * 10.6 + rng.random::<f32>() * (28.0_f32 * 28.0 - 10.6 * 10.6)).sqrt();
        let ang = rng.random::<f32>() * TAU;
        let x = ang.cos() * rad;
        let z = ang.sin() * rad;
        if in_clearing(x, z, 0.0) {
            continue;
        }
        let mut r = rng.random::<f32>() * total_brush_w;
        let mut k = 0;
        while k + 1 < brush_kinds.len() {
            r -= brush_kinds[k].1;
            if r <= 0.0 {
                break;
            }
            k += 1;
        }
        let s = 0.7 + rng.random::<f32>() * 0.7;
        let sy = s * (0.9 + rng.random::<f32>() * 0.3);
        let far = rad >= SHADOW_R;
        let mut entity = commands.spawn((
            Mesh3d(brush_kinds[k].0.clone()),
            MeshMaterial3d(world_mat.clone()),
            Transform::from_xyz(x, GROUND_Y, z)
                .with_rotation(Quat::from_rotation_y(rng.random::<f32>() * TAU))
                .with_scale(Vec3::new(s, sy, s)),
        ));
        if !brush_kinds[k].2 || far {
            entity.insert(NotShadowCaster);
        }
    }

    // 6. Atmosphere: ambient light, moonlight, horizon band, and moon disc
    commands.insert_resource(GlobalAmbientLight {
        color: Color::srgb_u8(0x4a, 0x5b, 0x80),
        brightness: 950.0,
        affects_lightmapped_meshes: true,
    });

    commands.spawn((
        DirectionalLight {
            color: Color::srgb_u8(0xc4, 0xd4, 0xf2),
            illuminance: 10_000.0,
            shadow_maps_enabled: true,
            ..default()
        },
        CascadeShadowConfigBuilder {
            num_cascades: 1,
            maximum_distance: 80.0,
            ..default()
        }
        .build(),
        Transform::from_xyz(-18.0, 34.0, -10.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    // Subtle horizon glow band for the low title camera
    let horizon_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.075, 0.095, 0.15),
        unlit: true,
        fog_enabled: false,
        ..default()
    });
    commands.spawn((
        Mesh3d(meshes.add(Cylinder::new(145.0, 22.0))),
        MeshMaterial3d(horizon_mat),
        Transform::from_xyz(0.0, 8.0, 0.0).with_scale(Vec3::new(-1.0, 1.0, 1.0)),
        NotShadowCaster,
    ));

    // Moon disc
    let moon_tex = images.add(moon_texture());
    let moon_mat = materials.add(StandardMaterial {
        base_color_texture: Some(moon_tex),
        unlit: true,
        alpha_mode: AlphaMode::Blend,
        fog_enabled: false,
        ..default()
    });
    commands.spawn((
        Mesh3d(meshes.add(Rectangle::new(30.0, 30.0))),
        MeshMaterial3d(moon_mat),
        Transform::from_xyz(-26.0, 27.0, -116.0).looking_at(Vec3::ZERO, Vec3::Y),
        NotShadowCaster,
    ));

    // Initialize solids navigation graph after all solids have been added
    solids.init_nav();

    commands.insert_resource(WorldMaterials {
        world_mat,
        ground_mat,
    });
}

/// Guttering porch lantern flame: two slow wobbles and the odd dip, worse in the wind.
pub fn update_world(
    time: Res<Time>,
    weather_level: Res<WorldWeatherLevel>,
    mut light_q: Query<&mut PointLight, With<LanternLight>>,
    glass_q: Query<&MeshMaterial3d<StandardMaterial>, With<LanternGlass>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let t = time.elapsed_secs();
    let w_level = weather_level.0;
    let dip = if (t * 0.9).sin() > 0.97 { 0.35 } else { 0.0 };
    let f = 0.85
        + 0.08 * (t * 7.3).sin()
        + 0.06 * (t * 13.1 + 1.0).sin()
        - dip
        - w_level * 0.2 * (0.5 + 0.5 * (t * 5.3).sin() * (t * 2.1).sin());

    for mut light in &mut light_q {
        light.intensity = 200_000.0 * f;
    }
    for mat_handle in &glass_q {
        if let Some(mut mat) = materials.get_mut(&mat_handle.0) {
            mat.emissive = LinearRgba::rgb(1.0, 0.69, 0.31) * (1.6 * f);
        }
    }
}
