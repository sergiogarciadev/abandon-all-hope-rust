import { midiHz } from '../core/math.js';
import { audio, setMusicInstance } from './audio.js';

// chords: [bass, ...pad voicing]. Mostly rootless voicings (the bass has the root),
// picked so each voice moves a step or less into the next chord
export const CH = {
  Dm9:       [38, 53, 57, 60, 64],  // F A C E
  Dmadd9:    [38, 53, 57, 62, 64],  // F A D E
  Bbmaj9:    [34, 53, 57, 60, 62],  // F A C D
  Bbmaj7:    [34, 53, 58, 62, 69],  // F Bb D A
  Bbmaj7s11: [34, 52, 57, 62, 65],  // E A D F
  Gm9:       [43, 53, 58, 62, 69],  // F Bb D A
  Gm7:       [43, 53, 58, 62, 67],  // F Bb D G
  Eo7:       [40, 52, 55, 58, 62],  // E G Bb D
  A7:        [45, 55, 61, 64, 69],  // G C# E A
  A7b9:      [45, 55, 61, 64, 70],  // G C# E Bb — the b9 pulls hard back to D minor
  Fmaj7:     [41, 53, 57, 60, 64],  // F A C E
  Fmaj9:     [41, 55, 57, 60, 64],  // G A C E
  Cadd9:     [36, 55, 60, 62, 64],  // G C D E
  C69:       [36, 55, 57, 62, 64],  // G A D E
  C7sus:     [36, 53, 55, 58, 60],  // F G Bb C
  C7:        [36, 52, 55, 58, 60],  // E G Bb C
  Am7:       [45, 55, 60, 64, 69],  // G C E A
  Ebmaj7s11: [39, 55, 58, 62, 69],  // G Bb D A — Neapolitan, the darkest turn in the score
};

// melodies: [16th step in the 8-bar section, midi note, length in 16ths]
export const tune = notes => new Map(notes.map(([s, m, l]) => [s, { m, l }]));

export const THEME = tune([ // the main theme, everywhere in D minor
  [0, 69, 4], [4, 74, 2], [6, 77, 2], [8, 76, 6], [14, 74, 2],
  [16, 77, 4], [20, 74, 4], [24, 70, 8],
  [32, 67, 2], [34, 70, 2], [36, 74, 4], [40, 79, 6], [46, 77, 2],
  [48, 76, 4], [52, 73, 4], [56, 69, 8],
  [64, 69, 4], [68, 74, 2], [70, 77, 2], [72, 81, 6], [78, 79, 2],
  [80, 77, 4], [84, 81, 4], [88, 84, 4], [92, 81, 4],
  [96, 79, 4], [100, 77, 2], [102, 74, 2], [104, 70, 4], [108, 74, 4],
  [112, 73, 4], [116, 76, 4], [120, 81, 4], [124, 73, 4],
]);
export const SOAR = tune([ // the climax: long lines climbing to the b9 over the dominant
  [0, 74, 6], [6, 72, 2], [8, 74, 4], [12, 77, 4],
  [16, 76, 8], [24, 74, 4], [28, 72, 4],
  [32, 72, 4], [36, 76, 4], [40, 81, 8],
  [48, 79, 4], [52, 77, 4], [56, 76, 4], [60, 74, 4],
  [64, 74, 6], [70, 72, 2], [72, 74, 4], [76, 81, 4],
  [80, 79, 8], [88, 76, 4], [92, 79, 4],
  [96, 82, 8], [104, 81, 4], [108, 79, 4],
  [112, 82, 4], [116, 81, 12],
]);
export const OMEN = tune([ // the breakdown: the wave bell's A–Bb–A, then a climb back into the fight
  [0, 81, 12], [12, 82, 4], [16, 81, 16],
  [32, 79, 6], [38, 77, 2], [40, 74, 8], [48, 76, 16],
  [64, 81, 12], [76, 82, 4], [80, 81, 16],
  [96, 82, 8], [104, 81, 4], [108, 79, 4],
  [112, 73, 4], [116, 76, 4], [120, 79, 4], [124, 82, 4],
]);
export const HAVEN = tune([ // the theme's shape in F major for the safe room, left hanging on the b9
  [0, 76, 6], [6, 74, 2], [8, 72, 8],
  [16, 72, 4], [20, 76, 4], [24, 79, 8],
  [32, 77, 6], [38, 76, 2], [40, 74, 8],
  [48, 72, 4], [52, 77, 4], [56, 76, 8],
  [64, 81, 6], [70, 79, 2], [72, 77, 8],
  [80, 76, 4], [84, 77, 4], [88, 81, 8],
  [96, 82, 6], [102, 81, 2], [104, 79, 4], [108, 77, 4],
  [112, 76, 4], [116, 73, 4], [120, 70, 8],
]);

// 8-bar sections: one or two chords a bar (two = a change at the half bar)
export const SECTIONS = {
  A: { bars: [['Dm9'], ['Bbmaj9'], ['Gm9'], ['A7', 'A7b9'], ['Dmadd9'], ['Fmaj7'], ['Gm7', 'Eo7'], ['A7b9']], mel: THEME },
  B: { bars: [['Bbmaj7'], ['Cadd9'], ['Am7'], ['Dm9'], ['Bbmaj7'], ['C69'], ['Ebmaj7s11'], ['A7b9']], mel: SOAR },
  C: { bars: [['Dmadd9'], ['Bbmaj7s11'], ['Gm9'], ['Dm9'], ['Dmadd9'], ['Ebmaj7s11'], ['Gm7'], ['A7b9']], mel: OMEN, pedal: 38 },
  I: { bars: [['Bbmaj7'], ['Gm9'], ['Ebmaj7s11'], ['Dmadd9'], ['Bbmaj7s11'], ['Gm9'], ['Eo7'], ['A7b9']] }, // title interlude
  F: { bars: [['Fmaj9'], ['Am7'], ['Bbmaj7'], ['C7sus', 'C7'], ['Fmaj9'], ['Dm9'], ['Gm9'], ['A7b9']], mel: HAVEN },
};

// a second voice under a melody note: the nearest chord tone at least a minor third below
export function harmonyUnder(m, chord) {
  for (let t = m - 3; t >= m - 9; t--) if (chord.some(c => (c - t) % 12 === 0)) return t;
  return null;
}
export const ARP_UPDOWN = [0, 1, 2, 3, 4, 5, 6, 7, 6, 5, 4, 3, 2, 1, 2, 3];
export const hum = v => v * (0.9 + Math.random() * 0.2); // a little velocity life

export const music = {
  mode: 'off', pending: null, step: 0, next: 0, timer: null, layer: 1, lowHp: false,
  sec: 'A', secN: 0, vary: false, // current section, sections played in this mode, second-pass variation
  lead: null,                      // last lead note { m, end }, for legato glides
  wave: 0, alive: 0,               // fed by the game loop
  start() {
    const c = audio.ctx;
    // persistent voice buses, each panned into the music chain
    const bus = (pan, to, v = 1) => {
      const g = c.createGain(); g.gain.value = v;
      if (pan && c.createStereoPanner) { const p = c.createStereoPanner(); p.pan.value = pan; g.connect(p); p.connect(to); }
      else g.connect(to);
      return g;
    };
    // sidechain: pads and arps duck under every kick, so the low end breathes
    this.pump = bus(0, audio.musicIn);
    this.pumpWet = bus(0, audio.musicWet);
    this.padLP = [-0.55, 0.55].map(pan => {
      const f = c.createBiquadFilter(); f.type = 'lowpass'; f.frequency.value = 900; f.Q.value = 0.9;
      f.connect(bus(pan, this.pump));
      return f;
    });
    this.arp = [bus(-0.35, this.pumpWet), bus(0.35, this.pumpWet)];
    this.leadBus = bus(0.08, audio.musicWet);
    this.harmBus = bus(-0.22, audio.musicWet);
    this.bellBus = bus(0.15, audio.musicWet);
    this.keysBus = bus(-0.1, audio.musicWet);
    this.bassBus = bus(0, audio.musicIn);
    this.kit = bus(0, audio.musicIn);
    this.hatBus = bus(0.25, audio.musicIn);
    this.crashBus = bus(-0.3, audio.musicIn);
    this.next = audio.t() + 0.1;
    this.timer = setInterval(() => this.tick(), 25);
    this.setMode('title');
  },
  setMode(m) {
    if (!audio.ctx || m === (this.pending || this.mode)) return;
    if (m !== 'dead') { this.pending = m; return; } // lands on the next beat (see advance)
    this.pending = null;
    this.mode = 'dead';
    const t = audio.t();
    audio.musicLP.frequency.cancelScheduledValues(t);
    audio.musicLP.frequency.setTargetAtTime(120, t, 0.25);
    audio.duck.gain.cancelScheduledValues(t);
    audio.duck.gain.setTargetAtTime(0, t, 0.3);
    clearTimeout(this.reviveT);
    this.reviveT = setTimeout(() => this.setMode('title'), 4200);
  },
  // switch modes on a beat, starting the new mode's score from the top
  enter(m) {
    this.pending = null;
    if (m === this.mode) return;
    const prev = this.mode;
    this.mode = m;
    this.step = 0; this.secN = 0; this.lead = null;
    clearTimeout(this.reviveT);
    if (prev === 'dead') audio.duck.gain.setTargetAtTime(1, audio.t(), 0.8);
    this.filter();
  },
  // music lowpass: wide open, darker between waves, choked while you're on your last heart
  filter() {
    const f = this.mode === 'dead' ? 120 : this.lowHp ? 900 : this.mode === 'calm' ? 6000 : 18000;
    audio.musicLP.frequency.setTargetAtTime(f, audio.t(), 0.4);
  },
  heat() { return 0.1 + this.wave * 0.06 + this.alive / 30 * 0.6; },
  bpm() {
    if (this.mode === 'combat') return Math.min(156, 126 + this.wave * 2);
    return this.mode === 'calm' ? 92 : 76;
  },
  swing() { return this.mode === 'calm' ? 0.16 : this.mode === 'title' ? 0.06 : 0; },
  sixteenth() { return 60 / this.bpm() / 4; },
  dur(steps) { return steps * this.sixteenth(); },
  tick() {
    const c = audio.ctx;
    if (!c || c.state !== 'running') return;
    const lowHp = this.mode === 'combat' && this.lowHp;
    if (lowHp !== this._lowHp) { this._lowHp = lowHp; this.filter(); }
    audio.echo.delayTime.setTargetAtTime(this.sixteenth() * 3, c.currentTime, 0.1); // dotted-8th echo
    if (this.next < c.currentTime) this.next = c.currentTime + 0.02; // fell behind (tab was hidden)
    while (this.next < c.currentTime + 0.12) this.advance();
  },
  advance() {
    if (this.pending && this.step % 4 === 0) this.enter(this.pending);
    const swing = this.step % 2 ? this.swing() * this.sixteenth() : 0; // late off-beats
    if (!audio.muted && this.mode !== 'dead' && this.mode !== 'off') this.play(this.step, this.next + swing);
    this.next += this.sixteenth();
    this.step++;
  },
  // which 8-bar section comes next. The fight goes A → B → A' → (C, or B' when it's hot)
  pickSection() {
    const n = this.secN++;
    if (this.mode === 'title') { this.sec = n % 3 === 2 ? 'I' : 'A'; this.vary = n % 3 === 1; return; }
    if (this.mode === 'calm') { this.sec = 'F'; this.vary = n % 2 === 1; return; }
    const k = n % 4;
    this.sec = k === 1 ? 'B' : k === 3 ? (this.heat() >= 0.55 ? 'B' : 'C') : 'A';
    this.vary = k >= 2;
  },

  // --- instruments ---
  // one enveloped oscillator: attack `a`, optional decay toward decay[0]×v (time constant
  // decay[1]), held for `len`, released over ~r. `from` glides in from another note.
  voice(dest, at, { w = 'pulse25', m, len, v, a = 0.005, r = 0.08, vib = 0, from, det = 0, decay }) {
    const c = audio.ctx, o = c.createOscillator(), g = c.createGain(), f = midiHz(m);
    if (audio.waves[w]) o.setPeriodicWave(audio.waves[w]); else o.type = w;
    o.detune.value = det;
    if (from !== undefined) { o.frequency.setValueAtTime(midiHz(from), at); o.frequency.exponentialRampToValueAtTime(f, at + 0.05); }
    else o.frequency.setValueAtTime(f, at);
    if (vib && len > 0.3) { // delayed vibrato, like a held note on a real instrument
      const l = c.createOscillator(), d = c.createGain();
      l.frequency.value = 5.2; d.gain.setValueAtTime(0, at + 0.18);
      d.gain.linearRampToValueAtTime(f * (Math.pow(2, vib / 12) - 1), at + Math.min(len, 0.5));
      l.connect(d); d.connect(o.frequency); l.start(at); l.stop(at + len + r * 2);
    }
    const end = at + Math.max(len, a);
    g.gain.setValueAtTime(0, at);
    g.gain.linearRampToValueAtTime(v, at + a);
    if (decay) g.gain.setTargetAtTime(v * decay[0], at + a, decay[1]);
    g.gain.setTargetAtTime(0, end, r / 3);
    o.connect(g); g.connect(dest); o.start(at); o.stop(end + r * 2);
  },
  // stereo pad: a 25% and a 50% pulse per note, detuned apart and split left/right
  pad(at, notes, steps, v, a) {
    for (const m of notes) [0, 1].forEach(k =>
      this.voice(this.padLP[k], at, { w: k ? 'pulse50' : 'pulse25', m, len: this.dur(steps), v, a, r: 0.7, det: k ? 8 : -8 }));
  },
  padTone(at, f) { for (const lp of this.padLP) lp.frequency.setTargetAtTime(f, at, 0.4); },
  bass(at, m, steps, v) {
    const len = this.dur(steps) * 0.85;
    this.voice(this.bassBus, at, { w: 'tri', m, len, v, a: 0.004, r: 0.05 });
    this.voice(this.bassBus, at, { w: 'sine', m, len, v: v * 0.5, a: 0.004, r: 0.05 }); // body under the 4-bit grit
  },
  leadNote(at, m, steps, v, { bus = this.leadBus, w = 'pulse25', vib = 0.22 } = {}) {
    // long legato notes slide in from the last one
    const l = this.lead, glide = l && steps >= 4 && Math.abs(at - l.end) < 0.04 && Math.abs(m - l.m) <= 5 ? l.m : undefined;
    this.voice(bus, at, { w, m, len: this.dur(steps) * 0.92, v, a: 0.01, r: 0.12, vib: steps >= 4 ? vib : 0, from: glide });
    if (bus === this.leadBus) this.lead = { m, end: at + this.dur(steps) };
  },
  // music-box tine: a bright strike that rings down, with a quick metallic overtone
  bell(at, m, v) {
    this.voice(this.bellBus, at, { w: 'sine', m, len: 0.01, v, a: 0.002, r: 1.8 });
    this.voice(this.bellBus, at, { w: 'sine', m: m + 12, len: 0.01, v: v * 0.3, a: 0.002, r: 0.5 });
    this.voice(this.bellBus, at, { w: 'tri', m: m + 24, len: 0.01, v: v * 0.1, a: 0.001, r: 0.12 });
  },
  // electric piano: a sine that blooms and settles, with a little tine bark on top
  keys(at, m, steps, v) {
    this.voice(this.keysBus, at, { w: 'sine', m, len: this.dur(steps), v, a: 0.004, r: 0.5, decay: [0.45, 0.4] });
    this.voice(this.keysBus, at, { w: 'tri', m: m + 12, len: 0.04, v: v * 0.16, a: 0.002, r: 0.2 });
  },
  kick(at, v = 0.9) {
    audio.osc(this.kit, at, { w: 'sine', f: 160, f2: 44, dur: 0.18, v });
    audio.nz(this.kit, at, { buf: 'white', type: 'highpass', f: 2500, dur: 0.012, v: v * 0.22 }); // beater click
    for (const p of [this.pump, this.pumpWet]) { p.gain.setValueAtTime(0.3, at); p.gain.setTargetAtTime(1, at + 0.03, 0.09); }
  },
  snare(at, v = 0.5) {
    audio.nz(this.kit, at, { rate: 0.55, type: 'bandpass', f: 2200, q: 0.8, dur: 0.16, v });
    audio.osc(this.kit, at, { w: 'tri', f: 230, f2: 150, dur: 0.07, v: v * 0.6 });
  },
  hat(at, v = 0.12, open = false) {
    audio.nz(this.hatBus, at + (Math.random() - 0.5) * 0.004, { buf: 'metal', type: 'highpass', f: 7000, dur: open ? 0.2 : 0.035, v: hum(v) });
  },
  brush(at, v) { audio.nz(this.hatBus, at, { buf: 'white', type: 'bandpass', f: 3500, q: 0.7, dur: 0.14, v: hum(v), a: 0.03 }); },
  crash(at, v = 0.22) {
    audio.nz(this.crashBus, at, { buf: 'white', type: 'highpass', f: 4500, dur: 1.8, v });
    audio.nz(this.crashBus, at, { buf: 'metal', type: 'highpass', f: 8000, dur: 1.1, v: v * 0.4 });
  },
  tom(at, f, v = 0.6) { audio.osc(this.kit, at, { w: 'sine', f, f2: f * 0.55, dur: 0.22, v }); },
  // filtered noise swelling into the next downbeat
  riser(at, len, v) {
    const c = audio.ctx, s = c.createBufferSource(), f = c.createBiquadFilter(), g = c.createGain();
    s.buffer = audio.bufs.white; s.loop = true;
    f.type = 'bandpass'; f.Q.value = 1.4;
    f.frequency.setValueAtTime(300, at); f.frequency.exponentialRampToValueAtTime(6000, at + len);
    g.gain.setValueAtTime(0.0001, at); g.gain.exponentialRampToValueAtTime(v, at + len);
    g.gain.linearRampToValueAtTime(0, at + len + 0.03);
    s.connect(f); f.connect(g); g.connect(this.crashBus); s.start(at); s.stop(at + len + 0.05);
  },
  heart(at) {
    const o = audio.out(0.9, 0, 0, audio.post);
    audio.osc(o, at, { w: 'sine', f: 62, f2: 40, dur: 0.14, v: 1 });
    audio.osc(o, at + 0.16, { w: 'sine', f: 55, f2: 38, dur: 0.12, v: 0.7 });
  },

  play(step, at) {
    if (step % 128 === 0) this.pickSection();
    const s = step % 16, bar = (step >> 4) % 8, sec = SECTIONS[this.sec];
    const chords = sec.bars[bar], ch = CH[chords[s < 8 ? 0 : chords.length - 1]];
    const bassOf = b => sec.pedal && b < 7 ? sec.pedal : CH[sec.bars[b][0]][0];
    const g = {
      s, bar, sec, ch, pad: ch.slice(1), root: bassOf(bar), bassOf,
      change: s === 0 || (s === 8 && chords.length === 2), // a chord lands here
      span: chords.length === 2 ? 8 : 16,                   // ...and lasts this many 16ths
      mel: sec.mel?.get(step % 128),
    };
    if (this.mode === 'title') this.playTitle(at, g);
    else if (this.mode === 'calm') this.playCalm(at, g);
    else this.playCombat(at, g);
  },

  playTitle(at, { s, ch, pad, root, change, span, mel }) {
    // a slow pad and a held low root under everything
    if (change) { this.pad(at, pad, span, 0.026, 0.9); this.padTone(at, 1100); }
    if (s === 0) this.voice(this.bassBus, at, { w: 'tri', m: root, len: this.dur(15), v: 0.28, a: 0.3, r: 0.8 });
    if (this.sec === 'I') {
      // interlude: stray chimes picked from the chord, high and far apart
      if (s % 6 === 3 && Math.random() < 0.6) this.bell(at, pad[(Math.random() * 4) | 0] + 12, 0.08);
      return;
    }
    if (mel) {
      this.bell(at, mel.m, 0.27);
      if (this.vary) { const h = harmonyUnder(mel.m, ch); if (h) this.bell(at + 0.012, h, 0.12); } // second pass: in harmony
    }
    if (this.vary && s % 4 === 2) this.voice(this.arp[(s >> 2) & 1], at, { w: 'pulse12', m: pad[(s >> 2) % 4] + 12, len: this.dur(2), v: 0.035 });
  },

  playCalm(at, { s, bar, ch, pad, root, bassOf, change, span, mel }) {
    // electric piano comping: a rolled chord on the change, a short stab on the "and" of 3
    if (change) pad.forEach((m, i) => this.keys(at + i * 0.018, m, span, hum(0.05)));
    else if (s === 10 && span === 16) pad.forEach((m, i) => this.keys(at + i * 0.012, m, 2, hum(0.032)));
    if (change) this.pad(at, pad, span, 0.008, 0.4);
    if (s === 0) this.padTone(at, 1400);
    // walking bass: root, fifth, octave, then a half step into the next bar's root
    if (s % 4 === 0) {
      const next = bassOf((bar + 1) % 8);
      this.bass(at, [root, root + 7, root + 12, next > root ? next - 1 : next + 1][s >> 2], 4, hum(0.24));
    }
    // brushes: swept backbeat, ticking quarters
    if (s === 4 || s === 12) this.brush(at, 0.05);
    if (s % 4 === 0) this.hat(at, 0.035);
    else if (s % 2 === 0) this.hat(at, 0.02);
    // the theme in F major on the first pass; the second leaves room for the keys
    if (mel && !this.vary) this.leadNote(at, mel.m, mel.l, 0.13, { w: 'tri', vib: 0.15 });
  },

  playCombat(at, { s, bar, ch, pad, root, bassOf, change, span, mel }) {
    // pick the arrangement layer at each bar line so changes land on the beat
    if (s === 0) { const h = this.heat(); this.layer = h < 0.35 ? 1 : h < 0.7 ? 2 : 3; }
    const L = this.layer, half = this.sec === 'C', last = bar === 7;
    // pad: opens up with the layer; the breakdown sweeps it open across its 8 bars
    if (change) this.pad(at, pad, span, 0.014 + 0.007 * L, 0.06);
    if (s === 0) this.padTone(at, half ? 500 + bar * 320 : 650 + L * 650);
    if (s === 0 && bar === 0 && this.secN > 1 && L >= 2) this.crash(at);

    if (half) {
      // --- breakdown: half time, a pedal in the bass, the omen on the bells ---
      if (s === 0) this.bass(at, root, 16, 0.42);
      if (last && s % 2 === 0) this.bass(at, root + 12 * ((s >> 1) & 1), 2, 0.35); // pumps back up into the fight
      if (s === 0 || (L >= 2 && s === 11)) this.kick(at, 0.85);
      if (s === 8 && !last) this.snare(at, 0.55);
      if (s % 4 === 2) this.hat(at, 0.07);
      if (last) {
        if (s === 0) this.riser(at, this.dur(16), 0.18);
        if (s >= 8 || s % 2 === 0) this.snare(at, 0.12 + s * 0.02); // roll, building
      }
      if (mel) this.bell(at, mel.m, 0.2);
      if (this.lowHp && s % 4 === 0) this.heart(at);
      return;
    }

    // --- bass: octave-jumping 8ths, galloping 16ths at full intensity, and a
    // chromatic step into the next bar's root on the last 8th ---
    const gallop = [1, 0, 1, 1, 1, 0, 1, 1, 1, 0, 1, 1, 1, 0, 1, 1];
    if (L < 3 ? s % 2 === 0 : gallop[s]) {
      let m = root + (s % 4 === 2 ? 12 : 0);
      if (s === 14 && !last) { const nb = bassOf(bar + 1); if (nb !== root) m = nb + (nb > root ? -1 : 1); }
      this.bass(at, m, L < 3 ? 2 : 1, hum(0.3 + 0.03 * L));
    }

    // --- drums, with a tom fill into each new section ---
    const fill = last && s >= 12 && L >= 2;
    if (fill) this.tom(at, [200, 165, 135, 105][s - 12]);
    else {
      if (s === 0 || s === 8 || (L >= 2 && s === 10) || (L >= 3 && s === 3)) this.kick(at);
      if (L >= 2 && (s === 4 || s === 12)) this.snare(at, hum(0.5));
      if (L >= 3 && (s === 7 || s === 15)) this.snare(at, hum(0.1)); // ghost notes
      const open = this.sec === 'B' && L >= 2 && s % 4 === 2;
      if (L >= 3 || s % 2 === 0) this.hat(at, open ? 0.1 : s % 4 === 2 ? 0.16 : 0.08, open);
    }

    // --- arpeggio over the pad voicing, ping-ponging across the stereo field ---
    const arp = [...pad, ...pad.map(n => n + 12)];
    if (L === 1 ? s % 4 === 2 : L === 2 ? s % 2 === 0 : true) {
      const i = L === 1 ? 4 + ((s >> 2) & 3) : L === 2 ? (s >> 1) : ARP_UPDOWN[s];
      this.voice(this.arp[s & 2 ? 1 : 0], at + (Math.random() - 0.5) * 0.004, { w: 'pulse12', m: arp[i], len: this.dur(L === 3 ? 1 : 2) * 0.8, v: hum(0.042), r: 0.05 });
    }

    // --- the tune: soft triangle at layer 1, the pulse lead from 2, doubled an octave
    // down at 3, and a twin-lead harmony on second passes and in the climax ---
    if (mel) {
      if (L === 1) this.leadNote(at, mel.m, mel.l, 0.12, { w: 'tri' });
      else this.leadNote(at, mel.m, mel.l, 0.18);
      if (L >= 3) this.voice(this.leadBus, at, { w: 'pulse50', m: mel.m - 12, len: this.dur(mel.l) * 0.9, v: 0.06, r: 0.1 });
      if (L >= 2 && (this.vary || this.sec === 'B')) {
        const h = harmonyUnder(mel.m, ch);
        if (h) this.leadNote(at, h, mel.l, 0.09, { bus: this.harmBus, w: 'pulse50', vib: 0.15 });
      }
    }
    if (this.lowHp && s % 4 === 0) this.heart(at);
  },
};

// Register music instance with audio so audio events (waveStart, death, etc.) can drive modes
setMusicInstance(music);
audio.onEnsureCallback = () => music.start();
