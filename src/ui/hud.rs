use bevy::asset::RenderAssetUsages;
use bevy::ecs::system::SystemParam;
use bevy::image::{ImageSampler, ImageSamplerDescriptor};
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

use crate::audio::AudioEngine;
use crate::core::state::{BannerState, DamageFlash, GameState, GameStats, ToastState};
use crate::entities::zombies::{Corpse, Zombie};

pub const HEART_ICON: [&str; 8] = [
    "........",
    ".##..##.",
    "########",
    "########",
    "########",
    ".######.",
    "..####..",
    "...##...",
];

pub fn icon_image(bitmap: &[&str; 8], color: Color) -> Image {
    let rgba = color.to_srgba().to_u8_array();
    let dim = [rgba[0], rgba[1], rgba[2], (rgba[3] as f32 * 0.45) as u8];
    let mut data = Vec::with_capacity(8 * 8 * 4);
    for row in bitmap {
        for ch in row.chars().take(8) {
            match ch {
                '#' | 'X' => data.extend_from_slice(&rgba),
                'o' => data.extend_from_slice(&dim),
                _ => data.extend_from_slice(&[0, 0, 0, 0]),
            }
        }
    }
    let mut img = Image::new(
        Extent3d {
            width: 8,
            height: 8,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    );
    img.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor::nearest());
    img
}

#[derive(Resource)]
pub struct HudIcons {
    pub heart_full: Handle<Image>,
    pub heart_empty: Handle<Image>,
}

#[derive(Component)]
pub struct HudRoot;

#[derive(Component)]
pub struct FlashOverlay;

#[derive(Component)]
pub struct WaveText;

#[derive(Component)]
pub struct KillsText;

#[derive(Component)]
pub struct LeftText;

#[derive(Component)]
pub struct MutedText;

#[derive(Component)]
pub struct HeartPip(pub usize);

#[derive(Component)]
pub struct PowerupsText;

#[derive(Component)]
pub struct MagPip(pub usize);

#[derive(Component)]
pub struct ReloadBar;

#[derive(Component)]
pub struct ReloadBarFill;

#[derive(Component)]
pub struct ReloadHint;

#[derive(Component)]
pub struct BannerRoot;

#[derive(Component)]
pub struct BannerTitleText;

#[derive(Component)]
pub struct BannerSubText;

#[derive(Component)]
pub struct ToastText;

pub fn setup_hud(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    let heart_full = images.add(icon_image(&HEART_ICON, Color::srgb_u8(0xd6, 0x45, 0x3c)));
    let heart_empty = images.add(icon_image(&HEART_ICON, Color::srgb_u8(0x2a, 0x2d, 0x33)));
    commands.insert_resource(HudIcons {
        heart_full: heart_full.clone(),
        heart_empty,
    });

    // Full-screen red damage flash
    commands.spawn((
        FlashOverlay,
        Pickable::IGNORE,
        GlobalZIndex(20),
        Node {
            position_type: PositionType::Absolute,
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            ..default()
        },
        BackgroundColor(Color::srgba(1.0, 0.12, 0.12, 0.0)),
    ));

    // Gameplay HUD container
    commands
        .spawn((
            HudRoot,
            Pickable::IGNORE,
            GlobalZIndex(5),
            Visibility::Hidden,
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::SpaceBetween,
                padding: UiRect::new(Val::Px(18.0), Val::Px(18.0), Val::Px(14.0), Val::Px(20.0)),
                ..default()
            },
        ))
        .with_children(|root| {
            // Top bar
            root.spawn((
                Pickable::IGNORE,
                Node {
                    width: Val::Percent(100.0),
                    justify_content: JustifyContent::SpaceBetween,
                    align_items: AlignItems::FlexStart,
                    ..default()
                },
            ))
            .with_children(|top| {
                // Top-left: Night + Hearts + Powerups
                top.spawn((
                    Pickable::IGNORE,
                    Node {
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(6.0),
                        ..default()
                    },
                ))
                .with_children(|left| {
                    left.spawn((
                        WaveText,
                        Text::new("WAVE 1"),
                        TextFont {
                            font_size: FontSize::Px(22.0),
                            ..default()
                        },
                        TextColor(Color::srgb_u8(0xd6, 0x45, 0x3c)),
                        TextShadow {
                            offset: Vec2::new(0.0, 2.0),
                            color: Color::BLACK,
                        },
                    ));
                    left.spawn((
                        Pickable::IGNORE,
                        Node {
                            flex_direction: FlexDirection::Row,
                            column_gap: Val::Px(5.0),
                            ..default()
                        },
                    ))
                    .with_children(|hearts| {
                        for i in 0..10 {
                            hearts.spawn((
                                HeartPip(i),
                                ImageNode::new(heart_full.clone()),
                                Node {
                                    width: Val::Px(22.0),
                                    height: Val::Px(22.0),
                                    ..default()
                                },
                            ));
                        }
                    });
                    left.spawn((
                        PowerupsText,
                        Text::new(""),
                        TextFont {
                            font_size: FontSize::Px(16.0),
                            ..default()
                        },
                        TextColor(Color::srgb_u8(0xe8, 0xd2, 0x8a)),
                    ));
                });

                // Top-right: Kills, Left, Muted
                top.spawn((
                    Pickable::IGNORE,
                    Node {
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::FlexEnd,
                        row_gap: Val::Px(4.0),
                        ..default()
                    },
                ))
                .with_children(|right| {
                    right.spawn((
                        KillsText,
                        Text::new("KILLS 0"),
                        TextFont {
                            font_size: FontSize::Px(20.0),
                            ..default()
                        },
                        TextColor(Color::srgb_u8(0x8f, 0xae, 0x6b)),
                        TextShadow {
                            offset: Vec2::new(0.0, 2.0),
                            color: Color::BLACK,
                        },
                    ));
                    right.spawn((
                        LeftText,
                        Text::new("0 LEFT"),
                        TextFont {
                            font_size: FontSize::Px(17.0),
                            ..default()
                        },
                        TextColor(Color::srgb_u8(0xcd, 0xd2, 0xd8)),
                        TextShadow {
                            offset: Vec2::new(0.0, 2.0),
                            color: Color::BLACK,
                        },
                    ));
                    right.spawn((
                        MutedText,
                        Text::new(""),
                        TextFont {
                            font_size: FontSize::Px(15.0),
                            ..default()
                        },
                        TextColor(Color::srgb_u8(0xd6, 0x45, 0x3c)),
                    ));
                });
            });

            // Bottom-center: Magazine pips + reload bar + hint
            root.spawn((
                Pickable::IGNORE,
                Node {
                    width: Val::Percent(100.0),
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    row_gap: Val::Px(6.0),
                    ..default()
                },
            ))
            .with_children(|bottom| {
                bottom
                    .spawn((
                        Pickable::IGNORE,
                        Node {
                            flex_direction: FlexDirection::Row,
                            column_gap: Val::Px(4.0),
                            ..default()
                        },
                    ))
                    .with_children(|mag| {
                        for i in 0..20 {
                            mag.spawn((
                                MagPip(i),
                                Node {
                                    width: Val::Px(9.0),
                                    height: Val::Px(18.0),
                                    ..default()
                                },
                                BackgroundColor(Color::srgb_u8(0xe8, 0xd2, 0x8a)),
                            ));
                        }
                    });
                bottom
                    .spawn((
                        ReloadBar,
                        Visibility::Hidden,
                        Node {
                            width: Val::Px(180.0),
                            height: Val::Px(6.0),
                            border: UiRect::all(Val::Px(1.0)),
                            ..default()
                        },
                        BackgroundColor(Color::srgb_u8(0x1a, 0x1c, 0x22)),
                        BorderColor::all(Color::srgb_u8(0x2c, 0x2f, 0x37)),
                    ))
                    .with_child((
                        ReloadBarFill,
                        Node {
                            width: Val::Percent(0.0),
                            height: Val::Percent(100.0),
                            ..default()
                        },
                        BackgroundColor(Color::srgb_u8(0xe8, 0xd2, 0x8a)),
                    ));
                bottom.spawn((
                    ReloadHint,
                    Text::new("[R] RELOAD"),
                    Visibility::Hidden,
                    TextFont {
                        font_size: FontSize::Px(14.0),
                        ..default()
                    },
                    TextColor(Color::srgb_u8(0x6a, 0x70, 0x78)),
                ));
            });
        });

    // Center Banner + Pickup Toast
    commands
        .spawn((
            BannerRoot,
            Pickable::IGNORE,
            GlobalZIndex(8),
            Node {
                position_type: PositionType::Absolute,
                top: Val::Percent(34.0),
                left: Val::Px(0.0),
                width: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                row_gap: Val::Px(8.0),
                ..default()
            },
        ))
        .with_children(|b| {
            b.spawn((
                BannerTitleText,
                Text::new(""),
                TextFont {
                    font_size: FontSize::Px(64.0),
                    ..default()
                },
                TextColor(Color::srgba(0.84, 0.27, 0.24, 0.0)),
                TextShadow {
                    offset: Vec2::new(0.0, 4.0),
                    color: Color::BLACK,
                },
            ));
            b.spawn((
                BannerSubText,
                Text::new(""),
                TextFont {
                    font_size: FontSize::Px(20.0),
                    ..default()
                },
                TextColor(Color::srgba(0.54, 0.56, 0.60, 0.0)),
                TextShadow {
                    offset: Vec2::new(0.0, 2.0),
                    color: Color::BLACK,
                },
            ));
        });

    commands
        .spawn((
            Pickable::IGNORE,
            GlobalZIndex(8),
            Node {
                position_type: PositionType::Absolute,
                top: Val::Percent(22.0),
                left: Val::Px(0.0),
                width: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                ..default()
            },
        ))
        .with_child((
            ToastText,
            Text::new(""),
            TextFont {
                font_size: FontSize::Px(28.0),
                ..default()
            },
            TextColor(Color::srgba(1.0, 1.0, 1.0, 0.0)),
            TextShadow {
                offset: Vec2::new(0.0, 3.0),
                color: Color::BLACK,
            },
        ));
}

#[derive(SystemParam)]
pub struct HudQueries<'w, 's> {
    hud_root: Query<'w, 's, &'static mut Visibility, With<HudRoot>>,
    flash: Query<'w, 's, &'static mut BackgroundColor, (With<FlashOverlay>, Without<MagPip>, Without<ReloadBarFill>)>,
    wave: Query<'w, 's, &'static mut Text, (With<WaveText>, Without<KillsText>, Without<LeftText>, Without<MutedText>, Without<PowerupsText>, Without<BannerTitleText>, Without<BannerSubText>, Without<ToastText>)>,
    kills: Query<'w, 's, &'static mut Text, (With<KillsText>, Without<WaveText>, Without<LeftText>, Without<MutedText>, Without<PowerupsText>, Without<BannerTitleText>, Without<BannerSubText>, Without<ToastText>)>,
    left: Query<'w, 's, &'static mut Text, (With<LeftText>, Without<WaveText>, Without<KillsText>, Without<MutedText>, Without<PowerupsText>, Without<BannerTitleText>, Without<BannerSubText>, Without<ToastText>)>,
    muted: Query<'w, 's, &'static mut Text, (With<MutedText>, Without<WaveText>, Without<KillsText>, Without<LeftText>, Without<PowerupsText>, Without<BannerTitleText>, Without<BannerSubText>, Without<ToastText>)>,
    powerups: Query<'w, 's, &'static mut Text, (With<PowerupsText>, Without<WaveText>, Without<KillsText>, Without<LeftText>, Without<MutedText>, Without<BannerTitleText>, Without<BannerSubText>, Without<ToastText>)>,
    hearts: Query<'w, 's, (&'static HeartPip, &'static mut ImageNode, &'static mut Visibility), (Without<HudRoot>, Without<MagPip>, Without<ReloadBar>, Without<ReloadHint>)>,
    mag_pips: Query<'w, 's, (&'static MagPip, &'static mut BackgroundColor, &'static mut Visibility), (Without<HudRoot>, Without<FlashOverlay>, Without<HeartPip>, Without<ReloadBar>, Without<ReloadBarFill>, Without<ReloadHint>)>,
    reload_bar: Query<'w, 's, &'static mut Visibility, (With<ReloadBar>, Without<HudRoot>, Without<HeartPip>, Without<MagPip>, Without<ReloadHint>)>,
    reload_fill: Query<'w, 's, &'static mut Node, With<ReloadBarFill>>,
    reload_hint: Query<'w, 's, &'static mut Visibility, (With<ReloadHint>, Without<HudRoot>, Without<HeartPip>, Without<MagPip>, Without<ReloadBar>)>,
    banner_title: Query<'w, 's, (&'static mut Text, &'static mut TextColor), (With<BannerTitleText>, Without<WaveText>, Without<KillsText>, Without<LeftText>, Without<MutedText>, Without<PowerupsText>, Without<BannerSubText>, Without<ToastText>)>,
    banner_sub: Query<'w, 's, (&'static mut Text, &'static mut TextColor), (With<BannerSubText>, Without<WaveText>, Without<KillsText>, Without<LeftText>, Without<MutedText>, Without<PowerupsText>, Without<BannerTitleText>, Without<ToastText>)>,
    toast: Query<'w, 's, (&'static mut Text, &'static mut TextColor), (With<ToastText>, Without<WaveText>, Without<KillsText>, Without<LeftText>, Without<MutedText>, Without<PowerupsText>, Without<BannerTitleText>, Without<BannerSubText>)>,
}

pub fn update_hud(
    time: Res<Time>,
    state: Res<State<GameState>>,
    stats: Res<GameStats>,
    icons: Res<HudIcons>,
    audio: Res<AudioEngine>,
    mut flash: ResMut<DamageFlash>,
    mut banner: ResMut<BannerState>,
    mut toast: ResMut<ToastState>,
    live_zombies: Query<Entity, (With<Zombie>, Without<Corpse>)>,
    mut q: HudQueries,
) {
    let dt = time.delta_secs();
    let show_hud = *state.get() != GameState::Title;
    for mut vis in &mut q.hud_root {
        *vis = if show_hud {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }

    // Damage flash decay
    if flash.alpha > 0.0 {
        flash.alpha = (flash.alpha - dt * 2.5).max(0.0);
    }
    for mut bg in &mut q.flash {
        bg.0 = Color::srgba(1.0, 0.12, 0.12, flash.alpha * 0.55);
    }

    // Banner & Toast decay
    if banner.timer > 0.0 {
        banner.timer = (banner.timer - dt).max(0.0);
    }
    if toast.timer > 0.0 {
        toast.timer = (toast.timer - dt).max(0.0);
    }
    let b_alpha = (banner.timer * 2.0).min(1.0);
    for (mut txt, mut col) in &mut q.banner_title {
        txt.0 = banner.title.clone();
        col.0 = Color::srgba(0.84, 0.27, 0.24, b_alpha);
    }
    for (mut txt, mut col) in &mut q.banner_sub {
        txt.0 = banner.sub.clone();
        col.0 = Color::srgba(0.54, 0.56, 0.60, b_alpha);
    }
    let t_alpha = (toast.timer * 2.0).min(1.0);
    for (mut txt, mut col) in &mut q.toast {
        txt.0 = toast.text.clone();
        let lin = toast.color.to_srgba();
        col.0 = Color::srgba(lin.red, lin.green, lin.blue, t_alpha);
    }

    if !show_hud {
        return;
    }

    for mut txt in &mut q.wave {
        txt.0 = format!("WAVE {}", stats.wave);
    }
    for mut txt in &mut q.kills {
        txt.0 = format!("KILLS {}", stats.kills);
    }
    let remaining = live_zombies.iter().count() + stats.to_spawn as usize;
    for mut txt in &mut q.left {
        txt.0 = if stats.intermission > 0.0 || *state.get() == GameState::ChoosingUpgrade {
            String::new()
        } else {
            format!("{remaining} LEFT")
        };
    }
    for mut txt in &mut q.muted {
        txt.0 = if audio.muted {
            "[M] MUTED".to_string()
        } else {
            String::new()
        };
    }

    // Hearts
    let max_hp = stats.max_hp.clamp(0, 10) as usize;
    for (pip, mut img, mut vis) in &mut q.hearts {
        if pip.0 < max_hp {
            *vis = Visibility::Inherited;
            img.image = if (pip.0 as i32) < stats.hp {
                icons.heart_full.clone()
            } else {
                icons.heart_empty.clone()
            };
        } else {
            *vis = Visibility::Hidden;
        }
    }

    // Powerups
    let clock = |t: f32| -> String {
        let s = t.ceil() as i32;
        format!("{}:{:02}", s / 60, s % 60)
    };
    let mut pu_lines = Vec::new();
    if stats.p_damage > 0.0 {
        pu_lines.push(format!("2X DMG {}", clock(stats.p_damage)));
    }
    if stats.p_ammo > 0.0 {
        pu_lines.push(format!("INF AMMO {}", clock(stats.p_ammo)));
    }
    for mut txt in &mut q.powerups {
        txt.0 = pu_lines.join("\n");
    }

    // Magazine pips
    let mag_size = stats.mag_size.clamp(0, 20) as usize;
    let live_col = if stats.p_ammo > 0.0 {
        Color::srgb_u8(0xff, 0x7a, 0x2e)
    } else {
        Color::srgb_u8(0xe8, 0xd2, 0x8a)
    };
    let empty_col = Color::srgb_u8(0x4a, 0x30, 0x30);
    for (pip, mut bg, mut vis) in &mut q.mag_pips {
        if pip.0 < mag_size {
            *vis = Visibility::Inherited;
            bg.0 = if (pip.0 as i32) < stats.mag {
                live_col
            } else {
                empty_col
            };
        } else {
            *vis = Visibility::Hidden;
        }
    }

    // Reload bar + hint
    for mut vis in &mut q.reload_bar {
        *vis = if stats.reloading {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }
    for mut node in &mut q.reload_fill {
        let pct = if stats.reload_time > 0.0 {
            (stats.reload_t / stats.reload_time * 100.0).clamp(0.0, 100.0)
        } else {
            0.0
        };
        node.width = Val::Percent(pct);
    }
    for mut vis in &mut q.reload_hint {
        *vis = if !stats.reloading && stats.mag <= (stats.mag_size / 3).max(1) {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }
}
