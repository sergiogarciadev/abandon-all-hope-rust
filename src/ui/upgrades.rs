use bevy::prelude::*;
use rand::seq::SliceRandom;

use crate::audio::{AudioEngine, UiSound};
use crate::core::state::{BannerState, GameState, GameStats};
use crate::entities::drops::DROP_CHANCE;
use crate::ui::hud::icon_image;
use crate::world::weather::WeatherState;

#[derive(Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum UpgradeCat {
    Firepower,
    Handling,
    Survival,
    Fortune,
}

impl UpgradeCat {
    #[allow(dead_code)]
    pub fn color(self) -> Color {
        match self {
            UpgradeCat::Firepower => Color::srgb_u8(0xd6, 0x45, 0x3c),
            UpgradeCat::Handling => Color::srgb_u8(0xe8, 0xd2, 0x8a),
            UpgradeCat::Survival => Color::srgb_u8(0x8f, 0xae, 0x6b),
            UpgradeCat::Fortune => Color::srgb_u8(0x6f, 0xa8, 0xdc),
        }
    }

    #[allow(dead_code)]
    pub fn label(self) -> &'static str {
        match self {
            UpgradeCat::Firepower => "FIREPOWER",
            UpgradeCat::Handling => "HANDLING",
            UpgradeCat::Survival => "SURVIVAL",
            UpgradeCat::Fortune => "FORTUNE",
        }
    }
}

#[allow(dead_code)]
pub struct UpgradeDef {
    pub id: &'static str,
    pub cat: UpgradeCat,
    pub name: &'static str,
    pub desc: &'static str,
    pub max: u32,
    pub ok: fn(&GameStats) -> bool,
    pub stat: fn(&GameStats) -> (String, String),
    pub apply: fn(&mut GameStats),
}

pub const UPGRADES: [UpgradeDef; 10] = [
    UpgradeDef {
        id: "mag",
        cat: UpgradeCat::Handling,
        name: "EXTENDED MAG",
        desc: "+4 rounds per magazine",
        max: 5,
        ok: |_| true,
        stat: |s| (format!("{}", s.mag_size), format!("{} rds", s.mag_size + 4)),
        apply: |s| {
            s.mag_size += 4;
            s.mag = s.mag_size;
        },
    },
    UpgradeDef {
        id: "reload",
        cat: UpgradeCat::Handling,
        name: "QUICK HANDS",
        desc: "reload 25% faster",
        max: 4,
        ok: |_| true,
        stat: |s| (format!("{:.2}", s.reload_time), format!("{:.2}s", s.reload_time * 0.75)),
        apply: |s| {
            s.reload_time *= 0.75;
        },
    },
    UpgradeDef {
        id: "damage",
        cat: UpgradeCat::Firepower,
        name: "HOLLOW POINTS",
        desc: "+50% bullet damage",
        max: 6,
        ok: |_| true,
        stat: |s| (format!("{:.1}x", s.damage), format!("{:.1}x", s.damage + 0.5)),
        apply: |s| {
            s.damage += 0.5;
        },
    },
    UpgradeDef {
        id: "rate",
        cat: UpgradeCat::Firepower,
        name: "HAIR TRIGGER",
        desc: "fire 20% faster",
        max: 4,
        ok: |_| true,
        stat: |s| (format!("{:.1}", 1.0 / s.fire_rate), format!("{:.1}/s", 1.0 / (s.fire_rate * 0.8))),
        apply: |s| {
            s.fire_rate *= 0.8;
        },
    },
    UpgradeDef {
        id: "pierce",
        cat: UpgradeCat::Firepower,
        name: "PIERCING",
        desc: "shots pierce +1 zombie",
        max: 3,
        ok: |_| true,
        stat: |s| (format!("{}", s.pierce), format!("{} thru", s.pierce + 1)),
        apply: |s| {
            s.pierce += 1;
        },
    },
    UpgradeDef {
        id: "spread",
        cat: UpgradeCat::Firepower,
        name: "BUCKSHOT",
        desc: "+1 bullet per shot",
        max: 3,
        ok: |_| true,
        stat: |s| (format!("{}", 1 + s.spread), format!("{} per shot", 2 + s.spread)),
        apply: |s| {
            s.spread += 1;
        },
    },
    UpgradeDef {
        id: "hp",
        cat: UpgradeCat::Survival,
        name: "THICK SKIN",
        desc: "+1 max health, and heal it",
        max: 3,
        ok: |_| true,
        stat: |s| (format!("{}", s.max_hp), format!("{} max hp", s.max_hp + 1)),
        apply: |s| {
            s.max_hp += 1;
            s.hp += 1;
        },
    },
    UpgradeDef {
        id: "speed",
        cat: UpgradeCat::Survival,
        name: "FLEET FOOT",
        desc: "move 12% faster",
        max: 3,
        ok: |_| true,
        stat: |s| (
            format!("{}%", ((s.speed / 6.2) * 100.0).round() as i32),
            format!("{}%", ((s.speed * 1.12 / 6.2) * 100.0).round() as i32),
        ),
        apply: |s| {
            s.speed *= 1.12;
        },
    },
    UpgradeDef {
        id: "luck",
        cat: UpgradeCat::Fortune,
        name: "SCAVENGER",
        desc: "more supply drops",
        max: 3,
        ok: |_| true,
        stat: |s| (
            format!("{}%", ((DROP_CHANCE + s.drop_bonus) * 100.0).round() as i32),
            format!("{}% drops", ((DROP_CHANCE + s.drop_bonus + 0.03) * 100.0).round() as i32),
        ),
        apply: |s| {
            s.drop_bonus += 0.03;
        },
    },
    UpgradeDef {
        id: "heal",
        cat: UpgradeCat::Survival,
        name: "PATCH UP",
        desc: "restore all health",
        max: u32::MAX,
        ok: |s| s.hp < s.max_hp,
        stat: |s| (format!("{}", s.hp), format!("{} hp", s.max_hp)),
        apply: |s| {
            s.hp = s.max_hp;
        },
    },
];

#[derive(Resource, Default)]
pub struct UpgradePicker {
    pub choices: Vec<usize>,
    pub lockout: f32,
    pub confirm_timer: f32,
    pub picked: Option<usize>,
}

impl UpgradePicker {
    pub fn open(&mut self, stats: &GameStats) {
        let mut eligible: Vec<usize> = UPGRADES
            .iter()
            .enumerate()
            .filter(|(_, u)| {
                let count = stats.upgrades.get(u.id).copied().unwrap_or(0);
                count < u.max && (u.ok)(stats)
            })
            .map(|(i, _)| i)
            .collect();
        let mut rng = rand::rng();
        eligible.shuffle(&mut rng);
        eligible.truncate(3);
        self.choices = eligible;
        self.lockout = 0.35;
        self.confirm_timer = -1.0;
        self.picked = None;
    }

    pub fn pick(&mut self, slot: usize, stats: &mut GameStats, audio: &AudioEngine) {
        if self.lockout > 0.0 || self.picked.is_some() {
            return;
        }
        let Some(&idx) = self.choices.get(slot) else {
            return;
        };
        let u = &UPGRADES[idx];
        (u.apply)(stats);
        *stats.upgrades.entry(u.id.to_string()).or_insert(0) += 1;
        audio.upgrade(u.cat);
        self.picked = Some(slot);
        self.confirm_timer = 0.48;
    }
}

#[derive(Resource)]
pub struct UpgradeIcons {
    pub handles: [Handle<Image>; 10],
}

pub fn upgrade_bitmap(id: &str) -> [&'static str; 8] {
    match id {
        "mag" => [
            "XX.XX.XX",
            "XX.XX.XX",
            "oo.oo.oo",
            "oo.oo.oo",
            "oo.oo.oo",
            "oo.oo.oo",
            "XX.XX.XX",
            "........",
        ],
        "reload" => [
            "..XXXX..",
            ".X....XX",
            "X....XXX",
            "X.......",
            "X......X",
            "X......X",
            ".X....X.",
            "..XXXX..",
        ],
        "rate" => [
            "....XXX.",
            "...XXX..",
            "..XXX...",
            ".XXXXXX.",
            "...XXX..",
            "..XXX...",
            ".XX.....",
            ".X......",
        ],
        "damage" => [
            ".XXXXXX.",
            "XXXXXXXX",
            "X..XX..X",
            "X..XX..X",
            "XXXXXXXX",
            ".XXX.XX.",
            "..XXXX..",
            "..X.X.X.",
        ],
        "pierce" => [
            ".o..o...",
            ".o..o.X.",
            ".o..o..X",
            "XXXXXXXX",
            ".o..o..X",
            ".o..o.X.",
            ".o..o...",
            "........",
        ],
        "spread" => [
            "......XX",
            "....oo..",
            "..oo....",
            "XXooooXX",
            "..oo....",
            "....oo..",
            "......XX",
            "........",
        ],
        "hp" => [
            "XXXXXXXX",
            "XooooooX",
            "XoooXooX",
            "XooXXXoX",
            ".XooXoX.",
            ".XooooX.",
            "..XooX..",
            "...XX...",
        ],
        "speed" => [
            "........",
            "XX..XX..",
            ".XX..XX.",
            "..XX..XX",
            ".XX..XX.",
            "XX..XX..",
            "........",
            "........",
        ],
        "luck" => [
            "XXXXXXXX",
            "XX....XX",
            "X.X..X.X",
            "X..XX..X",
            "X..XX..X",
            "X.X..X.X",
            "XX....XX",
            "XXXXXXXX",
        ],
        _ => [
            ".XX..XX.",
            "XXXXXXXX",
            "XXXXXXXX",
            "XXXXXXXX",
            ".XXXXXX.",
            "..XXXX..",
            "...XX...",
            "........",
        ],
    }
}

pub fn setup_upgrade_icons(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    let handles = std::array::from_fn(|i| {
        let u = &UPGRADES[i];
        images.add(icon_image(&upgrade_bitmap(u.id), u.cat.color()))
    });
    commands.insert_resource(UpgradeIcons { handles });
}

#[allow(deprecated)]
pub fn update_upgrade_picker_timer(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    mut picker: ResMut<UpgradePicker>,
    mut stats: ResMut<GameStats>,
    mut weather: ResMut<WeatherState>,
    mut banner: ResMut<BannerState>,
    mut next_state: ResMut<NextState<GameState>>,
    audio: Res<AudioEngine>,
    #[allow(deprecated)]
    cards: Query<(&PickerCard, &Interaction), Changed<Interaction>>,
) {
    let dt = time.delta_secs();
    if picker.lockout > 0.0 {
        picker.lockout = (picker.lockout - dt).max(0.0);
    }

    if picker.picked.is_none() && picker.lockout <= 0.0 {
        let mut slot = if keys.just_pressed(KeyCode::Digit1) {
            Some(0)
        } else if keys.just_pressed(KeyCode::Digit2) {
            Some(1)
        } else if keys.just_pressed(KeyCode::Digit3) {
            Some(2)
        } else {
            None
        };
        for (card, inter) in &cards {
            #[allow(deprecated)]
            match *inter {
                Interaction::Pressed => slot = Some(card.0),
                Interaction::Hovered => audio.ui(UiSound::Move),
                Interaction::None => {}
            }
        }
        if let Some(s) = slot {
            picker.pick(s, &mut stats, &audio);
        }
    }

    if picker.confirm_timer >= 0.0 {
        picker.confirm_timer -= dt;
        if picker.confirm_timer <= 0.0 {
            picker.confirm_timer = -1.0;
            crate::game::start_wave(&mut stats, &mut weather, &audio, &mut banner);
            next_state.set(GameState::Playing);
        }
    }
}

#[derive(Component)]
pub struct PickerRoot;

#[derive(Component)]
pub struct PickerCard(pub usize);

pub fn spawn_upgrade_picker(
    mut commands: Commands,
    picker: Res<UpgradePicker>,
    stats: Res<GameStats>,
    icons: Res<UpgradeIcons>,
) {
    commands
        .spawn((
            PickerRoot,
            GlobalZIndex(10),
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
            BackgroundColor(Color::srgba(0.02, 0.02, 0.03, 0.82)),
        ))
        .with_children(|root| {
            root.spawn((
                Text::new(format!("WAVE {} CLEARED", stats.wave)),
                TextFont {
                    font_size: FontSize::Px(56.0),
                    ..default()
                },
                TextColor(Color::srgb_u8(0xd6, 0x45, 0x3c)),
                TextShadow {
                    offset: Vec2::new(0.0, 4.0),
                    color: Color::BLACK,
                },
            ));
            root.spawn((
                Text::new("CHOOSE ONE"),
                TextFont {
                    font_size: FontSize::Px(18.0),
                    ..default()
                },
                TextColor(Color::srgb_u8(0x8a, 0x90, 0x98)),
            ));
            root.spawn(Node {
                flex_direction: FlexDirection::Row,
                column_gap: Val::Px(22.0),
                margin: UiRect::top(Val::Px(18.0)),
                ..default()
            })
            .with_children(|row| {
                for (slot, &idx) in picker.choices.iter().enumerate() {
                    let u = &UPGRADES[idx];
                    let accent = u.cat.color();
                    let (before, after) = (u.stat)(&stats);
                    let owned = stats.upgrades.get(u.id).copied().unwrap_or(0);
                    let max_pips = u.max.min(6);

                    #[allow(deprecated)]
                    row.spawn((
                        PickerCard(slot),
                        Interaction::default(),
                        Node {
                            width: Val::Px(230.0),
                            padding: UiRect::all(Val::Px(16.0)),
                            flex_direction: FlexDirection::Column,
                            align_items: AlignItems::Center,
                            row_gap: Val::Px(10.0),
                            border: UiRect::all(Val::Px(1.0)),
                            ..default()
                        },
                        BackgroundColor(Color::srgb_u8(0x0e, 0x0c, 0x0d)),
                        BorderColor::all(Color::srgb_u8(0x2b, 0x26, 0x27)),
                    ))
                    .with_children(|card| {
                        // Top 3px accent stripe
                        card.spawn((
                            Node {
                                width: Val::Percent(100.0),
                                height: Val::Px(4.0),
                                ..default()
                            },
                            BackgroundColor(accent),
                        ));
                        // Category + shortcut
                        card.spawn(Node {
                            width: Val::Percent(100.0),
                            justify_content: JustifyContent::SpaceBetween,
                            align_items: AlignItems::Center,
                            ..default()
                        })
                        .with_children(|top| {
                            top.spawn((
                                Text::new(u.cat.label()),
                                TextFont {
                                    font_size: FontSize::Px(14.0),
                                    ..default()
                                },
                                TextColor(accent),
                            ));
                            top.spawn((
                                Text::new(format!("[{}]", slot + 1)),
                                TextFont {
                                    font_size: FontSize::Px(15.0),
                                    ..default()
                                },
                                TextColor(Color::srgb_u8(0xcd, 0xd2, 0xd8)),
                            ));
                        });
                        // Pixel-art icon
                        card.spawn((
                            ImageNode::new(icons.handles[idx].clone()),
                            Node {
                                width: Val::Px(64.0),
                                height: Val::Px(64.0),
                                margin: UiRect::vertical(Val::Px(6.0)),
                                ..default()
                            },
                        ));
                        // Name & desc
                        card.spawn((
                            Text::new(u.name),
                            TextFont {
                                font_size: FontSize::Px(22.0),
                                ..default()
                            },
                            TextColor(Color::srgb_u8(0xe9, 0xe2, 0xd6)),
                            TextShadow {
                                offset: Vec2::new(0.0, 2.0),
                                color: Color::BLACK,
                            },
                        ));
                        card.spawn((
                            Text::new(u.desc),
                            TextFont {
                                font_size: FontSize::Px(15.0),
                                ..default()
                            },
                            TextColor(Color::srgb_u8(0x9a, 0xa0, 0xa8)),
                        ));
                        // Stat preview
                        card.spawn((
                            Text::new(format!("{before} -> {after}")),
                            TextFont {
                                font_size: FontSize::Px(17.0),
                                ..default()
                            },
                            TextColor(accent),
                        ));
                        // Level pips
                        if u.max < 20 {
                            card.spawn(Node {
                                flex_direction: FlexDirection::Row,
                                column_gap: Val::Px(5.0),
                                margin: UiRect::top(Val::Px(4.0)),
                                ..default()
                            })
                            .with_children(|pips| {
                                for p in 0..max_pips {
                                    let col = if p < owned {
                                        accent
                                    } else if p == owned {
                                        Color::srgb_u8(0xe8, 0xd2, 0x8a)
                                    } else {
                                        Color::srgb_u8(0x24, 0x26, 0x2c)
                                    };
                                    pips.spawn((
                                        Node {
                                            width: Val::Px(14.0),
                                            height: Val::Px(6.0),
                                            ..default()
                                        },
                                        BackgroundColor(col),
                                    ));
                                }
                            });
                        }
                    });
                }
            });
        });
}

pub fn despawn_upgrade_picker(
    mut commands: Commands,
    query: Query<Entity, With<PickerRoot>>,
) {
    for entity in &query {
        commands.entity(entity).despawn();
    }
}
