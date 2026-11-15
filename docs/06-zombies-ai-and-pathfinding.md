# Chapter 6: Zombies AI & Pathfinding

Zombies start outside the cabin, navigate around solid yard obstacles like the rusted pickup truck, pick the nearest door or window, batter it down (or clamber over the window sill), and hunt the survivor inside.

---

## 6.1 Obstacle Avoidance & Visibility Graph (`src/world/solids.rs`)

Because the yard has rotated solid obstacles (`SolidBox`), a zombie walking straight toward a door could get wedged against the pickup truck.

1. **`init_nav()`**: Around every `SolidBox`, we place 4 corner waypoints (`round_nodes`) padded outward by `Z_RADIUS + ROUND_PAD` (`0.4 + 0.35`). Then we precompute an all-pairs visibility matrix (`node_links`) using `clear_path` and a 2D slab intersection test (`seg_hits_box`).
2. **`route_around()`**:
   - First, push the zombie's virtual start position out of any overlapping solid (`push_out`).
   - If `clear_path(start, goal)` is true, walk straight to the goal and clear `via`.
   - Otherwise, run **Dijkstra** backward from the goal across `round_nodes`, then pick the visible corner minimizing `dist(zombie, corner) + dist(corner, goal) - loyalty`, where `loyalty = 1.0` if `via == Some(i)` so zombies never dither between two symmetric routes.

---

## 6.2 Breaching Cabin Openings (`src/world/openings.rs`)

Zombies outside the cabin (`!inside_cabin(pos, HALF)`) commit to one opening (`ud.opening = nearest_opening(...)`) and progress through two stages:
- **Stage 0**: Walk to the outer waypoint `(ox, oz)` so they approach the door or window square-on.
- **Stage 1**:
  - If it is a broken door (`hp <= 0`), walk straight through `(ix, iz)`.
  - If the opening is still shut (`hp > 0`), face it and batter it (`hit_opening` on `swing >= Z_STRIKE`). Doors take 12 blows before toppling off their hinges; windows crack on the first blow and shatter on the second.
  - If a window is broken (`hp <= 0`) and `climber.is_none()`, `start_climb` claims the sill and `climb_step` arcs the zombie over the sill (`CLIMB_TIME = 1.1s`) onto the landing `(lx, lz)`.
- **Stuck Recovery**: If a zombie has been walking for `2.0s` and moved less than `0.6` units, it switches to the next-nearest opening.

---

## 6.3 Melee, Turning, & Crowd Separation (`chase_zombies`, `separate_zombies`)

- **Arcing Turns (`walk_toward`)**: Zombies turn at a limited rate (`ud.turn`) and slow down (`cos(off).max(0.15)`) when facing off-target so they arc smoothly instead of orbiting.
- **Dodgable Melee**: Within `Z_REACH = 1.2` (and clear of walls), a zombie plants its feet, winds up (`Z_SWING = 0.85s`), and strikes at `Z_STRIKE = 0.2s`. Stepping outside `Z_HIT_RANGE = 1.65` dodges the blow.
- **Crowd Separation (`separate_zombies`)**: Pairwise pushes keep zombies `Z_SPACE = 0.8` apart, `Z_CLOSE = 0.95` from the survivor, and outside all `SolidBox`es.
