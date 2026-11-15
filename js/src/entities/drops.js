import * as THREE from 'three';
import { scene } from '../core/scene.js';
import { state, drops } from '../core/state.js';
import { player } from './player.js';
import { audio } from '../audio/audio.js';
import { showToast } from '../ui/hud.js';
import { glowTexture } from '../core/textures.js';
import { bakeBlocks, seeded } from '../core/blocks.js';
import { HALF } from '../world/world.js';

export const DROP_TYPES = [
  { id: 'med',  name: '+1 HEALTH',      color: 0xd6453c, weight: 40 },
  { id: 'dmg',  name: 'DOUBLE DAMAGE',  color: 0xff7a2e, weight: 25 },
  { id: 'ammo', name: 'BOTTOMLESS MAG', color: 0xe8d28a, weight: 25 },
];

export const DROP_MODELS = {
  med: [  // first-aid case, crosses on top and front
    [0.52, 0.34,  0.36,  0, 0,     0,     0xe6e2da, 0, 0, 0, 0.05],
    [0.3,  0.02,  0.1,   0, 0.175, 0,     0xc8302a, 0, 0, 0, 0.006],
    [0.1,  0.022, 0.3,   0, 0.176, 0,     0xc8302a, 0, 0, 0, 0.006],
    [0.24, 0.08,  0.02,  0, 0,     0.185, 0xc8302a, 0, 0, 0, 0.006],
    [0.08, 0.22,  0.022, 0, 0,     0.186, 0xc8302a, 0, 0, 0, 0.006],
  ],
  dmg: [  // a pair of hot rounds
    [0.13, 0.3,  0.13, -0.09, -0.06, 0, 0xc9a14a, 0, 0, 0, 0.03],
    [0.13, 0.3,  0.13,  0.09, -0.06, 0, 0xc9a14a, 0, 0, 0, 0.03],
    [0.11, 0.16, 0.11, -0.09,  0.17, 0, 0xff5a1e, 0, 0, 0, 0.04],
    [0.11, 0.16, 0.11,  0.09,  0.17, 0, 0xff5a1e, 0, 0, 0, 0.04],
    [0.06, 0.07, 0.06, -0.09,  0.27, 0, 0xff9a5a, 0, 0, 0, 0.02],
    [0.06, 0.07, 0.06,  0.09,  0.27, 0, 0xff9a5a, 0, 0, 0, 0.02],
  ],
  ammo: [ // ammo can with a stenciled band
    [0.52, 0.3,  0.28,  0, 0,     0,     0x4f5634, 0, 0, 0, 0.04],
    [0.54, 0.06, 0.3,   0, 0.17,  0,     0x3d4228, 0, 0, 0, 0.02],
    [0.24, 0.04, 0.06,  0, 0.215, 0,     0x2a2c24, 0, 0, 0, 0.01],
    [0.3,  0.06, 0.006, 0, 0,     0.143, 0xd8b84a, 0, 0, 0, 0.002],
  ],
};

export const dropMat = new THREE.MeshStandardMaterial({ vertexColors: true, roughness: 0.5, emissive: 0x1c1c1c });
export const dropGlow = glowTexture();
for (const t of DROP_TYPES) {
  t.geo = bakeBlocks(DROP_MODELS[t.id], seeded(3));
  t.glow = new THREE.SpriteMaterial({ map: dropGlow, color: t.color, transparent: true, opacity: 0.95, blending: THREE.AdditiveBlending, depthWrite: false });
}

export const DROP_CHANCE = 0.05; // per kill, before SCAVENGER
export const DROP_PITY = 30;     // guaranteed drop after this many dry kills
export const DROP_LIFE = 12;     // seconds before an uncollected drop vanishes

export function maybeDrop(pos) {
  state.sinceDrop++;
  if (Math.random() > DROP_CHANCE + state.dropBonus && state.sinceDrop < DROP_PITY) return;
  state.sinceDrop = 0;
  // no medkits at full health — they'd be a wasted drop
  const pool = DROP_TYPES.filter(t => t.id !== 'med' || state.hp < state.maxHp);
  let r = Math.random() * pool.reduce((a, t) => a + t.weight, 0);
  spawnDrop(pool.find(t => (r -= t.weight) < 0) || pool[0], pos);
}

export function spawnDrop(type, pos) {
  const d = new THREE.Group();
  const item = new THREE.Mesh(type.geo, dropMat); item.castShadow = true; item.scale.setScalar(1.25); d.add(item);
  const glow = new THREE.Sprite(type.glow); glow.scale.setScalar(2.6); d.add(glow);
  d.position.set(pos.x, 0.7, pos.z);
  d.userData = { type, life: DROP_LIFE, t: Math.random() * 6 };
  scene.add(d);
  drops.push(d);
}

export function collect(type) {
  audio.pickup(type.id);
  showToast(type.name, '#' + type.color.toString(16).padStart(6, '0'));
  if (type.id === 'med') state.hp = Math.min(state.maxHp, state.hp + 1);
  else if (type.id === 'dmg') state.pDamage = 10;
  else if (type.id === 'ammo') { state.pAmmo = 8; state.reloading = false; state.mag = state.magSize; }
}

export function updateDrops(dt) {
  const lim = HALF - 0.9;
  for (let i = drops.length - 1; i >= 0; i--) {
    const d = drops[i], ud = d.userData;
    ud.life -= dt; ud.t += dt;
    const p = d.position;
    const dx = player.position.x - p.x, dz = player.position.z - p.z;
    const dist = Math.hypot(dx, dz);
    if (dist < 0.9) { collect(ud.type); scene.remove(d); drops.splice(i, 1); continue; }
    if (dist < 2.5) {
      // magnet: close drops fly to the player
      p.x += (dx / dist) * 10 * dt; p.z += (dz / dist) * 10 * dt;
    } else {
      // the player can't leave the cabin, so drops that land outside slide in
      const cx = Math.max(-lim, Math.min(lim, p.x)), cz = Math.max(-lim, Math.min(lim, p.z));
      const ox = cx - p.x, oz = cz - p.z, od = Math.hypot(ox, oz);
      if (od > 0.01) { const st = Math.min(od, 6 * dt); p.x += (ox / od) * st; p.z += (oz / od) * st; }
    }
    p.y = 0.7 + Math.sin(ud.t * 3) * 0.15;
    d.rotation.y += dt * 2;
    d.visible = ud.life > 3 || ((ud.t * 8) | 0) % 2 === 0; // blink before vanishing
    if (ud.life <= 0) { scene.remove(d); drops.splice(i, 1); }
  }
}
