# Abandon All Hope — Learning Bevy by Porting a 3D Game

Welcome to the book-length guide accompanying the Rust + Bevy (`0.20.1`) port of *Abandon All Hope*. Each chapter corresponds to an incremental, runnable milestone in the git history.

## Chapters

1. [How to Use Bevy](01-how-to-use-bevy.md) — ECS fundamentals, `App`, `Startup` vs `Update`, and project setup
2. [Create an Empty World](02-create-an-empty-world.md) — Procedural beveled voxel meshes, cabin geometry, lighting, and AABB collisions
3. [Add Models](03-add-models.md) — Hierarchical survivor rig, procedural plaid texture, rifle, and 4 zombie archetypes
4. [Animate Models](04-animate-models.md) — Procedural gait cycles, 2-bone analytic IK, recoil, and corpse stacking
5. [Aim and Fire the Gun](05-aim-and-fire.md) — Ground-plane raycasting, bullet sub-stepping, muzzle flash, blood, and debris
6. [Zombies AI and Pathfinding](06-zombies-ai-and-pathfinding.md) — Dijkstra navigation around walls, opening attacks, and window clambering
7. [Waves, Upgrades, Drops, and Weather](07-waves-upgrades-and-weather.md) — Bevy `States`, wave scaling, pity-timer drops, `LineList` rain, and lightning
8. [Procedural Audio and Adaptive Music](08-procedural-audio.md) — Real-time 44.1 kHz chip synthesizer, LFSR noise, reverb, and 4-mode music
9. [Creating the UI and CSS Visual Effects](09-create-the-ui.md) — HUD, 8×8 pixel icons, 3-card upgrade picker, film grain, and title screen
10. [Putting It All Together](10-putting-it-all-together.md) — Full module map and running the finished game
