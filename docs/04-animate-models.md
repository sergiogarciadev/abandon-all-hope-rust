# Chapter 4: Animating the Models

We do not use skeletal `.gltf` animations. Instead, both the survivor and every zombie are driven by **procedural two-bone Inverse Kinematics (IK)** and **phase-based gait cycles** every frame.

---

## 4.1 Two-Bone IK & Bone Placement (`src/entities/animation.rs`)

Given a shoulder/hip joint `a`, a target hand/foot position `t`, bone lengths `l1` and `l2`, and a `pole` vector indicating which way the knee or elbow bends, `ik2` solves the middle joint using the **Law of Cosines**:

```rust
let x = (l1 * l1 - l2 * l2 + d * d) / (2.0 * d);
let p_dir = (pole - d_vec * pole.dot(d_vec)).normalize_or_zero();
let h = (l1 * l1 - x * x).max(0.0).sqrt();
a + d_vec * x + p_dir * h
```

Once we have the three joint positions (`a -> mid -> end`), `bone_transform(a, mid, pole)` computes the midpoint and orthonormal basis (`Quat::from_mat3`) to orient each block limb between its two joints.

### The Gait Cycle (`foot_cycle`)
Each foot cycles through `p ∈ [0, 1)`:
- **Stance phase (`p < beta`)**: The boot stays planted on the ground (`lift = 0`) and slides backward under the body (`s: 0.5 -> -0.5`).
- **Swing phase (`p >= beta`)**: The boot lifts in a sine arc (`sin(π * t)`), pitches toe-off to heel-strike, and swings forward.

---

## 4.2 Survivor Rig Animation (`animate_player`)

1. **Bladed Stance & Backpedaling**: The survivor's lower body (`PlayerLower`) turns toward the movement direction—or flips 180° with hysteresis when backpedaling—clamped within `±1.0` rad of the aim direction.
2. **Upper Body & Rifle**: `PlayerUpper` blades `-0.62` rad so the left shoulder leads toward the target, leans into movement, breathes when idle, and kicks back on recoil (`rig.kick`).
3. **Locked Hands & Reload Path**: Both hands stay locked onto `GRIP_R` and `GRIP_L` via `ik2`. During a reload, the support hand interpolates along `RELOAD_PATH`—ripping the empty mag out, reaching to the chest-rig pouch (`POUCH`), and seating a fresh magazine.

---

## 4.3 Zombie Shamble, Limp, Attack, and Death (`pose_zombie_rig`)

Each zombie has randomized gait personality (`limp`, `limp_side`, `hunch`, `tilt`, `arms`, `phase`):
- **Limp**: One leg drags with a reduced lift height (`0.03`) while the pelvis drops onto the bad side.
- **Lope**: On later waves (`speed > 3.0`), `beta` shrinks from `0.6` to `0.42`, turning the shamble into a loping run.
- **Blows & Flinch**: Winding up rears the torso back and raises both arms; slamming forward clubs them down; bullet hits snap the head and torso back (`flinch`).
- **Corpses (`update_corpses`)**: Dead zombies crumple, topple onto the floorboards (stacking `BODY_THICK` above any corpses underneath), spread a blood pool, freeze after `1.0s`, and sink into the boards after `CORPSE_LIFE = 20s`.
