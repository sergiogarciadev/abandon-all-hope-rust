import * as THREE from 'three';
import { BOX, bakeBlocks, seeded } from '../core/blocks.js';
import { rnd, ease } from '../core/math.js';
import { spawnDebris } from '../effects/debris.js';
import { audio } from '../audio/audio.js';

export const OPENING_INSET = 1.5; // how far in/out the entry waypoints sit from the wall plane
export const SILL = { x: 0.85, z: 1.05 }, HEAD = { x: 1.63, z: 1.83 }; // window openings fall between log courses
export const DOOR_TOP = 2.03, JAMB = 0.14;
export const DOOR_HP = 12, WINDOW_HP = 2; // blows it takes to break one in
export const CLIMB_TIME = 1.1; // seconds to clamber in over a sill

export const LOG_D = 0.62;
export const POST = 0x3a2b1d, WEATHERED = 0x6d6352;

export const glassMat = new THREE.MeshStandardMaterial({ color: 0x9fb6c9, roughness: 0.08, metalness: 0.4, transparent: true, opacity: 0.3, depthWrite: false });
export const glassCracked = new THREE.MeshStandardMaterial({ color: 0xd8e4ee, roughness: 0.45, metalness: 0.2, transparent: true, opacity: 0.55, depthWrite: false });
export const woodChip = new THREE.MeshStandardMaterial({ color: 0x6d6352, roughness: 0.9 });

export const openings = [];

const _place = new THREE.Matrix4();
const atMatrix = (x, z, ry = 0) => _place.makeRotationY(ry).setPosition(x, 0, z);

// An opening's geometry and waypoints. Zombies come in via the OUTER waypoint, stand at
// the FRONT to break it, then go through to the INNER one (or climb to the LANDING).
// Its parts are modeled in a local frame: x along the wall, +z out of the cabin.
export function makeOpening(axis, fixed, u0, u1, kind, wallT = 0.7) {
  const c = (u0 + u1) / 2, s = Math.sign(fixed), ht = wallT / 2;
  const p = out => axis === 'x' ? [c, fixed + s * out] : [fixed + s * out, c]; // world x, z at `out` from the wall
  const [x, z] = p(0), [ox, oz] = p(OPENING_INSET), [ix, iz] = p(-OPENING_INSET), [fx, fz] = p(0.85), [lx, lz] = p(-0.95);
  return {
    kind, axis, u0, u1, w: u1 - u0, x, z, ox, oz, ix, iz, fx, fz, lx, lz,
    sill: SILL[axis], head: HEAD[axis],
    rotY: axis === 'x' ? (s > 0 ? 0 : Math.PI) : s * Math.PI / 2,
    box: axis === 'x' ? { minX: u0, maxX: u1, minZ: fixed - ht, maxZ: fixed + ht } : { minX: fixed - ht, maxX: fixed + ht, minZ: u0, maxZ: u1 },
    maxHp: kind === 'door' ? DOOR_HP : WINDOW_HP, hp: 0,
    climber: null, // the zombie clambering through a broken window (one at a time)
    jolt: 0, fallT: -1,
  };
}

// frame it in the baked walls, and build the parts that move or break
export function fitOpening(o, world, wallB, worldMat, wrng) {
  const place = atMatrix(o.x, o.z, o.rotY), hw = o.w / 2, frng = seeded(Math.round(o.x * 7 + o.z * 13) + 99);
  const root = new THREE.Group();
  root.position.set(o.x, 0, o.z); root.rotation.y = o.rotY;
  world.add(root);
  if (o.kind === 'door') {
    wallB.add([
      [JAMB, DOOR_TOP + 0.1, LOG_D + 0.1, -hw + JAMB / 2, (DOOR_TOP + 0.1) / 2, 0, POST, 0, 0, 0, 0.03],  // posts
      [JAMB, DOOR_TOP + 0.1, LOG_D + 0.1, hw - JAMB / 2, (DOOR_TOP + 0.1) / 2, 0, POST, 0, 0, 0, 0.03],
      [o.w + 0.1, 0.14, LOG_D + 0.12, 0, DOOR_TOP + 0.02, 0, POST, 0, 0, 0, 0.03],                     // header
      [o.w - 0.1, 0.05, LOG_D, 0, 0.0, 0, POST, 0, 0, 0, 0.02],                                       // threshold
    ], wrng, place);
    // a ledged-and-braced plank door, hinged at its left edge, braces on the inside face
    const lw = o.w - 2 * JAMB - 0.02, lh = 1.92, n = 4, pw = lw / n;
    const blocks = [];
    for (let i = 0; i < n; i++) blocks.push([pw - 0.012, lh, 0.06, pw * (i + 0.5), lh / 2, 0, i % 2 ? WEATHERED : 0x5f5646, 0, 0, 0, 0.012]);
    blocks.push([lw - 0.04, 0.14, 0.04, lw / 2, 0.35, -0.05, POST, 0, 0, 0, 0.012], [lw - 0.04, 0.14, 0.04, lw / 2, lh - 0.35, -0.05, POST, 0, 0, 0, 0.012]);
    const bl = Math.hypot(lw - 0.1, lh - 0.7);
    blocks.push([0.12, bl, 0.04, lw / 2, lh / 2, -0.05, POST, 0, 0, -Math.atan2(lw - 0.1, lh - 0.7), 0.012]);
    blocks.push([0.05, 0.05, 0.08, lw - 0.15, 1.0, 0.06, 0x2a2a2a, 0, 0, 0, 0.01]);                                     // handle
    for (const y of [0.35, lh - 0.35]) blocks.push([0.32, 0.05, 0.015, 0.16, y, 0.035, 0x2a2a2a, 0, 0, 0, 0.005]);     // strap hinges
    const leafMesh = new THREE.Mesh(bakeBlocks(blocks, frng), worldMat);
    leafMesh.castShadow = true; leafMesh.receiveShadow = true;
    o.fall = new THREE.Group(); o.fall.position.z = -0.02; root.add(o.fall);                   // pivot it falls in about
    o.leaf = new THREE.Group(); o.leaf.position.set(-hw + JAMB + 0.01, 0.02, 0); o.fall.add(o.leaf); // hinge line
    o.leaf.add(leafMesh);
  } else {
    const h = o.head - o.sill, mid = (o.sill + o.head) / 2, iw = o.w - 0.2;
    wallB.add([
      [0.1, h + 0.1, LOG_D + 0.06, -hw + 0.05, mid, 0, POST, 0, 0, 0, 0.02],              // jambs
      [0.1, h + 0.1, LOG_D + 0.06, hw - 0.05, mid, 0, POST, 0, 0, 0, 0.02],
      [o.w + 0.12, 0.08, LOG_D + 0.08, 0, o.head, 0, POST, 0, 0, 0, 0.02],              // head
      [o.w + 0.3, 0.07, LOG_D + 0.24, 0, o.sill, 0.05, WEATHERED, 0, 0, 0, 0.02],        // sill board, lipped outside
    ], wrng, place);
    o.pane = new THREE.Group(); o.pane.position.y = mid; root.add(o.pane);
    o.glass = new THREE.Mesh(new THREE.BoxGeometry(iw, h - 0.08, 0.02), glassMat);
    const bars = new THREE.Mesh(bakeBlocks([[0.05, h - 0.08, 0.05, 0, 0, 0, POST], [iw, 0.05, 0.05, 0, 0, 0, POST]], frng), worldMat);
    bars.castShadow = true;
    o.bars = bars;
    o.pane.add(o.glass, bars);
    // what's left in the frame once it's smashed: jagged teeth round the edges
    const teeth = [];
    for (let i = 0; i < 9; i++) {
      const side = i % 3, t = frng() - 0.5, len = 0.12 + frng() * 0.2;
      const [px, py] = side === 0 ? [t * iw, -(h - 0.1) / 2 + 0.05] : side === 1 ? [-iw / 2 + 0.05, t * (h - 0.1)] : [iw / 2 - 0.05, t * (h - 0.1)];
      teeth.push([0.03 + frng() * 0.06, len, 0.015, px, py, 0, 0xffffff, 0, 0, (frng() - 0.5) * 1.6, 0]);
    }
    for (let i = 0; i < 3; i++) teeth.push([0.04 + frng() * 0.05, 0.1 + frng() * 0.12, 0.015, (frng() - 0.5) * iw, (h - 0.1) / 2 - 0.04, 0, 0xffffff, 0, 0, Math.PI + (frng() - 0.5) * 1.2, 0]);
    o.shards = new THREE.Mesh(bakeBlocks(teeth, frng, true), glassMat);
    o.pane.add(o.shards);
  }
}

export function resetOpenings() {
  for (const o of openings) {
    Object.assign(o, { hp: o.maxHp, climber: null, jolt: 0, fallT: -1 });
    if (o.kind === 'door') {
      o.leaf.rotation.y = 0;
      o.fall.rotation.x = 0; o.fall.position.set(0, 0, -0.02);
    } else {
      o.glass.visible = true; o.glass.material = glassMat;
      o.bars.visible = true;
      o.shards.visible = false;
    }
  }
}

// bits flying off a door or window, into the cabin
export function burst(o, n, mat, size, speed) {
  const sx = o.ix - o.x, sz = o.iz - o.z, l = Math.hypot(sx, sz);
  for (let i = 0; i < n; i++) {
    const m = new THREE.Mesh(BOX, mat);
    m.scale.set(size * rnd(0.4, 1), size * rnd(0.2, 0.6), size * rnd(1, 2.5));
    const along = rnd(-0.5, 0.5) * (o.w - 0.3);
    m.position.set(o.x + (o.axis === 'x' ? along : 0), o.kind === 'door' ? rnd(0.3, 1.8) : rnd(o.sill + 0.05, o.head - 0.05), o.z + (o.axis === 'z' ? along : 0));
    m.rotation.set(Math.random() * 3, Math.random() * 3, Math.random() * 3);
    const v = speed * rnd(0.5, 1.2);
    spawnDebris(m, new THREE.Vector3(sx / l * v + rnd(-1, 1), rnd(0.5, 2.5), sz / l * v + rnd(-1, 1)),
      { life: rnd(2, 3.5), bounce: 0.02, spin: new THREE.Vector3(rnd(-12, 12), rnd(-12, 12), rnd(-12, 12)) });
  }
}

// a zombie's blow (or the survivor's bullet, for glass) lands on a door or window
export function hitOpening(o) {
  if (o.hp <= 0) return;
  o.hp--;
  o.jolt = 1;
  if (o.kind === 'door') {
    audio.bash(o);
    burst(o, 2, woodChip, 0.12, 2);
  } else if (o.hp > 0) {
    audio.crack(o);
    o.glass.material = glassCracked;
  }
  if (o.hp <= 0) breakOpening(o);
}

export function breakOpening(o) {
  o.hp = 0;
  if (o.kind === 'door') {
    // off its hinges and down onto the floor
    o.fallT = 0;
    o.leaf.rotation.y = 0;
    audio.doorBreak(o);
    burst(o, 9, woodChip, 0.16, 3.5);
  } else {
    o.glass.visible = false;
    o.bars.visible = false;
    o.shards.visible = true;
    audio.shatter(o);
    burst(o, 16, glassMat, 0.09, 4);
  }
}

// does an unbroken door or window stand at (x, z)? (they stop bullets and blows)
export function shutAt(x, z) {
  for (const o of openings) {
    if (o.hp > 0 && x >= o.box.minX && x <= o.box.maxX && z >= o.box.minZ && z <= o.box.maxZ) return o;
  }
  return null;
}

export function updateOpenings(dt) {
  for (const o of openings) {
    o.jolt *= Math.exp(-9 * dt);
    if (o.kind === 'window') { o.pane.position.z = -0.03 * o.jolt; continue; }
    if (o.fallT >= 0) {
      // topples in like a slab: speeds up, slaps flat with a little bounce
      o.fallT += dt;
      const k = Math.min(1, o.fallT / 0.5), slap = k >= 1 ? 0.05 * Math.exp(-(o.fallT - 0.5) * 10) * Math.sin((o.fallT - 0.5) * 30) : 0;
      o.fall.rotation.x = -(Math.PI / 2 - 0.02) * k * k + slap;
      o.fall.position.set(0, 0.08 * k, -0.02 - 0.35 * k);
      continue;
    }
    if (o.hp <= 0) continue;
    // kicked inward on each blow
    o.leaf.rotation.y = o.jolt * 0.07;
  }
}

// start a zombie clambering in through a broken window
export function startClimb(z, o) {
  const ud = z.userData;
  o.climber = z;
  ud.climb = 0; ud.climbO = o;
  ud.climbFrom = z.position.clone();
  ud.heading = Math.atan2(o.lx - z.position.x, o.lz - z.position.z);
  z.rotation.y = ud.heading;
}

export function climbStep(z, dt) {
  const ud = z.userData, o = ud.climbO;
  ud.climb = Math.min(1, ud.climb + dt / CLIMB_TIME);
  const k = ud.climb, e = ease(k);
  z.position.x = ud.climbFrom.x + (o.lx - ud.climbFrom.x) * e;
  z.position.z = ud.climbFrom.z + (o.lz - ud.climbFrom.z) * e;
  // up onto the sill, over on its belly, and down inside
  z.position.y = (o.sill - 0.15) * Math.min(1, Math.sin(Math.PI * k) * 1.4);
  z.rotation.x = 0.75 * Math.sin(Math.PI * k);
  if (k >= 1) endClimb(z);
}

export function endClimb(z, died) {
  const ud = z.userData, o = ud.climbO;
  if (o.climber === z) o.climber = null;
  if (died && ud.climb < 0.5) z.position.copy(ud.climbFrom); // falls back out
  else z.position.set(o.lx, 0, o.lz);
  z.position.y = 0; z.rotation.x = 0;
  ud.climb = -1; ud.climbO = null; ud.opening = null; ud.stage = 0;
}

export function nearestOpening(pos, skip) {
  let best = null, bd = Infinity;
  for (const o of openings) {
    if (o === skip) continue;
    const d = (o.x - pos.x) ** 2 + (o.z - pos.z) ** 2;
    if (d < bd) { bd = d; best = o; }
  }
  return best;
}
