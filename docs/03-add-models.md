# Chapter 3: Adding Models

With the cabin and forest built, it is time to populate the world with characters: the **survivor** and **four distinct zombie archetypes**. Just like the environment, every character is built procedurally from chamfered blocks (`bevel_box_mesh` and `bake_blocks`).

---

## 3.1 The Survivor Rig (`src/entities/player.rs`)

Instead of baking the survivor into one static mesh, we build a hierarchical rig of Bevy entities using `commands.entity(parent).add_child(child)` so that limbs, torso, head, and rifle can be posed independently every frame:

```
Player (root)
 ├── Lower
 │    ├── Pelvis (pants, belt, brass buckle, hip pouch, knife sheath)
 │    └── Legs[0..2] (thigh, shin + knee pad, boot)
 ├── Upper
 │    ├── Torso (8×8 procedural buffalo-plaid flannel, chest rig, backpack, bedroll)
 │    └── Head (beard, eyes, brows, blaze-orange beanie)
 ├── Gun (stock, pistol grip, receiver, Mag, barrel, scope, Muzzle)
 │    └── MuzzleFlash (core, 6 cone spikes, additive glow)
 ├── SpareMag (hidden until mid-reload)
 └── Arms[0..2] (upper arm, forearm + rolled cuff, hand)
```

### Procedural Plaid Flannel Texture
The survivor's jacket uses an `8×8` pixel `Image` generated at startup with `ImageFilterMode::Nearest` and `ImageAddressMode::Repeat`. Up close it shows crisp red-and-black buffalo check; from the top-down camera, mipmaps blend it into a warm dark crimson that contrasts against the green-skinned zombies.

---

## 3.2 Baked Zombie Kits (`src/entities/zombies.rs`)

A horde can have up to 30 active zombies at once. To keep draw calls low while preserving articulated limbs and jaws, we pre-bake **4 zombie kits** (`ZOMBIE_LOOKS`) at startup:
1. **Farmhand**: Denim shirt, brown pants
2. **Hunter**: Olive field jacket with exposed ribs
3. **Tank Top**: Bald with torn scalp, tank top, one shoe missing
4. **Camper**: Mustard hoodie with exposed ribs

Each kit stores `Handle<Mesh>` for `head`, `jaw`, `eyes`, `torso`, `pelvis`, `upper_arm`, `forearm`, `thigh`, `shin`, and `feet: [left, right]`. Every spawned zombie shares these mesh handles and two materials (`skin_mat` and `hurt_mat`), swapping materials on a bullet hit for an instant emissive flash.
