pub mod music;
pub mod synth;

use std::sync::mpsc::{self, Sender};
use std::sync::{Arc, Mutex};

use bevy::prelude::*;
use rand::RngExt;

use crate::core::state::{GameState, GameStats};
use crate::entities::drops::DropKind;
use crate::entities::player::PlayerRig;
use crate::entities::zombies::{Corpse, Zombie};
use crate::ui::upgrades::UpgradeCat;
use crate::world::weather::WeatherState;

use music::MusicMode;
use synth::{AudioCmd, ChipMixer, FilterKind, VoiceSpec, Waveform};

#[inline]
fn midi_hz(m: i32) -> f32 {
    440.0 * 2.0_f32.powf((m as f32 - 69.0) / 12.0)
}

#[allow(dead_code)]
#[derive(Clone, Copy, Debug)]
pub enum UiSound {
    Move,
    Back,
    Select,
}

#[derive(Resource)]
pub struct AudioEngine {
    tx: Sender<AudioCmd>,
    listener: Arc<Mutex<Vec3>>,
    pub muted: bool,
    step_alt: Mutex<bool>,
}

impl AudioEngine {
    fn send(&self, cmd: AudioCmd) {
        let _ = self.tx.send(cmd);
    }

    fn play(&self, spec: VoiceSpec) {
        self.send(AudioCmd::Play(spec));
    }

    fn place(&self, pos: Vec3) -> (f32, f32) {
        let l = *self.listener.lock().unwrap();
        let dx = pos.x - l.x;
        let dz = pos.z - l.z;
        let pan = (dx / 12.0).clamp(-0.9, 0.9);
        let near = 1.0 / (1.0 + dx.hypot(dz) / 7.0);
        (pan, near)
    }

    pub fn set_listener(&self, pos: Vec3) {
        *self.listener.lock().unwrap() = pos;
    }

    pub fn set_weather(&self, level: f32, gust: f32) {
        self.send(AudioCmd::SetWeather { level, gust });
    }

    pub fn set_music_state(&self, mode: MusicMode, wave: u32, alive: usize, low_hp: bool) {
        self.send(AudioCmd::SetMusicState {
            mode,
            wave,
            alive,
            low_hp,
        });
    }

    pub fn set_muted(&mut self, muted: bool) {
        self.muted = muted;
        self.send(AudioCmd::SetMuted(muted));
    }

    pub fn set_paused(&self, paused: bool) {
        self.send(AudioCmd::SetPaused(paused));
    }

    pub fn shot(&self, hot: bool, spread: bool) {
        self.play(VoiceSpec {
            waveform: Waveform::Pulse25,
            freq: if hot { 850.0 } else { 680.0 },
            freq_end: Some(45.0),
            dur: 0.11,
            gain: 0.55,
            attack: 0.001,
            wet: 0.22,
            ..Default::default()
        });
        self.play(VoiceSpec {
            waveform: Waveform::Metal,
            dur: 0.06,
            gain: 0.55,
            wet: 0.2,
            filter: Some((FilterKind::Bandpass, 3800.0, None, 0.7)),
            ..Default::default()
        });
        self.play(VoiceSpec {
            waveform: Waveform::Noise,
            rate: 0.7,
            dur: 0.18,
            gain: 0.5,
            wet: 0.22,
            filter: Some((FilterKind::Lowpass, 2400.0, Some(300.0), 0.7)),
            ..Default::default()
        });
        if spread {
            self.play(VoiceSpec {
                delay: 0.01,
                waveform: Waveform::Noise,
                rate: 0.5,
                dur: 0.22,
                gain: 0.35,
                wet: 0.22,
                filter: Some((FilterKind::Lowpass, 1800.0, None, 0.7)),
                ..Default::default()
            });
        }
        self.play(VoiceSpec {
            waveform: Waveform::Sine,
            freq: 120.0,
            freq_end: Some(38.0),
            dur: 0.14,
            gain: 0.58,
            wet: 0.15,
            ..Default::default()
        });
    }

    pub fn hit(&self, pos: Vec3) {
        let (pan, near) = self.place(pos);
        self.play(VoiceSpec {
            waveform: Waveform::Noise,
            rate: 0.85,
            dur: 0.13,
            gain: 0.5 * near,
            pan,
            wet: 0.18,
            filter: Some((FilterKind::Bandpass, 1400.0, Some(350.0), 2.0)),
            ..Default::default()
        });
        self.play(VoiceSpec {
            waveform: Waveform::Tri,
            freq: 180.0,
            freq_end: Some(55.0),
            dur: 0.08,
            gain: 0.35 * near,
            pan,
            wet: 0.18,
            ..Default::default()
        });
        self.play(VoiceSpec {
            waveform: Waveform::Metal,
            dur: 0.04,
            gain: 0.2 * near,
            pan,
            filter: Some((FilterKind::Highpass, 4500.0, None, 0.7)),
            ..Default::default()
        });
    }

    pub fn zdeath(&self, pos: Vec3) {
        let (pan, near) = self.place(pos);
        let mut rng = rand::rng();
        self.play(VoiceSpec {
            waveform: Waveform::Noise,
            rate: 0.5,
            dur: 0.32,
            gain: 0.65 * near,
            pan,
            wet: 0.35,
            filter: Some((FilterKind::Lowpass, 1800.0, Some(200.0), 0.7)),
            ..Default::default()
        });
        self.play(VoiceSpec {
            waveform: Waveform::Tri,
            freq: 140.0,
            freq_end: Some(35.0),
            dur: 0.22,
            gain: 0.6 * near,
            pan,
            wet: 0.35,
            ..Default::default()
        });
        self.play(VoiceSpec {
            delay: 0.04,
            waveform: Waveform::Metal,
            rate: 0.6,
            dur: 0.15,
            gain: 0.35 * near,
            pan,
            wet: 0.35,
            filter: Some((FilterKind::Bandpass, 2200.0, None, 1.5)),
            ..Default::default()
        });
        self.play(VoiceSpec {
            waveform: Waveform::Saw,
            freq: rng.random_range(65.0..85.0),
            freq_end: Some(40.0),
            dur: 0.35,
            gain: 0.32 * near,
            attack: 0.06,
            pan,
            wet: 0.35,
            filter: Some((FilterKind::Bandpass, 420.0, None, 2.5)),
            ..Default::default()
        });
    }

    pub fn groan(&self, pos: Vec3) {
        let (pan, near) = self.place(pos);
        let mut rng = rand::rng();
        let f = rng.random_range(55.0..95.0);
        let f2 = f * rng.random_range(0.7..0.95);
        let dur = rng.random_range(0.7..1.4);
        let formant = rng.random_range(450.0..750.0);
        self.play(VoiceSpec {
            waveform: Waveform::Saw,
            freq: f,
            freq_end: Some(f2),
            dur,
            gain: rng.random_range(0.18..0.3) * near,
            attack: 0.08,
            pan,
            wet: 0.4,
            filter: Some((FilterKind::Bandpass, formant, None, 2.5)),
            ..Default::default()
        });
    }

    pub fn swipe(&self, pos: Vec3) {
        let (pan, near) = self.place(pos);
        self.play(VoiceSpec {
            waveform: Waveform::White,
            dur: 0.18,
            gain: 0.36 * near,
            pan,
            wet: 0.1,
            filter: Some((FilterKind::Bandpass, 1800.0, Some(400.0), 1.8)),
            ..Default::default()
        });
    }

    pub fn bash(&self, pos: Vec3, k: f32) {
        let (pan, near) = self.place(pos);
        let g = 0.55 * near * k;
        self.play(VoiceSpec {
            waveform: Waveform::Tri,
            freq: 110.0,
            freq_end: Some(42.0),
            dur: 0.18,
            gain: g,
            pan,
            wet: 0.25,
            ..Default::default()
        });
        self.play(VoiceSpec {
            waveform: Waveform::Noise,
            rate: 0.6,
            dur: 0.2,
            gain: g * 0.9,
            pan,
            wet: 0.25,
            filter: Some((FilterKind::Bandpass, 1100.0, Some(240.0), 1.2)),
            ..Default::default()
        });
    }

    pub fn crack(&self, pos: Vec3) {
        let (pan, near) = self.place(pos);
        self.play(VoiceSpec {
            waveform: Waveform::Pulse12,
            freq: 2200.0,
            freq_end: Some(900.0),
            dur: 0.04,
            gain: 0.3 * near,
            pan,
            wet: 0.25,
            ..Default::default()
        });
        self.play(VoiceSpec {
            waveform: Waveform::Metal,
            dur: 0.12,
            gain: 0.42 * near,
            pan,
            wet: 0.25,
            filter: Some((FilterKind::Highpass, 6500.0, None, 0.7)),
            ..Default::default()
        });
    }

    pub fn shatter(&self, pos: Vec3) {
        let (pan, near) = self.place(pos);
        let mut rng = rand::rng();
        self.play(VoiceSpec {
            waveform: Waveform::Metal,
            dur: 0.45,
            gain: 0.65 * near,
            pan,
            wet: 0.4,
            filter: Some((FilterKind::Highpass, 4500.0, None, 0.7)),
            ..Default::default()
        });
        self.play(VoiceSpec {
            waveform: Waveform::White,
            dur: 0.3,
            gain: 0.5 * near,
            pan,
            wet: 0.4,
            filter: Some((FilterKind::Bandpass, 2600.0, None, 1.5)),
            ..Default::default()
        });
        self.play(VoiceSpec {
            waveform: Waveform::Tri,
            freq: 160.0,
            freq_end: Some(50.0),
            dur: 0.16,
            gain: 0.5 * near,
            pan,
            wet: 0.4,
            ..Default::default()
        });
        for i in 0..4 {
            self.play(VoiceSpec {
                delay: 0.04 + i as f32 * 0.06,
                waveform: Waveform::Pulse12,
                freq: rng.random_range(1800.0..3400.0),
                dur: 0.03,
                gain: 0.2 * near,
                pan,
                wet: 0.4,
                ..Default::default()
            });
        }
    }

    pub fn door_break(&self, pos: Vec3) {
        let (pan, near) = self.place(pos);
        self.play(VoiceSpec {
            waveform: Waveform::Sine,
            freq: 90.0,
            freq_end: Some(32.0),
            dur: 0.35,
            gain: 0.8 * near,
            pan,
            wet: 0.45,
            ..Default::default()
        });
        self.play(VoiceSpec {
            waveform: Waveform::Noise,
            rate: 0.4,
            dur: 0.5,
            gain: 0.8 * near,
            pan,
            wet: 0.45,
            filter: Some((FilterKind::Lowpass, 900.0, None, 0.7)),
            ..Default::default()
        });
    }

    pub fn step(&self) {
        let mut alt = self.step_alt.lock().unwrap();
        *alt = !*alt;
        let f = if *alt { 130.0 } else { 115.0 };
        self.play(VoiceSpec {
            waveform: Waveform::Tri,
            freq: f,
            freq_end: Some(55.0),
            dur: 0.05,
            gain: 0.10,
            wet: 0.08,
            ..Default::default()
        });
        self.play(VoiceSpec {
            waveform: Waveform::Noise,
            rate: 0.9,
            dur: 0.04,
            gain: 0.07,
            wet: 0.08,
            filter: Some((FilterKind::Lowpass, 1200.0, None, 0.7)),
            ..Default::default()
        });
    }

    pub fn reload_out(&self) {
        self.play(VoiceSpec {
            waveform: Waveform::Pulse25,
            freq: 720.0,
            freq_end: Some(340.0),
            dur: 0.05,
            gain: 0.18,
            pan: 0.1,
            wet: 0.1,
            ..Default::default()
        });
        self.play(VoiceSpec {
            waveform: Waveform::Metal,
            dur: 0.07,
            gain: 0.22,
            pan: 0.1,
            wet: 0.1,
            filter: Some((FilterKind::Bandpass, 2400.0, None, 0.7)),
            ..Default::default()
        });
    }

    pub fn reload_in(&self) {
        self.play(VoiceSpec {
            waveform: Waveform::Pulse25,
            freq: 420.0,
            freq_end: Some(880.0),
            dur: 0.06,
            gain: 0.28,
            pan: 0.05,
            wet: 0.12,
            ..Default::default()
        });
        self.play(VoiceSpec {
            delay: 0.02,
            waveform: Waveform::Metal,
            dur: 0.09,
            gain: 0.32,
            pan: 0.05,
            wet: 0.12,
            filter: Some((FilterKind::Highpass, 3200.0, None, 0.7)),
            ..Default::default()
        });
        self.play(VoiceSpec {
            delay: 0.06,
            waveform: Waveform::Tri,
            freq: 210.0,
            freq_end: Some(95.0),
            dur: 0.06,
            gain: 0.22,
            pan: 0.05,
            wet: 0.12,
            ..Default::default()
        });
    }

    pub fn hurt(&self) {
        self.send(AudioCmd::HurtMuffle);
        self.play(VoiceSpec {
            waveform: Waveform::Sine,
            freq: 85.0,
            freq_end: Some(30.0),
            dur: 0.35,
            gain: 0.85,
            wet: 0.35,
            post_muffle: true,
            ..Default::default()
        });
        self.play(VoiceSpec {
            waveform: Waveform::Noise,
            rate: 0.4,
            dur: 0.25,
            gain: 0.75,
            wet: 0.35,
            post_muffle: true,
            filter: Some((FilterKind::Lowpass, 600.0, None, 0.7)),
            ..Default::default()
        });
    }

    pub fn pickup(&self, kind: DropKind) {
        let notes = match kind {
            DropKind::Med => [60, 64, 67, 72],
            DropKind::Dmg => [58, 62, 65, 70],
            DropKind::Ammo => [55, 60, 67, 72],
        };
        for (i, m) in notes.iter().enumerate() {
            let d = i as f32 * 0.055;
            self.play(VoiceSpec {
                delay: d,
                waveform: Waveform::Pulse25,
                freq: midi_hz(*m),
                dur: 0.18,
                gain: 0.18,
                wet: 0.3,
                ..Default::default()
            });
            self.play(VoiceSpec {
                delay: d,
                waveform: Waveform::Tri,
                freq: midi_hz(m - 12),
                dur: 0.12,
                gain: 0.14,
                wet: 0.3,
                ..Default::default()
            });
        }
    }

    pub fn thunder(&self, near: f32) {
        let mut rng = rand::rng();
        let pan = rng.random_range(-0.4..0.4);
        let base = 0.45 + 0.45 * near;
        for i in 0..4 {
            self.play(VoiceSpec {
                delay: i as f32 * rng.random_range(0.2..0.6),
                waveform: Waveform::Noise,
                rate: 0.08,
                dur: rng.random_range(1.8..4.2),
                gain: base * rng.random_range(0.4..0.75),
                attack: rng.random_range(0.05..0.3),
                pan,
                wet: 0.5,
                filter: Some((FilterKind::Lowpass, rng.random_range(140.0..320.0), None, 0.7)),
                ..Default::default()
            });
        }
    }

    pub fn wave_start(&self, wave: u32) {
        for (r, v) in [(1.0, 1.0), (2.01, 0.5), (2.76, 0.4), (5.4, 0.2), (8.93, 0.1)] {
            self.play(VoiceSpec {
                waveform: Waveform::Sine,
                freq: midi_hz(50) * r,
                dur: 3.2 / r.sqrt(),
                gain: v * 0.28,
                attack: 0.005,
                wet: 0.6,
                ..Default::default()
            });
        }
        self.play(VoiceSpec {
            waveform: Waveform::Tri,
            freq: midi_hz(38),
            dur: 1.6,
            gain: 0.3,
            wet: 0.5,
            ..Default::default()
        });
        if wave > 1 {
            for (i, m) in [57, 58, 57].iter().enumerate() {
                self.play(VoiceSpec {
                    delay: 0.12 + i as f32 * 0.22,
                    waveform: Waveform::Pulse25,
                    freq: midi_hz(*m),
                    hold: Some(0.18),
                    gain: 0.12,
                    wet: 0.45,
                    ..Default::default()
                });
            }
        }
    }

    pub fn wave_clear(&self) {
        for (i, m) in [62, 66, 69, 74].iter().enumerate() {
            self.play(VoiceSpec {
                delay: i as f32 * 0.09,
                waveform: Waveform::Pulse25,
                freq: midi_hz(*m),
                hold: Some(0.08),
                gain: 0.18,
                wet: 0.35,
                ..Default::default()
            });
        }
        self.play(VoiceSpec {
            delay: 0.36,
            waveform: Waveform::Pulse25,
            freq: midi_hz(78),
            hold: Some(0.7),
            gain: 0.18,
            vib: 0.3,
            wet: 0.35,
            ..Default::default()
        });
        self.play(VoiceSpec {
            delay: 0.36,
            waveform: Waveform::Tri,
            freq: midi_hz(50),
            hold: Some(1.0),
            gain: 0.28,
            wet: 0.35,
            ..Default::default()
        });
    }

    pub fn upgrade(&self, cat: UpgradeCat) {
        let up = match cat {
            UpgradeCat::Firepower => 0,
            UpgradeCat::Handling => 2,
            UpgradeCat::Survival => 5,
            UpgradeCat::Fortune => 7,
        };
        self.play(VoiceSpec {
            waveform: Waveform::Pulse12,
            freq: 180.0,
            freq_end: Some(1700.0),
            dur: 0.3,
            gain: 0.08,
            wet: 0.35,
            ..Default::default()
        });
        for (i, m) in [62, 69, 74, 78, 81].iter().enumerate() {
            self.play(VoiceSpec {
                delay: i as f32 * 0.05,
                waveform: Waveform::Pulse25,
                freq: midi_hz(m + up),
                dur: 0.2,
                gain: 0.16,
                wet: 0.35,
                ..Default::default()
            });
        }
    }

    pub fn death(&self) {
        for (i, m) in [69, 68, 67].iter().enumerate() {
            self.play(VoiceSpec {
                delay: 0.35 + i as f32 * 0.32,
                waveform: Waveform::Pulse25,
                freq: midi_hz(*m),
                hold: Some(0.26),
                gain: 0.18,
                wet: 0.45,
                post_muffle: true,
                ..Default::default()
            });
        }
        self.play(VoiceSpec {
            delay: 1.31,
            waveform: Waveform::Pulse25,
            freq: midi_hz(66),
            freq_end: Some(midi_hz(61)),
            hold: Some(1.3),
            gain: 0.18,
            vib: 0.5,
            wet: 0.45,
            post_muffle: true,
            ..Default::default()
        });
        for (i, m) in [57, 56, 55, 50].iter().enumerate() {
            self.play(VoiceSpec {
                delay: 0.35 + i as f32 * 0.32,
                waveform: Waveform::Tri,
                freq: midi_hz(*m - 12),
                hold: Some(if i < 3 { 0.26 } else { 1.6 }),
                gain: 0.3,
                wet: 0.45,
                post_muffle: true,
                ..Default::default()
            });
        }
    }

    #[allow(dead_code)]
    pub fn ui(&self, kind: UiSound) {
        match kind {
            UiSound::Move => self.play(VoiceSpec {
                waveform: Waveform::Pulse12,
                freq: midi_hz(81),
                dur: 0.035,
                gain: 0.12,
                wet: 0.12,
                ..Default::default()
            }),
            UiSound::Back => self.play(VoiceSpec {
                waveform: Waveform::Pulse12,
                freq: midi_hz(74),
                freq_end: Some(midi_hz(69)),
                dur: 0.07,
                gain: 0.12,
                wet: 0.12,
                ..Default::default()
            }),
            UiSound::Select => {
                for (i, m) in [76, 83].iter().enumerate() {
                    self.play(VoiceSpec {
                        delay: i as f32 * 0.05,
                        waveform: Waveform::Pulse12,
                        freq: midi_hz(*m),
                        dur: 0.06,
                        gain: 0.12,
                        wet: 0.12,
                        ..Default::default()
                    });
                }
            }
        }
    }
}

pub fn setup_audio(mut commands: Commands) {
    let (tx, rx) = mpsc::channel();
    std::thread::Builder::new()
        .name("chip-audio".into())
        .spawn(move || {
            match rodio::DeviceSinkBuilder::open_default_sink() {
                Ok(mut sink) => {
                    sink.log_on_drop(false);
                    let player = rodio::Player::connect_new(sink.mixer());
                    player.append(ChipMixer::new(rx));
                    player.play();
                    // Keep sink + player alive for the lifetime of the app
                    std::mem::forget(player);
                    std::mem::forget(sink);
                    loop {
                        std::thread::park();
                    }
                }
                Err(e) => {
                    warn!("Audio output unavailable ({e}); continuing silently");
                    for _ in rx {}
                }
            }
        })
        .ok();

    commands.insert_resource(AudioEngine {
        tx,
        listener: Arc::new(Mutex::new(Vec3::ZERO)),
        muted: false,
        step_alt: Mutex::new(false),
    });
}

pub fn sync_audio_state(
    keys: Res<ButtonInput<KeyCode>>,
    state: Res<State<GameState>>,
    stats: Res<GameStats>,
    weather: Res<WeatherState>,
    rig: Res<PlayerRig>,
    mut audio: ResMut<AudioEngine>,
    player_q: Query<&Transform>,
    live_zombies: Query<Entity, (With<Zombie>, Without<Corpse>)>,
) {
    if keys.just_pressed(KeyCode::KeyM) {
        let m = !audio.muted;
        audio.set_muted(m);
    }

    if let Ok(tf) = player_q.get(rig.root) {
        audio.set_listener(tf.translation);
    }
    audio.set_weather(weather.level, weather.gust);

    let mode = match state.get() {
        GameState::Title => MusicMode::Title,
        GameState::ChoosingUpgrade => MusicMode::Calm,
        GameState::Playing | GameState::Paused => {
            if stats.intermission > 0.0 {
                MusicMode::Calm
            } else {
                MusicMode::Combat
            }
        }
        GameState::Dead => MusicMode::Dead,
    };
    let alive = live_zombies.iter().count();
    let low_hp = stats.hp == 1;
    audio.set_music_state(mode, stats.wave, alive, low_hp);
    audio.set_paused(*state.get() == GameState::Paused);
}
