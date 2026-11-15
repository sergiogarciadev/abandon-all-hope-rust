// --- solid props (see SOLIDS): zombies are shoved out of them and plan a way round them ---
export const SOLIDS = [];
export function addSolid(x, z, ry, hx, hz) {
  SOLIDS.push({ x, z, hx, hz, c: Math.cos(ry), s: Math.sin(ry) });
}

export const Z_RADIUS = 0.4;   // a zombie's footprint
export const ROUND_PAD = 0.35; // how wide of a corner they swing
const _local = { x: 0, z: 0 };

export function toLocal(b, x, z) {
  const dx = x - b.x, dz = z - b.z;
  _local.x = dx * b.c - dz * b.s; _local.z = dx * b.s + dz * b.c;
  return _local;
}

export function pushOutOfSolids(pos) {
  for (const b of SOLIDS) {
    const { x: lx, z: lz } = toLocal(b, pos.x, pos.z);
    const ex = b.hx + Z_RADIUS, ez = b.hz + Z_RADIUS;
    const px = ex - Math.abs(lx), pz = ez - Math.abs(lz);
    if (px <= 0 || pz <= 0) continue;
    // out through the nearest face, a hair clear of it so it doesn't read as still touching
    const nx = px < pz ? Math.sign(lx) * (ex + 0.01) : lx, nz = px < pz ? lz : Math.sign(lz) * (ez + 0.01);
    pos.x = b.x + nx * b.c + nz * b.s;
    pos.z = b.z - nx * b.s + nz * b.c;
  }
}

// does the segment a→b (prop-local) cross the box ±hx, ±hz? (slab test)
export function segHitsBox(ax, az, bx, bz, hx, hz) {
  let t0 = 0, t1 = 1;
  const dx = bx - ax, dz = bz - az;
  if (Math.abs(dx) < 1e-9) { if (Math.abs(ax) >= hx) return false; }
  else {
    const ta = (-hx - ax) / dx, tb = (hx - ax) / dx;
    t0 = Math.max(t0, Math.min(ta, tb)); t1 = Math.min(t1, Math.max(ta, tb));
  }
  if (Math.abs(dz) < 1e-9) { if (Math.abs(az) >= hz) return false; }
  else {
    const ta = (-hz - az) / dz, tb = (hz - az) / dz;
    t0 = Math.max(t0, Math.min(ta, tb)); t1 = Math.min(t1, Math.max(ta, tb));
  }
  return t0 < t1;
}

// --- finding a way round the props: a shortest path over their corners (padded out so a
// zombie clears them), searched whenever a prop blocks the straight line to the target ---
const SEE_SLOP = 0.05; // a zombie pressed against a prop can still see along its side
export let ROUND_NODES = [];
export let NODE_LINKS = [];

export function clearPath(ax, az, bx, bz) {
  for (const b of SOLIDS) {
    const p = toLocal(b, ax, az), px = p.x, pz = p.z, q = toLocal(b, bx, bz);
    if (segHitsBox(px, pz, q.x, q.z, b.hx + Z_RADIUS - SEE_SLOP, b.hz + Z_RADIUS - SEE_SLOP)) return false;
  }
  return true;
}

export function initSolidsNav() {
  ROUND_NODES = SOLIDS.flatMap(b => [[1, 1], [1, -1], [-1, -1], [-1, 1]].map(([sx, sz]) => {
    const kx = sx * (b.hx + Z_RADIUS + ROUND_PAD), kz = sz * (b.hz + Z_RADIUS + ROUND_PAD);
    return { x: b.x + kx * b.c + kz * b.s, z: b.z - kx * b.s + kz * b.c };
  }));
  NODE_LINKS = ROUND_NODES.map(a => ROUND_NODES.map(b =>
    a !== b && clearPath(a.x, a.z, b.x, b.z) ? Math.hypot(a.x - b.x, a.z - b.z) : Infinity));
}

const _goal = { x: 0, z: 0 }, _from = { x: 0, z: 0 }, _toGoal = [], _done = [];
// Where a zombie should head to get to (tx, tz): straight there if nothing's in the way,
// otherwise the corner in sight that starts the shortest way round. It doesn't count a
// corner it's already standing at, and stays loyal to its last pick (ud.via) unless another
// way is clearly shorter, so it doesn't dither between two even routes.
export function routeAround(z, tx, tz) {
  const ud = z.userData;
  _goal.x = tx; _goal.z = tz;
  _from.x = z.position.x; _from.z = z.position.z;
  pushOutOfSolids(_from); // plan from where it'll be once it's been shoved clear
  const { x, z: fz } = _from;
  if (clearPath(x, fz, tx, tz)) { ud.via = -1; return _goal; }
  // how far each corner is from the goal, going round (Dijkstra out from the goal)
  const n = ROUND_NODES.length, dist = _toGoal;
  for (let i = 0; i < n; i++) {
    const k = ROUND_NODES[i];
    dist[i] = clearPath(k.x, k.z, tx, tz) ? Math.hypot(tx - k.x, tz - k.z) : Infinity;
    _done[i] = false;
  }
  for (let r = 0; r < n; r++) {
    let u = -1;
    for (let i = 0; i < n; i++) if (!_done[i] && (u < 0 || dist[i] < dist[u])) u = i;
    if (dist[u] === Infinity) break;
    _done[u] = true;
    for (let i = 0; i < n; i++) dist[i] = Math.min(dist[i], dist[u] + NODE_LINKS[u][i]);
  }
  let best = -1, bestCost = Infinity, near = -1, nearD = Infinity;
  for (let i = 0; i < n; i++) {
    const k = ROUND_NODES[i], d = Math.hypot(k.x - x, k.z - fz);
    if (d < 0.45 || !clearPath(x, fz, k.x, k.z)) continue;
    if (d < nearD) { nearD = d; near = i; }
    const cost = d + dist[i] - (ud.via === i ? 1 : 0);
    if (cost < bestCost) { bestCost = cost; best = i; }
  }
  if (best < 0) best = near; // no way through from here: at least get round the nearest corner
  if (best < 0) return _goal;
  ud.via = best;
  _goal.x = ROUND_NODES[best].x; _goal.z = ROUND_NODES[best].z;
  return _goal;
}

export function inside(pos, half = 10) {
  return Math.abs(pos.x) < half - 0.3 && Math.abs(pos.z) < half - 0.3;
}
