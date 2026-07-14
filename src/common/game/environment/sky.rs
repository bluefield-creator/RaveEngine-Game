use bevy::prelude::*;
use bevy::asset::RenderAssetUsages;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

#[derive(Component)]
pub struct SkyDome;

#[derive(Component)]
pub struct SunDisk;

#[derive(Component)]
pub struct MoonDisk;

pub fn generate_sky_gradient_image() -> Image {
    let width = 1;
    let height = 256;
    let mut data = vec![0u8; (width * height * 4) as usize];
    for y in 0..height {
        let v = y as f32 / (height - 1) as f32;
        let horizon_intensity = (1.0 - ((v - 0.5).abs() / 0.08).min(1.0)).powf(2.0);
        let base_color = if v <= 0.5 {
            let t = (v / 0.5).powf(1.1);
            let r_val = 0.15 + t * (0.75 - 0.15);
            let g_val = 0.55 + t * (0.92 - 0.55);
            let b_val = 0.90 + t * (1.00 - 0.90);
            (r_val, g_val, b_val)
        } else {
            let t = (v - 0.5) / 0.5;
            let r_val = 0.75 + t * (0.50 - 0.75);
            let g_val = 0.92 + t * (0.75 - 0.92);
            let b_val = 1.00 + t * (0.90 - 1.00);
            (r_val, g_val, b_val)
        };
        let r = base_color.0 + horizon_intensity * (1.0 - base_color.0);
        let g = base_color.1 + horizon_intensity * (1.0 - base_color.1);
        let b = base_color.2 + horizon_intensity * (1.0 - base_color.2);
        let idx = (y * 4) as usize;
        data[idx] = (r * 255.0).clamp(0.0, 255.0) as u8;
        data[idx + 1] = (g * 255.0).clamp(0.0, 255.0) as u8;
        data[idx + 2] = (b * 255.0).clamp(0.0, 255.0) as u8;
        data[idx + 3] = 255;
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
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    )
}

pub fn setup_sky(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
) {
    let sky_gradient_image = generate_sky_gradient_image();
    let sky_texture_handle = images.add(sky_gradient_image);
    
    let sun_rotation = Quat::from_euler(EulerRot::XYZ, -1.2, 0.3, 0.0);
    let sun_dir = sun_rotation.mul_vec3(Vec3::Z);
    let sun_position = sun_dir * 450.0;

    let sky_dome = commands.spawn((
        Mesh3d(meshes.add(Sphere::new(500.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color_texture: Some(sky_texture_handle),
            unlit: true,
            cull_mode: None,
            fog_enabled: false,
            ..default()
        })),
        Transform::from_xyz(0.0, 0.0, 0.0),
        bevy::light::NotShadowCaster,
        bevy::light::NotShadowReceiver,
        SkyDome,
    )).id();

    let sun_disk = commands.spawn((
        Mesh3d(meshes.add(Sphere::new(15.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(5.0, 5.0, 4.0),
            unlit: true,
            fog_enabled: false,
            ..default()
        })),
        Transform::from_translation(sun_position),
        bevy::light::NotShadowCaster,
        bevy::light::NotShadowReceiver,
        SunDisk,
    )).id();

    let moon_disk = commands.spawn((
        Mesh3d(meshes.add(Sphere::new(10.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(3.0, 3.0, 4.0),
            unlit: true,
            fog_enabled: false,
            ..default()
        })),
        Transform::from_translation(-sun_position),
        bevy::light::NotShadowCaster,
        bevy::light::NotShadowReceiver,
        MoonDisk,
    )).id();

    commands.entity(sky_dome).add_child(sun_disk);
    commands.entity(sky_dome).add_child(moon_disk);
}

pub fn sync_sky_dome(
    camera_query: Query<&Transform, (With<Camera3d>, Without<SkyDome>)>,
    mut sky_query: Query<&mut Transform, With<SkyDome>>,
) {
    if let Some(camera_transform) = camera_query.iter().next() {
        for mut sky_transform in &mut sky_query {
            sky_transform.translation = camera_transform.translation;
        }
    }
}