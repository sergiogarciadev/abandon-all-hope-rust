use bevy::light::NotShadowCaster;
use bevy::prelude::*;
use std::f32::consts::PI;

#[derive(Component)]
pub struct Debris {
    pub vel: Vec3,
    pub spin: Vec3,
    pub life: f32,
    pub max_life: f32,
    pub bounce: f32,
    pub smoke_size: Option<f32>,
    pub base_scale: Vec3,
}

#[derive(Resource)]
pub struct DebrisAssets {
    pub cube_mesh: Handle<Mesh>,
    pub casing_mesh: Handle<Mesh>,
    pub brass_mat: Handle<StandardMaterial>,
    pub wood_chip_mat: Handle<StandardMaterial>,
    pub glass_chip_mat: Handle<StandardMaterial>,
}

pub fn setup_debris_assets(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.insert_resource(DebrisAssets {
        cube_mesh: meshes.add(Cuboid::new(1.0, 1.0, 1.0)),
        casing_mesh: meshes.add(Cuboid::new(0.05, 0.05, 0.12)),
        brass_mat: materials.add(StandardMaterial {
            base_color: Color::srgb_u8(0xc9, 0xa1, 0x4a),
            perceptual_roughness: 0.35,
            metallic: 0.7,
            ..default()
        }),
        wood_chip_mat: materials.add(StandardMaterial {
            base_color: Color::srgb_u8(0x6d, 0x63, 0x52),
            perceptual_roughness: 0.9,
            ..default()
        }),
        glass_chip_mat: materials.add(StandardMaterial {
            base_color: Color::srgba_u8(0x9f, 0xb6, 0xc9, 77),
            perceptual_roughness: 0.08,
            metallic: 0.4,
            alpha_mode: AlphaMode::Blend,
            ..default()
        }),
    });
}

pub fn spawn_smoke(
    commands: &mut Commands,
    assets: &DebrisAssets,
    materials: &mut Assets<StandardMaterial>,
    pos: Vec3,
    rot: Quat,
    vel: Vec3,
    size: f32,
) {
    let mat = materials.add(StandardMaterial {
        base_color: Color::srgba_u8(0x8d, 0x8f, 0x94, 82),
        perceptual_roughness: 1.0,
        alpha_mode: AlphaMode::Blend,
        ..default()
    });
    commands.spawn((
        Mesh3d(assets.cube_mesh.clone()),
        MeshMaterial3d(mat),
        Transform::from_translation(pos)
            .with_rotation(rot)
            .with_scale(Vec3::splat(size)),
        NotShadowCaster,
        Debris {
            vel,
            spin: Vec3::ZERO,
            life: 0.55,
            max_life: 0.55,
            bounce: 0.0,
            smoke_size: Some(size),
            base_scale: Vec3::splat(size),
        },
    ));
}

pub fn update_debris(
    mut commands: Commands,
    time: Res<Time>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut q: Query<(
        Entity,
        &mut Transform,
        &mut Debris,
        &MeshMaterial3d<StandardMaterial>,
    )>,
) {
    let dt = time.delta_secs();
    for (e, mut tf, mut d, mat_h) in &mut q {
        d.life -= dt;
        if let Some(smoke_s) = d.smoke_size {
            let k = 1.0 - (d.life / d.max_life).clamp(0.0, 1.0);
            tf.scale = Vec3::splat(smoke_s * (1.0 + k * 2.5));
            if let Some(mut mat) = materials.get_mut(&mat_h.0) {
                let a = 0.32 * (1.0 - k) * (1.0 - k);
                mat.base_color.set_alpha(a);
            }
            d.vel *= (-3.0 * dt).exp();
            tf.translation += d.vel * dt;
            tf.translation.y += 0.5 * dt;
        } else {
            d.vel.y -= 16.0 * dt;
            tf.translation += d.vel * dt;
            let spin_delta = Quat::from_euler(
                EulerRot::YXZ,
                d.spin.y * dt,
                d.spin.x * dt,
                d.spin.z * dt,
            );
            tf.rotation = spin_delta * tf.rotation;
            if tf.translation.y < d.bounce {
                tf.translation.y = d.bounce;
                d.vel.x *= 0.45;
                d.vel.z *= 0.45;
                d.vel.y = if d.vel.y.abs() > 1.5 {
                    -d.vel.y * 0.35
                } else {
                    0.0
                };
                d.spin *= 0.4;
                if d.vel.y == 0.0 {
                    d.spin = Vec3::ZERO;
                    let (_, y, z) = tf.rotation.to_euler(EulerRot::YXZ);
                    tf.rotation = Quat::from_euler(EulerRot::YXZ, y, 0.0, (z / PI).round() * PI);
                }
            }
            if d.life < 0.4 {
                tf.scale = d.base_scale * (d.life / 0.4).max(0.01);
            }
        }
        if d.life <= 0.0 {
            commands.entity(e).despawn();
        }
    }
}
