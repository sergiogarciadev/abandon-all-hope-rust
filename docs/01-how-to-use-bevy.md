# Chapter 1: How to Use Bevy

Welcome to **Abandon All Hope in Rust & Bevy**! In this book, we will port a complete 3D procedural zombie survival shooter from Three.js (`./js`) to Rust using **Bevy 0.20**.

Every chapter builds on the previous one in small, runnable steps—just like a real game development workflow.

---

## 1.1 Why Bevy? (ECS Architecture)

In the original JavaScript codebase, game objects were Three.js `Group`s with arbitrary `userData` bags and global mutable arrays (`zombies`, `bullets`, `drops`). In **Bevy**, everything is organized around the **Entity Component System (ECS)** pattern:

- **Entities**: Unique IDs representing game objects (the player, a zombie, a bullet, a log wall, the camera).
- **Components**: Plain Rust `struct`s or `enum`s attached to entities (e.g., `Transform`, `Velocity`, `Health`, `Zombie`).
- **Resources**: Global singleton data shared across the app (e.g., `ClearColor`, `GameStats`, `Time`, `Assets<Mesh>`).
- **Systems**: Plain Rust functions whose parameters declare what data they read or mutate (`Query`, `Res`, `ResMut`, `Commands`).
- **Plugins**: Modular bundles of systems, resources, and sub-apps (like `DefaultPlugins` for windowing, rendering, input, and audio).

```
App
 ├── Resources: ClearColor, GameStats, Time, ...
 ├── States: GameState (Title, Playing, ChoosingUpgrade, Paused, Dead)
 └── Entities
      ├── [Player]  -> Transform, PlayerRig, Health
      ├── [Zombie]  -> Transform, ZombieAI, ZombieRig
      └── [Camera]  -> Camera3d, Transform
```

---

## 1.2 Setting Up `Cargo.toml`

We use the latest Bevy (`0.20.1`) and `rand` (`0.10.3`). Because Bevy compiles many rendering crates, enabling `opt-level = 2` for dependencies even in `dev` profile makes 3D gameplay smooth while keeping our own code fast to recompile:

```toml
[package]
name = "abandon_all_hope"
version = "0.1.0"
edition = "2024"

[dependencies]
bevy = "0.20.1"
rand = "0.10.3"

[profile.dev.package."*"]
opt-level = 2

[profile.dev]
opt-level = 1
```

---

## 1.3 Our First Bevy App (`src/main.rs`)

In `src/main.rs`, we:
1. Set the background `ClearColor` to `#05060a` (matching `scene.background` and `style.css` in `./js`).
2. Add `DefaultPlugins` with a custom `WindowPlugin` titled `"ABANDON ALL HOPE"`.
3. Register our `GameState` enum using `.init_state::<GameState>()` so later chapters can gate systems with `in_state(GameState::Playing)`.
4. Spawn a `Camera3d` positioned at `(0, 30, 13)` looking at the origin—the exact `CAM_OFFSET` from `js/src/core/scene.js`.

```rust
fn main() {
    App::new()
        .insert_resource(ClearColor(Color::srgb_u8(0x05, 0x06, 0x0a)))
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "ABANDON ALL HOPE".into(),
                resolution: (1280, 720).into(),
                ..default()
            }),
            ..default()
        }))
        .init_state::<GameState>()
        .add_systems(Startup, setup_basic_camera)
        .run();
}
```

With this foundation compiling, we are ready to build the cabin, yard, forest, and night sky in Chapter 2.
