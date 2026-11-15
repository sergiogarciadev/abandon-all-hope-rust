import * as THREE from 'three';
import { scene } from '../core/scene.js';
import { BOX, bevelBox } from '../core/blocks.js';
import { state } from '../core/state.js';
import { audio, setAudioListener } from '../audio/audio.js';
import { wrapA, damp, ease } from '../core/math.js';
import { footCycle, ik2, bone } from './animation.js';
import { casingGeo, smokeMats, getNextSmokeMat, spawnDebris } from '../effects/debris.js';

// ===================================================================
// PLAYER — a blocky survivor rig. Arms and legs are two-bone IK chains:
// both hands stay locked onto the rifle (so recoil and the reload carry
// the arms with them) and the feet follow a procedural run cycle that
// plants each boot while it's on the ground.
// ===================================================================
export const player = new THREE.Group();
setAudioListener(player);

// one rig block, sized in world units
function blk(mat, w, h, d, x, y, z, parent, r) {
  const m = new THREE.Mesh(bevelBox(w, h, d, r), mat);
  m.position.set(x, y, z);
  m.castShadow = true;
  parent.add(m);
  return m;
}

// red-and-black buffalo check flannel, 8x8 texels a repeat: crisp pixels up
// close, and the mipmaps blend it to a dark red at gameplay distance
function plaidTexture() {
  const c = document.createElement('canvas');
  c.width = c.height = 8;
  const g = c.getContext('2d');
  for (let i = 0; i < 8; i++) for (let j = 0; j < 8; j++) {
    const warp = i < 4, weft = j < 4;
    g.fillStyle = warp && weft ? '#191314' : warp || weft ? '#5c1b18' : '#a8332b';
    g.fillRect(i, j, 1, 1);
  }
  const t = new THREE.CanvasTexture(c);
  t.wrapS = t.wrapT = THREE.RepeatWrapping;
  t.magFilter = THREE.NearestFilter;
  t.anisotropy = 4;
  t.colorSpace = THREE.SRGBColorSpace;
  return t;
}

// materials — a hunter's kit: flannel, olive chest rig, blaze-orange beanie
// (the beanie and pack are what read from the top-down camera)
const mat = (color, o) => new THREE.MeshStandardMaterial({ color, roughness: 0.9, ...o });
const M = {
  flannel: mat(0xffffff, { map: plaidTexture() }),
  lining:  mat(0xa88d68, { roughness: 1 }),   // sherpa collar
  vest:    mat(0x515737),
  vestDk:  mat(0x3a3e27),
  pants:   mat(0x39404d),
  leather: mat(0x5e3f27, { roughness: 0.7 }),
  sole:    mat(0x17130f),
  skin:    mat(0xe0915c),
  hair:    mat(0x3a2517, { roughness: 1 }),
  beard:   mat(0x5b3920, { roughness: 1 }),
  beanie:  mat(0xe0662a, { roughness: 1 }),
  cuff:    mat(0xb9501f, { roughness: 1 }),
  eye:     mat(0x16100d, { roughness: 0.4 }),
  canvas:  mat(0x6f5d3f),                     // pack + rolled cuffs
  bedroll: mat(0x3e4a36),
  brass:   mat(0xc9a14a, { roughness: 0.35, metalness: 0.7 }),
  metal:   mat(0x2b2e34, { roughness: 0.42, metalness: 0.45 }),
  wood:    mat(0x7b4a29, { roughness: 0.65 }),
  rubber:  mat(0x151518),
  lens:    mat(0x10161e, { roughness: 0.15, emissive: 0x2a6fb8, emissiveIntensity: 0.2 }),
};

// rig proportions (world units; the survivor stands ~2.6 tall)
const THIGH = 0.44, SHIN = 0.42, ANKLE_H = 0.15, HIP_H = 0.97;
const UPPER_ARM = 0.42, FOREARM = 0.42;

// --- lower body: `lower` turns with the legs, `pelvis` bobs/rolls/twists with the stride ---
const lower = new THREE.Group(); player.add(lower);
const pelvis = new THREE.Group(); lower.add(pelvis);
blk(M.pants, 0.62, 0.3, 0.4, 0, 0.02, 0, pelvis);              // hips
blk(M.leather, 0.66, 0.08, 0.43, 0, 0.12, 0, pelvis, 0.02);    // belt
blk(M.brass, 0.1, 0.07, 0.03, 0, 0.12, 0.22, pelvis, 0.01);    // buckle
blk(M.vestDk, 0.12, 0.17, 0.17, 0.36, 0.02, 0.04, pelvis);     // hip pouch (left)
blk(M.leather, 0.07, 0.28, 0.1, -0.35, -0.04, -0.03, pelvis);  // knife sheath (right)

// +X is the survivor's left (they face +Z)
const legs = [1, -1].map((side, i) => {
  const thigh = blk(M.pants, 0.26, THIGH + 0.08, 0.29, 0, 0, 0, lower);
  const shin = blk(M.pants, 0.24, SHIN + 0.06, 0.27, 0, 0, 0, lower);
  blk(M.vestDk, 0.2, 0.17, 0.07, 0, SHIN / 2 - 0.03, 0.15, shin);  // knee pad
  const boot = new THREE.Group(); lower.add(boot);                  // origin at the ankle
  blk(M.leather, 0.27, 0.17, 0.29, 0, 0, -0.01, boot);              // shaft
  blk(M.leather, 0.27, 0.1, 0.42, 0, -0.08, 0.06, boot);            // foot
  blk(M.sole, 0.29, 0.05, 0.45, 0, -0.13, 0.06, boot, 0.015);       // sole
  return {
    side, thigh, shin, boot, phase: i * 0.5, prev: 0,
    hip: new THREE.Vector3(side * 0.16, 0, 0),
    hipP: new THREE.Vector3(), foot: new THREE.Vector3(), knee: new THREE.Vector3(), ankle: new THREE.Vector3(),
  };
});

// --- upper body (origin at the waist) ---
const upper = new THREE.Group(); upper.rotation.order = 'YXZ'; player.add(upper);
blk(M.flannel, 0.74, 0.6, 0.42, 0, 0.36, 0, upper);             // jacket
blk(M.flannel, 0.84, 0.26, 0.46, 0, 0.62, 0, upper);            // shoulder yoke
blk(M.lining, 0.5, 0.1, 0.4, 0, 0.78, -0.02, upper);            // sherpa collar
blk(M.vest, 0.6, 0.4, 0.07, 0, 0.38, 0.22, upper);              // chest rig
for (const x of [-0.17, 0, 0.17]) blk(M.vestDk, 0.15, 0.17, 0.08, x, 0.33, 0.28, upper); // mag pouches
blk(M.vest, 0.6, 0.48, 0.06, 0, 0.42, -0.23, upper);            // rig back panel
for (const x of [-0.21, 0.21]) blk(M.vest, 0.12, 0.05, 0.5, x, 0.77, 0, upper); // shoulder straps
blk(M.canvas, 0.48, 0.42, 0.18, 0, 0.36, -0.33, upper);         // pack
blk(M.vestDk, 0.3, 0.14, 0.06, 0, 0.27, -0.44, upper);          // pack pocket
blk(M.bedroll, 0.68, 0.17, 0.17, 0, 0.66, -0.36, upper);        // bedroll
for (const x of [-0.2, 0.2]) blk(M.leather, 0.05, 0.19, 0.19, x, 0.66, -0.36, upper, 0.01); // bedroll straps

// head turns back toward the aim line from the bladed torso
const head = new THREE.Group(); head.position.set(0, 0.8, 0.02); head.rotation.order = 'YXZ'; upper.add(head);
blk(M.skin, 0.2, 0.14, 0.2, 0, 0.04, 0, head);                  // neck
blk(M.skin, 0.52, 0.5, 0.5, 0, 0.32, 0, head);                  // head
blk(M.hair, 0.54, 0.2, 0.2, 0, 0.36, -0.17, head);              // hair at the back
for (const x of [-0.25, 0.25]) blk(M.beard, 0.05, 0.2, 0.3, x, 0.19, 0.08, head, 0.015); // beard along the jaw
blk(M.beard, 0.46, 0.12, 0.06, 0, 0.12, 0.24, head, 0.02);      // beard
blk(M.beard, 0.2, 0.04, 0.03, 0, 0.21, 0.26, head, 0.01);       // moustache
blk(M.skin, 0.08, 0.1, 0.07, 0, 0.27, 0.27, head, 0.015);       // nose
for (const x of [-0.12, 0.12]) {
  blk(M.eye, 0.07, 0.07, 0.02, x, 0.34, 0.255, head, 0.005);    // eyes
  blk(M.hair, 0.13, 0.04, 0.03, x, 0.41, 0.255, head, 0.008);   // brows
  blk(M.skin, 0.05, 0.11, 0.1, x * 2.25, 0.32, 0, head, 0.012); // ears
}
blk(M.cuff, 0.58, 0.1, 0.56, 0, 0.49, -0.01, head);             // beanie cuff
blk(M.beanie, 0.55, 0.2, 0.53, 0, 0.6, -0.01, head);            // beanie
blk(M.beanie, 0.18, 0.06, 0.18, 0, 0.72, -0.01, head, 0.02);    // knit top

// --- rifle: origin at the butt plate, barrel along +Z ---
const gun = new THREE.Group(); gun.rotation.order = 'YXZ'; player.add(gun);
blk(M.rubber, 0.13, 0.19, 0.04, 0, -0.02, 0.02, gun, 0.01);     // butt pad
blk(M.wood, 0.12, 0.17, 0.24, 0, -0.02, 0.15, gun);             // stock
blk(M.wood, 0.1, 0.2, 0.1, 0, -0.11, 0.29, gun).rotation.x = 0.35; // pistol grip
blk(M.metal, 0.14, 0.15, 0.4, 0, 0.03, 0.44, gun);              // receiver
blk(M.metal, 0.04, 0.03, 0.12, 0, -0.07, 0.37, gun, 0.01);      // trigger guard
const mag = blk(M.metal, 0.09, 0.24, 0.12, 0, -0.14, 0.52, gun); mag.rotation.x = -0.12;
blk(M.wood, 0.13, 0.13, 0.36, 0, 0.01, 0.8, gun);               // handguard
blk(M.metal, 0.06, 0.06, 0.46, 0, 0.03, 1.18, gun, 0.015);      // barrel
blk(M.metal, 0.085, 0.085, 0.08, 0, 0.03, 1.42, gun, 0.015);    // muzzle brake
blk(M.metal, 0.03, 0.07, 0.03, 0, 0.09, 1.34, gun, 0.008);      // front sight
blk(M.rubber, 0.075, 0.075, 0.4, 0, 0.17, 0.46, gun, 0.02);     // scope tube
blk(M.rubber, 0.1, 0.1, 0.08, 0, 0.17, 0.28, gun, 0.02);        // eyepiece
blk(M.rubber, 0.11, 0.11, 0.09, 0, 0.17, 0.66, gun, 0.02);      // objective bell
blk(M.lens, 0.08, 0.08, 0.01, 0, 0.17, 0.706, gun, 0.004);      // lens glint
for (const z of [0.38, 0.56]) blk(M.metal, 0.04, 0.07, 0.04, 0, 0.11, z, gun, 0.01); // scope rings
const GRIP_R = new THREE.Vector3(0, -0.1, 0.3);    // firing hand on the pistol grip
const GRIP_L = new THREE.Vector3(0, -0.07, 0.74);  // support hand under the handguard
const MAG_Y = mag.position.y;
const MAG_BASE = new THREE.Vector3(0, -0.29, 0.53); // bottom of the seated magazine
const EJECT = new THREE.Vector3(-0.08, 0.06, 0.48); // ejection port (right side)
// the fresh magazine the support hand brings up from the chest rig
const spareMag = new THREE.Mesh(mag.geometry, M.metal); spareMag.castShadow = true; player.add(spareMag);
// bullets spawn here
export const muzzle = new THREE.Object3D(); muzzle.position.set(0, 0.03, 1.47); gun.add(muzzle);

// --- arms: built along local Y, laid between the joints every frame ---
const arms = [1, -1].map(side => {
  const up = blk(M.flannel, 0.2, UPPER_ARM + 0.1, 0.21, 0, 0, 0, player);
  blk(M.flannel, 0.25, 0.24, 0.25, 0, UPPER_ARM / 2, 0, up);         // shoulder
  const fore = blk(M.flannel, 0.18, FOREARM + 0.08, 0.19, 0, 0, 0, player);
  blk(M.canvas, 0.21, 0.08, 0.22, 0, -FOREARM / 2 + 0.05, 0, fore);  // rolled cuff
  const hand = blk(M.skin, 0.14, 0.15, 0.17, 0, 0, 0, player);
  return {
    side, up, fore, hand,
    shoulder: new THREE.Vector3(side * 0.41, 0.7, 0),
    pole: side > 0 ? new THREE.Vector3(0.5, -1, -0.1) : new THREE.Vector3(-0.8, -1, -0.5), // elbows down & out
    sh: new THREE.Vector3(), elbow: new THREE.Vector3(), wrist: new THREE.Vector3(),
  };
});

// --- muzzle flash: a real light (with shadows) plus an additive star of blocks and a soft glow ---
const FLASH_COLOR = 0xffa64d, FLASH_HOT = 0xff6a2a; // normal / DOUBLE DAMAGE
const muzzleLight = new THREE.PointLight(FLASH_COLOR, 0, 14, 2);
muzzleLight.castShadow = true;
muzzleLight.shadow.mapSize.set(512, 512);
muzzleLight.shadow.camera.near = 0.1;
muzzleLight.shadow.bias = -0.004;
// only re-rendered on frames the flash is lit, so an idle gun costs no shadow passes
muzzleLight.shadow.autoUpdate = false;
muzzleLight.shadow.needsUpdate = true;
scene.add(muzzleLight);

function makeGlowTexture() {
  const c = document.createElement('canvas');
  c.width = c.height = 64;
  const g = c.getContext('2d');
  const grad = g.createRadialGradient(32, 32, 0, 32, 32, 32);
  grad.addColorStop(0, 'rgba(255,255,255,1)');
  grad.addColorStop(0.25, 'rgba(255,255,255,0.45)');
  grad.addColorStop(1, 'rgba(255,255,255,0)');
  g.fillStyle = grad;
  g.fillRect(0, 0, 64, 64);
  const t = new THREE.CanvasTexture(c);
  t.colorSpace = THREE.SRGBColorSpace;
  return t;
}

const additive = color => new THREE.MeshBasicMaterial({
  color, transparent: true, opacity: 0, blending: THREE.AdditiveBlending, depthWrite: false,
});
const flashCoreMat = additive(0xfff3d0), flashFireMat = additive(FLASH_COLOR);
// four-sided cone, base at the origin, tip at +Z: a faceted tongue of flame
const FLAME = new THREE.ConeGeometry(0.5, 1, 4).rotateX(Math.PI / 2).translate(0, 0, 0.5);
const flash = new THREE.Group(); flash.visible = false; muzzle.add(flash);
const flashCore = new THREE.Mesh(new THREE.OctahedronGeometry(0.5), flashCoreMat); flash.add(flashCore);
const flashSpikes = [];
for (let i = 0; i < 6; i++) {
  // 0: white-hot core jet, 1: the orange jet around it, 2-5: petals fanning out to the sides
  const pivot = new THREE.Group();
  pivot.rotation.z = i * Math.PI / 2 + Math.PI / 4;
  const tilt = new THREE.Group(); tilt.rotation.x = i > 1 ? -1.05 : 0; pivot.add(tilt);
  const spike = new THREE.Mesh(FLAME, i ? flashFireMat : flashCoreMat); tilt.add(spike);
  flash.add(pivot);
  flashSpikes.push(spike);
}
const flashGlow = new THREE.Sprite(new THREE.SpriteMaterial({
  map: makeGlowTexture(), color: FLASH_COLOR, transparent: true, opacity: 0,
  blending: THREE.AdditiveBlending, depthWrite: false,
}));
flashGlow.position.z = 0.15;
flash.add(flashGlow);
scene.add(player);

// --- animation state ---
export const rig = {
  phase: 0,      // gait cycle, 0..1
  gait: 0,       // 0 standing .. 1 full stride
  turnW: 0,      // stepping in place while turning on the spot
  turning: false,
  hipYaw: 0,     // lower body's world yaw
  back: false,   // backpedaling (hips face away from the travel direction)
  stride: new THREE.Vector3(0, 0, 1), // travel direction in hip space
  leanF: 0, leanS: 0,
  kick: 0,       // recoil, 1 on the shot
  flash: 0,      // muzzle flash, 1 on the shot
  flashPeak: 0,
  reload: 0,     // reload pose weight
  magDropped: false,
  t: 0,
};

const BETA = 0.38;      // fraction of the gait cycle each boot spends planted (under half: a run)
const HIP_MAX = 1.0;    // furthest the hips turn away from the aim (rad)
const IDLE_HIP = -0.35; // standing: hips bladed, left foot leading
const BLADE = -0.62;    // torso turned so the left shoulder leads toward the target
const POCKET = new THREE.Vector3(-0.13, 0.6, 0.22);   // right shoulder pocket the stock sits in (upper-local)
const POUCH = new THREE.Vector3(0.17, 0.33, 0.36);    // chest-rig pouch the spare mag comes from (upper-local)
const KNEE_POLE = new THREE.Vector3(0, 0, 1);
const PLAYER_SPEED = 6.2;

// called by fire(): kick the rifle back, light the flash, spit a casing and some smoke
export function muzzleFlash(hot) {
  rig.kick = 1;
  rig.flash = 1;
  rig.flashPeak = 40 + Math.random() * 20;
  const color = hot ? FLASH_HOT : FLASH_COLOR;
  muzzleLight.color.setHex(color);
  flashFireMat.color.setHex(color);
  flashGlow.material.color.setHex(color);
  // a different star every shot
  flash.rotation.z = Math.random() * Math.PI;
  flash.scale.setScalar(0.85 + Math.random() * 0.4);
  flashCore.scale.setScalar(0.16 + Math.random() * 0.06);
  flashSpikes.forEach((s, i) => {
    const len = i > 1 ? 0.2 + Math.random() * 0.22 : (i ? 0.65 : 0.4) + Math.random() * 0.35;
    const w = i > 1 ? 0.09 : i ? 0.24 : 0.13;
    s.scale.set(w, w, len);
  });
  flashGlow.scale.setScalar(2 + Math.random() * 0.6);

  gun.updateWorldMatrix(true, false);
  // brass flips out of the ejection port to the right
  const casing = new THREE.Mesh(casingGeo, M.brass);
  casing.position.copy(EJECT).applyMatrix4(gun.matrixWorld);
  casing.quaternion.setFromRotationMatrix(gun.matrixWorld);
  const right = new THREE.Vector3(-1, 0, 0).transformDirection(gun.matrixWorld);
  const back = new THREE.Vector3(0, 0, -1).transformDirection(gun.matrixWorld);
  const vel = right.multiplyScalar(2.4 + Math.random() * 1.2).addScaledVector(back, 0.6).add(new THREE.Vector3(0, 3 + Math.random(), 0));
  spawnDebris(casing, vel, {
    life: 2.5, bounce: 0.025,
    spin: new THREE.Vector3(Math.random() * 20 - 10, Math.random() * 20 - 10, Math.random() * 20 - 10),
  });
  // a puff of smoke from the muzzle
  const smoke = new THREE.Mesh(BOX, getNextSmokeMat());
  muzzle.getWorldPosition(smoke.position);
  smoke.rotation.set(Math.random() * 3, Math.random() * 3, 0);
  const fwd = new THREE.Vector3(0, 0, 1).transformDirection(gun.matrixWorld);
  spawnDebris(smoke, fwd.multiplyScalar(1.6), { life: 0.55, smoke: 0.14 + Math.random() * 0.06 });
}

const _v = new THREE.Vector3(), _w = new THREE.Vector3(), _gR = new THREE.Vector3(), _gL = new THREE.Vector3();
// the support hand's path through a reload: [u (0..1 of the reload), point, its space]
const MAG_OUT = MAG_BASE.clone().setY(MAG_BASE.y - 0.24), MAG_LINEUP = MAG_BASE.clone().setY(MAG_BASE.y - 0.2);
const RELOAD_PATH = [
  [0, GRIP_L, 'gun'],       // on the handguard
  [0.12, MAG_BASE, 'gun'],  // grab the mag
  [0.22, MAG_OUT, 'gun'],   // rip it out
  [0.45, POUCH, 'upper'],   // fresh one off the chest rig
  [0.55, POUCH, 'upper'],
  [0.78, MAG_LINEUP, 'gun'], // line it up under the magwell
  [0.95, MAG_BASE, 'gun'],  // seat it
];
const _ra = new THREE.Vector3(), _rb = new THREE.Vector3();
function reloadHand(u, out) {
  const at = (k, v) => v.copy(RELOAD_PATH[k][1]).applyMatrix4(RELOAD_PATH[k][2] === 'gun' ? gun.matrix : upper.matrix);
  let i = RELOAD_PATH.length - 1;
  while (i > 0 && u < RELOAD_PATH[i][0]) i--;
  if (i === RELOAD_PATH.length - 1) return at(i, out);
  const t = (u - RELOAD_PATH[i][0]) / (RELOAD_PATH[i + 1][0] - RELOAD_PATH[i][0]);
  return out.copy(at(i, _ra)).lerp(at(i + 1, _rb), ease(t));
}

// pose the whole rig for this frame. (mx, mz) is the survivor's world velocity.
export function animatePlayer(dt, mx, mz) {
  const pace = Math.min(1, Math.hypot(mx, mz) / state.speed);
  const moving = pace > 0.05;
  const aim = player.rotation.y;
  rig.t += dt;
  rig.gait = damp(rig.gait, pace, moving ? 9 : 7, dt);

  // --- lower-body heading: the hips turn toward the travel direction (or away
  // from it when backpedaling) but never more than HIP_MAX off the aim, and the
  // feet step along whatever direction is left over ---
  const moveYaw = Math.atan2(mx, mz);
  let hipTarget = rig.hipYaw;
  if (moving) {
    const rel = wrapA(moveYaw - aim);
    if (rig.back ? Math.abs(rel) < 1.25 : Math.abs(rel) > 1.9) rig.back = !rig.back; // hysteresis
    hipTarget = aim + THREE.MathUtils.clamp(rig.back ? wrapA(rel - Math.PI) : rel, -HIP_MAX, HIP_MAX);
    rig.turning = false;
  } else {
    // standing: feet stay planted until the aim twists the hips too far, then step around
    const idle = aim + IDLE_HIP;
    if (Math.abs(wrapA(idle - rig.hipYaw)) > 0.8) rig.turning = true;
    if (rig.turning) {
      hipTarget = idle;
      if (Math.abs(wrapA(idle - rig.hipYaw)) < 0.06) rig.turning = false;
    }
  }
  rig.hipYaw = wrapA(rig.hipYaw + wrapA(hipTarget - rig.hipYaw) * (1 - Math.exp(-10 * dt)));
  const hipRel = wrapA(rig.hipYaw - aim);
  lower.rotation.y = hipRel;
  if (moving) { const s = moveYaw - rig.hipYaw; rig.stride.set(Math.sin(s), 0, Math.cos(s)); }

  // --- gait clock: cadence rises gently with speed (FLEET FOOT) ---
  rig.turnW = damp(rig.turnW, rig.turning ? 1 : 0, 12, dt);
  const drive = Math.max(rig.gait, rig.turnW * 0.7);
  const freq = 2.45 * Math.sqrt(state.speed / PLAYER_SPEED); // strides per second
  if (drive > 0.02) rig.phase = (rig.phase + freq * dt) % 1;
  const ph = rig.phase * Math.PI * 2;
  // stride length is what the speed asks for, capped at what the legs can reach
  const L = Math.min(0.86, state.speed * BETA / freq) * rig.gait;
  const liftH = 0.2 * rig.gait + 0.1 * rig.turnW;

  // --- pelvis: lowest as each boot takes the weight, highest in between ---
  const bob = -0.04 * rig.gait * Math.cos(2 * (ph - Math.PI * BETA));
  const pelvisY = HIP_H - 0.11 * rig.gait + bob; // runners sit lower, which buys the legs reach
  pelvis.position.y = pelvisY;
  pelvis.rotation.y = 0.14 * rig.gait * Math.sin(ph) * rig.stride.z; // hips swing with the legs
  pelvis.rotation.z = 0.05 * rig.gait * Math.sin(ph);                // hip drop
  pelvis.updateMatrix();

  // --- legs: each boot is planted for BETA of the cycle, then arcs forward ---
  for (const leg of legs) {
    const p = (rig.phase + leg.phase) % 1;
    const { s, lift, pitch } = footCycle(p, BETA);
    if (p < leg.prev && drive > 0.3) audio.step();    // a footfall each time a boot lands
    leg.prev = p;
    const restZ = leg.side > 0 ? 0.12 : -0.1;          // standing: left foot leads
    leg.foot.set(leg.side * (0.17 + 0.03 * (1 - rig.gait)), ANKLE_H + lift * liftH, restZ * (1 - rig.gait))
      .addScaledVector(rig.stride, s * L);
    leg.hipP.copy(leg.hip).applyMatrix4(pelvis.matrix);
    ik2(leg.hipP, leg.foot, THIGH, SHIN, KNEE_POLE, leg.knee);
    leg.ankle.subVectors(leg.foot, leg.knee).normalize().multiplyScalar(SHIN).add(leg.knee);
    bone(leg.thigh, leg.hipP, leg.knee, KNEE_POLE);
    bone(leg.shin, leg.knee, leg.ankle, KNEE_POLE);
    leg.boot.position.copy(leg.ankle);
    leg.boot.rotation.set(pitch * rig.gait * Math.sign(rig.stride.z || 1), leg.side * 0.08, 0);
  }

  // --- upper body: bladed toward the gun, half-following the hips, leaning into the run ---
  const breathe = Math.sin(rig.t * 2.1) * (1 - rig.gait);
  const tw = BLADE + 0.3 * (hipRel - BLADE) - 0.07 * rig.gait * Math.sin(ph) * rig.stride.z;
  const travel = moveYaw - (aim + tw);
  rig.leanF = damp(rig.leanF, moving ? Math.cos(travel) : 0, 8, dt);
  rig.leanS = damp(rig.leanS, moving ? Math.sin(travel) : 0, 8, dt);
  rig.kick *= Math.exp(-14 * dt);
  upper.position.set(0, pelvisY + 0.1 + 0.006 * breathe, 0);
  upper.rotation.set(0.05 + 0.1 * rig.leanF * rig.gait - 0.08 * rig.kick, tw, -0.07 * rig.leanS * rig.gait);
  upper.updateMatrix();

  // --- reload: tip the rifle in, strip the mag, pull a fresh one off the chest rig ---
  rig.reload = damp(rig.reload, state.reloading ? 1 : 0, state.reloading ? 14 : 9, dt);
  const rl = ease(rig.reload);
  const u = state.reloading ? state.reloadT / state.reloadTime : 1;
  head.rotation.set(0.1 + 0.32 * rl, -tw * 0.85 + 0.25 * rl, 0);

  // --- rifle: butt in the shoulder pocket, barrel down the aim line ---
  _v.copy(POCKET).applyMatrix4(upper.matrix);
  gun.position.set(_v.x + 0.1 * rl, _v.y - 0.1 * rl + 0.012 * breathe, _v.z - 0.11 * rig.kick - 0.08 * rl);
  gun.rotation.set(-0.16 * rig.kick - 0.35 * rl + 0.01 * Math.sin(rig.t * 1.3), 0.15 * rl, 0.65 * rl);
  gun.updateMatrix();

  // magazine: rides out with the hand, drops free, the spare rides in from the pouch
  if (!state.reloading || u < 0.12) rig.magDropped = false;
  let magDrop = 0;
  if (state.reloading && u >= 0.12 && u < 0.22) magDrop = 0.24 * ease((u - 0.12) / 0.1);
  if (state.reloading && u >= 0.78) magDrop = 0.2 * (1 - ease(Math.min(1, (u - 0.78) / 0.17)));
  mag.position.y = MAG_Y - magDrop;
  mag.visible = !state.reloading || u < 0.22 || u >= 0.78;
  if (state.reloading && u >= 0.22 && !rig.magDropped) {
    rig.magDropped = true;
    const old = new THREE.Mesh(mag.geometry, M.metal);
    mag.getWorldPosition(old.position); mag.getWorldQuaternion(old.quaternion);
    spawnDebris(old, new THREE.Vector3(0, -1, 0), { life: 4, bounce: 0.05, spin: new THREE.Vector3(3, 0, 2) });
  }

  // hands: the firing hand never leaves the grip; the support hand works the reload
  _gR.copy(GRIP_R).applyMatrix4(gun.matrix);
  _gL.copy(GRIP_L).applyMatrix4(gun.matrix);
  if (rig.reload > 0.001) _gL.lerp(reloadHand(u, _w), rl);
  for (const arm of arms) {
    arm.sh.copy(arm.shoulder).applyMatrix4(upper.matrix);
    const target = arm.side < 0 ? _gR : _gL;
    ik2(arm.sh, target, UPPER_ARM, FOREARM, arm.pole, arm.elbow);
    arm.wrist.subVectors(target, arm.elbow).normalize().multiplyScalar(FOREARM).add(arm.elbow);
    bone(arm.up, arm.sh, arm.elbow, arm.pole);
    bone(arm.fore, arm.elbow, arm.wrist, arm.pole);
    arm.hand.position.copy(arm.wrist);
    arm.hand.quaternion.copy(gun.quaternion);
  }
  spareMag.visible = state.reloading && u >= 0.45 && u < 0.78;
  if (spareMag.visible) {
    spareMag.quaternion.copy(gun.quaternion);
    spareMag.position.copy(arms[0].wrist).add(_w.set(0, 0.13, 0).applyQuaternion(gun.quaternion));
  }

  // --- muzzle flash: a few frames of fire, a real light with a fast falloff ---
  rig.flash = Math.max(0, rig.flash - dt / 0.07);
  const f = rig.flash;
  flash.visible = f > 0.3;
  flashCoreMat.opacity = Math.min(1, f * 1.4);
  flashFireMat.opacity = Math.min(1, f * 1.2);
  flashGlow.material.opacity = 0.85 * f;
  muzzleLight.intensity = rig.flashPeak * f * f;
  if (f > 0) {
    muzzle.getWorldPosition(muzzleLight.position);
    muzzleLight.position.x += Math.sin(aim) * 0.4; // just ahead of the brake, so the barrel
    muzzleLight.position.z += Math.cos(aim) * 0.4; // doesn't shadow the shooter
    muzzleLight.shadow.needsUpdate = true;
  }
}
