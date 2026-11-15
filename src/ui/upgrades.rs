use bevy::prelude::*;
use rand::seq::SliceRandom;

use crate::audio::AudioEngine;
use crate::core::state::{GameState, GameStats};
use crate::entities::drops::DROP_CHANCE;
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

pub fn update_upgrade_picker_timer(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    mut picker: ResMut<UpgradePicker>,
    mut stats: ResMut<GameStats>,
    mut weather: ResMut<WeatherState>,
    mut next_state: ResMut<NextState<GameState>>,
    audio: Res<AudioEngine>,
) {
    let dt = time.delta_secs();
    if picker.lockout > 0.0 {
        picker.lockout = (picker.lockout - dt).max(0.0);
    }

    if picker.picked.is_none() && picker.lockout <= 0.0 {
        let slot = if keys.just_pressed(KeyCode::Digit1) {
            Some(0)
        } else if keys.just_pressed(KeyCode::Digit2) {
            Some(1)
        } else if keys.just_pressed(KeyCode::Digit3) {
            Some(2)
        } else {
            None
        };
        if let Some(s) = slot {
            picker.pick(s, &mut stats, &audio);
        }
    }

    if picker.confirm_timer >= 0.0 {
        picker.confirm_timer -= dt;
        if picker.confirm_timer <= 0.0 {
            picker.confirm_timer = -1.0;
            crate::game::start_wave(&mut stats, &mut weather, &audio);
            next_state.set(GameState::Playing);
        }
    }
}

#[derive(Component)]
pub struct TempUpgradeOverlay;

/// Temporary Step 7 upgrade prompt so `ChoosingUpgrade` is visible before Step 9 UI.
pub fn spawn_temp_upgrade_ui(
    mut commands: Commands,
    picker: Res<UpgradePicker>,
    stats: Res<GameStats>,
) {
    let mut lines = vec![format!("WAVE {} CLEARED — CHOOSE AN UPGRADE (PRESS 1, 2, OR 3)\n", stats.wave)];
    for (slot, &idx) in picker.choices.iter().enumerate() {
        let u = &UPGRADES[idx];
        let (before, after) = (u.stat)(&stats);
        lines.push(format!("[{}] {} — {} ({} -> {})", slot + 1, u.name, u.desc, before, after));
    }
    let body = lines.join("\n");
    info!("{body}");

    commands
        .spawn((
            TempUpgradeOverlay,
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(24.0),
                left: Val::Px(24.0),
                padding: UiRect::all(Val::Px(16.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.03, 0.04, 0.06, 0.85)),
        ))
        .with_child((
            Text::new(body),
            TextFont {
                font_size: FontSize::Px(20.0),
                ..default()
            },
            TextColor(Color::srgb(0.95, 0.92, 0.84)),
        ));
}

pub fn despawn_temp_upgrade_ui(
    mut commands: Commands,
    query: Query<Entity, With<TempUpgradeOverlay>>,
) {
    for entity in &query {
        commands.entity(entity).despawn();
    }
}
