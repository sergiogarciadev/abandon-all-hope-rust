use bevy::asset::RenderAssetUsages;
use bevy::light::NotShadowCaster;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;
use rand::RngExt;
use std::f32::consts::FRAC_PI_2;

use crate::core::state::{GameState, GameStats};
use crate::entities::player::PlayerRig;

use super::world::{WorldMaterials, WorldWeatherLevel};

const STORM_STEPS: [(u32, f32); 5] = [(4, 0.0), (5, 0.15), (8, 0.4), (12, 0.7), (15, 1.0)];
pub const BOLT_LEVEL: f32 = 0.7;

const RAIN_MAX: usize = 3000;
const RAIN_AREA: f32 = 72.0;
const RAIN_TOP: f32 = 16.0;
const SPLASHES: usize = 160;

pub fn storm_for(wave: u32) -> f32 {
    for i in 1..STORM_STEPS.len() {
        let (w0, s0) = STORM_STEPS[i - 1];
        let (w1, s1) = STORM_STEPS[i];
        if wave <= w1 {
            if wave <= w0 {
                return s0;
            }
            return s0 + (s1 - s0) * (wave - w0) as f32 / (w1 - w0) as f32;
        }
    }
    1.0
}

pub struct SplashSlot {
    pub entity: Entity,
    pub x: f32,
    pub z: f32,
    pub born: f32,
}

#[derive(Resource)]
pub struct WeatherState {
    pub level: f32,
    pub target: f32,
    pub t: f32,
    pub drop: f32,
    pub gust: f32,
    pub flash_t: f32,
    pub flash: f32,
    pub next_bolt: f32,
    pub rain_mesh: Handle<Mesh>,
    pub rain_mat: Handle<StandardMaterial>,
    pub rain_entity: Entity,
    pub rain_seeds: Vec<[f32; 3]>,
    pub splashes: Vec<SplashSlot>,
    pub splash_i: usize,
    pub splash_due: f32,
}

impl WeatherState {
    pub fn set(&mut self, level: f32) {
        self.level = level;
        self.target = level;
        self.flash_t = -1.0;
        self.flash = 0.0;
        self.next_bolt = 8.0;
    }
}

pub fn setup_weather(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let mut rng = rand::rng();
    let mut rain_seeds = Vec::with_capacity(RAIN_MAX);
    for _ in 0..RAIN_MAX {
        rain_seeds.push([rng.random(), rng.random(), rng.random()]);
    }

    let positions = vec![[0.0_f32, -10.0, 0.0]; RAIN_MAX * 2];
    let indices: Vec<u32> = (0..(RAIN_MAX * 2) as u32).collect();
    let mut mesh = Mesh::new(PrimitiveTopology::LineList, RenderAssetUsages::default());
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_indices(Indices::U32(indices));
    let rain_mesh = meshes.add(mesh);

    let rain_mat = materials.add(StandardMaterial {
        base_color: Color::srgba(0.8, 0.87, 0.95, 0.0),
        emissive: LinearRgba::rgb(0.4, 0.45, 0.55),
        alpha_mode: AlphaMode::Blend,
        unlit: true,
        ..default()
    });

    let rain_entity = commands
        .spawn((
            Mesh3d(rain_mesh.clone()),
            MeshMaterial3d(rain_mat.clone()),
            Visibility::Hidden,
            NotShadowCaster,
        ))
        .id();

    let splash_mesh = meshes.add(Rectangle::new(0.1, 0.1));
    let splash_mat = materials.add(StandardMaterial {
        base_color: Color::srgba(0.75, 0.82, 0.88, 0.55),
        emissive: LinearRgba::rgb(0.35, 0.42, 0.5),
        alpha_mode: AlphaMode::Add,
        unlit: true,
        ..default()
    });

    let mut splashes = Vec::with_capacity(SPLASHES);
    for _ in 0..SPLASHES {
        let entity = commands
            .spawn((
                Mesh3d(splash_mesh.clone()),
                MeshMaterial3d(splash_mat.clone()),
                Transform::from_xyz(0.0, -5.0, 0.0)
                    .with_rotation(Quat::from_rotation_x(-FRAC_PI_2))
                    .with_scale(Vec3::ZERO),
                NotShadowCaster,
            ))
            .id();
        splashes.push(SplashSlot {
            entity,
            x: 0.0,
            z: 0.0,
            born: -9.0,
        });
    }

    commands.insert_resource(WeatherState {
        level: 0.0,
        target: 0.0,
        t: 0.0,
        drop: 0.0,
        gust: 1.0,
        flash_t: -1.0,
        flash: 0.0,
        next_bolt: 8.0,
        rain_mesh,
        rain_mat,
        rain_entity,
        rain_seeds,
        splashes,
        splash_i: 0,
        splash_due: 0.0,
    });
}

pub fn update_weather(
    time: Res<Time>,
    state: Res<State<GameState>>,
    rig: Res<PlayerRig>,
    mut weather: ResMut<WeatherState>,
    mut world_weather: ResMut<WorldWeatherLevel>,
    world_mats: Res<WorldMaterials>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut clear_color: ResMut<ClearColor>,
    mut ambient: ResMut<GlobalAmbientLight>,
    mut moon_q: Query<&mut DirectionalLight>,
    mut fog_q: Query<&mut DistanceFog>,
    mut transforms: Query<&mut Transform>,
    mut visibilities: Query<&mut Visibility>,
    _stats: Res<GameStats>,
) {
    let dt = time.delta_secs();
    let p_pos = transforms
        .get(rig.root)
        .map(|t| t.translation)
        .unwrap_or(Vec3::ZERO);

    let step = (weather.target - weather.level).clamp(-dt * 0.08, dt * 0.08);
    weather.level += step;
    let l = weather.level;
    world_weather.0 = l;
    weather.t += dt;

    weather.gust = 1.0
        + l * (0.45 * (weather.t * 0.63).sin()
            + 0.3 * (weather.t * 1.71 + 2.0).sin()
            + 0.15 * (weather.t * 4.1).sin());

    let speed = 11.0 + 13.0 * l;
    let streak_len = 0.25 + 1.0 * l;
    let wind = Vec2::new(7.0 * l * l * weather.gust, 2.5 * l * l * weather.gust);
    weather.drop += speed * dt;
    let center = Vec3::new(p_pos.x, 0.0, p_pos.z - 4.0);

    let active_drops = ((RAIN_MAX as f32 * l).round() as usize).min(RAIN_MAX);
    if let Ok(mut vis) = visibilities.get_mut(weather.rain_entity) {
        *vis = if l > 0.005 {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }
    if let Some(mut mat) = materials.get_mut(&weather.rain_mat) {
        mat.base_color.set_alpha(0.3 + 0.35 * l);
    }

    if l > 0.005 {
        if let Some(mut mesh) = meshes.get_mut(&weather.rain_mesh) {
            if let Some(bevy::mesh::VertexAttributeValues::Float32x3(pos)) =
                mesh.attribute_mut(Mesh::ATTRIBUTE_POSITION)
            {
                let trail = Vec3::new(-wind.x, speed, -wind.y).normalize_or_zero() * streak_len;
                for i in 0..RAIN_MAX {
                    if i >= active_drops {
                        pos[i * 2] = [0.0, -20.0, 0.0];
                        pos[i * 2 + 1] = [0.0, -20.0, 0.0];
                        continue;
                    }
                    let [sx, sy, sz] = weather.rain_seeds[i];
                    let y = (sz * RAIN_TOP - weather.drop * (0.85 + 0.3 * sx)).rem_euclid(RAIN_TOP);
                    let drift = (RAIN_TOP - y) / speed;
                    let px = center.x + (sx - 0.5) * RAIN_AREA + wind.x * drift;
                    let pz = center.z + (sy - 0.5) * RAIN_AREA + wind.y * drift;
                    pos[i * 2] = [px, y, pz];
                    pos[i * 2 + 1] = [px + trail.x, y + trail.y, pz + trail.z];
                }
            }
        }
    }

    // Splashes
    let mut rng = rand::rng();
    weather.splash_due += l * 520.0 * dt;
    while weather.splash_due >= 1.0 {
        weather.splash_due -= 1.0;
        let idx = weather.splash_i;
        weather.splash_i = (weather.splash_i + 1) % SPLASHES;
        let t = weather.t;
        let s = &mut weather.splashes[idx];
        s.x = p_pos.x + (rng.random::<f32>() * 52.0 - 26.0);
        s.z = p_pos.z + (rng.random::<f32>() * 36.0 - 22.0);
        s.born = t;
    }

    for s in &weather.splashes {
        let k = (weather.t - s.born) / 0.16;
        let live = k >= 0.0 && k < 1.0 && l > 0.005;
        if let Ok(mut tf) = transforms.get_mut(s.entity) {
            if live {
                tf.translation = Vec3::new(s.x, 0.03, s.z);
                tf.scale = Vec3::splat(1.0 + 2.5 * k);
            } else {
                tf.scale = Vec3::ZERO;
            }
        }
    }

    // Lightning
    if l >= BOLT_LEVEL - 0.001 && *state.get() == GameState::Playing {
        weather.next_bolt -= dt;
        if weather.next_bolt <= 0.0 {
            weather.flash_t = 0.0;
            let k = ((l - BOLT_LEVEL) / (1.0 - BOLT_LEVEL)).clamp(0.0, 1.0);
            let base = 16.0 + (3.5 - 16.0) * k;
            weather.next_bolt = base * (0.6 + rng.random::<f32>() * 0.8);
        }
    }

    if weather.flash_t >= 0.0 {
        weather.flash_t += dt;
        let t = weather.flash_t;
        weather.flash = (-t * 14.0).exp()
            + if t > 0.13 {
                0.8 * (-(t - 0.13) * 9.0).exp()
            } else {
                0.0
            };
        if t > 1.2 {
            weather.flash_t = -1.0;
            weather.flash = 0.0;
        }
    }

    // Sky, fog, lights, and wet roughness
    let night = Color::srgb_u8(0x05, 0x06, 0x0a).to_linear();
    let bolt_sky = Color::srgb_u8(0x3a, 0x44, 0x58).to_linear();
    let f_t = (weather.flash * 0.7).clamp(0.0, 1.0);
    let sky_col = Color::LinearRgba(LinearRgba::new(
        night.red + (bolt_sky.red - night.red) * f_t,
        night.green + (bolt_sky.green - night.green) * f_t,
        night.blue + (bolt_sky.blue - night.blue) * f_t,
        1.0,
    ));

    clear_color.0 = sky_col;
    for mut fog in &mut fog_q {
        fog.color = sky_col;
        fog.falloff = FogFalloff::Linear {
            start: 32.0 - 14.0 * l,
            end: 62.0 - 18.0 * l,
        };
    }

    for mut moon in &mut moon_q {
        moon.illuminance = 10_000.0 * ((2.8 - 1.1 * l + weather.flash * 14.0) / 2.8);
    }
    ambient.brightness = 950.0 * ((3.0 - 0.7 * l + weather.flash * 3.0) / 3.0);

    if let Some(mut mat) = materials.get_mut(&world_mats.world_mat) {
        mat.perceptual_roughness = 0.92 - 0.35 * l;
    }
    if let Some(mut mat) = materials.get_mut(&world_mats.ground_mat) {
        mat.perceptual_roughness = 1.0 - 0.4 * l;
    }
}
