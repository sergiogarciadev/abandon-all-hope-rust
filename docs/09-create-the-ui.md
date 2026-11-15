# Chapter 9: Creating the UI and CSS Visual Effects

In the original web version, the interface lived in `index.html` and `style.css` — complete with a dark left-hand scrim, animated film grain, startup fade-in, pixel-art heart & upgrade icons, a red damage flash, and 10-step volume meters. In this chapter we port every visual detail into **Bevy UI**.

---

## 1. Procedural Pixel-Art Icons (`8x8` Nearest-Neighbor)

Instead of loading external `.png` sprites, `icon_image` in `src/ui/hud.rs` turns 8×8 ASCII bitmaps (`HEART_ICON` and each `UpgradeDef.icon`) into `Image` assets with `ImageSamplerDescriptor::nearest()`. Passing those handles to `ImageNode` gives razor-sharp retro icons at any UI scale.

---

## 2. The Gameplay HUD and Damage Flash

`setup_hud` and `update_hud` in `src/ui/hud.rs` build and drive:

- **Damage Flash (`FlashOverlay`)**: A full-screen `PositionType::Absolute` node with `Pickable::IGNORE` whose `BackgroundColor` alpha tracks `DamageFlash.alpha * 0.55` when a zombie lands a hit.
- **Top Bar**: Displays `NIGHT N`, up to 10 heart pips (swapping between `heart_full` and `heart_empty`), active powerup timers (`2X DMG`, `INF AMMO`), `KILLS`, `LEFT`, and `[M] MUTED`.
- **Ammo Cluster**: Bottom-right magazine pips (`#e8d28a` gold, `#ff7a2e` orange during *Bottomless Mag*), a sliding reload progress bar (`ReloadBarFill`), and `[R] RELOAD` when low.
- **Center Banner & Pickup Toast**: Fade out smoothly as `BannerEvent.timer` and `ToastEvent.timer` count down.

---

## 3. Between-Wave 3-Card Upgrade Picker

In `GameState::ChoosingUpgrade`, `PickerRoot` reveals three interactive cards (`PickerCard`). Each card mirrors the CSS layout:

- A `5px` top accent stripe colored by `UpgradeCat` (`Firepower`, `Handling`, `Survival`, `Fortune`)
- Category label + shortcut (`[1]`, `[2]`, `[3]`)
- `40x40` nearest-neighbor pixel icon
- Current → next stat preview (`1 -> 2`)
- Level pips: owned pips in category color, the next pip blinking gold, and empty pips dimmed

---

## 4. Title Screen, Film Grain, and Persistent Storage

In `src/ui/title.rs`, `setup_title_ui` and `update_title_ui` recreate the horror atmosphere over the live 3D cabin and wandering `TitleZombie`s:

- **Startup Fade-In (`FadeInOverlay`)**: A black full-screen overlay (`GlobalZIndex(50)`) fades from `1.0` to `0.0` over `1.1s`.
- **Animated Film Grain (`GrainOverlay`)**: A `128x128` procedural monochrome noise `Image` regenerated ~16 times per second at subtle alpha (`14/255`).
- **Left Scrim & Menus**: `Splash` (*PRESS ANY KEY OR CLICK*) → `Menu` (`01 PLAY`, `02 HOW TO PLAY`, `03 SETTINGS`) → `HowPanel` / `SettingsPanel` (10-bar volume meters + screen shake toggle).
- **Persistence (`src/ui/storage.rs`)**: `load_storage` and `save_storage` persist `master`, `sfx`, `music`, `shake`, `best_wave`, and `best_kills` to `$XDG_CONFIG_HOME/abandon-all-hope/save.txt`.

---

## 5. Keeping Large UI Systems Clean with `SystemParam`

Bevy systems have a 16-parameter tuple limit. To keep `update_hud`, `update_overlays`, and `update_title_ui` clean and readable, we group related queries into `#[derive(SystemParam)]` structs (`HudQueries`, `OverlayQueries`, `TitleUiQueries`, `RunEntities`, `ResetContext`).
