# Chapter 2: Creating the World

In this chapter, we build the entire 3D environment procedurally—without loading a single `.glb` or `.png` file from disk. Everything in **Abandon All Hope** is constructed from chamfered blocks, vertex colors, and procedural pixel textures.

---

## 2.1 Chamfered Blocks & `BlockBatch` (`src/core/blocks.rs`)

Plain 12-triangle cubes look flat under top-down moonlight because their edges meet at 90°. By beveling every edge at 45° (`bevel_box_raw`), each block catches a thin specular/diffuse highlight along its silhouette, keeping logs, furniture, and limbs readable from 30 units up.

To avoid thousands of draw calls, `BlockBatch` transforms each `BlockSpec` into world (or parent-local) coordinates, perturbs its vertex color slightly (`0.9 + rng * 0.2`) for a hand-painted look, and merges all blocks into a single `Mesh` with `Mesh::ATTRIBUTE_COLOR`:

```rust
let mut batch = BlockBatch::new();
batch.add(&[
    BlockSpec::new(1.9, 0.06, 0.3, -1.7, 1.6, in_n + 0.15, 0x5e4730).with_bevel(0.015),
], &mut rng, None, false);
commands.spawn((
    Mesh3d(meshes.add(batch.build())),
    MeshMaterial3d(world_mat.clone()),
));
```

---

## 2.2 The Cabin, Yard, and Forest (`src/world/world.rs`)

Our `setup_world` system constructs:
1. **Forest Floor**: A `128×128` procedural moss/needle/dirt `Image` with `ImageFilterMode::Nearest` and `ImageAddressMode::Repeat` tiled `25×` across a `200×200` `Plane3d`.
2. **Cabin Interior**: Staggered oak floorboards, a braided oval rug, loose pages, a tin can, old blood stains, pantry shelves with jars and stacked books, a deer-skull trophy, calendar, coat & hat, and mirror.
3. **Log Walls & Openings (`src/world/openings.rs`)**: Interleaved log courses with chinking, end-grain caps, fieldstone footings, two hinged plank doors, and three paned windows with jagged broken-glass shards ready to reveal when shattered.
4. **The Yard**: Stacked firewood, a chopping block with an embedded axe, a rusted pickup truck (registered in `SolidsNav` so zombies route around it), a broken fence, stumps, a fallen trunk, and a porch lantern with a flickering `PointLight`.
5. **The Forest**: 900 procedural trees (pines, spruces, oaks, dead snags) and 1,000 brush tufts, ferns, and rocks scattered outside the clearings up to `R_MAX = 70.0` (past the linear `DistanceFog` end at `62.0`).
6. **Atmosphere**: `GlobalAmbientLight`, a shadow-casting moonlight `DirectionalLight`, a horizon glow band, and a procedural `256×256` moon disc.

---

## 2.3 Guttering Lantern Flame

A small `update_world` system modulates the porch lantern's `PointLight::intensity` and `StandardMaterial::emissive` using layered sine waves and the current `weather_level`:

```rust
let f = 0.85 + 0.08 * (t * 7.3).sin() + 0.06 * (t * 13.1 + 1.0).sin() - dip
    - w_level * 0.2 * (0.5 + 0.5 * (t * 5.3).sin() * (t * 2.1).sin());
```
