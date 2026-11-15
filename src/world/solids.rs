use bevy::prelude::*;

pub const Z_RADIUS: f32 = 0.4;
pub const ROUND_PAD: f32 = 0.35;
const SEE_SLOP: f32 = 0.05;

#[derive(Clone, Copy)]
pub struct SolidBox {
    pub x: f32,
    pub z: f32,
    pub hx: f32,
    pub hz: f32,
    pub c: f32,
    pub s: f32,
}

impl SolidBox {
    pub fn to_local(&self, x: f32, z: f32) -> (f32, f32) {
        let dx = x - self.x;
        let dz = z - self.z;
        (dx * self.c - dz * self.s, dx * self.s + dz * self.c)
    }

    pub fn to_world(&self, lx: f32, lz: f32) -> (f32, f32) {
        (
            self.x + lx * self.c + lz * self.s,
            self.z - lx * self.s + lz * self.c,
        )
    }
}

/// Solid world props (like the pickup truck) that zombies push out of and route around.
#[derive(Resource, Default)]
pub struct SolidsNav {
    pub solids: Vec<SolidBox>,
    pub round_nodes: Vec<Vec2>,
    pub node_links: Vec<Vec<f32>>,
}

impl SolidsNav {
    pub fn add_solid(&mut self, x: f32, z: f32, ry: f32, hx: f32, hz: f32) {
        self.solids.push(SolidBox {
            x,
            z,
            hx,
            hz,
            c: ry.cos(),
            s: ry.sin(),
        });
    }

    pub fn push_out(&self, pos: &mut Vec2) {
        for b in &self.solids {
            let (lx, lz) = b.to_local(pos.x, pos.y);
            let ex = b.hx + Z_RADIUS;
            let ez = b.hz + Z_RADIUS;
            let px = ex - lx.abs();
            let pz = ez - lz.abs();
            if px <= 0.0 || pz <= 0.0 {
                continue;
            }
            let (nx, nz) = if px < pz {
                (lx.signum() * (ex + 0.01), lz)
            } else {
                (lx, lz.signum() * (ez + 0.01))
            };
            let (wx, wz) = b.to_world(nx, nz);
            pos.x = wx;
            pos.y = wz;
        }
    }

    pub fn clear_path(&self, ax: f32, az: f32, bx: f32, bz: f32) -> bool {
        for b in &self.solids {
            let (px, pz) = b.to_local(ax, az);
            let (qx, qz) = b.to_local(bx, bz);
            if seg_hits_box(
                px,
                pz,
                qx,
                qz,
                b.hx + Z_RADIUS - SEE_SLOP,
                b.hz + Z_RADIUS - SEE_SLOP,
            ) {
                return false;
            }
        }
        true
    }

    pub fn init_nav(&mut self) {
        self.round_nodes.clear();
        for b in &self.solids {
            for (sx, sz) in [(1.0, 1.0), (1.0, -1.0), (-1.0, -1.0), (-1.0, 1.0)] {
                let kx = sx * (b.hx + Z_RADIUS + ROUND_PAD);
                let kz = sz * (b.hz + Z_RADIUS + ROUND_PAD);
                let (wx, wz) = b.to_world(kx, kz);
                self.round_nodes.push(Vec2::new(wx, wz));
            }
        }
        let n = self.round_nodes.len();
        self.node_links = vec![vec![f32::INFINITY; n]; n];
        for i in 0..n {
            for j in 0..n {
                if i != j {
                    let a = self.round_nodes[i];
                    let b = self.round_nodes[j];
                    if self.clear_path(a.x, a.y, b.x, b.y) {
                        self.node_links[i][j] = a.distance(b);
                    }
                }
            }
        }
    }

    /// Returns the next waypoint toward `(tx, tz)`, routing around `SolidBox`es via Dijkstra.
    pub fn route_around(&self, pos: Vec2, tx: f32, tz: f32, via: &mut Option<usize>) -> Vec2 {
        let mut from = pos;
        self.push_out(&mut from);
        if self.clear_path(from.x, from.y, tx, tz) {
            *via = None;
            return Vec2::new(tx, tz);
        }

        let n = self.round_nodes.len();
        if n == 0 {
            return Vec2::new(tx, tz);
        }

        let mut dist = vec![f32::INFINITY; n];
        let mut done = vec![false; n];
        for i in 0..n {
            let k = self.round_nodes[i];
            if self.clear_path(k.x, k.y, tx, tz) {
                dist[i] = k.distance(Vec2::new(tx, tz));
            }
        }

        for _ in 0..n {
            let mut u = None;
            for i in 0..n {
                if !done[i] && u.map_or(true, |best| dist[i] < dist[best]) {
                    u = Some(i);
                }
            }
            let Some(u) = u else { break };
            if dist[u].is_infinite() {
                break;
            }
            done[u] = true;
            for i in 0..n {
                let cand = dist[u] + self.node_links[u][i];
                if cand < dist[i] {
                    dist[i] = cand;
                }
            }
        }

        let mut best = None;
        let mut best_cost = f32::INFINITY;
        let mut near = None;
        let mut near_d = f32::INFINITY;

        for i in 0..n {
            let k = self.round_nodes[i];
            let d = k.distance(from);
            if d < 0.45 || !self.clear_path(from.x, from.y, k.x, k.y) {
                continue;
            }
            if d < near_d {
                near_d = d;
                near = Some(i);
            }
            let loyalty = if *via == Some(i) { 1.0 } else { 0.0 };
            let cost = d + dist[i] - loyalty;
            if cost < best_cost {
                best_cost = cost;
                best = Some(i);
            }
        }

        let pick = best.or(near);
        if let Some(idx) = pick {
            *via = Some(idx);
            self.round_nodes[idx]
        } else {
            Vec2::new(tx, tz)
        }
    }
}

pub fn seg_hits_box(ax: f32, az: f32, bx: f32, bz: f32, hx: f32, hz: f32) -> bool {
    let mut t0 = 0.0_f32;
    let mut t1 = 1.0_f32;
    let dx = bx - ax;
    let dz = bz - az;
    if dx.abs() < 1e-9 {
        if ax.abs() >= hx {
            return false;
        }
    } else {
        let ta = (-hx - ax) / dx;
        let tb = (hx - ax) / dx;
        t0 = t0.max(ta.min(tb));
        t1 = t1.min(ta.max(tb));
    }
    if dz.abs() < 1e-9 {
        if az.abs() >= hz {
            return false;
        }
    } else {
        let ta = (-hz - az) / dz;
        let tb = (hz - az) / dz;
        t0 = t0.max(ta.min(tb));
        t1 = t1.min(ta.max(tb));
    }
    t0 < t1
}

pub fn inside_cabin(pos: Vec3, half: f32) -> bool {
    pos.x.abs() < half - 0.3 && pos.z.abs() < half - 0.3
}
