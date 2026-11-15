import * as THREE from 'three';
import { scene } from '../core/scene.js';
import { rnd } from '../core/math.js';
import { state } from '../core/state.js';
import { audio } from '../audio/audio.js';
import { ambient, moon, worldMat, ground, setWeatherRef } from './world.js';
import { player } from '../entities/player.js';

// WEATHER — the night turns as the waves climb: drizzle from wave 5, steady rain by
// 8, a downpour with lightning and thunder by 12, and a howling storm by 15. One
// storm level (0..1) drives all of it: how much rain falls, how fast and how slanted,
// splashes on the ground, wet surfaces, dimmer moonlight, closer fog, how hard the wind
// blows in the mix, and how often lightning strikes. (The rain itself makes no sound.)
export const STORM_STEPS = [[4, 0], [5, 0.15], [8, 0.4], [12, 0.7], [15, 1]]; // [wave, storm level]
export const BOLT_LEVEL = 0.7; // lightning from here (wave 12), striking more and more often up to the full storm

export function stormFor(wave) {
  for (let i = 1; i < STORM_STEPS.length; i++) {
    const [w0, s0] = STORM_STEPS[i - 1], [w1, s1] = STORM_STEPS[i];
    if (wave <= w1) return wave <= w0 ? s0 : s0 + (s1 - s0) * (wave - w0) / (w1 - w0);
  }
  return 1;
}

export const NIGHT = new THREE.Color(0x05060a), BOLT_SKY = new THREE.Color(0x3a4458);

export const weather = {
  level: 0, target: 0, t: 0, drop: 0, gust: 1,
  flashT: -1, flash: 0, nextBolt: 8, thunderIn: -1, thunderNear: 0, splashI: 0, splashDue: 0, soundT: 0,
  set(level) {
    this.level = this.target = level;
    this.flashT = -1;
    this.thunderIn = -1;
    this.nextBolt = 8;
  },
};
setWeatherRef(weather);

// rain: streaks falling through a box that follows the survivor, placed by the GPU
const RAIN_MAX = 7000, RAIN_AREA = 72, RAIN_TOP = 16;
const rainGeo = new THREE.BufferGeometry();
{
  const seed = new Float32Array(RAIN_MAX * 6), tip = new Float32Array(RAIN_MAX * 2);
  for (let i = 0; i < RAIN_MAX; i++) {
    const a = Math.random(), b = Math.random(), c = Math.random();
    seed.set([a, b, c, a, b, c], i * 6); tip[i * 2 + 1] = 1; // two ends of one streak
  }
  rainGeo.setAttribute('position', new THREE.BufferAttribute(new Float32Array(RAIN_MAX * 6), 3)); // placeholder: the shader positions them
  rainGeo.setAttribute('seed', new THREE.BufferAttribute(seed, 3));
  rainGeo.setAttribute('tip', new THREE.BufferAttribute(tip, 1));
}

export const rain = new THREE.LineSegments(rainGeo, new THREE.ShaderMaterial({
  transparent: true, depthWrite: false,
  uniforms: { drop: { value: 0 }, center: { value: new THREE.Vector3() }, speed: { value: 12 }, len: { value: 0.4 }, wind: { value: new THREE.Vector2() }, opacity: { value: 0 } },
  vertexShader: `
    attribute vec3 seed; attribute float tip;
    uniform float drop, speed, len; uniform vec3 center; uniform vec2 wind;
    varying float vTip;
    const float TOP = ${RAIN_TOP.toFixed(1)}, AREA = ${RAIN_AREA.toFixed(1)};
    void main() {
      float y = mod(seed.z * TOP - drop * (0.85 + 0.3 * seed.x), TOP); // height above the ground
      vec3 p = vec3(center.x + (seed.x - 0.5) * AREA, y, center.z + (seed.y - 0.5) * AREA);
      p.xz += wind * (TOP - y) / speed;                         // blown along as it falls
      p += normalize(vec3(-wind.x, speed, -wind.y)) * len * tip; // the streak trails up-wind
      vTip = tip;
      gl_Position = projectionMatrix * viewMatrix * vec4(p, 1.0);
    }`,
  fragmentShader: `
    uniform float opacity; varying float vTip;
    void main() { gl_FragColor = vec4(0.8, 0.87, 0.95, opacity * (1.0 - vTip * 0.8)); }`,
}));
rain.frustumCulled = false;
rain.visible = false;
scene.add(rain);

// splashes: little rings that flick out where drops land
const SPLASHES = 320;
export const splash = new THREE.InstancedMesh(new THREE.RingGeometry(0.02, 0.05, 10).rotateX(-Math.PI / 2),
  new THREE.MeshBasicMaterial({ color: 0xbfd0e0, transparent: true, opacity: 0.7, blending: THREE.AdditiveBlending, depthWrite: false }), SPLASHES);
const splashAt = Array.from({ length: SPLASHES }, () => ({ x: 0, z: 0, born: -9 }));
splash.frustumCulled = false;
splash.instanceMatrix.setUsage(THREE.DynamicDrawUsage);
scene.add(splash);

const _sm = new THREE.Matrix4(), _sc = new THREE.Color();

export function strikeLightning() {
  weather.flashT = 0;
  weather.thunderNear = Math.random();
  weather.thunderIn = 0.15 + (1 - weather.thunderNear) * 2.2; // the farther the strike, the longer the wait
}

export function updateWeather(dt) {
  const w = weather, L = (w.level += Math.max(-dt * 0.08, Math.min(dt * 0.08, w.target - w.level)));
  w.t += dt;
  // gusts come and go, harder as it builds
  w.gust = 1 + L * (0.45 * Math.sin(w.t * 0.63) + 0.3 * Math.sin(w.t * 1.71 + 2) + 0.15 * Math.sin(w.t * 4.1));
  // --- rain ---
  const u = rain.material.uniforms, speed = 11 + 13 * L;
  rain.visible = L > 0.005;
  rain.geometry.setDrawRange(0, Math.round(RAIN_MAX * L) * 2);
  u.speed.value = speed;
  u.len.value = 0.25 + 1.0 * L;
  u.opacity.value = 0.3 + 0.35 * L;
  u.wind.value.set(7 * L * L * w.gust, 2.5 * L * L * w.gust); // out of the west, a touch north
  u.drop.value = (w.drop += speed * dt);
  u.center.value.set(player.position.x, 0, player.position.z - 4);
  // --- splashes, spread over what the camera can see ---
  w.splashDue += L * 520 * dt;
  for (; w.splashDue >= 1; w.splashDue--) {
    const s = splashAt[w.splashI = (w.splashI + 1) % SPLASHES];
    s.x = player.position.x + rnd(-26, 26); s.z = player.position.z + rnd(-22, 14); s.born = w.t;
  }
  for (let i = 0; i < SPLASHES; i++) {
    const s = splashAt[i], k = (w.t - s.born) / 0.22;
    const live = k >= 0 && k < 1 && L > 0.005;
    _sm.makeScale(live ? 1 + 2.5 * k : 0, 1, live ? 1 + 2.5 * k : 0).setPosition(s.x, 0.03, s.z);
    splash.setMatrixAt(i, _sm);
    splash.setColorAt(i, _sc.setScalar(live ? (1 - k) * (1 - k) * (0.35 + 0.5 * L) : 0));
  }
  splash.instanceMatrix.needsUpdate = true;
  splash.instanceColor.needsUpdate = true;
  // --- lightning: from wave 12, more and more often ---
  if (L >= BOLT_LEVEL - 0.001 && state.running) {
    if ((w.nextBolt -= dt) <= 0) {
      strikeLightning();
      w.nextBolt = THREE.MathUtils.lerp(16, 3.5, (L - BOLT_LEVEL) / (1 - BOLT_LEVEL)) * rnd(0.6, 1.4);
    }
  }
  if (w.thunderIn >= 0 && (w.thunderIn -= dt) < 0) audio.thunder(w.thunderNear);
  // the flash: a bright stroke, a dip, a second stroke, then it dies away
  if (w.flashT >= 0) {
    const t = (w.flashT += dt);
    w.flash = Math.exp(-t * 14) + (t > 0.13 ? 0.8 * Math.exp(-(t - 0.13) * 9) : 0);
    if (t > 1.2) { w.flashT = -1; w.flash = 0; }
  }
  // --- the world gets wet, dark and close ---
  scene.fog.near = 32 - 14 * L;
  scene.fog.far = 62 - 18 * L;
  scene.fog.color.copy(NIGHT).lerp(BOLT_SKY, w.flash * 0.7);
  scene.background.copy(scene.fog.color);
  moon.intensity = 2.8 - 1.1 * L + w.flash * 14;
  ambient.intensity = 3.0 - 0.7 * L + w.flash * 3;
  worldMat.roughness = 0.92 - 0.35 * L;
  ground.material.roughness = 1 - 0.4 * L;
  if ((w.soundT -= dt) <= 0) { w.soundT = 0.2; audio.weather(L, w.gust); }
}
