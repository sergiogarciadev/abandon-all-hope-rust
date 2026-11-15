import * as THREE from 'three';
import { camera, scene, CAM_OFFSET } from '../core/scene.js';
import { state, zombies } from '../core/state.js';
import { settings, store } from '../core/storage.js';
import { rnd, damp } from '../core/math.js';
import { audio } from '../audio/audio.js';
import { pushOutOfSolids, routeAround } from '../world/solids.js';
import { player } from '../entities/player.js';
import { sky } from '../world/world.js';
import { weather } from '../world/weather.js';
import { spawnZombie, walkToward, animateZombie } from '../entities/zombies.js';
import { resetOpenings } from '../world/openings.js';
import { isOnWavedash } from '../platform/wavedash.js';

// ===================================================================
// TITLE SCREEN — the menu sits over a live shot of the cabin: the door shut, the survivor
// out of sight inside, a few of the dead shambling across the yard, the camera on a slow drift.
// PLAY swoops the camera down into the gameplay view and those zombies become wave 1.
// ===================================================================
export const TITLE_ZOMBIES = 4;
export const SWOOP_TIME = 1.8;
export const title = { active: true, t: 0, groan: 3 };
const menuEl = document.getElementById('menu');
const TITLE_CAM = new THREE.Vector3(2, 0, 24); // roughly where the title camera stands

let onClearRun = null;
let onPlay = null;

export function setTitleHandlers({ clearRun, play } = {}) {
  if (clearRun) onClearRun = clearRun;
  if (play) onPlay = play;
}

// The dead cross the yard between the back wall and the camera, side to side through the
// shot, some each way so neither side empties. Past crossX() they're out of frame (wider
// screens see more of the yard); there they wait a beat and come back in from the far edge.
const CROSS_Z = [11.5, 15.5];
export const crossX = () => 2 + 5 * camera.aspect;

export function crossYard(z, dir, x = -dir * crossX()) {
  const ud = z.userData, lz = rnd(...CROSS_Z);
  z.position.set(x, 0, lz);
  pushOutOfSolids(z.position);
  Object.assign(ud, {
    cross: { x: dir * (crossX() + 1), z: THREE.MathUtils.clamp(lz + rnd(-2, 2), ...CROSS_Z) },
    // now and then one stops partway to sway a while before carrying on
    stopX: Math.random() < 0.35 ? rnd(-5, 5) : null,
    speed: rnd(0.7, 1.05), heading: dir * Math.PI / 2, idle: 0, via: -1,
  });
  z.rotation.y = ud.heading;
}

export function enterTitle(splash = false) {
  title.active = true;
  if (onClearRun) onClearRun();
  resetOpenings();
  weather.set(0); // the title is a clear night
  state.running = false;
  state.dmgCd = 0;
  player.position.set(0, 0, 0); // in the middle of the cabin, below the line of the walls
  player.rotation.y = 0;
  // the first crowd starts spread along the way, alternating directions
  for (let i = 0; i < TITLE_ZOMBIES; i++) {
    const x = crossX() * (2 * (i + rnd(0.2, 0.8)) / TITLE_ZOMBIES - 1);
    crossYard(spawnZombie(new THREE.Vector3()), i % 2 ? 1 : -1, x);
  }
  document.body.classList.add('titling');
  if (menuEl) {
    menuEl.classList.remove('gone');
    menuEl.classList.toggle('splash', !!splash);
  }
  menu.open(null);
  menu.select(0, true);
  showBest();
}

export function wanderZombie(z, dt) {
  const ud = z.userData, c = ud.cross, dir = Math.sign(c.x);
  let walking = false;
  if (ud.idle > 0) ud.idle -= dt; // stand and sway a while
  else if (ud.stopX !== null && (z.position.x - ud.stopX) * dir > 0) {
    ud.stopX = null;
    ud.idle = rnd(1.5, 3.5);
  } else if (z.position.x * dir > crossX()) { // out of shot: round again after a beat
    crossYard(z, dir);
    ud.idle = rnd(0, 1);
  } else {
    const g = routeAround(z, c.x, c.z);
    walkToward(z, g.x - z.position.x, g.z - z.position.z, ud.speed, 1.5, dt);
    walking = true;
  }
  ud.gait = damp(ud.gait, walking ? 1 : 0, 4, dt);
  animateZombie(z, dt, 99);
}

export function updateTitle(dt) {
  title.t += dt;
  // the odd groan from out in the dark
  if ((title.groan -= dt) <= 0 && zombies.length) {
    const gz = zombies[(Math.random() * zombies.length) | 0];
    audio.groan(gz.position);
    gz.userData.groan = 1;
    title.groan = rnd(3, 7);
  }
}

// --- camera: a slow cinematic drift on the title, the follow cam in a run, a swoop between ---
const TITLE_FOV = 42, GAME_FOV = 55;
const followPos = new THREE.Vector3();
const titlePos = new THREE.Vector3(), titleLook = new THREE.Vector3(), camLook = new THREE.Vector3();
const camTmp = new THREE.Vector3();
let camMix = 0; // 0 = title shot, 1 = follow cam
const swoop = t => t < 0.5 ? 4 * t * t * t : 1 - Math.pow(-2 * t + 2, 3) / 2; // ease in-out cubic

export function updateCamera(dt) {
  camTmp.copy(player.position).add(CAM_OFFSET);
  if (state.shake > 0) {
    state.shake = Math.max(0, state.shake - dt * 2.2);
    const s = settings.shake ? state.shake : 0;
    camTmp.x += (Math.random() - 0.5) * s;
    camTmp.z += (Math.random() - 0.5) * s;
    camTmp.y += (Math.random() - 0.5) * s;
  }
  if (camMix === 0) followPos.copy(camTmp);
  else followPos.lerp(camTmp, 1 - Math.pow(0.001, dt));
  camMix = THREE.MathUtils.clamp(camMix + (title.active ? -1 : 1) * dt / SWOOP_TIME, 0, 1);

  // title shot: low in the yard, drifting side to side and breathing like a handheld
  const t = title.t;
  titlePos.set(TITLE_CAM.x + 1.4 * Math.sin(t * 0.045), 2 + 0.2 * Math.sin(t * 0.07) + 0.03 * Math.sin(t * 1.3), TITLE_CAM.z - 1.2 * Math.sin(t * 0.031));
  titleLook.set(-1.5 + 0.7 * Math.sin(t * 0.045 + 0.6), 2.2 + 0.02 * Math.sin(t * 1.1), 8);

  const e = swoop(camMix);
  camera.position.lerpVectors(titlePos, followPos, e);
  camLook.lerpVectors(titleLook, player.position, e);
  camera.lookAt(camLook);
  const fov = THREE.MathUtils.lerp(TITLE_FOV, GAME_FOV, e);
  if (Math.abs(camera.fov - fov) > 1e-3) { camera.fov = fov; camera.updateProjectionMatrix(); }
  sky.position.copy(camera.position);
}

// --- the menu itself: keyboard (arrows/WASD, enter, esc) and mouse both drive it ---
const WAVEDASH_URL = 'https://wavedash.com/games/abandon-all-hope';
if (isOnWavedash()) {
  document.querySelector('#menuList [data-act="wavedash"]')?.remove();
}

const menuButtons = [...document.querySelectorAll('#menuList button')];
const settingRows = [...document.querySelectorAll('#settingRows .srow')];

export const menu = {
  sel: 0, panel: null, row: 0,
  select(i, quiet) {
    if (!menuButtons.length) return;
    i = (i + menuButtons.length) % menuButtons.length;
    if (i !== this.sel && !quiet) audio.ui('move');
    this.sel = i;
    menuButtons.forEach((b, k) => b.classList.toggle('sel', k === i));
    const b = menuButtons[i], m = document.getElementById('menuMarker');
    if (b && m) {
      m.style.height = b.offsetHeight + 'px';
      m.style.transform = `translateY(${b.offsetTop}px)`;
    }
  },
  open(panel) {
    this.panel = panel;
    document.getElementById('howPanel')?.classList.toggle('open', panel === 'how');
    document.getElementById('settingsPanel')?.classList.toggle('open', panel === 'settings');
    if (panel === 'settings') this.selectRow(this.row, true);
    this.hints();
  },
  activate(i = this.sel) {
    if (!menuButtons[i]) return;
    const act = menuButtons[i].dataset.act;
    this.select(i, true);
    if (act === 'play') {
      this.open(null);
      if (onPlay) onPlay(true);
      return;
    }
    if (act === 'wavedash') {
      audio.ui('select');
      window.open(WAVEDASH_URL, '_top');
      return;
    }
    audio.ui(this.panel === act ? 'back' : 'select');
    this.open(this.panel === act ? null : act);
  },
  selectRow(i, quiet) {
    if (!settingRows.length) return;
    i = (i + settingRows.length) % settingRows.length;
    if (i !== this.row && !quiet) audio.ui('move');
    this.row = i;
    settingRows.forEach((r, k) => r.classList.toggle('sel', k === i));
  },
  adjust(key, dir) {
    if (key === 'shake') settings.shake = dir === 0 ? !settings.shake : dir > 0;
    else settings[key] = THREE.MathUtils.clamp(settings[key] + dir, 0, 10);
    store.set('settings', settings);
    audio.applyVolume();
    audio.ui('move');
    this.drawSettings();
  },
  drawSettings() {
    for (const r of settingRows) {
      const k = r.dataset.key, v = settings[k];
      if (k === 'shake') {
        const t = r.querySelector('.toggle');
        if (t) t.innerHTML = `<i class="${v ? 'on' : ''}" data-v="1">ON</i><i class="${v ? '' : 'on'}" data-v="0">OFF</i>`;
        const em = r.querySelector('em');
        if (em) em.textContent = '';
      } else {
        const m = r.querySelector('.meter');
        if (m) m.innerHTML = Array.from({ length: 10 }, (_, n) => `<i class="${n < v ? 'on' : ''}" data-v="${n + 1}"></i>`).join('');
        const em = r.querySelector('em');
        if (em) em.textContent = v * 10;
      }
    }
  },
  hints() {
    const hintsEl = document.getElementById('menuHints');
    if (!hintsEl) return;
    const keys = this.panel === 'settings'
      ? [['↑', '↓', 'SELECT'], ['←', '→', 'ADJUST'], ['ESC', 'BACK']]
      : this.panel ? [['ESC', 'BACK']]
      : [['↑', '↓', 'SELECT'], ['ENTER', 'CONFIRM']];
    hintsEl.innerHTML = keys.map(k =>
      `<span>${k.slice(0, -1).map(c => `<kbd>${c}</kbd>`).join('')}${k[k.length - 1]}</span>`).join('');
  },
  // keyboard: returns true if the title used the key
  key(e) {
    if (!title.active || !menuEl || menuEl.classList.contains('gone')) return false;
    if (e.code === 'KeyM') return false; // mute works everywhere
    if (menuEl.classList.contains('splash')) { this.leaveSplash(); return true; }
    const c = e.code, up = c === 'ArrowUp' || c === 'KeyW', down = c === 'ArrowDown' || c === 'KeyS';
    const left = c === 'ArrowLeft' || c === 'KeyA', right = c === 'ArrowRight' || c === 'KeyD';
    const ok = c === 'Enter' || c === 'Space', back = c === 'Escape' || c === 'Backspace';
    if (this.panel === 'settings') {
      const rowEl = settingRows[this.row];
      if (!rowEl) return false;
      const key = rowEl.dataset.key;
      if (up) this.selectRow(this.row - 1);
      else if (down) this.selectRow(this.row + 1);
      else if (left || right) this.adjust(key, left ? -1 : 1);
      else if (ok && key === 'shake') this.adjust(key, 0);
      else if (back) { audio.ui('back'); this.open(null); }
    } else if (this.panel) {
      if (back || ok) { audio.ui('back'); this.open(null); }
    } else if (up) this.select(this.sel - 1);
    else if (down) this.select(this.sel + 1);
    else if (ok) this.activate();
    return true;
  },
  leaveSplash() {
    if (!menuEl || !menuEl.classList.contains('splash')) return;
    audio.ensure();
    menuEl.classList.remove('splash');
    this.select(0, true);
    this.hints();
  },
};

menu.drawSettings();

export function showBest() {
  const b = store.get('best', null);
  const bestEl = document.getElementById('bestRun');
  if (bestEl) {
    bestEl.innerHTML = b
      ? `<span class="lbl">BEST NIGHT</span>WAVE <b>${b.wave}</b> · <b>${b.kills}</b> KILLS`
      : '';
  }
}

menuButtons.forEach((b, i) => {
  b.addEventListener('mouseenter', () => { if (!menu.panel || menu.panel !== b.dataset.act) menu.select(i); });
  b.addEventListener('click', () => menu.activate(i));
});

settingRows.forEach((r, i) => {
  r.addEventListener('mouseenter', () => menu.selectRow(i));
  r.addEventListener('click', e => {
    const v = e.target.dataset.v, key = r.dataset.key;
    if (v === undefined) { if (key === 'shake') menu.adjust(key, 0); return; }
    if (key === 'shake') menu.adjust(key, v === '1' ? 1 : -1);
    else menu.adjust(key, +v - settings[key]);
  });
});

if (menuEl) {
  menuEl.addEventListener('click', e => {
    if (menuEl.classList.contains('splash')) return menu.leaveSplash();
    // clicking the open scene (not the column or a panel) closes a panel
    if (menu.panel && !e.target.closest('.mpanel, #menuList')) { audio.ui('back'); menu.open(null); }
  });
}
