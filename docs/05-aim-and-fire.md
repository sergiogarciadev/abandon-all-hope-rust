# Chapter 5: Aim and Fire the Gun

In this chapter, we give the survivor movement, mouse-aiming, shooting, reloading, bullet collision, and physical effects (muzzle flash, brass casings, gun smoke, and blood particles).

---

## 5.1 Movement & Plane-Raycast Aiming (`player_movement_and_aim`)

1. **WASD / Arrow Movement**: Input axes are normalized so diagonal movement is not faster, scaled by `stats.speed`, and clamped inside the cabin (`±(HALF - 0.7)`).
2. **Mouse Aiming**: We convert the 2D cursor position into a 3D ray via `camera.viewport_to_world(cam_gt, cursor_pos)` and intersect it with the horizontal plane at rifle height (`y = 0.9`):

```rust
if let Some(dist) = ray.intersect_plane(Vec3::new(0.0, 0.9, 0.0), InfinitePlane3d::new(Vec3::Y)) {
    let hit = ray.get_point(dist);
    aim_state.aim_point = hit;
    p_tf.rotation = Quat::from_rotation_y((hit.x - p_tf.translation.x).atan2(hit.z - p_tf.translation.z));
}
```

---

## 5.2 Firing, Spread, & Muzzle Flash (`weapon_input_and_fire`)

Holding the left mouse button (when `fire_cd <= 0`, not reloading, and `mag > 0`) spawns `1 + stats.spread` bullets fanned symmetrically by `0.13` rad (`BUCKSHOT`). Each bullet has:
- An additive white-hot core (`0xfff4d6`) and an outer glow (`0xffb04a`, or `0xff5a1e` during `DOUBLE DAMAGE`).
- A randomized 6-spike cone muzzle flash, dynamic `PointLight`, brass casing ejected to the right (`EJECT`), and expanding smoke puff (`Debris`).

Pressing `R` (or emptying the magazine) starts a reload (`stats.reload_time`), and at `22%` progress the empty magazine drops as tumbling debris.

---

## 5.3 Sub-Stepped Bullet Collision (`update_bullets`)

Fast bullets (`38.0` units/sec) can tunnel through thin cabin walls at low frame rates. We solve this in two ways:
1. **Wall & Opening Sub-stepping**: We sample the frame's travel segment in steps of `WALL_T * 0.5`. Log walls and shut doors stop bullets (jolting the door and chipping wood); shut windows shatter into jagged glass shards and let the bullet pass through.
2. **Segment-to-Zombie Distance**: Instead of checking only the bullet's endpoint, we project each zombie's `(x, z)` onto the bullet's travel segment (`t ∈ [0, 1]`). Piercing bullets record hit entities in a `HashSet<Entity>` so they never hit the same zombie twice.
