use std::f32::consts::PI;
use std::sync::mpsc::Receiver;
use std::time::Duration;

use rodio::Source;

use super::music::{MusicMode, MusicSequencer};

pub const SAMPLE_RATE: u32 = 44_100;
const SR: f32 = SAMPLE_RATE as f32;
const DELAY_SAMPLES: usize = (SAMPLE_RATE as usize * 22) / 100; // 0.22s stereo ping-pong delay

#[derive(Clone, Copy, Debug)]
pub enum Waveform {
    Sine,
    Pulse12,
    Pulse25,
    Pulse50,
    Tri,
    Saw,
    Noise,
    Metal,
    White,
}

#[derive(Clone, Copy, Debug)]
pub enum FilterKind {
    Lowpass,
    Highpass,
    Bandpass,
}

#[derive(Clone, Copy, Debug)]
pub struct Biquad {
    b0: f32,
    b1: f32,
    b2: f32,
    a1: f32,
    a2: f32,
    x1: f32,
    x2: f32,
    y1: f32,
    y2: f32,
}

impl Biquad {
    pub fn new(kind: FilterKind, freq: f32, q: f32) -> Self {
        let mut b = Self {
            b0: 1.0,
            b1: 0.0,
            b2: 0.0,
            a1: 0.0,
            a2: 0.0,
            x1: 0.0,
            x2: 0.0,
            y1: 0.0,
            y2: 0.0,
        };
        b.set(kind, freq, q);
        b
    }

    pub fn set(&mut self, kind: FilterKind, freq: f32, q: f32) {
        let f = freq.clamp(20.0, SR * 0.45);
        let q = q.max(0.1);
        let w0 = 2.0 * PI * f / SR;
        let cos_w0 = w0.cos();
        let alpha = w0.sin() / (2.0 * q);
        let a0 = 1.0 + alpha;
        let (b0, b1, b2) = match kind {
            FilterKind::Lowpass => {
                let c = 1.0 - cos_w0;
                (c * 0.5, c, c * 0.5)
            }
            FilterKind::Highpass => {
                let c = 1.0 + cos_w0;
                (c * 0.5, -c, c * 0.5)
            }
            FilterKind::Bandpass => (alpha, 0.0, -alpha),
        };
        self.b0 = b0 / a0;
        self.b1 = b1 / a0;
        self.b2 = b2 / a0;
        self.a1 = (-2.0 * cos_w0) / a0;
        self.a2 = (1.0 - alpha) / a0;
    }

    #[inline]
    pub fn process(&mut self, x: f32) -> f32 {
        let y = self.b0 * x + self.b1 * self.x1 + self.b2 * self.x2
            - self.a1 * self.y1
            - self.a2 * self.y2;
        self.x2 = self.x1;
        self.x1 = x;
        self.y2 = self.y1;
        self.y1 = y;
        y
    }
}

#[derive(Clone, Debug)]
pub struct VoiceSpec {
    pub delay: f32,
    pub waveform: Waveform,
    pub freq: f32,
    pub freq_end: Option<f32>,
    pub dur: f32,
    pub hold: Option<f32>,
    pub gain: f32,
    pub attack: f32,
    pub vib: f32,
    pub pan: f32,
    pub wet: f32,
    pub filter: Option<(FilterKind, f32, Option<f32>, f32)>,
    pub rate: f32,
    pub is_music: bool,
    pub post_muffle: bool,
}

impl Default for VoiceSpec {
    fn default() -> Self {
        Self {
            delay: 0.0,
            waveform: Waveform::Pulse50,
            freq: 440.0,
            freq_end: None,
            dur: 0.2,
            hold: None,
            gain: 0.5,
            attack: 0.002,
            vib: 0.0,
            pan: 0.0,
            wet: 0.15,
            filter: None,
            rate: 1.0,
            is_music: false,
            post_muffle: false,
        }
    }
}

pub struct ActiveVoice {
    spec: VoiceSpec,
    delay_samples: usize,
    sample_idx: usize,
    total_samples: usize,
    attack_samples: usize,
    hold_samples: usize,
    phase: f32,
    vib_phase: f32,
    lfsr: u16,
    lfsr_accum: f32,
    rng: u32,
    filter: Option<Biquad>,
}

impl ActiveVoice {
    pub fn new(spec: VoiceSpec, seed: u32) -> Self {
        let total_dur = spec.hold.map(|h| h + 0.08).unwrap_or(spec.dur).max(0.01);
        let total_samples = (total_dur * SR).round().max(1.0) as usize;
        let attack_samples = (spec.attack * SR).round().max(1.0) as usize;
        let hold_samples = spec.hold.map(|h| (h * SR).round() as usize).unwrap_or(0);
        let delay_samples = (spec.delay.max(0.0) * SR).round() as usize;
        let filter = spec.filter.map(|(k, f, _, q)| Biquad::new(k, f, q));
        Self {
            spec,
            delay_samples,
            sample_idx: 0,
            total_samples,
            attack_samples,
            hold_samples,
            phase: 0.0,
            vib_phase: 0.0,
            lfsr: (seed as u16 & 0x7FFF).max(1),
            lfsr_accum: 0.0,
            rng: seed.max(1),
            filter,
        }
    }

    #[inline]
    fn next_white(&mut self) -> f32 {
        let mut x = self.rng;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.rng = x;
        (x as f32 / u32::MAX as f32) * 2.0 - 1.0
    }

    #[inline]
    fn next_lfsr(&mut self, short_mode: bool) -> f32 {
        self.lfsr_accum += self.spec.rate.clamp(0.05, 2.0);
        while self.lfsr_accum >= 1.0 {
            self.lfsr_accum -= 1.0;
            let tap = if short_mode { 6 } else { 1 };
            let fb = (self.lfsr & 1) ^ ((self.lfsr >> tap) & 1);
            self.lfsr = (self.lfsr >> 1) | (fb << 14);
            if self.lfsr == 0 {
                self.lfsr = 1;
            }
        }
        if self.lfsr & 1 == 1 {
            0.8
        } else {
            -0.8
        }
    }

    /// Returns `(dry_sample, finished)`.
    pub fn step(&mut self) -> (f32, bool) {
        if self.delay_samples > 0 {
            self.delay_samples -= 1;
            return (0.0, false);
        }
        if self.sample_idx >= self.total_samples {
            return (0.0, true);
        }

        let t_norm = self.sample_idx as f32 / self.total_samples as f32;
        let mut freq = match self.spec.freq_end {
            Some(f2) if f2 > 0.0 && self.spec.freq > 0.0 => {
                self.spec.freq * (f2 / self.spec.freq).powf(t_norm)
            }
            _ => self.spec.freq,
        };

        if self.spec.vib > 0.0 {
            self.vib_phase = (self.vib_phase + 5.5 / SR) % 1.0;
            let depth = (2.0_f32.powf(self.spec.vib / 12.0) - 1.0) * (t_norm * 2.0).min(1.0);
            freq *= 1.0 + (2.0 * PI * self.vib_phase).sin() * depth;
        }

        self.phase = (self.phase + freq / SR).fract();
        let raw = match self.spec.waveform {
            Waveform::Sine => (2.0 * PI * self.phase).sin(),
            Waveform::Pulse12 => {
                if self.phase < 0.125 {
                    1.0
                } else {
                    -1.0
                }
            }
            Waveform::Pulse25 => {
                if self.phase < 0.25 {
                    1.0
                } else {
                    -1.0
                }
            }
            Waveform::Pulse50 => {
                if self.phase < 0.5 {
                    1.0
                } else {
                    -1.0
                }
            }
            Waveform::Tri => {
                // 32-step 4-bit NES triangle staircase
                let step = (self.phase * 32.0).floor() as i32;
                let v = if step < 16 { 15 - step } else { step - 16 };
                (v as f32 / 7.5) - 1.0
            }
            Waveform::Saw => self.phase * 2.0 - 1.0,
            Waveform::Noise => self.next_lfsr(false),
            Waveform::Metal => self.next_lfsr(true),
            Waveform::White => self.next_white(),
        };

        let filtered = if let (Some(flt), Some((kind, f1, f2, q))) =
            (&mut self.filter, self.spec.filter)
        {
            if let Some(f_end) = f2 {
                if self.sample_idx % 32 == 0 {
                    let f_cur = f1 * (f_end / f1.max(20.0)).powf(t_norm);
                    flt.set(kind, f_cur, q);
                }
            }
            flt.process(raw)
        } else {
            raw
        };

        let env = if self.sample_idx < self.attack_samples {
            self.sample_idx as f32 / self.attack_samples as f32
        } else if self.spec.hold.is_some() {
            if self.sample_idx < self.hold_samples {
                let rel = (self.sample_idx - self.attack_samples) as f32
                    / (self.hold_samples.saturating_sub(self.attack_samples).max(1)) as f32;
                1.0 - 0.35 * rel
            } else {
                let rel = (self.sample_idx - self.hold_samples) as f32
                    / (self.total_samples.saturating_sub(self.hold_samples).max(1)) as f32;
                0.65 * (1.0 - rel).max(0.0)
            }
        } else {
            let decay_pos = (self.sample_idx - self.attack_samples) as f32
                / (self.total_samples.saturating_sub(self.attack_samples).max(1)) as f32;
            (1.0 - decay_pos).powf(2.2)
        };

        self.sample_idx += 1;
        (filtered * env * self.spec.gain, false)
    }
}

pub enum AudioCmd {
    Play(VoiceSpec),
    SetWeather { level: f32, gust: f32 },
    SetMusicState {
        mode: MusicMode,
        wave: u32,
        alive: usize,
        low_hp: bool,
    },
    HurtMuffle,
    SetMuted(bool),
    SetPaused(bool),
}

pub struct ChipMixer {
    rx: Receiver<AudioCmd>,
    voices: Vec<ActiveVoice>,
    music: MusicSequencer,
    muted: bool,
    paused: bool,
    weather_level: f32,
    weather_gust: f32,
    wind_lfo_phase: f32,
    wind_bp: Biquad,
    wind_rng: u32,
    delay_l: Vec<f32>,
    delay_r: Vec<f32>,
    delay_idx: usize,
    muffle_lp_l: Biquad,
    muffle_lp_r: Biquad,
    muffle_freq: f32,
    muffle_target: f32,
    duck_gain: f32,
    sample_counter: u64,
    out_channel: u8,
    pending_r: f32,
    seed_counter: u32,
}

impl ChipMixer {
    pub fn new(rx: Receiver<AudioCmd>) -> Self {
        Self {
            rx,
            voices: Vec::with_capacity(96),
            music: MusicSequencer::new(),
            muted: false,
            paused: false,
            weather_level: 0.0,
            weather_gust: 1.0,
            wind_lfo_phase: 0.0,
            wind_bp: Biquad::new(FilterKind::Bandpass, 420.0, 1.1),
            wind_rng: 0x1234_5678,
            delay_l: vec![0.0; DELAY_SAMPLES],
            delay_r: vec![0.0; DELAY_SAMPLES],
            delay_idx: 0,
            muffle_lp_l: Biquad::new(FilterKind::Lowpass, 18_000.0, 0.707),
            muffle_lp_r: Biquad::new(FilterKind::Lowpass, 18_000.0, 0.707),
            muffle_freq: 18_000.0,
            muffle_target: 18_000.0,
            duck_gain: 1.0,
            sample_counter: 0,
            out_channel: 0,
            pending_r: 0.0,
            seed_counter: 1,
        }
    }

    fn drain_cmds(&mut self) {
        while let Ok(cmd) = self.rx.try_recv() {
            match cmd {
                AudioCmd::Play(spec) => {
                    if self.voices.len() < 128 {
                        self.seed_counter = self.seed_counter.wrapping_add(0x9E37_79B9);
                        self.voices.push(ActiveVoice::new(spec, self.seed_counter));
                    }
                }
                AudioCmd::SetWeather { level, gust } => {
                    self.weather_level = level.clamp(0.0, 1.0);
                    self.weather_gust = gust.max(0.2);
                }
                AudioCmd::SetMusicState {
                    mode,
                    wave,
                    alive,
                    low_hp,
                } => {
                    self.music.set_state(mode, wave, alive, low_hp);
                    self.muffle_target = if matches!(mode, MusicMode::Dead) {
                        500.0
                    } else {
                        18_000.0
                    };
                }
                AudioCmd::HurtMuffle => {
                    self.muffle_freq = 550.0;
                    self.muffle_lp_l.set(FilterKind::Lowpass, 550.0, 0.707);
                    self.muffle_lp_r.set(FilterKind::Lowpass, 550.0, 0.707);
                    self.duck_gain = 0.25;
                }
                AudioCmd::SetMuted(m) => self.muted = m,
                AudioCmd::SetPaused(p) => self.paused = p,
            }
        }
    }

    #[inline]
    fn next_wind_white(&mut self) -> f32 {
        let mut x = self.wind_rng;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.wind_rng = x;
        (x as f32 / u32::MAX as f32) * 2.0 - 1.0
    }

    fn step_frame(&mut self) -> (f32, f32) {
        if self.sample_counter % 64 == 0 {
            self.drain_cmds();
            // Smoothly recover muffle and ducking
            self.muffle_freq += (self.muffle_target - self.muffle_freq) * 0.004;
            self.muffle_lp_l.set(FilterKind::Lowpass, self.muffle_freq, 0.707);
            self.muffle_lp_r.set(FilterKind::Lowpass, self.muffle_freq, 0.707);
            self.duck_gain += (1.0 - self.duck_gain) * 0.005;

            // 0.17 Hz LFO for ambient night wind bandpass center frequency
            self.wind_lfo_phase = (self.wind_lfo_phase + 0.17 * (64.0 / SR)) % 1.0;
            let lfo = (2.0 * PI * self.wind_lfo_phase).sin();
            let wind_fc = 420.0 + lfo * (180.0 + 220.0 * self.weather_level);
            self.wind_bp.set(FilterKind::Bandpass, wind_fc, 1.1);
        }

        if self.muted || self.paused {
            self.sample_counter += 1;
            return (0.0, 0.0);
        }

        // Tick adaptive music sequencer and spawn any scheduled voices
        let music_specs = self.music.tick_sample();
        for spec in music_specs {
            if self.voices.len() < 128 {
                self.seed_counter = self.seed_counter.wrapping_add(0x9E37_79B9);
                self.voices.push(ActiveVoice::new(spec, self.seed_counter));
            }
        }

        let mut dry_l = 0.0_f32;
        let mut dry_r = 0.0_f32;
        let mut wet_l = 0.0_f32;
        let mut wet_r = 0.0_f32;
        let mut post_l = 0.0_f32;
        let mut post_r = 0.0_f32;

        let mut i = 0;
        while i < self.voices.len() {
            let (s, done) = self.voices[i].step();
            let pan = self.voices[i].spec.pan.clamp(-0.9, 0.9);
            let gain_l = (0.5 * (1.0 - pan)).sqrt();
            let gain_r = (0.5 * (1.0 + pan)).sqrt();
            let duck = if self.voices[i].spec.is_music {
                self.duck_gain
            } else {
                1.0
            };
            let sl = s * gain_l * duck;
            let sr = s * gain_r * duck;
            if self.voices[i].spec.post_muffle {
                post_l += sl;
                post_r += sr;
            } else {
                dry_l += sl;
                dry_r += sr;
            }
            let wet = self.voices[i].spec.wet;
            wet_l += sl * wet;
            wet_r += sr * wet;

            if done {
                self.voices.swap_remove(i);
            } else {
                i += 1;
            }
        }

        // Ambient night wind
        let wind_raw = self.next_wind_white();
        let wind_filt = self.wind_bp.process(wind_raw);
        let wind_gain = 0.05 + 0.18 * self.weather_level * self.weather_level * self.weather_gust;
        dry_l += wind_filt * wind_gain;
        dry_r += wind_filt * wind_gain;

        // Stereo ping-pong delay (0.22s, 0.28 cross-feedback)
        let dl = self.delay_l[self.delay_idx];
        let dr = self.delay_r[self.delay_idx];
        self.delay_l[self.delay_idx] = (wet_l + dr * 0.28).clamp(-1.5, 1.5);
        self.delay_r[self.delay_idx] = (wet_r + dl * 0.28).clamp(-1.5, 1.5);
        self.delay_idx = (self.delay_idx + 1) % DELAY_SAMPLES;

        let mix_l = self.muffle_lp_l.process(dry_l + dl * 0.45) + post_l;
        let mix_r = self.muffle_lp_r.process(dry_r + dr * 0.45) + post_r;

        self.sample_counter += 1;
        ((mix_l * 0.9).tanh(), (mix_r * 0.9).tanh())
    }
}

impl Iterator for ChipMixer {
    type Item = f32;

    fn next(&mut self) -> Option<f32> {
        if self.out_channel == 0 {
            let (l, r) = self.step_frame();
            self.pending_r = r;
            self.out_channel = 1;
            Some(l)
        } else {
            self.out_channel = 0;
            Some(self.pending_r)
        }
    }
}

impl Source for ChipMixer {
    fn current_span_len(&self) -> Option<usize> {
        None
    }

    fn channels(&self) -> std::num::NonZero<u16> {
        std::num::NonZero::new(2).unwrap()
    }

    fn sample_rate(&self) -> std::num::NonZero<u32> {
        std::num::NonZero::new(SAMPLE_RATE).unwrap()
    }

    fn total_duration(&self) -> Option<Duration> {
        None
    }
}
