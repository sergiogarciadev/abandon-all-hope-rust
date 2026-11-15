# Chapter 7: Waves, Upgrades, Drops, and Weather

With aiming, shooting, and zombie pathfinding in place, we now turn our sandbox into a real survival loop. In this chapter we introduce **Bevy `States`**, **wave progression**, **pickup drops with a pity timer**, **between-wave upgrades**, and **dynamic storm weather** with `LineList` rain and lightning.

---

## 1. Game Flow with Bevy `States`

In `src/core/state.rs` we define `GameState`:

```rust
#[derive(States, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum GameState {
    Title,
    #[default]
    Playing,
    ChoosingUpgrade,
    Paused,
    Dead,
}
```

In `src/main.rs`, gameplay systems (player movement, shooting, zombie AI, drops, blood, debris) are gated with `.run_if(in_state(GameState::Playing))`, while `update_upgrade_picker_timer` runs only in `GameState::ChoosingUpgrade`, and `update_weather`, `update_camera`, and `pause_toggle` run across states.

---

## 2. Wave Spawning and Scaling

In `src/game.rs`, `start_wave` increments `stats.wave` and scales:

- **Zombie count**: `6 + (wave - 1) * 4`
- **Spawn interval**: `(1.1 - wave * 0.05).max(0.32)` seconds
- **Zombie speed**: `2.2 + (wave - 1) * 0.2`
- **Zombie HP**: `2.0 + ((wave - 1) / 2) as f32`

`spawn_point(cam_pos)` chooses a random perimeter point inside the arena (`±HALF`) that lies **outside** the camera's visible rectangle so zombies never pop in on-screen.

When `to_spawn == 0` and no live zombies (`With<Zombie>, Without<Corpse>`) remain, a `1.6s` intermission displays `"NIGHT N SURVIVED"`, then `picker.open(&stats)` transitions into `GameState::ChoosingUpgrade`.

---

## 3. Drops and the Pity Timer

In `src/entities/drops.rs`, `maybe_drop` rolls on every zombie kill:

- Base chance is `DROP_CHANCE` (`5%`) plus `stats.drop_bonus` (from the *Scavenger* upgrade).
- A **pity timer** (`since_drop >= DROP_PITY = 30`) guarantees a drop if you have gone 30 kills without one.
- Three drop types (`Med`, `Dmg`, `Ammo`) use baked voxel meshes + additive radial glow quads and bob/spin on the ground for `12s` before despawning.
- Picking one up updates `stats` (`hp`, `p_damage`, or `p_ammo`) and populates `ToastEvent` for the HUD.

---

## 4. Between-Wave Upgrades

In `src/ui/upgrades.rs`, `UPGRADES` holds 10 upgrades across four categories (`Firepower`, `Handling`, `Survival`, `Fortune`). Each `UpgradeDef` defines:

- `max`: maximum stackable level
- `ok: fn(&GameStats) -> bool`: eligibility predicate (e.g., *Field Medic* only appears when `hp < max_hp`)
- `stat: fn(&GameStats) -> (String, String)`: current vs. next value for the card UI
- `apply: fn(&mut GameStats)`: mutates `GameStats` on selection

When `ChoosingUpgrade` is active, pressing `1`, `2`, or `3` (after a short `0.35s` lockout) picks the upgrade, plays a `0.48s` confirmation window, and starts the next wave.

---

## 5. Dynamic Rain, Splashes, and Lightning

In `src/world/weather.rs`, `storm_for(wave)` ramps storm intensity from `0.0` (waves 1–4) up to `1.0` (wave 15):

- **Rain streaks**: A single `Mesh` with `PrimitiveTopology::LineList` holds `3000` line segments (`6000` vertices). Each frame we update `Mesh::ATTRIBUTE_POSITION` directly on the CPU, wrapping drops around the player's position and tilting them with wind gusts.
- **Splashes**: `160` pooled quad entities snap to random ground positions around the player and expand/fade over `0.16s`.
- **Wetness & Fog**: `world_data.weather_level` darkens the moon and ambient light, pulls `DistanceFog` closer, and makes ground/wall materials glossier (`perceptual_roughness` drops).
- **Lightning**: Above `BOLT_LEVEL = 0.7`, random bolts spike `DirectionalLight` illuminance and tint the sky/fog pale blue-white before decaying exponentially.
