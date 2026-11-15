use std::fs;
use std::path::PathBuf;

use bevy::prelude::*;

#[derive(Resource, Clone, Debug)]
pub struct GameSettings {
    pub master: u8,
    pub sfx: u8,
    pub music: u8,
    pub shake: bool,
    pub best_wave: u32,
    pub best_kills: u32,
}

impl Default for GameSettings {
    fn default() -> Self {
        Self {
            master: 8,
            sfx: 8,
            music: 8,
            shake: true,
            best_wave: 0,
            best_kills: 0,
        }
    }
}

fn save_path() -> Option<PathBuf> {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))?;
    Some(base.join("abandon-all-hope").join("save.txt"))
}

pub fn load_storage() -> GameSettings {
    let mut s = GameSettings::default();
    let Some(path) = save_path() else {
        return s;
    };
    let Ok(text) = fs::read_to_string(path) else {
        return s;
    };
    for line in text.lines() {
        let Some((k, v)) = line.split_once('=') else {
            continue;
        };
        match k.trim() {
            "master" => {
                if let Ok(val) = v.trim().parse::<u8>() {
                    s.master = val.min(10);
                }
            }
            "sfx" => {
                if let Ok(val) = v.trim().parse::<u8>() {
                    s.sfx = val.min(10);
                }
            }
            "music" => {
                if let Ok(val) = v.trim().parse::<u8>() {
                    s.music = val.min(10);
                }
            }
            "shake" => {
                s.shake = v.trim() != "0" && v.trim() != "false";
            }
            "best_wave" => {
                if let Ok(val) = v.trim().parse::<u32>() {
                    s.best_wave = val;
                }
            }
            "best_kills" => {
                if let Ok(val) = v.trim().parse::<u32>() {
                    s.best_kills = val;
                }
            }
            _ => {}
        }
    }
    s
}

pub fn save_storage(s: &GameSettings) {
    let Some(path) = save_path() else {
        return;
    };
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let body = format!(
        "master={}\nsfx={}\nmusic={}\nshake={}\nbest_wave={}\nbest_kills={}\n",
        s.master,
        s.sfx,
        s.music,
        if s.shake { 1 } else { 0 },
        s.best_wave,
        s.best_kills
    );
    let _ = fs::write(path, body);
}
