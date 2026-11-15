import * as THREE from 'three';
import { scene, CAM_OFFSET } from '../core/scene.js';
import { state, zombies } from '../core/state.js';
import { player } from './player.js';
import { bakeBlocks, seeded } from '../core/blocks.js';
import { rnd, damp, wrapA, ease } from '../core/math.js';
import { footCycle, ik2, bone } from './animation.js';
import { audio } from '../audio/audio.js';
import { spawnBlood } from '../effects/blood.js';
import { maybeDrop } from './drops.js';
import { wvAddKill } from '../platform/wavedash.js';
import { pushOutOfSolids, routeAround, inside } from '../world/solids.js';
import { nearestOpening, hitOpening, shutAt, startClimb, climbStep, endClimb } from '../world/openings.js';
import { wallBoxes, EDGE } from '../world/world.js';

// ===================================================================
// ZOMBIES
// ===================================================================
// Built like the survivor (chamfered blocks, IK limbs, planted feet), but
// each body part is baked into one vertex-colored mesh so a horde of 30
// stays cheap: every zombie shares a handful of geometries and two materials.

export const Z_THIGH = 0.42, Z_SHIN = 0.4, Z_ANKLE = 0.11, Z_HIP = 0.9;
export const Z_UPPER_ARM = 0.4, Z_FOREARM = 0.39;
// the rig is modeled in slightly smaller units than the survivor's: Z_SCALE brings its legs and
// height to his, and Z_UPPER widens the torso and head to his shoulders, so they stand eye to eye
export const Z_SCALE = 1.08, Z_UPPER = 1.1;
export const GORE = 0x5c1010, BLOOD = 0x3a0807, BONE = 0xcfc4a2, SOCKET = 0x120a08;

// four dead locals; skin stays in the sick greens so they read as zombies at a glance
export const ZOMBIE_LOOKS = [
  { shirt: 0x4d6680, pants: 0x5a4632, skin: 0x7f9a66, hair: 0x2a2620, shoe: 0x2b2420 },                 // farmhand: denim shirt
  { shirt: 0x5b6239, pants: 0x6b5f43, skin: 0x8aa67a, hair: 0x4a3a28, shoe: 0x3a2c1f, ribs: true },     // hunter: field jacket
  { shirt: 0x9a9584, pants: 0x3a4a63, skin: 0x6f8a52, bald: true, tank: true, shoe: 0x1c1c1e, barefoot: 1 }, // tank top, one shoe gone
  { shirt: 0x86703a, pants: 0x38383b, skin: 0x82a07c, hair: 0x1b1714, shoe: 0x242226, ribs: true },     // camper: mustard hoodie
];
export const darker = (hex, k = 0.65) => new THREE.Color(hex).multiplyScalar(k).getHex();

// bake every body part for one look. Limbs are built along Y around their
// bone's midpoint (like the survivor's); the forearm carries the hand below it.
export function bakeZombie(L, seed) {
  const rng = seeded(seed);
  const skinDk = darker(L.skin, 0.72), shirtDk = darker(L.shirt), pantsDk = darker(L.pants);
  const sleeve = L.tank ? L.skin : L.shirt;
  const head = [
    [0.46, 0.44, 0.44, 0, 0.24, 0, L.skin],                       // skull
    [0.47, 0.06, 0.07, 0, 0.33, 0.2, skinDk],                     // heavy brow
    [0.11, 0.08, 0.03, -0.11, 0.27, 0.215, SOCKET], [0.11, 0.08, 0.03, 0.11, 0.27, 0.215, SOCKET],
    [0.06, 0.07, 0.04, 0, 0.2, 0.23, skinDk],                     // nose stump
    [0.25, 0.03, 0.03, 0, 0.06, 0.225, BONE, 0, 0, 0, 0.01],      // upper teeth
    [0.05, 0.1, 0.08, 0.235, 0.24, 0, L.skin],                    // one ear left
    [0.03, 0.1, 0.13, -0.232, 0.17, 0.07, GORE, 0, 0, 0, 0.01],   // torn cheek
  ];
  if (L.bald) {
    head.push([0.2, 0.03, 0.16, 0.06, 0.465, 0.03, GORE, 0, 0, 0, 0.01], [0.09, 0.03, 0.08, 0.07, 0.475, 0.03, BONE, 0, 0, 0, 0.01]); // scalp torn open
  } else {
    head.push([0.44, 0.16, 0.08, 0, 0.32, -0.2, L.hair]);        // matted hair at the back
    for (let i = 0; i < 3; i++) head.push([0.12 + rng() * 0.08, 0.06, 0.12 + rng() * 0.08, (rng() - 0.5) * 0.28, 0.48, (rng() - 0.6) * 0.3, L.hair]); // clumps
  }
  const torso = [
    [0.64, 0.6, 0.36, 0, 0.32, 0, L.shirt],                       // chest
    [0.7, 0.18, 0.38, 0, 0.56, -0.01, L.tank ? L.skin : L.shirt],  // shoulders
    [0.4, 0.2, 0.12, 0, 0.5, -0.2, L.tank ? L.skin : L.shirt],     // hunched back
    [0.2, 0.12, 0.2, 0, 0.67, 0.04, L.skin],                      // neck
    [0.15, 0.13, 0.39, -0.21, 0.0, 0, shirtDk, 0, 0, 0.25],        // torn hem
    [0.12, 0.1, 0.39, 0.19, -0.01, 0, shirtDk, 0, 0, -0.2],
    [0.18, 0.24, 0.02, -0.03, 0.47, 0.185, BLOOD],                // drool stain down the front
    [0.2, 0.17, 0.04, 0.15, 0.3, 0.17, L.skin],                   // hole torn in the shirt
    [0.12, 0.1, 0.03, 0.16, 0.3, 0.19, GORE],
  ];
  if (L.tank) torso.push([0.1, 0.18, 0.37, -0.17, 0.56, 0, L.shirt], [0.1, 0.18, 0.37, 0.17, 0.56, 0, L.shirt]); // straps
  if (L.ribs) {
    torso.push([0.26, 0.26, 0.04, -0.15, 0.33, 0.175, GORE]);     // ribs showing through
    for (let k = 0; k < 3; k++) torso.push([0.24, 0.035, 0.03, -0.15, 0.25 + k * 0.08, 0.2, BONE, 0, 0, 0, 0.01]);
  }
  const pelvis = [
    [0.52, 0.26, 0.32, 0, 0.02, 0, L.pants],
    [0.54, 0.06, 0.34, 0, 0.13, 0, 0x2a221a, 0, 0, 0, 0.015],     // belt
    [0.18, 0.16, 0.04, 0.12, 0.02, 0.17, L.tank ? L.shirt : shirtDk, 0, 0, 0.12], // shirt tail
  ];
  const upperArm = [[0.17, Z_UPPER_ARM + 0.06, 0.18, 0, 0, 0, sleeve]];
  if (!L.tank) upperArm.push([0.19, 0.07, 0.2, 0, -Z_UPPER_ARM / 2 + 0.03, 0, shirtDk]); // ragged sleeve end
  const hand = -Z_FOREARM / 2 - 0.06;
  const forearm = [
    [0.15, Z_FOREARM + 0.04, 0.16, 0, 0, 0, L.skin],
    [0.03, 0.12, 0.1, 0.075, 0.05, 0, GORE, 0, 0, 0, 0.01],       // bite
    [0.16, 0.14, 0.1, 0, hand, 0.01, skinDk],                     // hand
  ];
  for (const x of [-0.05, 0, 0.05]) forearm.push([0.035, 0.12, 0.035, x, hand - 0.12, 0.02 + rng() * 0.02, skinDk, 0.2 + rng() * 0.3, 0, 0, 0.008]); // clawed fingers
  const thigh = [[0.21, Z_THIGH + 0.06, 0.23, 0, 0, 0, L.pants]];
  const shin = [[0.19, Z_SHIN + 0.05, 0.21, 0, 0, 0, L.pants], [0.21, 0.07, 0.23, 0, -Z_SHIN / 2 + 0.02, 0, pantsDk]]; // frayed cuff
  const shoe = [[0.22, 0.12, 0.34, 0, -0.05, 0.05, L.shoe], [0.23, 0.03, 0.35, 0, -0.1, 0.05, 0x141210, 0, 0, 0, 0.01]];
  const bare = [[0.18, 0.08, 0.3, 0, -0.07, 0.06, skinDk], [0.16, 0.05, 0.06, 0, -0.09, 0.22, skinDk]];
  const eyes = [[0.06, 0.045, 0.02, -0.11, 0.27, 0.225, 0xffffff, 0, 0, 0, 0.005], [0.06, 0.045, 0.02, 0.11, 0.27, 0.225, 0xffffff, 0, 0, 0, 0.005]];
  const jaw = [
    [0.36, 0.09, 0.3, 0, -0.015, 0.15, L.skin],                   // hangs from a hinge under the skull
    [0.22, 0.025, 0.03, 0, 0.035, 0.29, BONE, 0, 0, 0, 0.01],     // lower teeth
    [0.2, 0.05, 0.02, 0, -0.035, 0.305, BLOOD, 0, 0, 0, 0.01],    // bloody chin
  ];
  const b = parts => bakeBlocks(parts, rng);
  return {
    head: b(head), jaw: b(jaw), eyes: b(eyes), torso: b(torso), pelvis: b(pelvis),
    upperArm: b(upperArm), forearm: b(forearm), thigh: b(thigh), shin: b(shin),
    feet: [b(shoe), b(L.barefoot ? bare : shoe)], // [left, right]
  };
}

export const ZOMBIE_KITS = ZOMBIE_LOOKS.map((L, i) => bakeZombie(L, 7 + i * 31));
export const zombieSkin = new THREE.MeshStandardMaterial({ vertexColors: true, roughness: 0.9 });
// on a hit the whole body swaps to this for a beat (same shader, no recompile)
export const zombieHurt = new THREE.MeshStandardMaterial({ vertexColors: true, roughness: 0.9, emissive: 0xd8e8b0, emissiveIntensity: 0.45 });
export const zombieEyes = new THREE.MeshBasicMaterial({ color: 0xd9e06a }); // a faint sick glow in the dark
export const bloodPoolMat = new THREE.MeshStandardMaterial({ color: 0x3a0606, roughness: 0.25 }); // wet: catches the muzzle flash

// Spawn points sit just past the edge of the gameplay view (worked out from where the
// follow camera will be, whatever the window's shape), so nothing pops in on screen.
const spawnCam = new THREE.PerspectiveCamera(55, 1, 0.1, 200);
const _sp = new THREE.Vector3();
function offscreen(x, z) {
  for (const y of [0, 2.2]) { // feet and head both out of frame
    _sp.set(x, y, z).project(spawnCam);
    if (_sp.z < 1 && Math.abs(_sp.x) < 1.08 && Math.abs(_sp.y) < 1.08) return false;
  }
  return true;
}

export function spawnPoint(out) {
  spawnCam.aspect = window.innerWidth / window.innerHeight;
  spawnCam.updateProjectionMatrix();
  spawnCam.position.copy(player.position).add(CAM_OFFSET);
  spawnCam.lookAt(player.position.x, 0, player.position.z);
  spawnCam.updateMatrixWorld();
  const ang = Math.random() * Math.PI * 2, cx = Math.sin(ang), cz = Math.cos(ang);
  let r = 12;
  const along = dist => [player.position.x + cx * dist, player.position.z + cz * dist];
  // walk out along a random bearing until it's off screen and clear of the cabin
  for (; r < 70; r += 1) {
    const [x, z] = along(r);
    if (offscreen(x, z) && (Math.abs(x) > EDGE + 1.5 || Math.abs(z) > EDGE + 1.5)) break;
  }
  [out.x, out.z] = along(r + 1.5 + Math.random() * 3);
  out.y = 0;
  pushOutOfSolids(out);
  return out;
}

export function spawnZombie(pos = spawnPoint(new THREE.Vector3())) {
  const kit = ZOMBIE_KITS[(Math.random() * ZOMBIE_KITS.length) | 0];
  const z = new THREE.Group();
  z.rotation.order = 'YXZ';
  const body = [];
  const mesh = (geo, parent, m = zombieSkin) => {
    const obj = new THREE.Mesh(geo, m);
    obj.castShadow = m !== zombieEyes;
    parent.add(obj);
    if (m === zombieSkin) body.push(obj);
    return obj;
  };
  const pelvis = mesh(kit.pelvis, z);
  const upper = new THREE.Group(); upper.rotation.order = 'YXZ'; upper.scale.setScalar(Z_UPPER); z.add(upper); // waist pivot
  mesh(kit.torso, upper);
  const head = new THREE.Group(); head.position.set(0, 0.68, 0.06); head.rotation.order = 'YXZ'; upper.add(head);
  mesh(kit.head, head);
  const eyes = mesh(kit.eyes, head, zombieEyes);
  const jaw = new THREE.Group(); jaw.position.set(0, 0, -0.06); head.add(jaw);
  mesh(kit.jaw, jaw);
  const side = s => ({
    side: s,
    upperArm: mesh(kit.upperArm, z), forearm: mesh(kit.forearm, z),
    thigh: mesh(kit.thigh, z), shin: mesh(kit.shin, z), foot: mesh(kit.feet[s > 0 ? 0 : 1], z),
    shoulder: new THREE.Vector3(s * 0.36, 0.58, 0), hip: new THREE.Vector3(s * 0.14, 0, 0),
    sh: new THREE.Vector3(), elbow: new THREE.Vector3(), wrist: new THREE.Vector3(), reach: new THREE.Vector3(),
    hipP: new THREE.Vector3(), knee: new THREE.Vector3(), ankle: new THREE.Vector3(), foot3: new THREE.Vector3(),
  });
  const limbs = [side(1), side(-1)];

  z.position.set(pos.x, 0, pos.z);
  z.scale.setScalar(Z_SCALE * (0.94 + Math.random() * 0.12));
  const limp = Math.random() < 0.45 ? 0.5 + Math.random() * 0.5 : 0;
  const pace = 0.85 + Math.random() * 0.3;
  z.userData = {
    hp: state.zombieHp,
    pace,                                               // its speed relative to the wave's
    speed: state.zombieSpeed * pace,
    hurt: 0, body, eyes, pelvis, upper, head, jaw, limbs,
    // gait personality
    phase: Math.random(),
    seed: Math.random() * 100,
    limp,                                               // 0 = sound legs; else how badly one drags
    limpSide: Math.random() < 0.5 ? 1 : -1,
    hunch: 0.25 + Math.random() * 0.3,
    tilt: (Math.random() - 0.5) * 0.5,                  // head lolls to one side
    arms: Math.random() < 0.7 ? 2 : Math.random() < 0.85 ? 1 : 0, // how many arms reach (the rest hang)
    flinch: 0, attack: 0, groan: 0,
    gait: 1,       // 1 = walking, 0 = feet planted
    swing: -1,     // seconds into a swing at the survivor; -1 = not swinging
    swingCd: 0.15, struck: false, standing: false, swingAt: null, // swingAt: a door/window it's battering, else you
    climb: -1,     // 0..1 through clambering in a window; -1 = not climbing
    via: -1,       // which prop corner it's heading round (see routeAround)
    stuckT: 0, stuckX: pos.x, stuckZ: pos.z, // progress check (see chaseZombie)
    opening: null, // committed entry opening; null = re-pick on next outside frame
    stage: 0,      // 0 = heading to outer waypoint, 1 = heading through the gap
    heading: Math.atan2(-z.position.x, -z.position.z), // current facing, starts toward cabin
    turn: 3.0 + Math.random() * 2.0,                   // max turn rate (rad/s), varied per zombie
  };
  z.rotation.y = z.userData.heading;
  scene.add(z);
  zombies.push(z);
  return z;
}

// on a bullet hit: flash, flinch back, and snap the head
export function hitZombie(z) {
  const u = z.userData;
  u.hurt = 0.12;
  u.flinch = 1;
  for (const m of u.body) m.material = zombieHurt;
}

const Z_POLE_KNEE = new THREE.Vector3(0, 0, 1);
const _zPole = new THREE.Vector3(), _zReach = new THREE.Vector3();

// pose a zombie for this frame: a lurching, uneven shamble that turns into a
// loping run on later waves, arms groping toward the survivor or hanging dead.
// `dead` (0..1) goes limp as it dies; `sag` buckles the knees.
export function animateZombie(z, dt, near, dead = 0, sag = dead) {
  const u = z.userData;
  const fall = dead ? u.fallDir : 0; // +1 toppling forward, -1 backward
  const live = 1 - dead;
  const gait = u.gait * live; // 0 when it's standing its ground
  const sp = u.speed * gait;
  const run = THREE.MathUtils.clamp((u.speed - 3) / 2, 0, 1); // later waves lope instead of shamble
  const freq = 1.1 + sp * 0.22;
  const beta = 0.6 - 0.18 * run;
  u.phase = (u.phase + freq * dt * live) % 1;
  const ph = u.phase * Math.PI * 2;
  const L = Math.min(0.7, sp * beta / freq);
  const t = performance.now() / 1000 + u.seed;
  u.flinch *= Math.exp(-7 * dt);
  u.groan *= Math.exp(-2.5 * dt);
  u.attack = damp(u.attack, near < 2.4 && u.swing < 0 ? live : 0, 6, dt);
  // the swing: rear back with the arms up (-1), slam down and lunge (+1), recover
  const sw = u.swing, rec = Z_SWING - Z_STRIKE - 0.1;
  const blow = sw < 0 || dead ? 0
    : sw < Z_STRIKE ? -ease(sw / Z_STRIKE)
    : sw < Z_STRIKE + 0.1 ? -1 + 2 * ease((sw - Z_STRIKE) / 0.1)
    : 1 - ease(Math.min(1, (sw - Z_STRIKE - 0.1) / rec));
  const rear = Math.max(0, -blow), slam = Math.max(0, blow);
  const clamber = u.climb >= 0 && !dead ? Math.sin(Math.PI * u.climb) : 0; // over a window sill

  // pelvis: dips on every step, and drops hard onto the bad leg
  const badStance = u.limp ? Math.max(0, Math.sin(ph + (u.limpSide > 0 ? 0 : Math.PI))) : 0;
  const pelvisY = Z_HIP - 0.05 - 0.06 * run - 0.05 * Math.abs(Math.sin(ph)) * gait - 0.06 * slam - 0.08 * u.limp * badStance - 0.3 * sag;
  u.pelvis.position.set(0, pelvisY, 0);
  u.pelvis.rotation.set(0, 0.12 * Math.sin(ph) * live, (0.1 * Math.sin(ph) + 0.12 * u.limp * badStance * u.limpSide) * live);
  u.pelvis.updateMatrix();

  // torso: hunched, lurching side to side against the hips, kicked back by hits
  u.upper.position.set(0, pelvisY + 0.08, 0);
  u.upper.rotation.set(
    (u.hunch + 0.12 * run + 0.25 * u.attack - 0.55 * rear + 0.5 * slam) * live - 0.6 * u.flinch + 0.35 * fall * dead, // goes over with the fall
    -0.15 * Math.sin(ph) * live,
    (-0.14 * Math.sin(ph) - 0.1 * u.limp * badStance * u.limpSide) * live + 0.04 * Math.sin(t * 0.7));
  u.upper.updateMatrix();
  // head lolls, nods with each step, comes up to look at its meal, snaps back when shot
  u.head.rotation.set(
    (-u.hunch * 0.7 + 0.12 * Math.abs(Math.sin(ph)) - 0.25 * u.attack - 0.2 * rear) * live - 0.7 * u.flinch + (fall > 0 ? 0.3 : -0.6) * dead,
    0.25 * Math.sin(t * 0.9) * live,
    u.tilt + 0.18 * Math.sin(t * 1.3 + 1) * live);
  u.jaw.rotation.x = (0.12 + 0.12 * Math.abs(Math.sin(t * 2.3)) + 0.45 * u.groan + 0.35 * u.attack * Math.abs(Math.sin(t * 9)) + 0.5 * rear) * live + 0.5 * dead;

  for (const l of u.limbs) {
    // --- legs: the good leg steps through, the bad one drags its toe ---
    const bad = u.limp && l.side === u.limpSide;
    const p = (u.phase + (l.side > 0 ? 0 : 0.5 + 0.06 * u.limp)) % 1;
    const { s, lift, pitch } = footCycle(p, beta);
    l.foot3.set(l.side * (0.15 + 0.06 * dead), Z_ANKLE + lift * (bad ? 0.03 : 0.12 + 0.1 * run) * gait + 0.3 * clamber, s * L - 0.2 * clamber);
    l.hipP.copy(l.hip).applyMatrix4(u.pelvis.matrix);
    ik2(l.hipP, l.foot3, Z_THIGH, Z_SHIN, Z_POLE_KNEE, l.knee);
    l.ankle.subVectors(l.foot3, l.knee).normalize().multiplyScalar(Z_SHIN).add(l.knee);
    bone(l.thigh, l.hipP, l.knee, Z_POLE_KNEE);
    bone(l.shin, l.knee, l.ankle, Z_POLE_KNEE);
    l.foot.position.copy(l.ankle);
    l.foot.rotation.set(bad ? 0.35 : pitch * live, l.side * (bad ? 0.35 : 0.12), 0);

    // --- arms: reaching ones grope ahead (and claw when close); the rest hang and swing ---
    l.sh.copy(l.shoulder).applyMatrix4(u.upper.matrix);
    const reaches = (l.side > 0 ? u.arms >= 1 : u.arms >= 2) && !dead;
    const k = l.side * 1.7;
    if (reaches) {
      const claw = u.attack * Math.sin(t * 9 + k);
      l.reach.set(-l.side * 0.08, -0.08 + 0.06 * Math.sin(ph + k) + 0.18 * u.attack + 0.12 * claw - 0.3 * u.flinch,
        0.72 + 0.07 * Math.sin(ph * 2 + k) + 0.2 * claw - 0.35 * u.flinch);
    } else if (dead) {
      l.reach.set(l.side * 0.45 * dead, -0.7, -0.15 * fall * dead); // limp, flung out to the sides
    } else {
      // dangling: a loose pendulum against the stride
      l.reach.set(l.side * 0.1, -0.84, 0.1 + 0.16 * Math.sin(ph + (l.side > 0 ? Math.PI : 0)) * gait + 0.25 * u.flinch);
    }
    // every arm joins the swing: raised overhead, then clubbed down in front
    if (rear) l.reach.lerp(_zReach.set(l.side * 0.22, 0.5, 0.18), rear);
    if (slam) l.reach.lerp(_zReach.set(-l.side * 0.06, -0.3, 0.78), slam);
    if (clamber) l.reach.lerp(_zReach.set(l.side * 0.24, -0.6, 0.45), clamber); // hands planted on the sill
    l.reach.add(l.sh);
    ik2(l.sh, l.reach, Z_UPPER_ARM, Z_FOREARM, _zPole.set(l.side * 0.6, -1, -0.3), l.elbow);
    l.wrist.subVectors(l.reach, l.elbow).normalize().multiplyScalar(Z_FOREARM).add(l.elbow);
    bone(l.upperArm, l.sh, l.elbow, _zPole);
    bone(l.forearm, l.elbow, l.wrist, _zPole);
  }
}

// --- the dead: crumple, topple onto the floor (or the pile), bleed out, and stay
// there CORPSE_LIFE seconds before sinking into the boards ---
export const corpses = [];
export const CORPSE_LIFE = 20;   // seconds a body stays down
export const MAX_CORPSES = 120;  // past this the oldest start sinking early
export const BODY_THICK = 0.3;   // how much each body lying underneath raises the next

export function updateCorpses(dt) {
  let bakes = 0;
  for (let i = corpses.length - 1; i >= 0; i--) {
    const z = corpses[i], u = z.userData;
    u.deadT += dt;
    if (!u.sinking && (u.deadT > CORPSE_LIFE || corpses.length - i > MAX_CORPSES)) sinkCorpse(z);
    if (u.baked) {
      if (u.sinking) {
        u.sinkT += dt;
        u.baked.position.y = -u.sinkT * (0.6 + u.restY * 0.5);
        u.pool.position.y = 0.045 - u.sinkT * 0.04;
        if (u.baked.position.y < -1.1 - u.restY) {
          scene.remove(u.baked, u.pool);
          u.baked.geometry.dispose();
          corpses.splice(i, 1);
        }
      }
      continue;
    }
    const crumple = Math.min(1, u.deadT / 0.25);
    const fall = Math.min(1, Math.max(0, (u.deadT - 0.1) / 0.45));
    animateZombie(z, dt, 99, crumple, crumple * (1 - 0.85 * fall)); // knees buckle, then straighten as it goes over
    // topple: accelerate like a falling body, with a small bounce at the bottom
    const f = fall * fall;
    z.rotation.x = u.fallDir * (f * 1.42 - (fall >= 1 ? 0.06 * Math.exp(-(u.deadT - 0.55) * 9) * Math.sin((u.deadT - 0.55) * 30) : 0));
    z.position.y = (0.12 + u.restY) * f;
    // the pool spreads out from under the body
    u.pool.scale.setScalar(Math.min(1, Math.max(0.05, (u.deadT - 0.4) / 1.4)) * u.poolSize);
    // settled: freeze it into one static mesh (a few per frame, so a crowd dropping at once doesn't hitch)
    if (u.deadT > 1.0 && bakes < 4) { bakeCorpse(z); bakes++; }
  }
}

export function sinkCorpse(z) {
  const u = z.userData;
  if (u.sinking) return;
  u.sinking = true; u.sinkT = 0;
  for (const c of corpses) if (c.userData.under.includes(z)) sinkCorpse(c);
}

const _bkN = new THREE.Matrix3(), _bkV = new THREE.Vector3();

export function bakeCorpse(z) {
  const u = z.userData;
  z.updateMatrixWorld(true);
  let n = 0;
  for (const m of u.body) n += m.geometry.attributes.position.count;
  const pos = new Float32Array(n * 3), nrm = new Float32Array(n * 3), col = new Float32Array(n * 3);
  let o = 0;
  for (const m of u.body) {
    const g = m.geometry, P = g.attributes.position.array, N = g.attributes.normal.array, C = g.attributes.color.array;
    _bkN.getNormalMatrix(m.matrixWorld);
    for (let i = 0; i < P.length; i += 3, o += 3) {
      _bkV.fromArray(P, i).applyMatrix4(m.matrixWorld).toArray(pos, o);
      _bkV.fromArray(N, i).applyMatrix3(_bkN).normalize().toArray(nrm, o);
      col[o] = C[i]; col[o + 1] = C[i + 1]; col[o + 2] = C[i + 2];
    }
  }
  const g = new THREE.BufferGeometry();
  g.setAttribute('position', new THREE.BufferAttribute(pos, 3));
  g.setAttribute('normal', new THREE.BufferAttribute(nrm, 3));
  g.setAttribute('color', new THREE.BufferAttribute(col, 3));
  u.baked = new THREE.Mesh(g, zombieSkin);
  u.baked.castShadow = u.baked.receiveShadow = true;
  scene.remove(z);
  scene.add(u.baked);
}

export function removeCorpses() {
  for (const c of corpses) {
    const u = c.userData;
    scene.remove(c, u.pool);
    if (u.baked) { scene.remove(u.baked); u.baked.geometry.dispose(); }
  }
  corpses.length = 0;
}

const bodyPoints = (x, z, dx, dz, s) => [0.5, 1.1, 1.7].map(t => [x + dx * t * s, z + dz * t * s]);

const poolGeo = (() => {
  const rng = seeded(99), slabs = [[0.8, 0.02, 0.8, 0, 0, 0, 0xffffff, 0, 0.3, 0, 0.008]];
  for (let i = 0; i < 6; i++) {
    const a = i * 1.1 + rng(), r = 0.3 + rng() * 0.25, s = 0.25 + rng() * 0.3;
    slabs.push([s, 0.02, s * (0.6 + rng() * 0.6), Math.cos(a) * r, 0.003 * i, Math.sin(a) * r, 0xffffff, 0, rng() * 3, 0, 0.008]);
  }
  return bakeBlocks(slabs, rng);
})();

export function zombieDies(z, dir) {
  const u = z.userData;
  u.deadT = 0;
  u.hurt = 0;
  for (const m of u.body) m.material = zombieSkin;
  u.eyes.visible = false; // the light goes out
  // fall along the shot (or away from the survivor) — forward if hit from behind
  const fx = dir ? dir.x : z.position.x - player.position.x, fz = dir ? dir.z : z.position.z - player.position.z;
  u.fallDir = fx * Math.sin(u.heading) + fz * Math.cos(u.heading) >= 0 ? 1 : -1;
  // land on top of any bodies already lying where this one falls
  const dx = Math.sin(u.heading) * u.fallDir, dz = Math.cos(u.heading) * u.fallDir;
  u.points = bodyPoints(z.position.x, z.position.z, dx, dz, z.scale.x);
  u.restY = 0; u.under = []; u.sinking = false; u.baked = null;
  for (const c of corpses) {
    const cu = c.userData;
    if (cu.sinking || !cu.points.some(([ax, az]) => u.points.some(([bx, bz]) => (ax - bx) ** 2 + (az - bz) ** 2 < 0.36))) continue;
    u.under.push(c);
    u.restY = Math.max(u.restY, cu.restY + BODY_THICK);
  }
  u.restY = Math.min(u.restY, 1.5);
  const pool = new THREE.Mesh(poolGeo, bloodPoolMat);
  pool.receiveShadow = true;
  pool.position.set(z.position.x + Math.sin(u.heading) * u.fallDir * 0.9, 0.045, z.position.z + Math.cos(u.heading) * u.fallDir * 0.9);
  pool.rotation.y = Math.random() * Math.PI;
  u.pool = pool; u.poolSize = 1.1 + Math.random() * 0.6;
  pool.scale.setScalar(0.05);
  scene.add(pool);
  corpses.push(z);
}

export function killZombie(z, idx, dir) {
  if (z.userData.climb >= 0) endClimb(z, true);
  spawnBlood(z.position);
  maybeDrop(z.position);
  zombieDies(z, dir);
  zombies.splice(idx, 1);
  state.kills++;
  wvAddKill();
}

// --- zombie melee: walk up, stop at arm's length, rear back and swing. The blow lands
// partway through, and only if you're still in range, so a step back dodges it ---
export const Z_REACH = 1.2;      // stops and swings when this close (center to center)
export const Z_LET_GO = 1.5;     // ...and only gives chase again once you're this far off
export const Z_CLOSE = 0.95;     // never gets nearer than this (no standing inside the survivor)
export const Z_SPACE = 0.8;      // zombies keep this far apart, so a crowd fans out around you
export const Z_SWING = 0.85;     // seconds from wind-up to recovered
export const Z_STRIKE = 0.2;     // the moment in the swing the blow lands
export const Z_HIT_RANGE = 1.65; // the blow connects if you're still this close
export const Z_TRACK = 7;        // rad/s a zombie turns to keep facing you while it stands and swings
export const HURT_IFRAMES = 1.5; // after a hit, the survivor can't be hit again for this long (and flickers)

// is there a wall (or a shut door or window) between two points? (samples the line)
export function wallBetween(a, b) {
  for (let i = 1; i < 4; i++) {
    const x = a.x + (b.x - a.x) * i / 4, z = a.z + (b.z - a.z) * i / 4;
    for (const w of wallBoxes) if (x >= w.minX && x <= w.maxX && z >= w.minZ && z <= w.maxZ) return true;
    if (shutAt(x, z)) return true;
  }
  return false;
}

// turn toward (dx, dz) at up to `rate` rad/s; returns how far off it's still facing
export function faceToward(z, dx, dz, rate, dt) {
  const ud = z.userData, dA = wrapA(Math.atan2(dx, dz) - ud.heading);
  ud.heading += Math.max(-rate * dt, Math.min(rate * dt, dA));
  z.rotation.y = ud.heading;
  return Math.abs(dA) - Math.min(Math.abs(dA), rate * dt);
}

// Ease the heading toward the target instead of snapping, so zombies arc into their
// turns, and slow down while facing well off it: a zombie that overshoots turns round
// and comes back instead of orbiting.
export function walkToward(z, dx, dz, speed, turn, dt) {
  const ud = z.userData, off = faceToward(z, dx, dz, turn, dt);
  const v = speed * Math.max(0.15, Math.cos(Math.min(off, Math.PI / 2)));
  z.position.x += Math.sin(ud.heading) * v * dt;
  z.position.z += Math.cos(ud.heading) * v * dt;
}

let playerHurtHandler = () => false;
export function setPlayerHurtHandler(fn) {
  playerHurtHandler = fn;
}

export function chaseZombie(z, dt) {
  const ud = z.userData, p = player.position;
  const pdx = p.x - z.position.x, pdz = p.z - z.position.z, pd = Math.hypot(pdx, pdz);
  let walking = false;
  if (ud.climb >= 0) {
    climbStep(z, dt);
  } else if (ud.swing >= 0) {
    // committed to the blow: feet planted, tracking its target through the wind-up
    const o = ud.swingAt;
    ud.swing += dt;
    if (ud.swing < Z_STRIKE) faceToward(z, o ? o.x - z.position.x : pdx, o ? o.z - z.position.z : pdz, Z_TRACK, dt);
    if (!ud.struck && ud.swing >= Z_STRIKE) {
      ud.struck = true;
      if (o) hitOpening(o);
      else if (!(pd < Z_HIT_RANGE && !wallBetween(z.position, p) && playerHurtHandler())) audio.swipe(z.position);
    }
    if (ud.swing >= Z_SWING) { ud.swing = -1; ud.swingCd = rnd(0.25, 0.6); }
  } else if (pd < (ud.standing ? Z_LET_GO : Z_REACH) && !wallBetween(z.position, p)) {
    // at arm's length: stand, square up, and swing as soon as it's facing you (no
    // cooldown left over from battering its way in)
    if (!ud.standing) ud.swingCd = 0;
    ud.standing = true;
    const off = faceToward(z, pdx, pdz, Z_TRACK, dt);
    ud.swingCd -= dt;
    if (ud.swingCd <= 0 && off < 0.5) { ud.swing = 0; ud.struck = false; ud.swingAt = null; }
  } else {
    ud.standing = false;
    // steer toward nearest opening if outside, else toward player
    let tx = p.x, tz = p.z;
    if (inside(z.position)) {
      ud.opening = null; ud.stage = 0; // through the wall — re-pick if knocked back out
    } else {
      // Commit to one opening (no per-frame re-pick): walk up via its outer waypoint so
      // it comes at the door or window square-on, then break it and go through
      if (!ud.opening) { ud.opening = nearestOpening(z.position); ud.stage = 0; }
      const o = ud.opening;
      if (ud.stage === 0) {
        tx = o.ox; tz = o.oz;
        if ((o.ox - z.position.x) ** 2 + (o.oz - z.position.z) ** 2 < 1.4) ud.stage = 1;
      }
      if (ud.stage === 1) {
        if (o.kind === 'door' && o.hp <= 0) { tx = o.ix; tz = o.iz; } // broken down: straight through
        else if ((o.x - z.position.x) ** 2 + (o.z - z.position.z) ** 2 > 1.45 ** 2) { tx = o.fx; tz = o.fz; }
        else {
          // at it: batter it in, or take a turn climbing through the broken window
          tx = null;
          const off = faceToward(z, o.x - z.position.x, o.z - z.position.z, Z_TRACK, dt);
          ud.swingCd -= dt;
          if (o.hp > 0) { if (ud.swingCd <= 0 && off < 0.5) { ud.swing = 0; ud.struck = false; ud.swingAt = o; } }
          else if (!o.climber) startClimb(z, o);
        }
      }
    }
    if (tx !== null) {
      const g = routeAround(z, tx, tz);
      // turn harder up close so the last few steps home in on you
      walkToward(z, g.x - z.position.x, g.z - z.position.z, ud.speed, ud.turn * (pd < 4 ? 2.5 : 1), dt);
      walking = true;
    }
  }
  // progress check: a zombie that's been trying to walk for 2s but has barely moved (jammed
  // in a crowd at a door, say) forgets its plan and makes for the next-nearest way in
  if (walking && (ud.stuckT += dt) > 2) {
    if (Math.hypot(z.position.x - ud.stuckX, z.position.z - ud.stuckZ) < 0.6 && ud.opening && !inside(z.position)) {
      ud.opening = nearestOpening(z.position, ud.opening);
      ud.stage = 0; ud.via = -1;
    }
    ud.stuckT = 0;
  }
  if (!walking || ud.stuckT === 0) { ud.stuckX = z.position.x; ud.stuckZ = z.position.z; if (!walking) ud.stuckT = 0; }
  ud.gait = damp(ud.gait, walking ? 1 : 0, 8, dt);
  animateZombie(z, dt, pd);
}

// keep the crowd from stacking into one body, out of the survivor, and out of the props
export function separateZombies() {
  for (let i = 0; i < zombies.length; i++) {
    const a = zombies[i].position;
    for (let k = i + 1; k < zombies.length; k++) {
      const b = zombies[k].position, dx = b.x - a.x, dz = b.z - a.z, d2 = dx * dx + dz * dz;
      if (d2 >= Z_SPACE * Z_SPACE || d2 < 1e-6) continue;
      const d = Math.sqrt(d2), push = (Z_SPACE - d) / 2 / d;
      a.x -= dx * push; a.z -= dz * push;
      b.x += dx * push; b.z += dz * push;
    }
  }
  for (const z of zombies) {
    if (z.userData.climb >= 0) continue; // on the sill: the climb places it
    if (state.running) {
      const dx = z.position.x - player.position.x, dz = z.position.z - player.position.z, d = Math.hypot(dx, dz);
      if (d < Z_CLOSE && d > 1e-6) { z.position.x += dx / d * (Z_CLOSE - d); z.position.z += dz / d * (Z_CLOSE - d); }
    }
    pushOutOfSolids(z.position);
  }
}
