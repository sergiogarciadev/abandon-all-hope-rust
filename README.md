# Abandon All Hope (Rust + Bevy Edition)

A 3D procedural zombie survival shooter ported from Three.js / Web Audio (`./js`) to **Rust** and **Bevy 0.20.1**, built incrementally with a companion tutorial book in [`./docs`](docs/SUMMARY.md).

## Features
- **Zero external art or audio files**: all 3D voxel meshes, pixel-art UI icons, film grain, 8-bit SFX, and adaptive 4-mode chiptune music are generated procedurally in code.
- **Procedural animation**: 2-bone analytic IK limbs, foot-cycle gaits, weapon recoil, reload magazine drops, and stacking corpses.
- **Tactical cabin defense**: breakable windows and doors, Dijkstra pathfinding around walls, window clambering, and 10 stackable upgrades.
- **Dynamic storm weather**: `LineList` rain streaks, ground splashes, glossy wet surfaces, and lightning strikes.

## Quick Start

```bash
cargo run --release
```

Read the full step-by-step Bevy guide starting at [docs/SUMMARY.md](docs/SUMMARY.md).
