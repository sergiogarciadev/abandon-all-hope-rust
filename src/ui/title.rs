#![allow(deprecated)]

use std::f32::consts::FRAC_PI_2;

use bevy::asset::RenderAssetUsages;
use bevy::ecs::system::SystemParam;
use bevy::image::{ImageSampler, ImageSamplerDescriptor};
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use rand::RngExt;

use crate::audio::{AudioEngine, UiSound};
use crate::core::math::damp;
use crate::core::state::{BannerState, CameraTransition, DamageFlash, GameState, GameStats, ToastState};
use crate::effects::blood::BloodParticle;
use crate::effects::debris::Debris;
use crate::entities::drops::DropItem;
use crate::entities::player::{PlayerRig, PlayerRigState};
use crate::entities::weapon::Bullet;
use crate::entities::zombies::{
    Zombie, ZombieAnim, ZombieAssets, pose_zombie_rig, spawn_zombie, walk_toward,
};
use crate::ui::storage::{GameSettings, save_storage};
use crate::world::openings::Openings;
use crate::world::solids::SolidsNav;
use crate::world::weather::WeatherState;

const TITLE_ZOMBIES: usize = 4;
const CROSS_Z: [f32; 2] = [11.5, 15.5];
const CROSS_X: f32 = 11.5;

#[derive(Component)]
pub struct TitleZombie {
    pub dir: f32,
    pub target_z: f32,
    pub stop_x: Option<f32>,
    pub idle: f32,
    pub speed: f32,
}

#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum TitlePanel {
    #[default]
    None,
    How,
    Settings,
}

#[derive(Resource)]
pub struct TitleUiState {
    pub splash: bool,
    pub sel: usize,
    pub panel: TitlePanel,
    pub setting_row: usize,
    pub grain_timer: f32,
    pub fade_timer: f32,
    pub groan_timer: f32,
}

impl Default for TitleUiState {
    fn default() -> Self {
        Self {
            splash: true,
            sel: 0,
            panel: TitlePanel::None,
            setting_row: 0,
            grain_timer: 0.0,
            fade_timer: 1.1,
            groan_timer: 3.0,
        }
    }
}

#[derive(Resource)]
pub struct GrainImage(pub Handle<Image>);

#[derive(Component)]
pub struct FadeInOverlay;

#[derive(Component)]
pub struct GrainOverlay;

#[derive(Component)]
pub struct TitleUiRoot;

#[derive(Component)]
pub struct SplashPrompt;

#[derive(Component)]
pub struct MenuContainer;

#[derive(Component)]
pub struct MenuItemBtn(pub usize);

#[derive(Component)]
pub struct MenuBestText;

#[derive(Component)]
pub struct HowPanelRoot;

#[derive(Component)]
pub struct SettingsPanelRoot;

#[derive(Component)]
pub struct SettingRow(pub usize);

#[derive(Component)]
pub struct MeterPip {
    pub row: usize,
    pub idx: usize,
}

#[derive(Component)]
pub struct ShakeToggleText;

#[derive(Component)]
pub struct PauseOverlay;

#[derive(Component)]
pub struct PauseStatText;

#[derive(Component)]
pub struct PauseBtn(pub usize); // 0 = Resume, 1 = Title

#[derive(Component)]
pub struct DeathOverlay;

#[derive(Component)]
pub struct DeathStatText;

#[derive(Component)]
pub struct DeathBestText;

#[derive(Component)]
pub struct DeathBtn(pub usize); // 0 = Try Again, 1 = Title

fn make_grain_image() -> Image {
    let mut rng = rand::rng();
    let mut data = Vec::with_capacity(256 * 256 * 4);
    for _ in 0..(256 * 256) {
        let v: u8 = rng.random_range(0..=255);
        data.extend_from_slice(&[v, v, v, 8]);
    }
    let mut img = Image::new(
        Extent3d {
            width: 256,
            height: 256,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD | RenderAssetUsages::MAIN_WORLD,
    );
    img.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor::nearest());
    img
}

pub fn spawn_title_zombies(
    mut commands: Commands,
    zombie_assets: Option<Res<ZombieAssets>>,
    solids: Res<SolidsNav>,
    existing: Query<Entity, With<TitleZombie>>,
) {
    let Some(zombie_assets) = zombie_assets else {
        return;
    };
    if !existing.is_empty() {
        return;
    }
    let mut rng = rand::rng();
    for i in 0..TITLE_ZOMBIES {
        let dir = if i % 2 == 1 { 1.0 } else { -1.0 };
        let x = CROSS_X * (2.0 * (i as f32 + rng.random_range(0.2..0.8)) / TITLE_ZOMBIES as f32 - 1.0);
        let lz = rng.random_range(CROSS_Z[0]..CROSS_Z[1]);
        let mut p2 = Vec2::new(x, lz);
        solids.push_out(&mut p2);
        let pos = Vec3::new(p2.x, 0.0, p2.y);
        let kind = rng.random_range(0..zombie_assets.kits.len());
        let speed = rng.random_range(0.7..1.05);
        let ent = spawn_zombie(&mut commands, &zombie_assets, kind, pos, 2.0, speed);
        commands.entity(ent).insert(TitleZombie {
            dir,
            target_z: rng.random_range(CROSS_Z[0]..CROSS_Z[1]),
            stop_x: if rng.random::<f32>() < 0.35 {
                Some(rng.random_range(-5.0..5.0))
            } else {
                None
            },
            idle: 0.0,
            speed,
        });
    }
}

pub fn wander_title_zombies(
    time: Res<Time>,
    solids: Res<SolidsNav>,
    audio: Res<AudioEngine>,
    mut title_state: ResMut<TitleUiState>,
    mut zombies: Query<(Entity, &mut TitleZombie, &mut Zombie, &mut ZombieAnim)>,
    mut transforms: Query<&mut Transform>,
) {
    let dt = time.delta_secs();
    let elapsed = time.elapsed_secs();
    let mut rng = rand::rng();

    title_state.groan_timer -= dt;
    let mut groaned = false;

    for (z_ent, mut tz, mut z, mut anim) in &mut zombies {
        let Ok(mut tf) = transforms.get_mut(z_ent) else {
            continue;
        };
        let mut walking = false;
        let p2 = Vec2::new(tf.translation.x, tf.translation.z);
        if tz.idle > 0.0 {
            tz.idle -= dt;
        } else if let Some(sx) = tz.stop_x {
            if (tf.translation.x - sx) * tz.dir > 0.0 {
                tz.stop_x = None;
                tz.idle = rng.random_range(1.5..3.5);
            } else {
                let g = solids.route_around(
                    p2,
                    tz.dir * (CROSS_X + 1.0),
                    tz.target_z,
                    &mut z.via,
                );
                walk_toward(
                    &mut z,
                    &mut tf,
                    g.x - p2.x,
                    g.y - p2.y,
                    tz.speed,
                    1.5,
                    dt,
                );
                walking = true;
            }
        } else if tf.translation.x * tz.dir > CROSS_X {
            let lz = rng.random_range(CROSS_Z[0]..CROSS_Z[1]);
            let mut np2 = Vec2::new(-tz.dir * CROSS_X, lz);
            solids.push_out(&mut np2);
            tf.translation = Vec3::new(np2.x, 0.0, np2.y);
            tz.target_z = rng.random_range(CROSS_Z[0]..CROSS_Z[1]);
            tz.stop_x = if rng.random::<f32>() < 0.35 {
                Some(rng.random_range(-5.0..5.0))
            } else {
                None
            };
            tz.idle = rng.random_range(0.0..1.0);
            z.heading = tz.dir * FRAC_PI_2;
            tf.rotation = Quat::from_rotation_y(z.heading);
        } else {
            let g = solids.route_around(
                p2,
                tz.dir * (CROSS_X + 1.0),
                tz.target_z,
                &mut z.via,
            );
            walk_toward(
                &mut z,
                &mut tf,
                g.x - p2.x,
                g.y - p2.y,
                tz.speed,
                1.5,
                dt,
            );
            walking = true;
        }

        if title_state.groan_timer <= 0.0 && !groaned && rng.random::<f32>() < 0.4 {
            groaned = true;
            anim.groan = 1.0;
            audio.groan(tf.translation);
            title_state.groan_timer = rng.random_range(3.0..7.0);
        }

        anim.gait = damp(anim.gait, if walking { 1.0 } else { 0.0 }, 4.0, dt);
        anim.speed = tz.speed;
        pose_zombie_rig(&mut anim, dt, elapsed, 99.0, 0.0, 0.0, &mut transforms);
    }
}

pub fn setup_title_ui(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    let grain_handle = images.add(make_grain_image());
    commands.insert_resource(GrainImage(grain_handle.clone()));

    // Startup fade-in overlay
    commands.spawn((
        FadeInOverlay,
        Pickable::IGNORE,
        GlobalZIndex(50),
        Node {
            position_type: PositionType::Absolute,
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            ..default()
        },
        BackgroundColor(Color::BLACK),
    ));

    // Title Screen Root
    commands
        .spawn((
            TitleUiRoot,
            GlobalZIndex(12),
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                top: Val::Px(0.0),
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                ..default()
            },
        ))
        .with_children(|root| {
            // Subtle tiled film grain over the title screen only
            root.spawn((
                GrainOverlay,
                Pickable::IGNORE,
                ImageNode {
                    image: grain_handle,
                    image_mode: NodeImageMode::Tiled {
                        tile_x: true,
                        tile_y: true,
                        stretch_value: 1.0,
                    },
                    ..default()
                },
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(0.0),
                    top: Val::Px(0.0),
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    ..default()
                },
            ));
            // Left dark scrim
            root.spawn((
                Pickable::IGNORE,
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(0.0),
                    top: Val::Px(0.0),
                    bottom: Val::Px(0.0),
                    width: Val::Percent(48.0),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.015, 0.015, 0.025, 0.84)),
            ));

            // Main column
            root.spawn(Node {
                position_type: PositionType::Absolute,
                left: Val::Px(80.0),
                top: Val::Percent(22.0),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(10.0),
                ..default()
            })
            .with_children(|col| {
                col.spawn((
                    Text::new("ABANDON ALL"),
                    TextFont {
                        font_size: FontSize::Px(36.0),
                        ..default()
                    },
                    TextColor(Color::srgb_u8(0xe9, 0xe2, 0xd6)),
                ));
                col.spawn((
                    Text::new("HOPE"),
                    TextFont {
                        font_size: FontSize::Px(108.0),
                        ..default()
                    },
                    TextColor(Color::srgb_u8(0xd6, 0x45, 0x3c)),
                ));
                col.spawn((
                    SplashPrompt,
                    Text::new("PRESS ANY KEY OR CLICK"),
                    TextFont {
                        font_size: FontSize::Px(22.0),
                        ..default()
                    },
                    TextColor(Color::srgb_u8(0xcd, 0xd2, 0xd8)),
                    Node {
                        margin: UiRect::top(Val::Px(48.0)),
                        ..default()
                    },
                ));

                // Menu items
                col.spawn((
                    MenuContainer,
                    Visibility::Hidden,
                    Node {
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(10.0),
                        margin: UiRect::top(Val::Px(36.0)),
                        ..default()
                    },
                ))
                .with_children(|list| {
                    for (idx, label) in ["01  PLAY", "02  HOW TO PLAY", "03  SETTINGS"]
                        .iter()
                        .enumerate()
                    {
                        #[allow(deprecated)]
                        list.spawn((
                            MenuItemBtn(idx),
                            Interaction::default(),
                            Node {
                                padding: UiRect::axes(Val::Px(18.0), Val::Px(8.0)),
                                border: UiRect::left(Val::Px(5.0)),
                                ..default()
                            },
                            BackgroundColor(Color::NONE),
                            BorderColor::all(Color::NONE),
                        ))
                        .with_child((
                            Text::new(*label),
                            TextFont {
                                font_size: FontSize::Px(32.0),
                                ..default()
                            },
                            TextColor(Color::srgb_u8(0x6c, 0x72, 0x7b)),
                        ));
                    }
                });
            });

            // Best run footer
            root.spawn((
                MenuBestText,
                Text::new(""),
                TextFont {
                    font_size: FontSize::Px(16.0),
                    ..default()
                },
                TextColor(Color::srgb_u8(0x8a, 0x90, 0x98)),
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(80.0),
                    bottom: Val::Px(32.0),
                    ..default()
                },
            ));

            // How To Play panel
            root.spawn((
                HowPanelRoot,
                Visibility::Hidden,
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(520.0),
                    top: Val::Percent(24.0),
                    width: Val::Px(440.0),
                    padding: UiRect::all(Val::Px(26.0)),
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(10.0),
                    border: UiRect::all(Val::Px(1.0)),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.05, 0.04, 0.05, 0.94)),
                BorderColor::all(Color::srgb_u8(0x2b, 0x26, 0x27)),
            ))
            .with_children(|p| {
                p.spawn((
                    Text::new("HOW TO PLAY"),
                    TextFont {
                        font_size: FontSize::Px(28.0),
                        ..default()
                    },
                    TextColor(Color::srgb_u8(0xe9, 0xe2, 0xd6)),
                ));
                for (k, v) in [
                    ("WASD", "Move"),
                    ("MOUSE", "Aim"),
                    ("LEFT CLICK", "Fire"),
                    ("R", "Reload"),
                    ("M", "Mute Audio"),
                    ("P / ESC", "Pause"),
                ] {
                    p.spawn((
                        Text::new(format!("{k:<12}  {v}")),
                        TextFont {
                            font_size: FontSize::Px(18.0),
                            ..default()
                        },
                        TextColor(Color::srgb_u8(0xcd, 0xd2, 0xd8)),
                    ));
                }
            });

            // Settings panel
            root.spawn((
                SettingsPanelRoot,
                Visibility::Hidden,
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(520.0),
                    top: Val::Percent(24.0),
                    width: Val::Px(460.0),
                    padding: UiRect::all(Val::Px(26.0)),
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(14.0),
                    border: UiRect::all(Val::Px(1.0)),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.05, 0.04, 0.05, 0.94)),
                BorderColor::all(Color::srgb_u8(0x2b, 0x26, 0x27)),
            ))
            .with_children(|p| {
                p.spawn((
                    Text::new("SETTINGS  (ARROW KEYS)"),
                    TextFont {
                        font_size: FontSize::Px(26.0),
                        ..default()
                    },
                    TextColor(Color::srgb_u8(0xe9, 0xe2, 0xd6)),
                ));
                for (row_idx, label) in ["MASTER", "SFX", "MUSIC"].iter().enumerate() {
                    p.spawn((
                        SettingRow(row_idx),
                        Node {
                            flex_direction: FlexDirection::Row,
                            justify_content: JustifyContent::SpaceBetween,
                            align_items: AlignItems::Center,
                            padding: UiRect::all(Val::Px(6.0)),
                            border: UiRect::left(Val::Px(3.0)),
                            ..default()
                        },
                        BorderColor::all(Color::NONE),
                    ))
                    .with_children(|row| {
                        row.spawn((
                            Text::new(*label),
                            TextFont {
                                font_size: FontSize::Px(18.0),
                                ..default()
                            },
                            TextColor(Color::srgb_u8(0xcd, 0xd2, 0xd8)),
                        ));
                        row.spawn(Node {
                            flex_direction: FlexDirection::Row,
                            column_gap: Val::Px(4.0),
                            ..default()
                        })
                        .with_children(|meter| {
                            for i in 0..10 {
                                meter.spawn((
                                    MeterPip {
                                        row: row_idx,
                                        idx: i,
                                    },
                                    Node {
                                        width: Val::Px(12.0),
                                        height: Val::Px(18.0),
                                        ..default()
                                    },
                                    BackgroundColor(Color::srgb_u8(0xd6, 0x45, 0x3c)),
                                ));
                            }
                        });
                    });
                }
                p.spawn((
                    SettingRow(3),
                    Node {
                        flex_direction: FlexDirection::Row,
                        justify_content: JustifyContent::SpaceBetween,
                        align_items: AlignItems::Center,
                        padding: UiRect::all(Val::Px(6.0)),
                        border: UiRect::left(Val::Px(3.0)),
                        ..default()
                    },
                    BorderColor::all(Color::NONE),
                ))
                .with_children(|row| {
                    row.spawn((
                        Text::new("SCREEN SHAKE"),
                        TextFont {
                            font_size: FontSize::Px(18.0),
                            ..default()
                        },
                        TextColor(Color::srgb_u8(0xcd, 0xd2, 0xd8)),
                    ));
                    row.spawn((
                        ShakeToggleText,
                        Text::new("ON"),
                        TextFont {
                            font_size: FontSize::Px(18.0),
                            ..default()
                        },
                        TextColor(Color::srgb_u8(0xe9, 0xe2, 0xd6)),
                    ));
                });
            });
        });

    // Pause Overlay
    commands
        .spawn((
            PauseOverlay,
            Visibility::Hidden,
            GlobalZIndex(15),
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                row_gap: Val::Px(16.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.02, 0.02, 0.03, 0.80)),
        ))
        .with_children(|p| {
            p.spawn((
                Text::new("PAUSED"),
                TextFont {
                    font_size: FontSize::Px(82.0),
                    ..default()
                },
                TextColor(Color::srgb_u8(0xe9, 0xe2, 0xd6)),
            ));
            p.spawn((
                PauseStatText,
                Text::new(""),
                TextFont {
                    font_size: FontSize::Px(18.0),
                    ..default()
                },
                TextColor(Color::srgb_u8(0x8a, 0x90, 0x98)),
            ));
            for (idx, lbl) in ["RESUME", "QUIT TO TITLE"].iter().enumerate() {
                #[allow(deprecated)]
                p.spawn((
                    PauseBtn(idx),
                    Interaction::default(),
                    Node {
                        padding: UiRect::axes(Val::Px(24.0), Val::Px(10.0)),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.1, 0.1, 0.12, 0.8)),
                ))
                .with_child((
                    Text::new(*lbl),
                    TextFont {
                        font_size: FontSize::Px(26.0),
                        ..default()
                    },
                    TextColor(Color::srgb_u8(0xcd, 0xd2, 0xd8)),
                ));
            }
        });

    // Death Overlay
    commands
        .spawn((
            DeathOverlay,
            Visibility::Hidden,
            GlobalZIndex(15),
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                row_gap: Val::Px(14.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.03, 0.01, 0.01, 0.86)),
        ))
        .with_children(|d| {
            d.spawn((
                Text::new("YOU DIED"),
                TextFont {
                    font_size: FontSize::Px(92.0),
                    ..default()
                },
                TextColor(Color::srgb_u8(0xb8, 0x33, 0x2b)),
            ));
            d.spawn((
                DeathStatText,
                Text::new(""),
                TextFont {
                    font_size: FontSize::Px(24.0),
                    ..default()
                },
                TextColor(Color::srgb_u8(0xd6, 0x45, 0x3c)),
            ));
            d.spawn((
                DeathBestText,
                Text::new(""),
                TextFont {
                    font_size: FontSize::Px(18.0),
                    ..default()
                },
                TextColor(Color::srgb_u8(0xe8, 0xd2, 0x8a)),
            ));
            for (idx, lbl) in ["TRY AGAIN", "TITLE"].iter().enumerate() {
                #[allow(deprecated)]
                d.spawn((
                    DeathBtn(idx),
                    Interaction::default(),
                    Node {
                        padding: UiRect::axes(Val::Px(24.0), Val::Px(10.0)),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.12, 0.08, 0.08, 0.85)),
                ))
                .with_child((
                    Text::new(*lbl),
                    TextFont {
                        font_size: FontSize::Px(26.0),
                        ..default()
                    },
                    TextColor(Color::srgb_u8(0xcd, 0xd2, 0xd8)),
                ));
            }
        });
}

#[derive(SystemParam)]
pub struct RunEntities<'w, 's> {
    zombies: Query<'w, 's, Entity, With<Zombie>>,
    bullets: Query<'w, 's, Entity, With<Bullet>>,
    drops: Query<'w, 's, Entity, With<DropItem>>,
    debris: Query<'w, 's, Entity, With<Debris>>,
    blood: Query<'w, 's, Entity, With<BloodParticle>>,
}

#[derive(SystemParam)]
pub struct ResetContext<'w> {
    pub rig: Res<'w, PlayerRig>,
    pub rig_state: ResMut<'w, PlayerRigState>,
    pub stats: ResMut<'w, GameStats>,
    pub weather: ResMut<'w, WeatherState>,
    pub openings: ResMut<'w, Openings>,
    pub banner: ResMut<'w, BannerState>,
    pub flash: ResMut<'w, DamageFlash>,
    pub toast: ResMut<'w, ToastState>,
    pub audio: Res<'w, AudioEngine>,
}

pub fn clear_run_entities(
    commands: &mut Commands,
    run_entities: &RunEntities,
    ctx: &mut ResetContext,
    transforms: &mut Query<&mut Transform>,
    mats: &mut Query<&mut MeshMaterial3d<StandardMaterial>>,
    visibilities: &mut Query<&mut Visibility>,
) {
    for e in &run_entities.zombies {
        commands.entity(e).despawn();
    }
    for e in &run_entities.bullets {
        commands.entity(e).despawn();
    }
    for e in &run_entities.drops {
        commands.entity(e).despawn();
    }
    for e in &run_entities.debris {
        commands.entity(e).despawn();
    }
    for e in &run_entities.blood {
        commands.entity(e).despawn();
    }
    ctx.openings.reset(mats, visibilities);
    *ctx.stats = GameStats::default();
    *ctx.rig_state = PlayerRigState::default();
    ctx.flash.alpha = 0.0;
    ctx.toast.timer = 0.0;
    ctx.banner.timer = 0.0;
    ctx.weather.set(0.0);
    if let Ok(mut tf) = transforms.get_mut(ctx.rig.root) {
        tf.translation = Vec3::ZERO;
        tf.rotation = Quat::IDENTITY;
    }
}

#[derive(SystemParam)]
#[allow(deprecated)]
pub struct TitleUiQueries<'w, 's> {
    fade: Query<'w, 's, &'static mut BackgroundColor, With<FadeInOverlay>>,
    root: Query<'w, 's, &'static mut Visibility, With<TitleUiRoot>>,
    splash: Query<'w, 's, (&'static mut Visibility, &'static mut TextColor), (With<SplashPrompt>, Without<TitleUiRoot>, Without<MenuContainer>, Without<HowPanelRoot>, Without<SettingsPanelRoot>)>,
    menu_box: Query<'w, 's, &'static mut Visibility, (With<MenuContainer>, Without<TitleUiRoot>, Without<SplashPrompt>, Without<HowPanelRoot>, Without<SettingsPanelRoot>)>,
    how_panel: Query<'w, 's, &'static mut Visibility, (With<HowPanelRoot>, Without<TitleUiRoot>, Without<SplashPrompt>, Without<MenuContainer>, Without<SettingsPanelRoot>)>,
    settings_panel: Query<'w, 's, &'static mut Visibility, (With<SettingsPanelRoot>, Without<TitleUiRoot>, Without<SplashPrompt>, Without<MenuContainer>, Without<HowPanelRoot>)>,
    #[allow(deprecated)]
    btns: Query<'w, 's, (&'static MenuItemBtn, &'static Interaction, &'static mut BorderColor, &'static Children)>,
    texts: Query<'w, 's, &'static mut TextColor, Without<SplashPrompt>>,
    best_txt: Query<'w, 's, &'static mut Text, (With<MenuBestText>, Without<ShakeToggleText>)>,
    setting_rows: Query<'w, 's, (&'static SettingRow, &'static mut BorderColor), Without<MenuItemBtn>>,
    pips: Query<'w, 's, (&'static MeterPip, &'static mut BackgroundColor), Without<FadeInOverlay>>,
    shake_txt: Query<'w, 's, &'static mut Text, (With<ShakeToggleText>, Without<MenuBestText>)>,
}

pub fn update_title_ui(
    mut commands: Commands,
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    state: Res<State<GameState>>,
    mut next_state: ResMut<NextState<GameState>>,
    mut ui_state: ResMut<TitleUiState>,
    mut settings: ResMut<GameSettings>,
    grain: Res<GrainImage>,
    mut images: ResMut<Assets<Image>>,
    run_entities: RunEntities,
    mut ctx: ResetContext,
    mut transforms: Query<&mut Transform>,
    mut mats: Query<&mut MeshMaterial3d<StandardMaterial>>,
    mut p: ParamSet<(TitleUiQueries, Query<&mut Visibility>)>,
) {
    let dt = time.delta_secs();
    let elapsed = time.elapsed_secs();
    let mut start_run = false;

    {
        let mut q = p.p0();

    // Startup fade-in
    if ui_state.fade_timer > 0.0 {
        ui_state.fade_timer = (ui_state.fade_timer - dt).max(0.0);
        let a = (ui_state.fade_timer / 1.1).clamp(0.0, 1.0);
        for mut bg in &mut q.fade {
            bg.0 = Color::srgba(0.0, 0.0, 0.0, a);
        }
    }

    // Animated film grain (~16 fps)
    ui_state.grain_timer += dt;
    if ui_state.grain_timer >= 0.06 {
        ui_state.grain_timer = 0.0;
        if let Some(mut img) = images.get_mut(&grain.0) {
            if let Some(ref mut data) = img.data {
                let mut rng = rand::rng();
                for chunk in data.chunks_exact_mut(4) {
                    let v: u8 = rng.random_range(0..=255);
                    chunk[0] = v;
                    chunk[1] = v;
                    chunk[2] = v;
                }
            }
        }
    }

    let on_title = *state.get() == GameState::Title;
    for mut vis in &mut q.root {
        *vis = if on_title {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }
    if !on_title {
        return;
    }

    // Best run footer
    for mut txt in &mut q.best_txt {
        txt.0 = if settings.best_wave > 0 {
            format!("BEST RUN:  NIGHT {}  /  {} KILLS", settings.best_wave, settings.best_kills)
        } else {
            String::new()
        };
    }

    if ui_state.splash {
        let a = 0.28 + 0.62 * (0.5 + 0.5 * (elapsed * 2.8).sin());
        for (mut vis, mut col) in &mut q.splash {
            *vis = Visibility::Inherited;
            col.0 = Color::srgba(0.8, 0.82, 0.85, a);
        }
        for mut vis in &mut q.menu_box {
            *vis = Visibility::Hidden;
        }
        if keys.get_just_pressed().next().is_some() || mouse.just_pressed(MouseButton::Left) {
            ui_state.splash = false;
            ctx.audio.ui(UiSound::Select);
        }
        return;
    }

    for (mut vis, _) in &mut q.splash {
        *vis = Visibility::Hidden;
    }
    for mut vis in &mut q.menu_box {
        *vis = Visibility::Inherited;
    }

    let up = keys.just_pressed(KeyCode::ArrowUp) || keys.just_pressed(KeyCode::KeyW);
    let down = keys.just_pressed(KeyCode::ArrowDown) || keys.just_pressed(KeyCode::KeyS);
    let left = keys.just_pressed(KeyCode::ArrowLeft) || keys.just_pressed(KeyCode::KeyA);
    let right = keys.just_pressed(KeyCode::ArrowRight) || keys.just_pressed(KeyCode::KeyD);
    let ok = keys.just_pressed(KeyCode::Enter) || keys.just_pressed(KeyCode::Space);
    let back = keys.just_pressed(KeyCode::Escape) || keys.just_pressed(KeyCode::Backspace);

    let mut activate: Option<usize> = None;
    if ui_state.panel == TitlePanel::Settings {
        if up {
            ui_state.setting_row = (ui_state.setting_row + 3) % 4;
            ctx.audio.ui(UiSound::Move);
        } else if down {
            ui_state.setting_row = (ui_state.setting_row + 1) % 4;
            ctx.audio.ui(UiSound::Move);
        } else if left || right || (ok && ui_state.setting_row == 3) {
            let dir: i8 = if left { -1 } else { 1 };
            match ui_state.setting_row {
                0 => settings.master = (settings.master as i8 + dir).clamp(0, 10) as u8,
                1 => settings.sfx = (settings.sfx as i8 + dir).clamp(0, 10) as u8,
                2 => settings.music = (settings.music as i8 + dir).clamp(0, 10) as u8,
                _ => settings.shake = !settings.shake,
            }
            save_storage(&settings);
            ctx.audio.apply_volume(settings.master, settings.sfx, settings.music);
            ctx.audio.ui(UiSound::Move);
        } else if back {
            ui_state.panel = TitlePanel::None;
            ctx.audio.ui(UiSound::Back);
        }
    } else if ui_state.panel == TitlePanel::How {
        if back || ok {
            ui_state.panel = TitlePanel::None;
            ctx.audio.ui(UiSound::Back);
        }
    } else {
        if up {
            ui_state.sel = (ui_state.sel + 2) % 3;
            ctx.audio.ui(UiSound::Move);
        } else if down {
            ui_state.sel = (ui_state.sel + 1) % 3;
            ctx.audio.ui(UiSound::Move);
        } else if ok {
            activate = Some(ui_state.sel);
        }
    }

    for (btn, inter, mut border, children) in &mut q.btns {
        #[allow(deprecated)]
        if *inter == Interaction::Hovered && ui_state.sel != btn.0 {
            ui_state.sel = btn.0;
            ctx.audio.ui(UiSound::Move);
        }
        #[allow(deprecated)]
        if *inter == Interaction::Pressed {
            activate = Some(btn.0);
        }
        let selected = ui_state.sel == btn.0;
        *border = BorderColor::all(if selected {
            Color::srgb_u8(0xd6, 0x45, 0x3c)
        } else {
            Color::NONE
        });
        for &child in children {
            if let Ok(mut tc) = q.texts.get_mut(child) {
                tc.0 = if selected {
                    Color::srgb_u8(0xec, 0xe6, 0xdb)
                } else {
                    Color::srgb_u8(0x6c, 0x72, 0x7b)
                };
            }
        }
    }

    if let Some(act) = activate {
        match act {
            0 => {
                // Start Run!
                ui_state.panel = TitlePanel::None;
                start_run = true;
            }
            1 => {
                ui_state.panel = if ui_state.panel == TitlePanel::How {
                    ctx.audio.ui(UiSound::Back);
                    TitlePanel::None
                } else {
                    ctx.audio.ui(UiSound::Select);
                    TitlePanel::How
                };
            }
            _ => {
                ui_state.panel = if ui_state.panel == TitlePanel::Settings {
                    ctx.audio.ui(UiSound::Back);
                    TitlePanel::None
                } else {
                    ctx.audio.ui(UiSound::Select);
                    TitlePanel::Settings
                };
            }
        }
    }

    for mut vis in &mut q.how_panel {
        *vis = if ui_state.panel == TitlePanel::How {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }
    for mut vis in &mut q.settings_panel {
        *vis = if ui_state.panel == TitlePanel::Settings {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }

    for (row, mut border) in &mut q.setting_rows {
        *border = BorderColor::all(if ui_state.setting_row == row.0 {
            Color::srgb_u8(0xd6, 0x45, 0x3c)
        } else {
            Color::NONE
        });
    }
    for (pip, mut bg) in &mut q.pips {
        let val = match pip.row {
            0 => settings.master,
            1 => settings.sfx,
            _ => settings.music,
        } as usize;
        bg.0 = if pip.idx < val {
            Color::srgb_u8(0xd6, 0x45, 0x3c)
        } else {
            Color::srgb_u8(0x24, 0x26, 0x2c)
        };
    }
    for mut txt in &mut q.shake_txt {
        txt.0 = if settings.shake { "ON".into() } else { "OFF".into() };
    }
    }

    if start_run {
        clear_run_entities(
            &mut commands,
            &run_entities,
            &mut ctx,
            &mut transforms,
            &mut mats,
            &mut p.p1(),
        );
        crate::game::start_wave(&mut ctx.stats, &mut ctx.weather, &ctx.audio, &mut ctx.banner);
        next_state.set(GameState::Playing);
    }
}

#[derive(SystemParam)]
#[allow(deprecated)]
pub struct OverlayQueries<'w, 's> {
    pause_root: Query<'w, 's, &'static mut Visibility, (With<PauseOverlay>, Without<DeathOverlay>)>,
    pause_stat: Query<'w, 's, &'static mut Text, (With<PauseStatText>, Without<DeathStatText>, Without<DeathBestText>)>,
    #[allow(deprecated)]
    pause_btns: Query<'w, 's, (&'static PauseBtn, &'static Interaction), Changed<Interaction>>,
    death_root: Query<'w, 's, &'static mut Visibility, (With<DeathOverlay>, Without<PauseOverlay>)>,
    death_stat: Query<'w, 's, &'static mut Text, (With<DeathStatText>, Without<PauseStatText>, Without<DeathBestText>)>,
    death_best: Query<'w, 's, &'static mut Text, (With<DeathBestText>, Without<PauseStatText>, Without<DeathStatText>)>,
    #[allow(deprecated)]
    death_btns: Query<'w, 's, (&'static DeathBtn, &'static Interaction), Changed<Interaction>>,
}

pub fn update_overlays(
    mut commands: Commands,
    state: Res<State<GameState>>,
    mut next_state: ResMut<NextState<GameState>>,
    mut settings: ResMut<GameSettings>,
    run_entities: RunEntities,
    mut ctx: ResetContext,
    mut transforms: Query<&mut Transform>,
    mut mats: Query<&mut MeshMaterial3d<StandardMaterial>>,
    mut p: ParamSet<(OverlayQueries, Query<&mut Visibility>)>,
) {
    let paused = *state.get() == GameState::Paused;
    let dead = *state.get() == GameState::Dead;
    let mut reset_to: Option<GameState> = None;
    let mut start_new_wave = false;

    {
        let mut q = p.p0();
        for mut vis in &mut q.pause_root {
            *vis = if paused {
                Visibility::Inherited
            } else {
                Visibility::Hidden
            };
        }
        for mut vis in &mut q.death_root {
            *vis = if dead {
                Visibility::Inherited
            } else {
                Visibility::Hidden
            };
        }

        if paused {
            for mut txt in &mut q.pause_stat {
                txt.0 = format!("NIGHT {}  •  {} KILLS", ctx.stats.wave, ctx.stats.kills);
            }
            for (btn, inter) in &q.pause_btns {
                if *inter == Interaction::Pressed {
                    ctx.audio.ui(UiSound::Select);
                    if btn.0 == 0 {
                        next_state.set(GameState::Playing);
                    } else {
                        reset_to = Some(GameState::Title);
                    }
                }
            }
        }

        if dead {
            let is_record = ctx.stats.wave > settings.best_wave
                || (ctx.stats.wave == settings.best_wave && ctx.stats.kills > settings.best_kills);
            if is_record {
                settings.best_wave = ctx.stats.wave;
                settings.best_kills = ctx.stats.kills;
                save_storage(&settings);
            }
            for mut txt in &mut q.death_stat {
                txt.0 = format!("NIGHT {}  •  {} KILLS", ctx.stats.wave, ctx.stats.kills);
            }
            for mut txt in &mut q.death_best {
                txt.0 = if is_record {
                    "NEW RECORD".to_string()
                } else if settings.best_wave > 0 {
                    format!("BEST: NIGHT {}  •  {} KILLS", settings.best_wave, settings.best_kills)
                } else {
                    String::new()
                };
            }
            for (btn, inter) in &q.death_btns {
                if *inter == Interaction::Pressed {
                    ctx.audio.ui(UiSound::Select);
                    if btn.0 == 0 {
                        start_new_wave = true;
                        reset_to = Some(GameState::Playing);
                    } else {
                        reset_to = Some(GameState::Title);
                    }
                }
            }
        }
    }

    if let Some(target_state) = reset_to {
        clear_run_entities(
            &mut commands,
            &run_entities,
            &mut ctx,
            &mut transforms,
            &mut mats,
            &mut p.p1(),
        );
        if start_new_wave {
            crate::game::start_wave(&mut ctx.stats, &mut ctx.weather, &ctx.audio, &mut ctx.banner);
        }
        next_state.set(target_state);
    }
}

pub fn update_camera_with_title(
    time: Res<Time>,
    state: Res<State<GameState>>,
    rig: Res<PlayerRig>,
    settings: Res<GameSettings>,
    mut stats: ResMut<GameStats>,
    mut cam_tr: ResMut<CameraTransition>,
    mut cam_q: Query<(&mut Transform, &mut Projection)>,
    player_q: Query<&Transform, Without<Projection>>,
) {
    let dt = time.delta_secs();
    let on_title = *state.get() == GameState::Title;
    cam_tr.title_t += dt;
    let target_mix = if on_title { 0.0 } else { 1.0 };
    let step = dt / 1.8;
    cam_tr.mix = (cam_tr.mix + (target_mix - cam_tr.mix).clamp(-step, step)).clamp(0.0, 1.0);

    // Smoothstep cubic swoop
    let m = cam_tr.mix;
    let e = if m < 0.5 {
        4.0 * m * m * m
    } else {
        1.0 - (-2.0 * m + 2.0).powi(3) / 2.0
    };

    stats.shake = (stats.shake - dt * 1.4).max(0.0);
    let p_pos = player_q
        .get(rig.root)
        .map(|t| t.translation)
        .unwrap_or(Vec3::ZERO);
    let mut follow_target = p_pos + Vec3::new(0.0, 30.0, 13.0);
    if settings.shake && stats.shake > 0.0 {
        let mut rng = rand::rng();
        let s = stats.shake * stats.shake * 0.45;
        follow_target += Vec3::new(
            (rng.random::<f32>() - 0.5) * 2.0 * s,
            0.0,
            (rng.random::<f32>() - 0.5) * 2.0 * s,
        );
    }
    if cam_tr.mix == 0.0 {
        cam_tr.follow_pos = follow_target;
    } else {
        let alpha = 1.0 - 0.001_f32.powf(dt);
        cam_tr.follow_pos = cam_tr.follow_pos.lerp(follow_target, alpha);
    }

    let t = cam_tr.title_t;
    let title_pos = Vec3::new(
        2.0 + 1.4 * (t * 0.045).sin(),
        2.0 + 0.2 * (t * 0.07).sin() + 0.03 * (t * 1.3).sin(),
        24.0 - 1.2 * (t * 0.031).sin(),
    );
    let title_look = Vec3::new(
        -1.5 + 0.7 * (t * 0.045 + 0.6).sin(),
        2.2 + 0.02 * (t * 1.1).sin(),
        8.0,
    );

    let pos = title_pos.lerp(cam_tr.follow_pos, e);
    let look = title_look.lerp(p_pos, e);
    let fov = (42.0 + (55.0 - 42.0) * e).to_radians();

    for (mut tf, mut proj) in &mut cam_q {
        tf.translation = pos;
        tf.look_at(look, Vec3::Y);
        if let Projection::Perspective(ref mut p) = *proj {
            p.fov = fov;
        }
    }
}
