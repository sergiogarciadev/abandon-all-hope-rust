import * as THREE from 'three';
import { renderer, scene, camera } from './core/scene.js';
import { state, BASE_STATS, bullets, zombies, particles, drops, debris } from './core/state.js';
import { store } from './core/storage.js';
import { rnd } from './core/math.js';
import { audio, setAudioListener, setupAudioListeners } from './audio/audio.js';
import { music } from './audio/music.js';
import { HALF, WALL_T, wallBoxes, updateWorld } from './world/world.js';
import { woodChip, shutAt, burst, breakOpening, resetOpenings, updateOpenings } from './world/openings.js';
import { weather, stormFor, updateWeather } from './world/weather.js';
import { player, animatePlayer } from './entities/player.js';
import { startReload, finishReload, fire } from './entities/weapon.js';
import { updateDrops } from './entities/drops.js';
import {
  spawnZombie,
  hitZombie,
  killZombie,
  zombieSkin,
  updateCorpses,
  removeCorpses,
  setPlayerHurtHandler,
  chaseZombie,
  separateZombies,
  HURT_IFRAMES,
} from './entities/zombies.js';
import { updateDebris } from './effects/debris.js';
import { hud, updateHUD, damageFlash, updateDamageFlash, showBanner } from './ui/hud.js';
import { openPicker, choose } from './ui/upgrades.js';
import { keys, mouse, firing, setFiring, aimPoint, aimPlane, raycaster, setupInputListeners } from './ui/input.js';
import { title, SWOOP_TIME, enterTitle, wanderZombie, updateTitle, updateCamera, menu, setTitleHandlers } from './ui/title.js';
import { wvSubmitScore, wvUnlock, initWavedash } from './platform/wavedash.js';

// ===================================================================
// GAME FLOW & STATE
// ===================================================================
const MAX_ALIVE = 30; // spawning pauses while this many zombies are up
let groanT = 1;

// clear everything a run leaves lying around
export function clearRun() {
  for (const b of bullets) scene.remove(b);
  for (const z of zombies) scene.remove(z);
  for (const p of particles) scene.remove(p);
  for (const d of drops) scene.remove(d);
  for (const d of debris) scene.remove(d);
  removeCorpses();
  bullets.length = 0; zombies.length = 0; particles.length = 0; drops.length = 0; debris.length = 0;
  hud.picker?.classList.add('hidden');
}

export function startWave(n) {
  state.wave = n;
  if (n >= 5) wvUnlock("WAVE_5");
  if (n >= 10) wvUnlock("WAVE_10");
  if (n >= 15) wvUnlock("WAVE_15");
  // Zombies toughen steadily but slower than upgrades let the player scale, so
  // runs can go long; speed caps below the player's so kiting always works.
  state.zombieSpeed = Math.min(5.2, 2.3 + n * 0.18);
  state.zombieHp = 2 + (n - 1) * 0.4;
  // anything already up counts toward the wave (the yard's shamblers when you leave the title)
  state.toSpawn = Math.max(0, 5 + n * 3 - zombies.length);
  for (const z of zombies) { z.userData.speed = state.zombieSpeed * z.userData.pace; z.userData.hp = state.zombieHp; }
  state.spawnTimer = 0;
  state.intermission = 0;
  weather.target = stormFor(n);
  const omens = { 1: 'they are already here', 5: 'it starts to rain', 8: 'the rain sets in', 12: 'the storm breaks', 15: 'god help you' };
  showBanner('WAVE ' + n, omens[n] || 'it gets worse');
  audio.waveStart(n);
}

// `keep` carries over what's on the field (coming from the title: the survivor stays where
// they are and the yard's zombies join wave 1, which starts once the camera is down)
export function resetGame(keep = false) {
  if (!keep) { clearRun(); resetOpenings(); }
  weather.set(0);
  Object.assign(state, {
    ...BASE_STATS, running: true, hp: BASE_STATS.maxHp, kills: 0, shots: 0, wave: 0,
    mag: BASE_STATS.magSize, reloading: false, reloadT: 0,
    fireCd: 0, dmgCd: 0, toSpawn: 0, spawnTimer: 0,
    intermission: 0, choosing: false, shake: 0, upgrades: {},
    pDamage: 0, pAmmo: 0, sinceDrop: 0,
  });
  if (keep) {
    state.intermission = SWOOP_TIME * 0.85;
    for (const z of zombies) Object.assign(z.userData, { cross: null, idle: 0, via: -1 });
  } else {
    player.position.set(0, 0, 0);
    startWave(1);
  }
  updateHUD();
}

export function startGame(fromTitle = false) {
  audio.ensure(); // create/resume AudioContext within the click gesture
  document.getElementById('death')?.classList.add('hidden');
  title.active = false;
  document.body.classList.remove('titling');
  const menuEl = document.getElementById('menu');
  if (menuEl) menuEl.classList.add('gone');
  resetGame(fromTitle);
}

export function setPaused(on) {
  if (on && !state.running) return; // only a live run can pause
  if (on === state.paused) return;
  state.paused = on;
  setFiring(false);
  for (const k in keys) keys[k] = false; // keyups are missed while the menu is up
  const pStat = document.getElementById('pauseStat');
  if (pStat) pStat.textContent = `WAVE ${state.wave} · ${state.kills} KILLS`;
  document.getElementById('pause')?.classList.toggle('hidden', !on);
  audio.pause(on);
}

export function quitToTitle() {
  setPaused(false);
  state.running = false;
  document.getElementById('death')?.classList.add('hidden');
  enterTitle(false);
  music.setMode('title');
}

export function die() {
  state.running = false;
  setFiring(false);
  const deathStat = document.getElementById('deathStat');
  if (deathStat) deathStat.textContent = `YOU REACHED WAVE ${state.wave} · ${state.kills} KILLS`;
  // best night is ranked by kills, like the leaderboard
  const best = store.get('best', null), record = state.kills > 0 && (!best || state.kills > best.kills);
  if (record) store.set('best', { wave: state.wave, kills: state.kills });
  const db = document.getElementById('deathBest');
  if (db) {
    db.textContent = record ? 'NEW BEST NIGHT' : best ? `BEST: WAVE ${best.wave} · ${best.kills} KILLS` : '';
    db.classList.toggle('record', record);
  }
  document.getElementById('death')?.classList.remove('hidden');
  audio.death();
  if (state.wave === 1 && state.shots === 0) wvUnlock("PACIFIST");
  wvSubmitScore(state.kills);
}

export function hurtPlayer() {
  if (state.dmgCd > 0) return false;
  state.hp--;
  state.dmgCd = HURT_IFRAMES;
  damageFlash();
  audio.hurt();
  if (state.hp <= 0) die();
  return true;
}

// ===================================================================
// UPDATE
// ===================================================================
export function update(dt) {
  const px = player.position.x, pz = player.position.z;
  if (state.running) {
    // --- movement ---
    let mx = 0, mz = 0;
    if (keys['KeyW'] || keys['ArrowUp']) mz -= 1;
    if (keys['KeyS'] || keys['ArrowDown']) mz += 1;
    if (keys['KeyA'] || keys['ArrowLeft']) mx -= 1;
    if (keys['KeyD'] || keys['ArrowRight']) mx += 1;
    const moving = !!(mx || mz);
    if (moving) {
      const l = Math.hypot(mx, mz);
      player.position.x += (mx / l) * state.speed * dt;
      player.position.z += (mz / l) * state.speed * dt;
    }
    // clamp inside cabin
    const lim = HALF - 0.7;
    player.position.x = Math.max(-lim, Math.min(lim, player.position.x));
    player.position.z = Math.max(-lim, Math.min(lim, player.position.z));
    // --- aim ---
    raycaster.setFromCamera(mouse, camera);
    if (raycaster.ray.intersectPlane(aimPlane, aimPoint)) {
      const a = Math.atan2(aimPoint.x - player.position.x, aimPoint.z - player.position.z);
      player.rotation.y = a;
    }

    // --- weapon ---
    state.fireCd -= dt;
    if (firing && !state.choosing) fire();
    if (state.reloading) {
      state.reloadT += dt;
      if (state.reloadT >= state.reloadTime) finishReload();
    }
    state.pDamage = Math.max(0, state.pDamage - dt);
    state.pAmmo = Math.max(0, state.pAmmo - dt);
    updateDrops(dt);

    // --- waves ---
    if (state.choosing) {
      // wave cleared — waiting on the upgrade pick
    } else if (state.intermission > 0) {
      state.intermission -= dt;
      if (state.intermission <= 0) startWave(state.wave + 1);
    } else {
      if (state.toSpawn > 0) {
        state.spawnTimer -= dt;
        if (state.spawnTimer <= 0 && zombies.length < MAX_ALIVE) {
          // spawn 1-2 at a time, faster on higher waves
          const burst = 1 + (Math.random() < Math.min(0.6, state.wave * 0.08) ? 1 : 0);
          for (let i = 0; i < burst && state.toSpawn > 0; i++) { spawnZombie(); state.toSpawn--; }
          state.spawnTimer = Math.max(0.35, 1.4 - state.wave * 0.08);
        }
      } else if (zombies.length === 0) {
        // wave cleared
        if (state.wave === 1) wvUnlock("WAVE_1"); // survived your first wave
        const healed = state.hp < state.maxHp;
        state.hp = Math.min(state.maxHp, state.hp + 1);
        audio.waveClear();
        openPicker(healed);
      }
    }

    state.dmgCd -= dt;

    // --- zombie groans: more of them up = a busier, closer chorus ---
    if (zombies.length) {
      groanT -= dt;
      if (groanT <= 0) {
        const gz = zombies[(Math.random() * zombies.length) | 0];
        audio.groan(gz.position);
        gz.userData.groan = 1; // its jaw drops with the groan
        groanT = rnd(0.5, 1.6) * 4 / Math.min(8, 2 + zombies.length * 0.5);
      }
    }
  }
  if (title.active) updateTitle(dt);
  music.wave = state.wave;
  music.alive = zombies.length;
  music.lowHp = state.running && state.hp === 1;

  // --- bullets ---
  for (let i = bullets.length - 1; i >= 0; i--) {
    const b = bullets[i];
    const bpx = b.position.x, bpz = b.position.z;
    b.position.addScaledVector(b.userData.dir, 38 * dt);
    b.userData.life -= dt;
    let hit = false;
    // wall collision: sample the path traveled this frame so fast bullets
    // can't tunnel through the thin walls (openings have no wall box; shut ones are checked below)
    const steps = Math.max(1, Math.ceil(Math.hypot(b.position.x - bpx, b.position.z - bpz) / (WALL_T * 0.5)));
    for (let s = 1; s <= steps && !hit; s++) {
      const t = s / steps;
      const sx = bpx + (b.position.x - bpx) * t, sz = bpz + (b.position.z - bpz) * t;
      for (const w of wallBoxes) {
        if (sx >= w.minX && sx <= w.maxX && sz >= w.minZ && sz <= w.maxZ) { hit = true; break; }
      }
      // a shut door stops the round; glass shatters and lets it through
      const o = !hit && shutAt(sx, sz);
      if (o?.kind === 'door') { hit = true; o.jolt = 0.4; burst(o, 1, woodChip, 0.08, 1.5); audio.bash(o, 0.4); }
      else if (o) breakOpening(o);
    }
    for (let j = zombies.length - 1; j >= 0 && !hit; j--) {
      const z = zombies[j];
      if (b.userData.hits.has(z)) continue; // a piercing bullet already went through this one
      // distance from the zombie to the path traveled this frame (not just the
      // end point), so bullets can't skip past zombies at low frame rates
      const sx = b.position.x - bpx, sz = b.position.z - bpz;
      const t = Math.max(0, Math.min(1, ((z.position.x - bpx) * sx + (z.position.z - bpz) * sz) / (sx * sx + sz * sz || 1)));
      const dx = z.position.x - (bpx + sx * t), dz = z.position.z - (bpz + sz * t);
      if (dx * dx + dz * dz < 0.55) {
        z.userData.hp -= state.damage * (state.pDamage > 0 ? 2 : 1);
        hitZombie(z);
        // small knockback
        z.position.addScaledVector(b.userData.dir, 0.25);
        if (z.userData.hp <= 0) { audio.zdeath(z.position); killZombie(z, j, b.userData.dir); }
        else audio.hit(z.position);
        b.userData.hits.add(z);
        if (b.userData.pierce > 0) b.userData.pierce--;
        else hit = true; // ends the loop; a piercing bullet keeps checking the rest
      }
    }
    if (hit || b.userData.life <= 0) { scene.remove(b); bullets.splice(i, 1); }
  }

  // --- zombies ---
  for (let j = zombies.length - 1; j >= 0; j--) {
    const z = zombies[j];
    const ud = z.userData;
    // hurt flash recover
    if (ud.hurt > 0) {
      ud.hurt -= dt;
      if (ud.hurt <= 0) for (const m of ud.body) m.material = zombieSkin;
    }
    if (state.running) chaseZombie(z, dt);
    else if (title.active) wanderZombie(z, dt);
  }
  if (state.running || title.active) separateZombies();

  // --- particles ---
  for (let i = particles.length - 1; i >= 0; i--) {
    const p = particles[i];
    p.userData.life -= dt;
    p.userData.v.y -= 16 * dt;
    p.position.addScaledVector(p.userData.v, dt);
    if (p.position.y < 0.05) { p.position.y = 0.05; p.userData.v.set(0, 0, 0); }
    if (p.userData.life <= 0) { scene.remove(p); particles.splice(i, 1); }
  }

  updateCorpses(dt);
  updateOpenings(dt);
  updateWeather(dt);
  updateWorld(dt);

  // --- survivor rig, driven by how far the survivor actually moved (so pushing into a
  // wall doesn't run in place). Runs on the title and death screens too, so the pose settles ---
  const inv = dt > 0 ? 1 / dt : 0;
  animatePlayer(dt, (player.position.x - px) * inv, (player.position.z - pz) * inv);
  // flicker while the post-hit grace period lasts
  player.visible = !state.running || state.dmgCd <= 0 || Math.floor(state.dmgCd * 12) % 2 === 0;
  updateDebris(dt);

  // damage flash fade
  updateDamageFlash(dt);

  updateCamera(dt);

  updateHUD();
}

// ===================================================================
// LOOP
// ===================================================================
let last = performance.now();
export function loop(now) {
  const dt = Math.min(0.05, (now - last) / 1000);
  last = now;
  if (!state.paused) update(dt);
  renderer.render(scene, camera);
  requestAnimationFrame(loop);
}

export function initGame() {
  setAudioListener(player);
  setPlayerHurtHandler(hurtPlayer);
  setTitleHandlers({
    clearRun,
    play: fromTitle => startGame(fromTitle),
  });
  setupInputListeners({
    onStartGame: () => startGame(false),
    onSetPaused: setPaused,
    onQuitToTitle: quitToTitle,
    onReload: () => startReload(),
    onChoose: choose,
    menuKey: e => menu.key(e),
  });
  setupAudioListeners();

  enterTitle(true);
  updateCamera(0);
  requestAnimationFrame(loop);

  initWavedash();
}
