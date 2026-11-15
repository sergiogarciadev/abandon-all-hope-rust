import { midiHz, rnd } from '../core/math.js';
import { settings } from '../core/storage.js';

let musicInstance = null;
export function setMusicInstance(m) {
  musicInstance = m;
}

let listenerObj = null;
export function setAudioListener(obj) {
  listenerObj = obj;
}

export const audio = {
  ctx: null, muted: false, waves: {}, bufs: {}, lastHit: 0, lastHover: 0, stepAlt: false,
  onEnsureCallback: null,

  ensure() {
    if (this.ctx) { if (this.ctx.state === 'suspended') this.ctx.resume(); return; }
    const AC = window.AudioContext || window.webkitAudioContext;
    if (!AC) return;
    this.ctx = new AC();
    this.buildVoices();
    this.buildMix();
    this.applyVolume();
    this.startWind();
    this.onEnsureCallback?.();
  },
  ok() { return this.ctx && !this.muted; },
  t() { return this.ctx.currentTime; },

  // --- chip voices: band-limited pulse/triangle tables + LFSR noise buffers ---
  buildVoices() {
    const c = this.ctx;
    // DFT one cycle of samples into a PeriodicWave (band-limited, so no aliasing fizz)
    const table = s => {
      const N = s.length, H = 64, re = new Float32Array(H), im = new Float32Array(H);
      for (let n = 1; n < H; n++) for (let k = 0; k < N; k++) {
        const ph = 2 * Math.PI * n * k / N;
        re[n] += s[k] * Math.cos(ph); im[n] += s[k] * Math.sin(ph);
      }
      return c.createPeriodicWave(re, im);
    };
    const cycle = fn => Array.from({ length: 256 }, (_, k) => fn(k / 256));
    this.waves.pulse12 = table(cycle(p => p < 0.125 ? 1 : -1));
    this.waves.pulse25 = table(cycle(p => p < 0.25 ? 1 : -1));
    this.waves.pulse50 = table(cycle(p => p < 0.5 ? 1 : -1));
    // the NES triangle is a 32-step, 4-bit staircase — that grit is the "classic" bass
    this.waves.tri = table(cycle(p => { const s = Math.floor(p * 32); return (s < 16 ? 15 - s : s - 16) / 7.5 - 1; }));

    // 15-bit LFSR noise: long mode = hiss, short mode (93-step loop) = metallic buzz
    const lfsr = (short, len) => {
      const b = c.createBuffer(1, len, c.sampleRate), d = b.getChannelData(0);
      let r = 1;
      for (let i = 0; i < len; i++) {
        const fb = (r & 1) ^ ((r >> (short ? 6 : 1)) & 1);
        r = (r >> 1) | (fb << 14);
        d[i] = r & 1 ? 0.8 : -0.8;
      }
      return b;
    };
    this.bufs.noise = lfsr(false, c.sampleRate * 2);
    this.bufs.metal = lfsr(true, 93 * 64);
    const w = c.createBuffer(1, c.sampleRate * 2, c.sampleRate), wd = w.getChannelData(0);
    for (let i = 0; i < wd.length; i++) wd[i] = Math.random() * 2 - 1;
    this.bufs.white = w;
  },

  // --- mix: sfx/music/reverb buses → master → muffle → post → mute → compressor ---
  buildMix() {
    const c = this.ctx;
    const gain = (v, to) => { const g = c.createGain(); g.gain.value = v; if (to) g.connect(to); return g; };
    const filt = (type, f, to) => { const b = c.createBiquadFilter(); b.type = type; b.frequency.value = f; if (to) b.connect(to); return b; };
    this.comp = c.createDynamicsCompressor();
    this.comp.threshold.value = -12; this.comp.knee.value = 8; this.comp.ratio.value = 5;
    this.comp.attack.value = 0.002; this.comp.release.value = 0.2;
    // brickwall limiter after the glue compressor so stacked impacts never clip
    this.limiter = c.createDynamicsCompressor();
    this.limiter.threshold.value = -2; this.limiter.knee.value = 0; this.limiter.ratio.value = 20;
    this.limiter.attack.value = 0.001; this.limiter.release.value = 0.08;
    this.comp.connect(this.limiter); this.limiter.connect(c.destination);
    this.mute = gain(this.level(), this.comp);
    this.post = gain(1, this.mute);                 // after the muffle: impacts that should stay crisp
    this.muffle = filt('lowpass', 20000, this.post); // swept down when you get hit
    this.master = gain(0.8, this.muffle);
    this.sfx = gain(1, this.master);
    this.verb = c.createConvolver(); this.verb.buffer = this.impulse(2.8);
    this.verb.connect(gain(0.45, this.master));
    // music: in → [echo] → lowpass → duck → volume → master, with a reverb send
    this.musicVol = gain(1, this.master);
    this.duck = gain(1, this.musicVol);
    this.musicLP = filt('lowpass', 18000, this.duck);
    this.musicIn = gain(0.5, this.musicLP);
    this.musicVerb = gain(1, this.verb);
    this.musicIn.connect(gain(0.22, this.musicVerb));
    this.echo = c.createDelay(1); this.echo.delayTime.value = 0.34;
    const fb = gain(0.36), dampFilt = filt('lowpass', 2600);
    this.echo.connect(dampFilt); dampFilt.connect(fb); fb.connect(this.echo); dampFilt.connect(this.musicLP);
    this.echoIn = gain(0.5, this.echo);
    this.musicWet = gain(1, this.musicIn); // dry + echo send in one
    this.musicWet.connect(this.echoIn);
  },
  // stereo noise tail with a darkening exponential decay — a big, dark room
  impulse(dur) {
    const c = this.ctx, n = Math.floor(c.sampleRate * dur), b = c.createBuffer(2, n, c.sampleRate);
    for (let ch = 0; ch < 2; ch++) {
      const d = b.getChannelData(ch); let lp = 0;
      for (let i = 0; i < n; i++) {
        const t = i / n, k = 0.6 - t * 0.5; // smoothing grows with time: highs die first
        lp += (Math.random() * 2 - 1 - lp) * k;
        d[i] = i < c.sampleRate * 0.012 ? 0 : lp * Math.pow(1 - t, 3.2);
      }
    }
    return b;
  },
  // ambient night wind: filtered noise with slowly wandering pitch and level
  startWind() {
    const c = this.ctx, src = c.createBufferSource(); src.buffer = this.bufs.white; src.loop = true;
    const bp = c.createBiquadFilter(); bp.type = 'bandpass'; bp.frequency.value = 420; bp.Q.value = 1.1;
    const g = c.createGain(); g.gain.value = 0;
    g.gain.linearRampToValueAtTime(0.06, c.currentTime + 5);
    const lfo = (f, depth, param) => {
      const o = c.createOscillator(); o.frequency.value = f;
      const d = c.createGain(); d.gain.value = depth; o.connect(d); d.connect(param); o.start();
    };
    lfo(0.07, 220, bp.frequency);
    lfo(0.13, 0.025, g.gain);
    src.connect(bp); bp.connect(g); g.connect(this.master); src.start();
    this.windGain = g;
  },
  // storm level 0..1 and the current gust (around 1)
  weather(level, gust) {
    if (!this.ctx) return;
    this.windGain.gain.setTargetAtTime(0.06 + 0.2 * level * level * gust, this.t(), 0.4);
  },
  // thunder: a long roll, louder the closer the strike (near: 0 far .. 1 overhead)
  thunder(near) {
    if (!this.ok()) return;
    const t = this.t(), o = this.out(0.45 + 0.45 * near, rnd(-0.4, 0.4), 0.5);
    for (let i = 0; i < 4; i++) // overlapping rumbles, so it rolls rather than booms
      this.nz(o, t + i * rnd(0.2, 0.6), { rate: 0.08, f: rnd(140, 320), dur: rnd(1.8, 4.2), v: rnd(0.5, 0.9), a: rnd(0.05, 0.3) });
    this.duckMusic(0.55, 1.8);
  },

  // --- building blocks ---
  // a per-sound output: gain → stereo pan → bus, plus a reverb send
  out(v, pan = 0, wet = 0.15, bus = this.sfx) {
    const c = this.ctx, g = c.createGain(); g.gain.value = v;
    let n = g;
    if (pan && c.createStereoPanner) {
      const p = c.createStereoPanner(); p.pan.value = Math.max(-0.9, Math.min(0.9, pan));
      g.connect(p); n = p;
    }
    n.connect(bus);
    if (wet) { const s = c.createGain(); s.gain.value = wet; n.connect(s); s.connect(this.verb); }
    return g;
  },
  // oscillator note. w: chip table name or native type. Percussive decay by default;
  // pass `hold` (s) for a gated, sustained note. `vib` = vibrato depth in semitones.
  osc(dest, at, { w = 'pulse50', f, f2, dur = 0.2, v = 1, a = 0.002, hold, vib = 0 }) {
    const c = this.ctx, o = c.createOscillator(), g = c.createGain();
    if (this.waves[w]) o.setPeriodicWave(this.waves[w]); else o.type = w;
    o.frequency.setValueAtTime(f, at);
    if (f2) o.frequency.exponentialRampToValueAtTime(f2, at + (hold || dur));
    if (vib) {
      const l = c.createOscillator(), d = c.createGain();
      l.frequency.value = 5.5; d.gain.setValueAtTime(0, at);
      d.gain.linearRampToValueAtTime(f * (Math.pow(2, vib / 12) - 1), at + Math.min(0.25, (hold || dur) * 0.6));
      l.connect(d); d.connect(o.frequency); l.start(at); l.stop(at + (hold || dur) + 0.1);
    }
    g.gain.setValueAtTime(0, at);
    g.gain.linearRampToValueAtTime(v, at + a);
    let end;
    if (hold) {
      g.gain.setTargetAtTime(v * 0.6, at + a, hold * 0.4);
      g.gain.setValueAtTime(v * 0.6, at + hold);   // anchor the release
      g.gain.linearRampToValueAtTime(0, at + hold + 0.05);
      end = at + hold + 0.06;
    } else {
      g.gain.exponentialRampToValueAtTime(0.0001, at + dur);
      end = at + dur + 0.02;
    }
    o.connect(g); g.connect(dest); o.start(at); o.stop(end);
  },
  // filtered noise burst. buf: noise | metal | white; rate < 1 pitches the LFSR down (crunchier)
  nz(dest, at, { buf = 'noise', rate = 1, type = 'lowpass', f = 2000, f2, q = 0.7, dur = 0.2, v = 1, a = 0.001 }) {
    const c = this.ctx, s = c.createBufferSource(), fl = c.createBiquadFilter(), g = c.createGain();
    s.buffer = this.bufs[buf]; s.loop = true; s.playbackRate.value = rate;
    fl.type = type; fl.Q.value = q;
    fl.frequency.setValueAtTime(f, at);
    if (f2) fl.frequency.exponentialRampToValueAtTime(f2, at + dur);
    g.gain.setValueAtTime(0, at);
    g.gain.linearRampToValueAtTime(v, at + a);
    g.gain.exponentialRampToValueAtTime(0.0001, at + dur);
    s.connect(fl); fl.connect(g); g.connect(dest);
    s.start(at, Math.random() * s.buffer.duration * 0.9); s.stop(at + dur + 0.02);
  },
  // throaty zombie voice: two detuned saws through vowel formants, wobbling and sagging
  growl(dest, at, { f, f2, dur, v, formant = 600, a = 0.08 }) {
    const c = this.ctx, g = c.createGain(), f1 = c.createBiquadFilter(), fm = c.createBiquadFilter();
    f1.type = 'bandpass'; f1.frequency.value = formant; f1.Q.value = 2.5;
    fm.type = 'bandpass'; fm.frequency.value = formant * 1.8; fm.Q.value = 4;
    const lfo = c.createOscillator(), ld = c.createGain();
    lfo.frequency.value = rnd(5, 9); ld.gain.value = f * 0.06; lfo.connect(ld);
    for (const det of [0, 9]) {
      const o = c.createOscillator(); o.type = 'sawtooth'; o.detune.value = det;
      o.frequency.setValueAtTime(f, at); o.frequency.exponentialRampToValueAtTime(f2, at + dur);
      ld.connect(o.frequency); o.connect(f1); o.connect(fm);
      o.start(at); o.stop(at + dur + 0.05);
    }
    lfo.start(at); lfo.stop(at + dur + 0.05);
    g.gain.setValueAtTime(0, at);
    g.gain.linearRampToValueAtTime(v, at + a);
    g.gain.setTargetAtTime(v * 0.7, at + a, dur * 0.3);
    g.gain.exponentialRampToValueAtTime(0.0001, at + dur);
    f1.connect(g); fm.connect(g); g.connect(dest);
  },
  // stereo position + distance falloff for something at world (x, z), relative to the player
  place(pos) {
    const lx = listenerObj ? listenerObj.position.x : 0;
    const lz = listenerObj ? listenerObj.position.z : 0;
    const dx = pos.x - lx, dz = pos.z - lz;
    return { pan: dx / 12, near: 1 / (1 + Math.hypot(dx, dz) / 7) };
  },
  // pull the music down for a moment (impacts, stingers)
  duckMusic(depth, dur) {
    const g = this.duck.gain, t = this.t();
    g.cancelScheduledValues(t); g.setValueAtTime(g.value, t);
    g.linearRampToValueAtTime(depth, t + 0.03);
    g.setTargetAtTime(1, t + dur, dur * 0.4);
  },
  // muffle the whole mix (concussion) — sounds on the `post` bus cut through
  muffleFor(dur, freq) {
    const f = this.muffle.frequency, t = this.t();
    f.cancelScheduledValues(t); f.setValueAtTime(freq, t);
    f.setTargetAtTime(20000, t + dur * 0.4, dur * 0.35);
  },

  // --- game sound effects ---
  shot(hot, spread) {
    if (!this.ok()) return;
    const t = this.t(), o = this.out(0.65, 0, 0.22);
    // crack: fast-swept pulse wave (the gunshot punch)
    this.osc(o, t, { w: 'pulse25', f: hot ? 850 : 680, f2: 45, dur: 0.11, v: 0.85, a: 0.001 });
    // mechanical bite + blast hiss
    this.nz(o, t, { buf: 'metal', type: 'bandpass', f: 3800, dur: 0.06, v: 0.9 });
    this.nz(o, t, { buf: 'noise', rate: 0.7, type: 'lowpass', f: 2400, f2: 300, dur: 0.18, v: 0.8 });
    if (spread) // BUCKSHOT extra blast
      this.nz(o, t + 0.01, { buf: 'noise', rate: 0.5, type: 'lowpass', f: 1800, dur: 0.22, v: 0.5 });
    // deep body sub
    this.osc(o, t, { w: 'sine', f: 120, f2: 38, dur: 0.14, v: 0.9 });
    this.duckMusic(0.75, 0.08);
  },
  // hit a zombie: juicy squelch + bone impact (placed in stereo)
  hit(pos) {
    if (!this.ok() || this.t() - this.lastHit < 0.03) return;
    this.lastHit = this.t();
    const { pan, near } = this.place(pos), t = this.t(), o = this.out(0.55 * near, pan, 0.18);
    this.nz(o, t, { buf: 'noise', rate: 0.85, type: 'bandpass', f: 1400, f2: 350, q: 2, dur: 0.13, v: 0.9 }); // squelch
    this.osc(o, t, { w: 'tri', f: 180, f2: 55, dur: 0.08, v: 0.6 });                                          // thud
    this.nz(o, t, { buf: 'metal', type: 'highpass', f: 4500, dur: 0.04, v: 0.35 });                           // bone click
  },
  // kill shot: deeper, wetter crunch with a low sub
  zdeath(pos) {
    if (!this.ok()) return;
    const { pan, near } = this.place(pos), t = this.t(), o = this.out(0.7 * near, pan, 0.35);
    this.nz(o, t, { buf: 'noise', rate: 0.5, type: 'lowpass', f: 1800, f2: 200, dur: 0.32, v: 1 });
    this.osc(o, t, { w: 'tri', f: 140, f2: 35, dur: 0.22, v: 0.9 });
    this.nz(o, t + 0.04, { buf: 'metal', rate: 0.6, type: 'bandpass', f: 2200, q: 1.5, dur: 0.15, v: 0.5 });
    // short death rasp
    this.growl(o, t, { f: rnd(65, 85), f2: 40, dur: 0.35, v: 0.45, formant: 420 });
  },
  // zombie vocal: wander groan, or aggro snarl
  groan(pos) {
    if (!this.ok()) return;
    const { pan, near } = this.place(pos), t = this.t(), o = this.out(0.4 * near, pan, 0.4);
    const f = rnd(55, 95), f2 = f * rnd(0.7, 0.95), dur = rnd(0.7, 1.4);
    this.growl(o, t, { f, f2, dur, v: rnd(0.35, 0.6), formant: rnd(450, 750) });
  },
  // claw swing through the air
  swipe(pos) {
    if (!this.ok()) return;
    const { pan, near } = this.place(pos), t = this.t(), o = this.out(0.45 * near, pan, 0.1);
    this.nz(o, t, { buf: 'white', type: 'bandpass', f: 1800, f2: 400, q: 1.8, dur: 0.18, v: 0.8 });
  },
  // claw lands on a wooden door
  bash(o, k = 1) {
    if (!this.ok()) return;
    const { pan, near } = this.place(o), t = this.t(), out = this.out(0.6 * near * k, pan, 0.25);
    this.osc(out, t, { w: 'tri', f: 110, f2: 42, dur: 0.18, v: 0.9 });                                    // heavy door slap
    this.nz(out, t, { buf: 'noise', rate: 0.6, type: 'bandpass', f: 1100, f2: 240, q: 1.2, dur: 0.2, v: 0.85 }); // wood groan
    this.nz(out, t, { buf: 'metal', type: 'highpass', f: 3000, dur: 0.05, v: 0.35 });                     // splinter click
  },
  // claw taps a window pane (first hit: cracks)
  crack(o) {
    if (!this.ok()) return;
    const { pan, near } = this.place(o), t = this.t(), out = this.out(0.6 * near, pan, 0.25);
    this.osc(out, t, { w: 'pulse12', f: 2200, f2: 900, dur: 0.04, v: 0.5 });
    this.nz(out, t, { buf: 'metal', type: 'highpass', f: 6500, dur: 0.12, v: 0.7 });
    this.nz(out, t, { buf: 'white', type: 'bandpass', f: 4200, q: 2, dur: 0.08, v: 0.5 });
  },
  // window smashes in
  shatter(o) {
    if (!this.ok()) return;
    const { pan, near } = this.place(o), t = this.t(), out = this.out(0.75 * near, pan, 0.4);
    this.nz(out, t, { buf: 'metal', type: 'highpass', f: 4500, dur: 0.45, v: 0.9 });                      // glass cascade
    this.nz(out, t, { buf: 'white', rate: 1, type: 'bandpass', f: 2600, q: 1.5, dur: 0.3, v: 0.7 });
    this.osc(out, t, { w: 'tri', f: 160, f2: 50, dur: 0.16, v: 0.7 });                                   // frame kick
    for (let i = 0; i < 4; i++)                                                                           // falling shards
      this.osc(out, t + 0.04 + i * 0.06, { w: 'pulse12', f: rnd(1800, 3400), dur: 0.03, v: 0.25 });
  },
  // door breaks off its hinges and crashes down
  doorBreak(o) {
    if (!this.ok()) return;
    const { pan, near } = this.place(o), t = this.t(), out = this.out(0.85 * near, pan, 0.45);
    this.osc(out, t, { w: 'sine', f: 90, f2: 32, dur: 0.35, v: 1 });                                     // heavy crash
    this.nz(out, t, { buf: 'noise', rate: 0.4, type: 'lowpass', f: 900, dur: 0.5, v: 1 });
    this.nz(out, t, { buf: 'metal', type: 'bandpass', f: 1800, q: 1.2, dur: 0.35, v: 0.8 });             // tearing hinges
  },
  // survivor footstep: alternate soft wood taps
  step() {
    if (!this.ok()) return;
    const t = this.t(), o = this.out(0.2, 0, 0.08);
    this.stepAlt = !this.stepAlt;
    this.osc(o, t, { w: 'tri', f: this.stepAlt ? 130 : 115, f2: 55, dur: 0.05, v: 0.45 });
    this.nz(o, t, { buf: 'noise', rate: 0.9, type: 'lowpass', f: 1200, dur: 0.04, v: 0.3 });
  },
  reloadOut() {
    if (!this.ok()) return;
    const t = this.t(), o = this.out(0.4, 0.1, 0.1);
    this.osc(o, t, { w: 'pulse25', f: 720, f2: 340, dur: 0.05, v: 0.35 });
    this.nz(o, t, { buf: 'metal', type: 'bandpass', f: 2400, dur: 0.07, v: 0.5 });
  },
  reloadIn() {
    if (!this.ok()) return;
    const t = this.t(), o = this.out(0.55, 0.05, 0.12);
    this.osc(o, t, { w: 'pulse25', f: 420, f2: 880, dur: 0.06, v: 0.5 });
    this.nz(o, t + 0.02, { buf: 'metal', type: 'highpass', f: 3200, dur: 0.09, v: 0.6 });
    this.osc(o, t + 0.06, { w: 'tri', f: 210, f2: 95, dur: 0.06, v: 0.4 }); // seating click
  },
  // survivor takes damage: sickening punch, heart drop, concussion muffle
  hurt() {
    if (!this.ok()) return;
    const t = this.t(), o = this.out(0.9, 0, 0.35, this.post); // cuts through the muffle
    this.osc(o, t, { w: 'sine', f: 85, f2: 30, dur: 0.35, v: 1 });
    this.nz(o, t, { buf: 'noise', rate: 0.4, type: 'lowpass', f: 600, dur: 0.25, v: 0.9 });
    this.muffleFor(1.2, 550);
    this.duckMusic(0.15, 0.9);
  },
  pickup(kind) {
    if (!this.ok()) return;
    const t = this.t(), o = this.out(0.5, 0, 0.3);
    // crisp ascending 8-bit arpeggio
    const notes = kind === 'med'  ? [60, 64, 67, 72] : // major
                  kind === 'dmg'  ? [58, 62, 65, 70] : // minor / aggressive
                                    [55, 60, 67, 72];   // 4ths / tech
    notes.forEach((m, i) => {
      this.osc(o, t + i * 0.055, { w: 'pulse25', f: midiHz(m), dur: 0.18, v: 0.3 });
      this.osc(o, t + i * 0.055, { w: 'tri', f: midiHz(m - 12), dur: 0.12, v: 0.25 });
    });
    this.nz(o, t, { buf: 'metal', rate: 1, type: 'highpass', f: 6000, dur: 0.4, v: 0.08 });       // shimmer
  },

  waveStart(n) {
    musicInstance?.setMode('combat');
    if (!this.ok()) return;
    const t = this.t(), o = this.out(0.6, 0, 0.6);
    // tolling bell: inharmonic partials with slow beating
    [[1, 1], [2.01, 0.5], [2.76, 0.4], [5.4, 0.2], [8.93, 0.1]].forEach(([r, v]) =>
      this.osc(o, t, { w: 'sine', f: midiHz(50) * r, dur: 3.2 / Math.sqrt(r), v: v * 0.5, a: 0.005 }));
    this.osc(o, t, { w: 'tri', f: midiHz(38), dur: 1.6, v: 0.5 });
    // the classic horror semitone: A – Bb – A
    if (n > 1) [57, 58, 57].forEach((m, i) => this.osc(o, t + 0.12 + i * 0.22, { w: 'pulse25', f: midiHz(m), hold: 0.18, v: 0.18 }));
    this.duckMusic(0.35, 1.2);
  },
  waveClear() {
    musicInstance?.setMode('calm');
    if (!this.ok()) return;
    const t = this.t(), o = this.out(0.55, 0, 0.35);
    // chip fanfare that resolves to D major — a moment of hope
    [62, 66, 69, 74].forEach((m, i) => this.osc(o, t + i * 0.09, { w: 'pulse25', f: midiHz(m), hold: 0.08, v: 0.3 }));
    this.osc(o, t + 0.36, { w: 'pulse25', f: midiHz(78), hold: 0.7, v: 0.3, vib: 0.3 });
    this.osc(o, t + 0.36, { w: 'pulse12', f: midiHz(74), hold: 0.7, v: 0.15 });
    this.osc(o, t + 0.36, { w: 'tri', f: midiHz(50), hold: 1, v: 0.5 });
    this.nz(o, t + 0.36, { buf: 'metal', type: 'highpass', f: 5000, dur: 1, v: 0.12 });
    this.duckMusic(0, 1.4);
  },
  deal(count) {
    if (!this.ok()) return;
    const t = this.t();
    for (let i = 0; i < count; i++) {
      const o = this.out(0.4, (i - (count - 1) / 2) * 0.5, 0.1);
      this.nz(o, t + 0.05 + i * 0.09, { buf: 'white', type: 'bandpass', f: 3200, f2: 1400, q: 1.5, dur: 0.07, v: 0.7 });
      this.osc(o, t + 0.05 + i * 0.09, { w: 'pulse12', f: midiHz(74 + i * 5), dur: 0.05, v: 0.12 });
    }
  },
  hover(i) {
    if (!this.ok() || this.t() - this.lastHover < 0.05) return;
    const t = this.lastHover = this.t();
    this.osc(this.out(0.3, (i - 1) * 0.4, 0.1), t, { w: 'pulse12', f: midiHz(81 + i * 2), dur: 0.04, v: 0.25 });
  },
  upgrade(cat) {
    if (!this.ok()) return;
    const t = this.t(), o = this.out(0.55, 0, 0.35), up = { firepower: 0, handling: 2, survival: 5, fortune: 7 }[cat] || 0;
    this.osc(o, t, { w: 'pulse12', f: 180, f2: 1700, dur: 0.3, v: 0.12 });                       // power-up sweep
    [62, 69, 74, 78, 81].forEach((m, i) => this.osc(o, t + i * 0.05, { w: 'pulse25', f: midiHz(m + up), dur: 0.2, v: 0.25 }));
  },
  death() {
    musicInstance?.setMode('dead');
    if (!this.ok()) return;
    const t = this.t() + 0.35, o = this.out(0.6, 0, 0.45, this.post);
    // the band sags into the floor, then the classic sad-trombone chromatic fall
    [69, 68, 67].forEach((m, i) => this.osc(o, t + i * 0.32, { w: 'pulse25', f: midiHz(m), hold: 0.26, v: 0.28 }));
    this.osc(o, t + 0.96, { w: 'pulse25', f: midiHz(66), f2: midiHz(61), hold: 1.3, v: 0.28, vib: 0.5 });
    [57, 56, 55, 50].forEach((m, i) => this.osc(o, t + i * 0.32, { w: 'tri', f: midiHz(m - 12), hold: i < 3 ? 0.26 : 1.6, v: 0.5 }));
    [[1, 1], [2.76, 0.4], [5.4, 0.2]].forEach(([r, v]) =>
      this.osc(o, t + 0.96, { w: 'sine', f: midiHz(38) * r, dur: 3, v: v * 0.35, a: 0.005 }));
  },
  toggleMute() {
    this.muted = !this.muted;
    if (this.ctx) this.applyVolume();
    return this.muted;
  },
  pause(on) {
    if (!this.ctx) return;
    // pull master down and cut reverb send so unpause doesn't wash
    this.master.gain.setTargetAtTime(on ? 0 : 0.8, this.t(), 0.05);
  },
  // the settings sliders run 0..10, where 8 is the mix as designed (10 is +4 dB, and the
  // limiter catches it); squared so each step sounds like an even step
  level() { return this.muted ? 0 : (settings.master / 8) ** 2; },
  applyVolume() {
    if (!this.ctx) return;
    const t = this.t(), m = (settings.music / 8) ** 2;
    this.mute.gain.setTargetAtTime(this.level(), t, 0.03);
    this.musicVol.gain.setTargetAtTime(m, t, 0.03);
    this.musicVerb.gain.setTargetAtTime(m, t, 0.03);
  },
  // menu blips: quiet and short
  ui(kind) {
    if (!this.ok() || this.t() - this.lastHover < 0.04) return;
    const t = this.lastHover = this.t(), o = this.out(0.3, 0, 0.12);
    if (kind === 'move') this.osc(o, t, { w: 'pulse12', f: midiHz(81), dur: 0.035, v: 0.2 });
    else if (kind === 'back') this.osc(o, t, { w: 'pulse12', f: midiHz(74), f2: midiHz(69), dur: 0.07, v: 0.18 });
    else [76, 83].forEach((m, i) => this.osc(o, t + i * 0.05, { w: 'pulse12', f: midiHz(m), dur: 0.06, v: 0.18 }));
  },
};

// Set up user gesture audio initialization listeners
export function setupAudioListeners() {
  audio.ensure();
  function initAudio() {
    audio.ensure();
    ['click', 'keydown', 'touchstart'].forEach(e => document.removeEventListener(e, initAudio));
  }
  ['click', 'keydown', 'touchstart'].forEach(e => document.addEventListener(e, initAudio));
  // park the audio clock while the tab is hidden so the score doesn't stutter
  document.addEventListener('visibilitychange', () => {
    if (!audio.ctx) return;
    if (document.hidden) audio.ctx.suspend(); else audio.ctx.resume();
  });
}
