# Chapter 10: Putting It All Together

Every module from Chapters 1 through 9 now plugs into `src/main.rs` as a cohesive, zero-asset-file 3D survival shooter.

---

## 1. Module Architecture

```
src/
├── main.rs                 # App wiring, GameState schedules
├── game.rs                 # Wave loop, camera follow/swoop/shake, ResetContext
├── core/
│   ├── math.rs             # SeededRng, hex_color, wrap_angle, damp, ease
│   ├── blocks.rs           # Procedural beveled voxel mesh baker + vertex AO
│   └── state.rs            # GameState, GameStats, GameSettings, BestRun
├── world/
│   ├── solids.rs           # AABB collisions, segment intersection, Dijkstra nav
│   ├── openings.rs         # Breakable windows, glass panes, wooden doors
│   ├── world.rs            # Cabin, furniture, yard, trees, moonlight, lanterns
│   └── weather.rs          # LineList rain, splash pool, fog/wetness, lightning
├── entities/
│   ├── animation.rs        # Procedural foot_cycle & 2-bone analytic IK
│   ├── player.rs           # Survivor rig, plaid texture, rifle, muzzle flash
│   ├── weapon.rs           # Aiming, recoil, casings, smoke, sub-stepped bullets
│   ├── zombies.rs          # 4 zombie kits, AI, window clambering, corpse stacking
│   └── drops.rs            # Med / Double Damage / Bottomless Mag + pity timer
├── effects/
│   ├── blood.rs            # Ballistic blood particles
│   └── debris.rs           # Tumbling glass, splinters, and spent magazines
├── audio/
│   ├── synth.rs            # 44.1 kHz chip waveforms, biquads, wind, delay, limiter
│   ├── music.rs            # Adaptive 4-mode chiptune sequencer
│   └── mod.rs              # AudioEngine command channel & SFX triggers
└── ui/
    ├── upgrades.rs         # 10 upgrades, eligibility, stat previews
    ├── storage.rs          # Persistent settings & best run
    ├── hud.rs              # HUD, damage flash, 3-card picker, pause & death
    └── title.rs            # Title screen, film grain, fade-in, title zombies
```

---

## 2. Running the Game

```bash
cargo run --release
```

### Controls
- **`W A S D` / Arrow Keys**: Move
- **`Mouse`**: Aim & fire
- **`R`**: Reload
- **`1` / `2` / `3` / Click**: Choose between-wave upgrade
- **`M`**: Mute / unmute audio
- **`Esc` / `P`**: Pause / resume
