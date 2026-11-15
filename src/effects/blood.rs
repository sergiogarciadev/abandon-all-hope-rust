use bevy::prelude::*;
use rand::RngExt;

#[derive(Component)]
pub struct BloodParticle {
    pub vel: Vec3,
    pub life: f32,
}

#[derive(Resource)]
pub struct BloodAssets {
    pub cube_mesh: Handle<Mesh>,
    pub blood_mat: Handle<StandardMaterial>,
}

pub fn setup_blood_assets(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.insert_resource(BloodAssets {
        cube_mesh: meshes.add(Cuboid::new(1.0, 1.0, 1.0)),
        blood_mat: materials.add(StandardMaterial {
            base_color: Color::srgb_u8(0x6a, 0x0e, 0x0e),
            perceptual_roughness: 0.35,
            ..default()
        }),
    });
}

pub fn spawn_blood(commands: &mut Commands, assets: &BloodAssets, pos: Vec3) {
    let mut rng = rand::rng();
    for _ in 0..12 {
        let s = 0.12 + rng.random::<f32>() * 0.18;
        commands.spawn((
            Mesh3d(assets.cube_mesh.clone()),
            MeshMaterial3d(assets.blood_mat.clone()),
            Transform::from_xyz(pos.x, 0.8, pos.z).with_scale(Vec3::splat(s)),
            BloodParticle {
                vel: Vec3::new(
                    (rng.random::<f32>() - 0.5) * 5.0,
                    3.0 + rng.random::<f32>() * 4.0,
                    (rng.random::<f32>() - 0.5) * 5.0,
                ),
                life: 0.6 + rng.random::<f32>() * 0.4,
            },
        ));
    }
}

pub fn update_blood_particles(
    mut commands: Commands,
    time: Res<Time>,
    mut q: Query<(Entity, &mut Transform, &mut BloodParticle)>,
) {
    let dt = time.delta_secs();
    for (e, mut tf, mut p) in &mut q {
        p.life -= dt;
        p.vel.y -= 16.0 * dt;
        tf.translation += p.vel * dt;
        if tf.translation.y < 0.05 {
            tf.translation.y = 0.05;
            p.vel = Vec3::ZERO;
        }
        if p.life <= 0.0 {
            commands.entity(e).despawn();
        }
    }
}
