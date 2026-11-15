import * as THREE from 'three';
import { scene } from '../core/scene.js';
import { state, bullets, makeTracer } from '../core/state.js';
import { player, muzzle, muzzleFlash } from './player.js';
import { aimPoint } from '../ui/input.js';
import { audio } from '../audio/audio.js';

export function startReload() {
  if (!state.running || state.reloading || state.mag === state.magSize) return;
  state.reloading = true;
  state.reloadT = 0;
  audio.reloadOut();
}

export function finishReload() {
  state.reloading = false;
  state.mag = state.magSize;
  audio.reloadIn();
}

export function fire() {
  if (state.fireCd > 0 || state.reloading || state.mag <= 0 || !state.running) return;
  if (state.pAmmo <= 0) {
    state.mag--;
    if (state.mag <= 0) startReload();
  }
  state.shots++;
  state.fireCd = state.fireRate;

  const aim = new THREE.Vector3(aimPoint.x - player.position.x, 0, aimPoint.z - player.position.z);
  if (aim.lengthSq() < 1e-4) return;
  aim.normalize();

  // spawn at muzzle world position; BUCKSHOT fans extra bullets around the aim line
  const mp = new THREE.Vector3();
  muzzle.getWorldPosition(mp);
  const count = 1 + state.spread;
  for (let k = 0; k < count; k++) {
    const a = (k - (count - 1) / 2) * 0.13;
    const c = Math.cos(a), s = Math.sin(a);
    const dir = new THREE.Vector3(aim.x * c - aim.z * s, 0, aim.x * s + aim.z * c);
    const b = makeTracer(state.pDamage > 0);
    b.position.copy(mp);
    b.lookAt(mp.clone().add(dir));
    b.userData = { dir, life: 0.9, pierce: state.pierce, hits: new Set() };
    scene.add(b);
    bullets.push(b);
  }

  muzzleFlash(state.pDamage > 0);
  state.shake = Math.max(state.shake, 0.12);
  audio.shot(state.pDamage > 0, state.spread);
}
