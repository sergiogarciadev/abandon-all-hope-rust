import * as THREE from 'three';
import { scene } from '../core/scene.js';
import { debris } from '../core/state.js';
import { bevelBox } from '../core/blocks.js';

// --- shell casings, dropped magazines and gun smoke ---
export const casingGeo = bevelBox(0.05, 0.05, 0.12, 0.012);
export const smokeMats = Array.from({ length: 24 }, () =>
  new THREE.MeshStandardMaterial({ color: 0x8d8f94, roughness: 1, transparent: true, opacity: 0, depthWrite: false }));

export let smokeI = 0;
export function getNextSmokeMat() {
  const mat = smokeMats[smokeI];
  smokeI = (smokeI + 1) % smokeMats.length;
  return mat;
}

export function spawnDebris(mesh, vel, o) {
  mesh.userData = { vel, spin: o.spin || new THREE.Vector3(), life: o.life, max: o.life, bounce: o.bounce, smoke: o.smoke, size: mesh.scale.clone() };
  scene.add(mesh);
  debris.push(mesh);
}

export function updateDebris(dt) {
  for (let i = debris.length - 1; i >= 0; i--) {
    const d = debris[i], u = d.userData;
    u.life -= dt;
    if (u.smoke) {
      // puffs swell, rise and thin out
      const k = 1 - u.life / u.max;
      d.scale.setScalar(u.smoke * (1 + k * 2.5));
      d.material.opacity = 0.32 * (1 - k) * (1 - k);
      u.vel.multiplyScalar(Math.exp(-3 * dt));
      d.position.addScaledVector(u.vel, dt);
      d.position.y += 0.5 * dt;
    } else {
      u.vel.y -= 16 * dt;
      d.position.addScaledVector(u.vel, dt);
      d.rotation.x += u.spin.x * dt; d.rotation.y += u.spin.y * dt; d.rotation.z += u.spin.z * dt;
      if (d.position.y < u.bounce) {
        // clatter on the boards, then lie flat
        d.position.y = u.bounce;
        u.vel.set(u.vel.x * 0.45, Math.abs(u.vel.y) > 1.5 ? -u.vel.y * 0.35 : 0, u.vel.z * 0.45);
        u.spin.multiplyScalar(0.4);
        if (u.vel.y === 0) { u.spin.set(0, 0, 0); d.rotation.x = 0; d.rotation.z = Math.round(d.rotation.z / Math.PI) * Math.PI; }
      }
      if (u.life < 0.4) d.scale.copy(u.size).multiplyScalar(Math.max(0.01, u.life / 0.4)); // shrink away
    }
    if (u.life <= 0) { scene.remove(d); debris.splice(i, 1); }
  }
}
