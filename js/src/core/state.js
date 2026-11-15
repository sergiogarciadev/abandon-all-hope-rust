import * as THREE from 'three';
import { BOX } from './blocks.js';

// the survivor's stats before any upgrades
export const BASE_STATS = {
  maxHp: 3,
  magSize: 8,
  reloadTime: 1.2,
  fireRate: 0.13,
  damage: 1,
  pierce: 0,
  spread: 0,
  speed: 6.2,
  dropBonus: 0,
};

export const state = {
  running: false,
  paused: false,
  hp: 3,
  maxHp: 3,
  kills: 0,
  shots: 0, // shots fired this run (for the PACIFIST achievement)
  wave: 0,
  mag: 8,
  magSize: 8,
  reloading: false,
  reloadT: 0,
  reloadTime: 1.2,
  fireCd: 0,
  fireRate: 0.13,
  dmgCd: 0,
  // upgradeable stats (see UPGRADES)
  damage: 1,
  pierce: 0,     // extra zombies each bullet passes through
  spread: 0,     // extra bullets per shot
  speed: 6.2,
  dropBonus: 0,
  upgrades: {},  // upgrade id -> times taken this run
  // timed power-ups from drops (seconds remaining)
  pDamage: 0,
  pAmmo: 0,
  sinceDrop: 0,  // kills since the last drop (pity timer)
  // wave control
  toSpawn: 0,
  spawnTimer: 0,
  intermission: 0,
  choosing: false, // wave cleared, waiting on an upgrade pick
  zombieSpeed: 2.2,
  zombieHp: 2,
  shake: 0,
};

export const bullets = [];
export const zombies = [];
export const particles = [];
export const drops = [];
export const debris = [];

// reusable materials for spawned objects
// tracers: a white-hot core inside a softer colored streak, both additive so they glow
export const glowMat = (color, opacity) =>
  new THREE.MeshBasicMaterial({ color, transparent: true, opacity, blending: THREE.AdditiveBlending, depthWrite: false });

export const tracerCore = glowMat(0xfff4d6, 1);
export const tracerGlow = glowMat(0xffb04a, 0.45);
export const tracerGlowHot = glowMat(0xff5a1e, 0.6); // hot: DOUBLE DAMAGE

export function makeTracer(hot) {
  const b = new THREE.Group(); // +Z is the direction of travel; the streak trails behind the head
  const core = new THREE.Mesh(BOX, tracerCore); core.scale.set(0.05, 0.05, 0.9); core.position.z = -0.35;
  const glow = new THREE.Mesh(BOX, hot ? tracerGlowHot : tracerGlow); glow.scale.set(0.15, 0.15, 1.05); glow.position.z = -0.42;
  b.add(core, glow);
  return b;
}

export const bloodMat = new THREE.MeshStandardMaterial({ color: 0x6a0e0e, roughness: 0.35 }); // wet, so the flash glints off it
