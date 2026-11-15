use bevy::prelude::*;
use std::f32::consts::PI;

use crate::core::math::ease;

pub struct FootCycle {
    pub s: f32,
    pub lift: f32,
    pub pitch: f32,
}

/// Where one foot is in its gait cycle `p` (`0..1`):
/// planted for the first `beta` of it (`s: 0.5 -> -0.5`), then lifted and carried forward.
pub fn foot_cycle(p: f32, beta: f32) -> FootCycle {
    if p < beta {
        FootCycle {
            s: 0.5 - p / beta,
            lift: 0.0,
            pitch: 0.0,
        }
    } else {
        let t = (p - beta) / (1.0 - beta);
        FootCycle {
            s: -0.5 + ease(t),
            lift: (PI * t).sin(),
            pitch: 0.6 * (1.0 - t) * (1.0 - t) - 0.3 * t * t,
        }
    }
}

/// Two-bone analytic IK: from joint `a`, reach toward `t` with bone lengths `l1` and `l2`,
/// bending the middle joint toward `pole`. Returns the middle joint position.
pub fn ik2(a: Vec3, t: Vec3, l1: f32, l2: f32, pole: Vec3) -> Vec3 {
    let diff = t - a;
    let d = diff
        .length()
        .clamp((l1 - l2).abs() + 1e-3, l1 + l2 - 1e-3);
    let d_dir = diff.normalize_or_zero();
    let x = (l1 * l1 - l2 * l2 + d * d) / (2.0 * d);
    let p_dir = (pole - d_dir * pole.dot(d_dir)).normalize_or_zero();
    let h = (l1 * l1 - x * x).max(0.0).sqrt();
    a + d_dir * x + p_dir * h
}

/// Computes a `Transform` placing a limb block (built along local `+Y`) from `a` to `b`,
/// with its front (`+Z`) facing `face`.
pub fn bone_transform(a: Vec3, b: Vec3, face: Vec3) -> Transform {
    let by = (a - b).normalize_or_zero();
    let mut bz = face - by * face.dot(by);
    if bz.length_squared() < 1e-6 {
        bz = Vec3::Z;
    } else {
        bz = bz.normalize();
    }
    let bx = by.cross(bz).normalize_or_zero();
    let rot = Quat::from_mat3(&Mat3::from_cols(bx, by, bz));
    Transform::from_translation((a + b) * 0.5).with_rotation(rot)
}
