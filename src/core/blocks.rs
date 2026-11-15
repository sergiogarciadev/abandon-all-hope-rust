use bevy::asset::RenderAssetUsages;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;

use super::math::SeededRng;

/// Specification for a single chamfered (or flat) block in a `BlockBatch`.
#[derive(Clone, Copy)]
pub struct BlockSpec {
    pub w: f32,
    pub h: f32,
    pub d: f32,
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub color: u32,
    pub rx: f32,
    pub ry: f32,
    pub rz: f32,
    pub bevel: f32,
}

impl BlockSpec {
    pub fn new(w: f32, h: f32, d: f32, x: f32, y: f32, z: f32, color: u32) -> Self {
        Self {
            w,
            h,
            d,
            x,
            y,
            z,
            color,
            rx: 0.0,
            ry: 0.0,
            rz: 0.0,
            bevel: 0.025,
        }
    }

    pub fn with_rot(mut self, rx: f32, ry: f32, rz: f32) -> Self {
        self.rx = rx;
        self.ry = ry;
        self.rz = rz;
        self
    }

    pub fn with_bevel(mut self, bevel: f32) -> Self {
        self.bevel = bevel;
        self
    }
}

/// Builds a chamfered box (45° beveled edges) as raw `(positions, normals)` triangle-list vertices.
pub fn bevel_box_raw(w: f32, h: f32, d: f32, mut r: f32) -> (Vec<Vec3>, Vec<Vec3>) {
    let e = [w * 0.5, h * 0.5, d * 0.5];
    r = r.min(e[0] * 0.45).min(e[1] * 0.45).min(e[2] * 0.45);

    let v = |s: [f32; 3], a: usize| -> Vec3 {
        Vec3::new(
            s[0] * if a == 0 { e[0] } else { e[0] - r },
            s[1] * if a == 1 { e[1] } else { e[1] - r },
            s[2] * if a == 2 { e[2] } else { e[2] - r },
        )
    };
    let sg = |a: usize, sa: f32, b: usize, sb: f32, c: usize, sc: f32| -> [f32; 3] {
        let mut s = [0.0; 3];
        s[a] = sa;
        s[b] = sb;
        s[c] = sc;
        s
    };

    let mut pos = Vec::with_capacity(132);
    let mut nrm = Vec::with_capacity(132);

    let mut push_face = |mut pts: Vec<Vec3>| {
        let p0 = pts[0];
        let mut n = (pts[1] - p0).cross(pts[2] - p0).normalize_or_zero();
        if n.dot(p0) < 0.0 {
            pts.reverse();
            n = -n;
        }
        for i in 1..(pts.len() - 1) {
            pos.push(pts[0]);
            pos.push(pts[i]);
            pos.push(pts[i + 1]);
            nrm.push(n);
            nrm.push(n);
            nrm.push(n);
        }
    };

    for a in 0..3 {
        let b = (a + 1) % 3;
        let c = (a + 2) % 3;
        for sa in [1.0, -1.0] {
            let flat = [(1.0, 1.0), (1.0, -1.0), (-1.0, -1.0), (-1.0, 1.0)]
                .iter()
                .map(|&(sb, sc)| v(sg(a, sa, b, sb, c, sc), a))
                .collect();
            push_face(flat);

            for sb in [1.0, -1.0] {
                let strip = vec![
                    v(sg(a, sa, b, sb, c, 1.0), a),
                    v(sg(a, sa, b, sb, c, -1.0), a),
                    v(sg(a, sa, b, sb, c, -1.0), b),
                    v(sg(a, sa, b, sb, c, 1.0), b),
                ];
                push_face(strip);
            }
        }
    }

    for x in [1.0, -1.0] {
        for y in [1.0, -1.0] {
            for z in [1.0, -1.0] {
                let corner = vec![v([x, y, z], 0), v([x, y, z], 1), v([x, y, z], 2)];
                push_face(corner);
            }
        }
    }

    (pos, nrm)
}

/// Builds a plain 12-triangle box for far-off trees or thin blades where bevels aren't visible.
pub fn plain_box_raw(w: f32, h: f32, d: f32) -> (Vec<Vec3>, Vec<Vec3>) {
    let hx = w * 0.5;
    let hy = h * 0.5;
    let hz = d * 0.5;

    let faces: [([Vec3; 4], Vec3); 6] = [
        (
            [
                Vec3::new(hx, -hy, hz),
                Vec3::new(hx, hy, hz),
                Vec3::new(hx, hy, -hz),
                Vec3::new(hx, -hy, -hz),
            ],
            Vec3::X,
        ),
        (
            [
                Vec3::new(-hx, -hy, -hz),
                Vec3::new(-hx, hy, -hz),
                Vec3::new(-hx, hy, hz),
                Vec3::new(-hx, -hy, hz),
            ],
            Vec3::NEG_X,
        ),
        (
            [
                Vec3::new(-hx, hy, hz),
                Vec3::new(-hx, hy, -hz),
                Vec3::new(hx, hy, -hz),
                Vec3::new(hx, hy, hz),
            ],
            Vec3::Y,
        ),
        (
            [
                Vec3::new(-hx, -hy, -hz),
                Vec3::new(-hx, -hy, hz),
                Vec3::new(hx, -hy, hz),
                Vec3::new(hx, -hy, -hz),
            ],
            Vec3::NEG_Y,
        ),
        (
            [
                Vec3::new(-hx, -hy, hz),
                Vec3::new(-hx, hy, hz),
                Vec3::new(hx, hy, hz),
                Vec3::new(hx, -hy, hz),
            ],
            Vec3::Z,
        ),
        (
            [
                Vec3::new(hx, -hy, -hz),
                Vec3::new(hx, hy, -hz),
                Vec3::new(-hx, hy, -hz),
                Vec3::new(-hx, -hy, -hz),
            ],
            Vec3::NEG_Z,
        ),
    ];

    let mut pos = Vec::with_capacity(36);
    let mut nrm = Vec::with_capacity(36);
    for (pts, n) in faces {
        pos.extend_from_slice(&[pts[0], pts[1], pts[2], pts[0], pts[2], pts[3]]);
        nrm.extend_from_slice(&[n; 6]);
    }
    (pos, nrm)
}

/// Collects colored blocks into one vertex-colored `Mesh` so a whole wall, prop, or limb is 1 draw call.
pub struct BlockBatch {
    positions: Vec<[f32; 3]>,
    normals: Vec<[f32; 3]>,
    colors: Vec<[f32; 4]>,
}

impl BlockBatch {
    pub fn new() -> Self {
        Self {
            positions: Vec::new(),
            normals: Vec::new(),
            colors: Vec::new(),
        }
    }

    pub fn add(
        &mut self,
        blocks: &[BlockSpec],
        rng: &mut SeededRng,
        parent: Option<Mat4>,
        flat: bool,
    ) -> &mut Self {
        for b in blocks {
            let (raw_p, raw_n) = if flat || b.bevel <= 0.0 {
                plain_box_raw(b.w, b.h, b.d)
            } else {
                bevel_box_raw(b.w, b.h, b.d, b.bevel)
            };

            let rot = Quat::from_euler(EulerRot::YXZ, b.ry, b.rx, b.rz);
            let mut m = Mat4::from_rotation_translation(rot, Vec3::new(b.x, b.y, b.z));
            if let Some(p) = parent {
                m = p * m;
            }
            let norm_mat = Mat3::from_mat4(m);

            let r_u8 = ((b.color >> 16) & 0xff) as u8;
            let g_u8 = ((b.color >> 8) & 0xff) as u8;
            let b_u8 = (b.color & 0xff) as u8;
            let shade = 0.9 + rng.next_f32() * 0.2;
            let lin = Color::srgb_u8(r_u8, g_u8, b_u8).to_linear();
            let col = [lin.red * shade, lin.green * shade, lin.blue * shade, 1.0];

            for (p, n) in raw_p.into_iter().zip(raw_n.into_iter()) {
                let wp = m.transform_point3(p);
                let wn = (norm_mat * n).normalize_or_zero();
                self.positions.push(wp.to_array());
                self.normals.push(wn.to_array());
                self.colors.push(col);
            }
        }
        self
    }

    pub fn build(self) -> Mesh {
        let count = self.positions.len() as u32;
        let indices: Vec<u32> = (0..count).collect();
        let mut mesh = Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        );
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, self.positions);
        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, self.normals);
        mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, self.colors);
        mesh.insert_indices(Indices::U32(indices));
        mesh
    }
}

pub fn bake_blocks(blocks: &[BlockSpec], rng: &mut SeededRng, flat: bool) -> Mesh {
    let mut batch = BlockBatch::new();
    batch.add(blocks, rng, None, flat);
    batch.build()
}
