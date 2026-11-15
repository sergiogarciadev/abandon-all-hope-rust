import * as THREE from 'three';
import { ease } from '../core/math.js';

// where one foot is in its gait cycle p (0..1): planted for the first `beta` of it,
// sliding back under the body (s: 0.5 -> -0.5 of the stride), then lifted and
// carried forward. Shared by the survivor and the zombies.
const _foot = { s: 0, lift: 0, pitch: 0 };
export function footCycle(p, beta) {
  if (p < beta) {
    _foot.s = 0.5 - p / beta; _foot.lift = 0; _foot.pitch = 0;
  } else {
    const t = (p - beta) / (1 - beta);
    _foot.s = -0.5 + ease(t);
    _foot.lift = Math.sin(Math.PI * t);
    _foot.pitch = 0.6 * (1 - t) * (1 - t) - 0.3 * t * t; // toe-off, then heel first
  }
  return _foot;
}

const _ikD = new THREE.Vector3(), _ikP = new THREE.Vector3();
// two-bone IK: from joint `a`, reach toward `t` with bones l1 then l2, bending
// the middle joint toward `pole`. Writes the middle joint into `out`.
export function ik2(a, t, l1, l2, pole, out) {
  _ikD.subVectors(t, a);
  const d = THREE.MathUtils.clamp(_ikD.length(), Math.abs(l1 - l2) + 1e-3, l1 + l2 - 1e-3);
  _ikD.normalize();
  const x = (l1 * l1 - l2 * l2 + d * d) / (2 * d); // law of cosines: distance along the reach
  _ikP.copy(pole).addScaledVector(_ikD, -pole.dot(_ikD)).normalize();
  return out.copy(a).addScaledVector(_ikD, x).addScaledVector(_ikP, Math.sqrt(Math.max(0, l1 * l1 - x * x)));
}

const _bx = new THREE.Vector3(), _by = new THREE.Vector3(), _bz = new THREE.Vector3(), _bm = new THREE.Matrix4();
// lay a limb block (built along local Y) from joint `a` to joint `b`, front toward `face`
export function bone(mesh, a, b, face) {
  _by.subVectors(a, b).normalize();
  _bz.copy(face).addScaledVector(_by, -face.dot(_by));
  if (_bz.lengthSq() < 1e-6) _bz.set(0, 0, 1);
  _bz.normalize();
  _bx.crossVectors(_by, _bz);
  _bm.makeBasis(_bx, _by, _bz);
  mesh.quaternion.setFromRotationMatrix(_bm);
  mesh.position.copy(a).add(b).multiplyScalar(0.5);
}
