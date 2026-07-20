use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use rand::RngExt;

#[derive(Component)]
#[allow(dead_code)]
pub struct CloudBillboard {
    pub speed: f32,
    pub size_x: f32,
    pub size_z: f32,
}

pub fn generate_cloud_image() -> Image {
    let width = 256;
    let height = 256;
    let mut data = vec![0u8; (width * height * 4) as usize];

    let hash = |px: f32, py: f32| -> f32 {
        let sin_val = (px * 127.1 + py * 311.7).sin() * 43_758.547;
        sin_val - sin_val.floor()
    };

    let noise = |x: f32, y: f32| -> f32 {
        let ix = x.floor();
        let iy = y.floor();
        let fx = x - ix;
        let fy = y - iy;

        let a = hash(ix, iy);
        let b = hash(ix + 1.0, iy);
        let c = hash(ix, iy + 1.0);
        let d = hash(ix + 1.0, iy + 1.0);

        let ux = fx * fx * (3.0 - 2.0 * fx);
        let uy = fy * fy * (3.0 - 2.0 * fy);

        let lerp = |v1: f32, v2: f32, t: f32| v1 + t * (v2 - v1);
        lerp(lerp(a, b, ux), lerp(c, d, ux), uy)
    };

    let fbm = |x: f32, y: f32, octaves: usize| -> f32 {
        let mut value = 0.0;
        let mut amplitude = 0.5;
        let mut frequency = 1.0;
        for _ in 0..octaves {
            value += amplitude * noise(x * frequency, y * frequency);
            frequency *= 2.0;
            amplitude *= 0.5;
        }
        value
    };

    for y in 0..height {
        for x in 0..width {
            let nx = x as f32 / width as f32;
            let ny = y as f32 / height as f32;

            let cx = nx - 0.5;
            let cy = ny - 0.5;
            let radial_mask = (1.0 - (cx.abs() / 0.5).min(1.0)) * (1.0 - (cy.abs() / 0.5).min(1.0));

            let n_val = fbm(nx * 3.0, ny * 3.0, 4);
            let cloud_val = ((n_val - 0.32) / 0.22).clamp(0.0, 1.0);
            let alpha = (cloud_val * radial_mask * 255.0).clamp(0.0, 255.0) as u8;

            let idx = ((y * width + x) * 4) as usize;
            data[idx] = 255;
            data[idx + 1] = 255;
            data[idx + 2] = 255;
            data[idx + 3] = alpha;
        }
    }

    Image::new(
        Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    )
}

pub fn setup_clouds(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
) {
    let cloud_image = generate_cloud_image();
    let cloud_texture_handle = images.add(cloud_image);
    let cloud_material_handle = materials.add(StandardMaterial {
        base_color_texture: Some(cloud_texture_handle),
        alpha_mode: AlphaMode::Blend,
        unlit: true,
        cull_mode: None,
        ..default()
    });

    let mesh_handle = meshes.add(Rectangle::new(1.0, 1.0));

    let mut rng = rand::rng();
    for _ in 0..150 {
        let x = rng.random_range(-2400.0..2400.0);
        let y = rng.random_range(110.0..140.0);
        let z = rng.random_range(-2400.0..2400.0);
        let size_x = rng.random_range(150.0..250.0);
        let size_z = rng.random_range(100.0..160.0);
        let speed = rng.random_range(2.0..6.0);

        commands.spawn((
            Mesh3d(mesh_handle.clone()),
            MeshMaterial3d(cloud_material_handle.clone()),
            Transform::from_xyz(x, y, z)
                .with_scale(Vec3::new(size_x, size_z, 1.0))
                .with_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)),
            CloudBillboard {
                speed,
                size_x,
                size_z,
            },
            bevy::light::NotShadowCaster,
            bevy::light::NotShadowReceiver,
        ));
    }
}

pub fn animate_and_wrap_clouds(
    time: Res<Time>,
    camera_query: Query<&Transform, (With<Camera3d>, Without<CloudBillboard>)>,
    mut cloud_query: Query<(&mut Transform, &CloudBillboard)>,
) {
    let Some(camera_transform) = camera_query.iter().next() else {
        return;
    };
    let camera_pos = camera_transform.translation;
    let wind_direction = Vec3::new(1.0, 0.0, 0.5).normalize();

    for (mut transform, cloud) in &mut cloud_query {
        transform.translation += wind_direction * cloud.speed * time.delta_secs();

        let to_camera = transform.translation - camera_pos;
        let dist_xz = Vec2::new(to_camera.x, to_camera.z).length();

        if dist_xz > 2400.0 {
            let offset_direction = -wind_direction;
            let spawn_offset = offset_direction * 1800.0;

            let mut rng = rand::rng();
            let jitter_side = Vec3::new(-wind_direction.z, 0.0, wind_direction.x);
            let jitter = jitter_side * rng.random_range(-1200.0..1200.0);

            transform.translation = camera_pos + spawn_offset + jitter;
            transform.translation.y = rng.random_range(110.0..140.0);
            transform.scale = Vec3::new(cloud.size_x, cloud.size_z, 1.0);
        } else {
            let scale_factor = if dist_xz > 1800.0 {
                ((2400.0 - dist_xz) / 600.0).clamp(0.0, 1.0)
            } else if dist_xz > 1400.0 {
                ((1800.0 - dist_xz) / 400.0).clamp(0.0, 1.0)
            } else {
                1.0
            };
            transform.scale = Vec3::new(
                cloud.size_x * scale_factor,
                cloud.size_z * scale_factor,
                1.0,
            );
        }
    }
}
