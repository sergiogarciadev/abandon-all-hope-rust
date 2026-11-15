# Chapter 8: Procedural Audio and Adaptive Music

Instead of shipping dozens of `.wav` or `.ogg` files, *Abandon All Hope* synthesizes every gunshot, shattering window, zombie groan, thunderclap, and chiptune soundtrack **live in real time**. In this chapter we build a 44.1 kHz stereo DSP mixer using `rodio` (`0.22.2`) and connect it to our Bevy systems through a lock-free command channel.

---

## 1. Architecture: Bevy ECS + Dedicated Audio Thread

Audio callbacks must never block on ECS locks. In `src/audio/mod.rs`, `setup_audio` spawns a background thread that opens `rodio::DeviceSinkBuilder::open_default_sink()`, attaches a `rodio::Player`, and feeds it our custom `ChipMixer` implementing `rodio::Source<Item = f32>`.

Bevy holds only an `AudioEngine` resource wrapping `std::sync::mpsc::Sender<AudioCmd>`. Every SFX call (`audio.shot(hot)`, `audio.hurt()`, `audio.thunder()`) simply pushes a lightweight `VoiceSpec` into the channel.

---

## 2. Chip Waveforms and LFSR Noise

In `src/audio/synth.rs`, `ActiveVoice` generates classic 8-bit waveforms sample by sample:

- **Pulse (`Pulse12`, `Pulse25`, `Pulse50`)**: Square waves with 12.5%, 25%, and 50% duty cycles. Narrower duty cycles sound brighter and reedier for bells and leads.
- **Triangle (`Tri`)**: Quantized to a **32-step 4-bit NES staircase**, giving basslines and kick drums their warm, slightly gritty punch.
- **LFSR Noise (`Noise`, `Metal`)**: A 15-bit Linear Feedback Shift Register. Tapping bit 1 produces long pseudo-random noise for gunshots and snares; tapping bit 6 produces the 93-step metallic timbre used for hi-hats and shattering glass.
- **White Noise (`White`)**: Xorshift32 noise for zombie swipes and brushed percussion.

Each voice supports exponential pitch slides (`freq -> freq_end`), an attack/release envelope, and an optional per-voice **biquad filter** (`Lowpass`, `Highpass`, `Bandpass`).

---

## 3. Master Effects: Wind, Reverb, and Hurt Muffle

`ChipMixer::step_frame` mixes all active voices and applies three global effects:

1. **Ambient Night Wind**: White noise through a `Bandpass` filter whose center frequency drifts with a slow LFO (`0.17 Hz`) and widens as `weather.level` rises.
2. **Stereo Ping-Pong Delay**: A `0.22s` ring buffer with `0.28` cross-feedback gives bells, groans, and thunder atmospheric space.
3. **Muffle Lowpass + Soft Limiter**: Normally wide open at `18 kHz`. When the player takes damage (`audio.hurt()`), the cutoff drops to `550 Hz` and smoothly recovers; on death (`GameState::Dead`), it stays choked at `500 Hz`. Finally, `(sample * 0.9).tanh()` prevents clipping.

---

## 4. Adaptive 4-Mode Sequencer

In `src/audio/music.rs`, `MusicSequencer` ticks in sample time inside the audio thread and switches modes based on `GameState`:

- **`Title` (76 BPM)**: Slow `Dm - Bb - Gm - A7` pads with a 12.5% pulse bell melody.
- **`Calm` (92 BPM, swung)**: Walking bass, electric-piano chords, and brushed hats during intermissions and upgrade selection.
- **`Combat` (126–156 BPM)**: Scales tempo with `stats.wave` and layers gallop bass, 16th-note arpeggios, twin leads, breakdown snare rolls, and a low-HP heartbeat when `stats.hp == 1`.
- **`Dead`**: Mutes the sequencer and chokes the master lowpass.

Pressing **`M`** at any time toggles `audio.muted` in `sync_audio_state`.
