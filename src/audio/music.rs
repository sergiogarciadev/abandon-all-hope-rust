use super::synth::{FilterKind, SAMPLE_RATE, VoiceSpec, Waveform};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MusicMode {
    Title,
    Calm,
    Combat,
    Dead,
}

#[derive(Clone, Copy, Debug)]
struct MelNote {
    step: usize,
    midi: i32,
    len: usize,
}

const fn midi_hz(m: i32) -> f32 {
    // 440 * 2^((m - 69) / 12) approximated at compile-time? Use runtime helper below.
    m as f32
}

#[inline]
fn note_hz(m: i32) -> f32 {
    440.0 * 2.0_f32.powf((m as f32 - 69.0) / 12.0)
}

#[derive(Clone, Copy)]
enum ChordId {
    Dm9,
    Dmadd9,
    Bbmaj9,
    Bbmaj7,
    Bbmaj7s11,
    Gm9,
    Gm7,
    Eo7,
    A7,
    A7b9,
    Fmaj7,
    Fmaj9,
    Cadd9,
    C69,
    C7sus,
    C7,
    Am7,
    Ebmaj7s11,
}

fn chord_notes(id: ChordId) -> [i32; 5] {
    match id {
        ChordId::Dm9 => [38, 53, 57, 60, 64],
        ChordId::Dmadd9 => [38, 53, 57, 62, 64],
        ChordId::Bbmaj9 => [34, 53, 57, 60, 62],
        ChordId::Bbmaj7 => [34, 53, 58, 62, 69],
        ChordId::Bbmaj7s11 => [34, 52, 57, 62, 65],
        ChordId::Gm9 => [43, 53, 58, 62, 69],
        ChordId::Gm7 => [43, 53, 58, 62, 67],
        ChordId::Eo7 => [40, 52, 55, 58, 62],
        ChordId::A7 => [45, 55, 61, 64, 69],
        ChordId::A7b9 => [45, 55, 61, 64, 70],
        ChordId::Fmaj7 => [41, 53, 57, 60, 64],
        ChordId::Fmaj9 => [41, 55, 57, 60, 64],
        ChordId::Cadd9 => [36, 55, 60, 62, 64],
        ChordId::C69 => [36, 55, 57, 62, 64],
        ChordId::C7sus => [36, 53, 55, 58, 60],
        ChordId::C7 => [36, 52, 55, 58, 60],
        ChordId::Am7 => [45, 55, 60, 64, 69],
        ChordId::Ebmaj7s11 => [39, 55, 58, 62, 69],
    }
}

const THEME: &[MelNote] = &[
    MelNote { step: 0, midi: 69, len: 4 },
    MelNote { step: 4, midi: 74, len: 2 },
    MelNote { step: 6, midi: 77, len: 2 },
    MelNote { step: 8, midi: 76, len: 6 },
    MelNote { step: 14, midi: 74, len: 2 },
    MelNote { step: 16, midi: 77, len: 4 },
    MelNote { step: 20, midi: 74, len: 4 },
    MelNote { step: 24, midi: 70, len: 8 },
    MelNote { step: 32, midi: 67, len: 2 },
    MelNote { step: 34, midi: 70, len: 2 },
    MelNote { step: 36, midi: 74, len: 4 },
    MelNote { step: 40, midi: 79, len: 6 },
    MelNote { step: 46, midi: 77, len: 2 },
    MelNote { step: 48, midi: 76, len: 4 },
    MelNote { step: 52, midi: 73, len: 4 },
    MelNote { step: 56, midi: 69, len: 8 },
    MelNote { step: 64, midi: 69, len: 4 },
    MelNote { step: 68, midi: 74, len: 2 },
    MelNote { step: 70, midi: 77, len: 2 },
    MelNote { step: 72, midi: 81, len: 6 },
    MelNote { step: 78, midi: 79, len: 2 },
    MelNote { step: 80, midi: 77, len: 4 },
    MelNote { step: 84, midi: 81, len: 4 },
    MelNote { step: 88, midi: 84, len: 4 },
    MelNote { step: 92, midi: 81, len: 4 },
    MelNote { step: 96, midi: 79, len: 4 },
    MelNote { step: 100, midi: 77, len: 2 },
    MelNote { step: 102, midi: 74, len: 2 },
    MelNote { step: 104, midi: 70, len: 4 },
    MelNote { step: 108, midi: 74, len: 4 },
    MelNote { step: 112, midi: 73, len: 4 },
    MelNote { step: 116, midi: 76, len: 4 },
    MelNote { step: 120, midi: 81, len: 4 },
    MelNote { step: 124, midi: 73, len: 4 },
];

const SOAR: &[MelNote] = &[
    MelNote { step: 0, midi: 74, len: 6 },
    MelNote { step: 6, midi: 72, len: 2 },
    MelNote { step: 8, midi: 74, len: 4 },
    MelNote { step: 12, midi: 77, len: 4 },
    MelNote { step: 16, midi: 76, len: 8 },
    MelNote { step: 24, midi: 74, len: 4 },
    MelNote { step: 28, midi: 72, len: 4 },
    MelNote { step: 32, midi: 72, len: 4 },
    MelNote { step: 36, midi: 76, len: 4 },
    MelNote { step: 40, midi: 81, len: 8 },
    MelNote { step: 48, midi: 79, len: 4 },
    MelNote { step: 52, midi: 77, len: 4 },
    MelNote { step: 56, midi: 76, len: 4 },
    MelNote { step: 60, midi: 74, len: 4 },
    MelNote { step: 64, midi: 74, len: 6 },
    MelNote { step: 70, midi: 72, len: 2 },
    MelNote { step: 72, midi: 74, len: 4 },
    MelNote { step: 76, midi: 81, len: 4 },
    MelNote { step: 80, midi: 79, len: 8 },
    MelNote { step: 88, midi: 76, len: 4 },
    MelNote { step: 92, midi: 79, len: 4 },
    MelNote { step: 96, midi: 82, len: 8 },
    MelNote { step: 104, midi: 81, len: 4 },
    MelNote { step: 108, midi: 79, len: 4 },
    MelNote { step: 112, midi: 82, len: 4 },
    MelNote { step: 116, midi: 81, len: 12 },
];

const OMEN: &[MelNote] = &[
    MelNote { step: 0, midi: 81, len: 12 },
    MelNote { step: 12, midi: 82, len: 4 },
    MelNote { step: 16, midi: 81, len: 16 },
    MelNote { step: 32, midi: 79, len: 6 },
    MelNote { step: 38, midi: 77, len: 2 },
    MelNote { step: 40, midi: 74, len: 8 },
    MelNote { step: 48, midi: 76, len: 16 },
    MelNote { step: 64, midi: 81, len: 12 },
    MelNote { step: 76, midi: 82, len: 4 },
    MelNote { step: 80, midi: 81, len: 16 },
    MelNote { step: 96, midi: 82, len: 8 },
    MelNote { step: 104, midi: 81, len: 4 },
    MelNote { step: 108, midi: 79, len: 4 },
    MelNote { step: 112, midi: 73, len: 4 },
    MelNote { step: 116, midi: 76, len: 4 },
    MelNote { step: 120, midi: 79, len: 4 },
    MelNote { step: 124, midi: 82, len: 4 },
];

const HAVEN: &[MelNote] = &[
    MelNote { step: 0, midi: 76, len: 6 },
    MelNote { step: 6, midi: 74, len: 2 },
    MelNote { step: 8, midi: 72, len: 8 },
    MelNote { step: 16, midi: 72, len: 4 },
    MelNote { step: 20, midi: 76, len: 4 },
    MelNote { step: 24, midi: 79, len: 8 },
    MelNote { step: 32, midi: 77, len: 6 },
    MelNote { step: 38, midi: 76, len: 2 },
    MelNote { step: 40, midi: 74, len: 8 },
    MelNote { step: 48, midi: 72, len: 4 },
    MelNote { step: 52, midi: 77, len: 4 },
    MelNote { step: 56, midi: 76, len: 8 },
    MelNote { step: 64, midi: 81, len: 6 },
    MelNote { step: 70, midi: 79, len: 2 },
    MelNote { step: 72, midi: 77, len: 8 },
    MelNote { step: 80, midi: 76, len: 4 },
    MelNote { step: 84, midi: 77, len: 4 },
    MelNote { step: 88, midi: 81, len: 8 },
    MelNote { step: 96, midi: 82, len: 6 },
    MelNote { step: 102, midi: 81, len: 2 },
    MelNote { step: 104, midi: 79, len: 4 },
    MelNote { step: 108, midi: 77, len: 4 },
    MelNote { step: 112, midi: 76, len: 4 },
    MelNote { step: 116, midi: 73, len: 4 },
    MelNote { step: 120, midi: 70, len: 8 },
];

const ARP_UPDOWN: [usize; 16] = [0, 1, 2, 3, 4, 5, 6, 7, 6, 5, 4, 3, 2, 1, 2, 3];

#[derive(Clone, Copy, PartialEq, Eq)]
enum SectionId {
    A,
    B,
    C,
    I,
    F,
}

fn section_bar(sec: SectionId, bar: usize) -> (ChordId, Option<ChordId>) {
    use ChordId::*;
    match sec {
        SectionId::A => match bar {
            0 => (Dm9, None),
            1 => (Bbmaj9, None),
            2 => (Gm9, None),
            3 => (A7, Some(A7b9)),
            4 => (Dmadd9, None),
            5 => (Fmaj7, None),
            6 => (Gm7, Some(Eo7)),
            _ => (A7b9, None),
        },
        SectionId::B => match bar {
            0 => (Bbmaj7, None),
            1 => (Cadd9, None),
            2 => (Am7, None),
            3 => (Dm9, None),
            4 => (Bbmaj7, None),
            5 => (C69, None),
            6 => (Ebmaj7s11, None),
            _ => (A7b9, None),
        },
        SectionId::C => match bar {
            0 => (Dmadd9, None),
            1 => (Bbmaj7s11, None),
            2 => (Gm9, None),
            3 => (Dm9, None),
            4 => (Dmadd9, None),
            5 => (Ebmaj7s11, None),
            6 => (Gm7, None),
            _ => (A7b9, None),
        },
        SectionId::I => match bar {
            0 => (Bbmaj7, None),
            1 => (Gm9, None),
            2 => (Ebmaj7s11, None),
            3 => (Dmadd9, None),
            4 => (Bbmaj7s11, None),
            5 => (Gm9, None),
            6 => (Eo7, None),
            _ => (A7b9, None),
        },
        SectionId::F => match bar {
            0 => (Fmaj9, None),
            1 => (Am7, None),
            2 => (Bbmaj7, None),
            3 => (C7sus, Some(C7)),
            4 => (Fmaj9, None),
            5 => (Dm9, None),
            6 => (Gm9, None),
            _ => (A7b9, None),
        },
    }
}

fn section_melody(sec: SectionId) -> Option<&'static [MelNote]> {
    match sec {
        SectionId::A => Some(THEME),
        SectionId::B => Some(SOAR),
        SectionId::C => Some(OMEN),
        SectionId::I => None,
        SectionId::F => Some(HAVEN),
    }
}

fn harmony_under(m: i32, chord: &[i32; 5]) -> Option<i32> {
    for t in (m - 9..=m - 3).rev() {
        if chord.iter().any(|&c| (c - t).rem_euclid(12) == 0) {
            return Some(t);
        }
    }
    None
}

pub struct MusicSequencer {
    mode: MusicMode,
    pending: Option<MusicMode>,
    step: usize,
    sec: SectionId,
    sec_n: usize,
    vary: bool,
    layer: u8,
    wave: u32,
    alive: usize,
    low_hp: bool,
    samples_until_step: f32,
    rng: u32,
}

impl MusicSequencer {
    pub fn new() -> Self {
        let _ = midi_hz(69);
        Self {
            mode: MusicMode::Title,
            pending: None,
            step: 0,
            sec: SectionId::A,
            sec_n: 0,
            vary: false,
            layer: 1,
            wave: 1,
            alive: 0,
            low_hp: false,
            samples_until_step: 0.0,
            rng: 0xDEAD_BEEF,
        }
    }

    pub fn set_state(&mut self, mode: MusicMode, wave: u32, alive: usize, low_hp: bool) {
        self.wave = wave;
        self.alive = alive;
        self.low_hp = low_hp;
        if mode != self.pending.unwrap_or(self.mode) {
            if mode == MusicMode::Dead {
                self.pending = None;
                self.mode = MusicMode::Dead;
            } else {
                self.pending = Some(mode);
            }
        }
    }

    #[inline]
    fn rand01(&mut self) -> f32 {
        let mut x = self.rng;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.rng = x;
        x as f32 / u32::MAX as f32
    }

    #[inline]
    fn hum(&mut self, v: f32) -> f32 {
        v * (0.9 + self.rand01() * 0.2)
    }

    fn heat(&self) -> f32 {
        0.1 + self.wave as f32 * 0.06 + (self.alive as f32 / 30.0) * 0.6
    }

    fn bpm(&self) -> f32 {
        match self.mode {
            MusicMode::Combat => (126.0 + self.wave as f32 * 2.0).min(156.0),
            MusicMode::Calm => 92.0,
            MusicMode::Title | MusicMode::Dead => 76.0,
        }
    }

    fn swing(&self) -> f32 {
        match self.mode {
            MusicMode::Calm => 0.16,
            MusicMode::Title => 0.06,
            _ => 0.0,
        }
    }

    fn sixteenth(&self) -> f32 {
        60.0 / self.bpm() / 4.0
    }

    fn pick_section(&mut self) {
        let n = self.sec_n;
        self.sec_n += 1;
        match self.mode {
            MusicMode::Title => {
                self.sec = if n % 3 == 2 { SectionId::I } else { SectionId::A };
                self.vary = n % 3 == 1;
            }
            MusicMode::Calm => {
                self.sec = SectionId::F;
                self.vary = n % 2 == 1;
            }
            MusicMode::Combat => {
                let k = n % 4;
                self.sec = match k {
                    1 => SectionId::B,
                    3 => {
                        if self.heat() >= 0.55 {
                            SectionId::B
                        } else {
                            SectionId::C
                        }
                    }
                    _ => SectionId::A,
                };
                self.vary = k >= 2;
            }
            MusicMode::Dead => {}
        }
    }

    pub fn tick_sample(&mut self) -> Vec<VoiceSpec> {
        let mut out = Vec::new();
        if self.samples_until_step > 0.0 {
            self.samples_until_step -= 1.0;
            return out;
        }

        let six = self.sixteenth();
        self.samples_until_step += six * SAMPLE_RATE as f32;

        if let Some(p) = self.pending {
            if self.step % 4 == 0 {
                self.pending = None;
                self.mode = p;
                self.step = 0;
                self.sec_n = 0;
            }
        }

        if self.mode != MusicMode::Dead {
            let swing_delay = if self.step % 2 == 1 {
                self.swing() * six
            } else {
                0.0
            };
            self.play_step(self.step, swing_delay, six, &mut out);
        }
        self.step += 1;
        out
    }

    fn play_step(&mut self, step: usize, delay: f32, six: f32, out: &mut Vec<VoiceSpec>) {
        if step % 128 == 0 {
            self.pick_section();
        }
        let s = step % 16;
        let bar = (step >> 4) % 8;
        let (c0, c1) = section_bar(self.sec, bar);
        let ch_id = if s < 8 { c0 } else { c1.unwrap_or(c0) };
        let ch = chord_notes(ch_id);
        let pad = [ch[1], ch[2], ch[3], ch[4]];
        let sec = self.sec;
        let bass_of = |b: usize| -> i32 {
            if sec == SectionId::C && b < 7 {
                38
            } else {
                chord_notes(section_bar(sec, b).0)[0]
            }
        };
        let root = bass_of(bar);
        let change = s == 0 || (s == 8 && c1.is_some());
        let span = if c1.is_some() { 8 } else { 16 };
        let mel = section_melody(self.sec)
            .and_then(|notes| notes.iter().find(|n| n.step == step % 128).copied());

        match self.mode {
            MusicMode::Title => {
                if change {
                    for &m in &pad {
                        out.push(VoiceSpec {
                            delay,
                            waveform: Waveform::Pulse25,
                            freq: note_hz(m),
                            hold: Some(span as f32 * six),
                            gain: 0.025,
                            attack: 0.25,
                            pan: -0.45,
                            wet: 0.35,
                            is_music: true,
                            ..Default::default()
                        });
                    }
                }
                if s == 0 {
                    out.push(VoiceSpec {
                        delay,
                        waveform: Waveform::Tri,
                        freq: note_hz(root),
                        hold: Some(15.0 * six),
                        gain: 0.22,
                        attack: 0.15,
                        is_music: true,
                        ..Default::default()
                    });
                }
                if self.sec == SectionId::I {
                    if s % 6 == 3 && self.rand01() < 0.6 {
                        let idx = (self.rand01() * 4.0) as usize % 4;
                        out.push(VoiceSpec {
                            delay,
                            waveform: Waveform::Sine,
                            freq: note_hz(pad[idx] + 12),
                            dur: 1.4,
                            gain: 0.08,
                            wet: 0.5,
                            is_music: true,
                            ..Default::default()
                        });
                    }
                    return;
                }
                if let Some(m) = mel {
                    out.push(VoiceSpec {
                        delay,
                        waveform: Waveform::Sine,
                        freq: note_hz(m.midi),
                        dur: 1.6,
                        gain: 0.22,
                        wet: 0.4,
                        is_music: true,
                        ..Default::default()
                    });
                    if self.vary {
                        if let Some(h) = harmony_under(m.midi, &ch) {
                            out.push(VoiceSpec {
                                delay: delay + 0.012,
                                waveform: Waveform::Sine,
                                freq: note_hz(h),
                                dur: 1.4,
                                gain: 0.10,
                                wet: 0.4,
                                is_music: true,
                                ..Default::default()
                            });
                        }
                    }
                }
                if self.vary && s % 4 == 2 {
                    let idx = (s >> 2) % 4;
                    out.push(VoiceSpec {
                        delay,
                        waveform: Waveform::Pulse12,
                        freq: note_hz(pad[idx] + 12),
                        dur: 2.0 * six,
                        gain: 0.035,
                        pan: if (s >> 2) & 1 == 0 { -0.35 } else { 0.35 },
                        wet: 0.3,
                        is_music: true,
                        ..Default::default()
                    });
                }
            }
            MusicMode::Calm => {
                if change {
                    for (i, &m) in pad.iter().enumerate() {
                        out.push(VoiceSpec {
                            delay: delay + i as f32 * 0.018,
                            waveform: Waveform::Sine,
                            freq: note_hz(m),
                            hold: Some(span as f32 * six * 0.9),
                            gain: self.hum(0.055),
                            attack: 0.005,
                            wet: 0.3,
                            is_music: true,
                            ..Default::default()
                        });
                    }
                }
                if s % 4 == 0 {
                    let next_b = bass_of((bar + 1) % 8);
                    let walk = [
                        root,
                        root + 7,
                        root + 12,
                        if next_b > root { next_b - 1 } else { next_b + 1 },
                    ][s >> 2];
                    out.push(VoiceSpec {
                        delay,
                        waveform: Waveform::Tri,
                        freq: note_hz(walk),
                        hold: Some(3.5 * six),
                        gain: self.hum(0.22),
                        is_music: true,
                        ..Default::default()
                    });
                }
                if s == 4 || s == 12 {
                    out.push(VoiceSpec {
                        delay,
                        waveform: Waveform::White,
                        dur: 0.14,
                        gain: self.hum(0.05),
                        attack: 0.02,
                        pan: 0.25,
                        filter: Some((FilterKind::Bandpass, 3500.0, None, 0.7)),
                        is_music: true,
                        ..Default::default()
                    });
                }
                if s % 2 == 0 {
                    out.push(VoiceSpec {
                        delay,
                        waveform: Waveform::Metal,
                        dur: 0.035,
                        gain: self.hum(if s % 4 == 0 { 0.035 } else { 0.02 }),
                        pan: 0.25,
                        filter: Some((FilterKind::Highpass, 7000.0, None, 0.7)),
                        is_music: true,
                        ..Default::default()
                    });
                }
                if let Some(m) = mel {
                    if !self.vary {
                        out.push(VoiceSpec {
                            delay,
                            waveform: Waveform::Tri,
                            freq: note_hz(m.midi),
                            hold: Some(m.len as f32 * six * 0.92),
                            gain: 0.13,
                            vib: if m.len >= 4 { 0.15 } else { 0.0 },
                            wet: 0.25,
                            is_music: true,
                            ..Default::default()
                        });
                    }
                }
            }
            MusicMode::Combat => {
                if s == 0 {
                    let h = self.heat();
                    self.layer = if h < 0.35 { 1 } else if h < 0.7 { 2 } else { 3 };
                }
                let l = self.layer;
                let half = self.sec == SectionId::C;
                let last = bar == 7;

                if change {
                    for &m in &pad {
                        out.push(VoiceSpec {
                            delay,
                            waveform: Waveform::Pulse25,
                            freq: note_hz(m),
                            hold: Some(span as f32 * six * 0.9),
                            gain: 0.015 + 0.006 * l as f32,
                            attack: 0.05,
                            pan: -0.45,
                            filter: Some((FilterKind::Lowpass, 900.0 + l as f32 * 450.0, None, 0.9)),
                            is_music: true,
                            ..Default::default()
                        });
                    }
                }

                if half {
                    if s == 0 {
                        out.push(VoiceSpec {
                            delay,
                            waveform: Waveform::Tri,
                            freq: note_hz(root),
                            hold: Some(15.0 * six),
                            gain: 0.36,
                            is_music: true,
                            ..Default::default()
                        });
                    }
                    if s == 0 || (l >= 2 && s == 11) {
                        out.push(VoiceSpec {
                            delay,
                            waveform: Waveform::Sine,
                            freq: 160.0,
                            freq_end: Some(44.0),
                            dur: 0.18,
                            gain: 0.75,
                            is_music: true,
                            ..Default::default()
                        });
                    }
                    if s == 8 && !last {
                        out.push(VoiceSpec {
                            delay,
                            waveform: Waveform::Noise,
                            rate: 0.55,
                            dur: 0.16,
                            gain: 0.45,
                            filter: Some((FilterKind::Bandpass, 2200.0, None, 0.8)),
                            is_music: true,
                            ..Default::default()
                        });
                    }
                    if let Some(m) = mel {
                        out.push(VoiceSpec {
                            delay,
                            waveform: Waveform::Sine,
                            freq: note_hz(m.midi),
                            dur: 1.5,
                            gain: 0.18,
                            wet: 0.4,
                            is_music: true,
                            ..Default::default()
                        });
                    }
                } else {
                    const GALLOP: [u8; 16] = [1, 0, 1, 1, 1, 0, 1, 1, 1, 0, 1, 1, 1, 0, 1, 1];
                    if (l < 3 && s % 2 == 0) || (l >= 3 && GALLOP[s] == 1) {
                        let mut m = root + if s % 4 == 2 { 12 } else { 0 };
                        if s == 14 && !last {
                            let nb = bass_of(bar + 1);
                            if nb != root {
                                m = nb + if nb > root { -1 } else { 1 };
                            }
                        }
                        let len_steps = if l < 3 { 2.0 } else { 1.0 };
                        let g = self.hum(0.28 + 0.03 * l as f32);
                        out.push(VoiceSpec {
                            delay,
                            waveform: Waveform::Tri,
                            freq: note_hz(m),
                            hold: Some(len_steps * six * 0.85),
                            gain: g,
                            is_music: true,
                            ..Default::default()
                        });
                    }

                    if s == 0 || s == 8 || (l >= 2 && s == 10) || (l >= 3 && s == 3) {
                        out.push(VoiceSpec {
                            delay,
                            waveform: Waveform::Sine,
                            freq: 160.0,
                            freq_end: Some(44.0),
                            dur: 0.18,
                            gain: 0.78,
                            is_music: true,
                            ..Default::default()
                        });
                    }
                    if l >= 2 && (s == 4 || s == 12) {
                        let g = self.hum(0.45);
                        out.push(VoiceSpec {
                            delay,
                            waveform: Waveform::Noise,
                            rate: 0.55,
                            dur: 0.16,
                            gain: g,
                            filter: Some((FilterKind::Bandpass, 2200.0, None, 0.8)),
                            is_music: true,
                            ..Default::default()
                        });
                    }
                    if l >= 3 || s % 2 == 0 {
                        let open = self.sec == SectionId::B && l >= 2 && s % 4 == 2;
                        let g = self.hum(if open { 0.09 } else if s % 4 == 2 { 0.14 } else { 0.07 });
                        out.push(VoiceSpec {
                            delay,
                            waveform: Waveform::Metal,
                            dur: if open { 0.2 } else { 0.035 },
                            gain: g,
                            pan: 0.25,
                            filter: Some((FilterKind::Highpass, 7000.0, None, 0.7)),
                            is_music: true,
                            ..Default::default()
                        });
                    }

                    // Arpeggio
                    let arp_do = (l == 1 && s % 4 == 2) || (l == 2 && s % 2 == 0) || l >= 3;
                    if arp_do {
                        let idx = if l == 1 {
                            4 + ((s >> 2) & 3)
                        } else if l == 2 {
                            s >> 1
                        } else {
                            ARP_UPDOWN[s]
                        };
                        let m = if idx < 4 {
                            pad[idx]
                        } else {
                            pad[idx - 4] + 12
                        };
                        let g = self.hum(0.042);
                        out.push(VoiceSpec {
                            delay,
                            waveform: Waveform::Pulse12,
                            freq: note_hz(m),
                            hold: Some((if l == 3 { 1.0 } else { 2.0 }) * six * 0.8),
                            gain: g,
                            pan: if s & 2 != 0 { 0.35 } else { -0.35 },
                            wet: 0.25,
                            is_music: true,
                            ..Default::default()
                        });
                    }

                    if let Some(m) = mel {
                        let w = if l == 1 { Waveform::Tri } else { Waveform::Pulse25 };
                        let g = if l == 1 { 0.12 } else { 0.17 };
                        out.push(VoiceSpec {
                            delay,
                            waveform: w,
                            freq: note_hz(m.midi),
                            hold: Some(m.len as f32 * six * 0.92),
                            gain: g,
                            vib: if m.len >= 4 { 0.22 } else { 0.0 },
                            pan: 0.08,
                            wet: 0.3,
                            is_music: true,
                            ..Default::default()
                        });
                        if l >= 3 {
                            out.push(VoiceSpec {
                                delay,
                                waveform: Waveform::Pulse50,
                                freq: note_hz(m.midi - 12),
                                hold: Some(m.len as f32 * six * 0.9),
                                gain: 0.06,
                                pan: 0.08,
                                wet: 0.25,
                                is_music: true,
                                ..Default::default()
                            });
                        }
                        if l >= 2 && (self.vary || self.sec == SectionId::B) {
                            if let Some(h) = harmony_under(m.midi, &ch) {
                                out.push(VoiceSpec {
                                    delay,
                                    waveform: Waveform::Pulse50,
                                    freq: note_hz(h),
                                    hold: Some(m.len as f32 * six * 0.92),
                                    gain: 0.09,
                                    vib: if m.len >= 4 { 0.15 } else { 0.0 },
                                    pan: -0.22,
                                    wet: 0.25,
                                    is_music: true,
                                    ..Default::default()
                                });
                            }
                        }
                    }
                }

                if self.low_hp && s % 4 == 0 {
                    out.push(VoiceSpec {
                        delay,
                        waveform: Waveform::Sine,
                        freq: 62.0,
                        freq_end: Some(40.0),
                        dur: 0.14,
                        gain: 0.75,
                        post_muffle: true,
                        is_music: false,
                        ..Default::default()
                    });
                    out.push(VoiceSpec {
                        delay: delay + 0.16,
                        waveform: Waveform::Sine,
                        freq: 55.0,
                        freq_end: Some(38.0),
                        dur: 0.12,
                        gain: 0.55,
                        post_muffle: true,
                        is_music: false,
                        ..Default::default()
                    });
                }
            }
            MusicMode::Dead => {}
        }
    }
}
