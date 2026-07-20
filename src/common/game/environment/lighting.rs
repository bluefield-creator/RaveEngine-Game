use bevy::prelude::*;

#[derive(Resource, Clone, Debug, Reflect)]
#[reflect(Resource)]
pub struct LightingService {
    pub time_of_day: f32,
    pub sun_intensity: f32,
    pub ambient_intensity: f32,
    pub sun_tint: Color,
    pub ambient_tint: Color,
    pub shadows_enabled: bool,
    pub fog_enabled: bool,
    pub fog_density: f32,
}

impl Default for LightingService {
    fn default() -> Self {
        Self {
            time_of_day: 12.0,
            sun_intensity: 1.0,
            ambient_intensity: 1.0,
            sun_tint: Color::WHITE,
            ambient_tint: Color::WHITE,
            shadows_enabled: true,
            fog_enabled: true,
            fog_density: 1.0,
        }
    }
}

pub fn sync_lighting_service(
    mut lighting_service: ResMut<LightingService>,
    playtest: Option<Res<crate::client::PlaytestState>>,
) {
    if !crate::client::is_playtesting(playtest) {
        return;
    }
    if let Ok(shared) = crate::studio::tools::SHARED_LIGHTING_SERVICE.read() {
        if (lighting_service.time_of_day - *shared).abs() > 0.001 {
            lighting_service.time_of_day = *shared;
        }
    }
}

pub fn update_lighting_system(
    mut commands: Commands,
    lighting_service: Res<LightingService>,
    mut ambient: ResMut<GlobalAmbientLight>,
    sky_dome_query: Query<
        &MeshMaterial3d<StandardMaterial>,
        With<crate::common::game::environment::sky::SkyDome>,
    >,
    mut celestial_query: Query<
        (
            &mut Transform,
            Option<&crate::common::game::environment::sky::SunDisk>,
            Option<&crate::common::game::environment::sky::MoonDisk>,
        ),
        Or<(
            With<crate::common::game::environment::sky::SunDisk>,
            With<crate::common::game::environment::sky::MoonDisk>,
        )>,
    >,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
    mut sun_light_query: Query<
        (&mut DirectionalLight, &mut Transform),
        (
            Without<crate::common::game::environment::sky::SunDisk>,
            Without<crate::common::game::environment::sky::MoonDisk>,
        ),
    >,
    mut camera_query: Query<(Entity, Option<&mut DistanceFog>), With<Camera3d>>,
) {
    if !lighting_service.is_changed() {
        return;
    }
    let time_of_day = lighting_service.time_of_day.rem_euclid(24.0);

    let altitude = (time_of_day - 6.0) / 24.0 * 2.0 * std::f32::consts::PI;
    let azimuth = 0.95;
    let tilt = 30.0f32.to_radians();
    let sun_rotation = Quat::from_rotation_y(azimuth)
        * Quat::from_rotation_z(tilt)
        * Quat::from_rotation_x(-altitude);
    let sun_dir = sun_rotation.mul_vec3(Vec3::Z);
    let sun_y = sun_dir.y;

    let (ambient_color, ambient_brightness) = if time_of_day >= 6.0 && time_of_day < 12.0 {
        let t = (time_of_day - 6.0) / 6.0;
        (
            interpolate_color(
                Color::srgb(0.40, 0.35, 0.45),
                Color::srgb(0.55, 0.75, 0.95),
                t,
            ),
            180.0 + t * 140.0,
        )
    } else if time_of_day >= 12.0 && time_of_day < 19.0 {
        let t = (time_of_day - 12.0) / 7.0;
        (
            interpolate_color(
                Color::srgb(0.55, 0.75, 0.95),
                Color::srgb(0.45, 0.25, 0.35),
                t,
            ),
            320.0 - t * 140.0,
        )
    } else if time_of_day >= 19.0 && time_of_day < 21.5 {
        let t = (time_of_day - 19.0) / 2.5;
        (
            interpolate_color(
                Color::srgb(0.45, 0.25, 0.35),
                Color::srgb(0.06, 0.09, 0.22),
                t,
            ),
            180.0 - t * 130.0,
        )
    } else if time_of_day >= 4.5 && time_of_day < 6.0 {
        let t = (time_of_day - 4.5) / 1.5;
        (
            interpolate_color(
                Color::srgb(0.06, 0.09, 0.22),
                Color::srgb(0.40, 0.35, 0.45),
                t,
            ),
            50.0 + t * 130.0,
        )
    } else {
        (Color::srgb(0.06, 0.09, 0.22), 50.0)
    };

    ambient.color = multiply_color(ambient_color, lighting_service.ambient_tint);
    ambient.brightness = ambient_brightness * lighting_service.ambient_intensity.max(0.0);

    let sun_color = if time_of_day >= 6.0 && time_of_day < 12.0 {
        let t = (time_of_day - 6.0) / 6.0;
        interpolate_color(
            Color::srgb(1.0, 0.50, 0.20),
            Color::srgb(1.0, 0.96, 0.85),
            t,
        )
    } else if time_of_day >= 12.0 && time_of_day < 19.0 {
        let t = (time_of_day - 12.0) / 7.0;
        interpolate_color(
            Color::srgb(1.0, 0.96, 0.85),
            Color::srgb(1.0, 0.40, 0.10),
            t,
        )
    } else if time_of_day >= 19.0 && time_of_day < 21.5 {
        let t = (time_of_day - 19.0) / 2.5;
        interpolate_color(
            Color::srgb(1.0, 0.40, 0.10),
            Color::srgb(0.55, 0.65, 0.90),
            t,
        )
    } else if time_of_day >= 4.5 && time_of_day < 6.0 {
        let t = (time_of_day - 4.5) / 1.5;
        interpolate_color(
            Color::srgb(0.55, 0.65, 0.90),
            Color::srgb(1.0, 0.50, 0.20),
            t,
        )
    } else {
        Color::srgb(0.55, 0.65, 0.90)
    };

    let sun_illuminance = if time_of_day >= 6.0 && time_of_day < 12.0 {
        let t = (time_of_day - 6.0) / 6.0;
        12_000.0 + t * 73_000.0
    } else if time_of_day >= 12.0 && time_of_day < 19.0 {
        let t = (time_of_day - 12.0) / 7.0;
        85_000.0 - t * 73_000.0
    } else if time_of_day >= 19.0 && time_of_day < 21.5 {
        let t = (time_of_day - 19.0) / 2.5;
        12_000.0 - t * 11_700.0
    } else if time_of_day >= 4.5 && time_of_day < 6.0 {
        let t = (time_of_day - 4.5) / 1.5;
        300.0 + t * 11_700.0
    } else {
        300.0
    };

    for (mut light, mut transform) in &mut sun_light_query {
        light.color = multiply_color(sun_color, lighting_service.sun_tint);
        light.illuminance = sun_illuminance * lighting_service.sun_intensity.max(0.0);
        light.shadow_maps_enabled = lighting_service.shadows_enabled;
        light.contact_shadows_enabled = lighting_service.shadows_enabled;
        transform.rotation = sun_rotation;
    }

    let sun_scale_factor = (sun_y * 10.0).clamp(0.0, 1.0);
    let moon_scale_factor = (-sun_y * 10.0).clamp(0.0, 1.0);

    for (mut transform, sun_opt, moon_opt) in &mut celestial_query {
        if sun_opt.is_some() {
            transform.translation = sun_dir * 450.0;
            transform.scale = Vec3::splat(sun_scale_factor);
        } else if moon_opt.is_some() {
            transform.translation = -sun_dir * 450.0;
            transform.scale = Vec3::splat(moon_scale_factor);
        }
    }

    for sky_dome_mat_handle in &sky_dome_query {
        if let Some(material) = materials.get_mut(&sky_dome_mat_handle.0) {
            if let Some(ref tex_handle) = material.base_color_texture {
                if let Some(mut image) = images.get_mut(tex_handle) {
                    update_sky_gradient(time_of_day, &mut *image);
                }
            }
        }
    }

    for (camera_entity, mut fog_opt) in &mut camera_query {
        if !lighting_service.fog_enabled {
            commands.entity(camera_entity).remove::<DistanceFog>();
            continue;
        }
        let (fog_color, density) = if time_of_day >= 6.0 && time_of_day < 12.0 {
            let t = (time_of_day - 6.0) / 6.0;
            (
                interpolate_color(
                    Color::srgb(0.70, 0.45, 0.35),
                    Color::srgb(0.55, 0.65, 0.75),
                    t,
                ),
                0.0003 - t * 0.0001,
            )
        } else if time_of_day >= 12.0 && time_of_day < 19.0 {
            let t = (time_of_day - 12.0) / 7.0;
            (
                interpolate_color(
                    Color::srgb(0.55, 0.65, 0.75),
                    Color::srgb(0.75, 0.35, 0.20),
                    t,
                ),
                0.0002 + t * 0.00015,
            )
        } else if time_of_day >= 19.0 && time_of_day < 21.5 {
            let t = (time_of_day - 19.0) / 2.5;
            (
                interpolate_color(
                    Color::srgb(0.75, 0.35, 0.20),
                    Color::srgb(0.04, 0.05, 0.10),
                    t,
                ),
                0.00035 + t * 0.00025,
            )
        } else if time_of_day >= 4.5 && time_of_day < 6.0 {
            let t = (time_of_day - 4.5) / 1.5;
            (
                interpolate_color(
                    Color::srgb(0.04, 0.05, 0.10),
                    Color::srgb(0.70, 0.45, 0.35),
                    t,
                ),
                0.0006 - t * 0.0003,
            )
        } else {
            (Color::srgb(0.04, 0.05, 0.10), 0.0006)
        };

        let density = density * lighting_service.fog_density.max(0.0);
        if let Some(ref mut fog) = fog_opt {
            fog.color = fog_color;
            fog.falloff = FogFalloff::Exponential { density };
        } else {
            commands.entity(camera_entity).insert(DistanceFog {
                color: fog_color,
                falloff: FogFalloff::Exponential { density },
                ..default()
            });
        }
    }
}

pub fn update_sky_gradient(time_of_day: f32, image: &mut Image) {
    let height = 256;
    if image.data.is_none() {
        return;
    }
    let data = image.data.as_mut().unwrap();
    if data.len() < height * 4 {
        return;
    }

    let (bottom_color, horizon_color, top_color, glow_color) = if time_of_day < 6.0 {
        let t = time_of_day / 6.0;
        let night_bottom = Color::srgb(0.05, 0.08, 0.20);
        let night_horizon = Color::srgb(0.08, 0.12, 0.28);
        let night_top = Color::srgb(0.02, 0.04, 0.12);
        let night_glow = Color::srgb(0.12, 0.18, 0.38);

        let sunrise_bottom = Color::srgb(0.15, 0.15, 0.30);
        let sunrise_horizon = Color::srgb(0.95, 0.55, 0.30);
        let sunrise_top = Color::srgb(0.15, 0.25, 0.50);
        let sunrise_glow = Color::srgb(1.0, 0.80, 0.40);

        (
            interpolate_color(night_bottom, sunrise_bottom, t),
            interpolate_color(night_horizon, sunrise_horizon, t),
            interpolate_color(night_top, sunrise_top, t),
            interpolate_color(night_glow, sunrise_glow, t),
        )
    } else if time_of_day < 12.0 {
        let t = (time_of_day - 6.0) / 6.0;
        let sunrise_bottom = Color::srgb(0.15, 0.15, 0.30);
        let sunrise_horizon = Color::srgb(0.95, 0.55, 0.30);
        let sunrise_top = Color::srgb(0.15, 0.25, 0.50);
        let sunrise_glow = Color::srgb(1.0, 0.80, 0.40);

        let day_bottom = Color::srgb(0.15, 0.55, 0.90);
        let day_horizon = Color::srgb(0.75, 0.92, 1.00);
        let day_top = Color::srgb(0.50, 0.75, 0.90);
        let day_glow = Color::srgb(1.0, 1.0, 1.0);

        (
            interpolate_color(sunrise_bottom, day_bottom, t),
            interpolate_color(sunrise_horizon, day_horizon, t),
            interpolate_color(sunrise_top, day_top, t),
            interpolate_color(sunrise_glow, day_glow, t),
        )
    } else if time_of_day < 19.0 {
        let t = (time_of_day - 12.0) / 7.0;
        let day_bottom = Color::srgb(0.15, 0.55, 0.90);
        let day_horizon = Color::srgb(0.75, 0.92, 1.00);
        let day_top = Color::srgb(0.50, 0.75, 0.90);
        let day_glow = Color::srgb(1.0, 1.0, 1.0);

        let sunset_bottom = Color::srgb(0.35, 0.10, 0.15);
        let sunset_horizon = Color::srgb(1.0, 0.40, 0.10);
        let sunset_top = Color::srgb(0.10, 0.12, 0.35);
        let sunset_glow = Color::srgb(1.0, 0.65, 0.20);

        (
            interpolate_color(day_bottom, sunset_bottom, t),
            interpolate_color(day_horizon, sunset_horizon, t),
            interpolate_color(day_top, sunset_top, t),
            interpolate_color(day_glow, sunset_glow, t),
        )
    } else {
        let t = ((time_of_day - 19.0) / 3.0).min(1.0);
        let sunset_bottom = Color::srgb(0.35, 0.10, 0.15);
        let sunset_horizon = Color::srgb(1.0, 0.40, 0.10);
        let sunset_top = Color::srgb(0.10, 0.12, 0.35);
        let sunset_glow = Color::srgb(1.0, 0.65, 0.20);

        let night_bottom = Color::srgb(0.05, 0.08, 0.20);
        let night_horizon = Color::srgb(0.08, 0.12, 0.28);
        let night_top = Color::srgb(0.02, 0.04, 0.12);
        let night_glow = Color::srgb(0.12, 0.18, 0.38);

        (
            interpolate_color(sunset_bottom, night_bottom, t),
            interpolate_color(sunset_horizon, night_horizon, t),
            interpolate_color(sunset_top, night_top, t),
            interpolate_color(sunset_glow, night_glow, t),
        )
    };

    let srgba_glow = glow_color.to_srgba();

    for y in 0..height {
        let v = y as f32 / (height - 1) as f32;
        let horizon_intensity = (1.0 - ((v - 0.5).abs() / 0.08).min(1.0)).powf(2.0);

        let base_color = if v <= 0.5 {
            let t = (v / 0.5).powf(1.1);
            interpolate_color(bottom_color, horizon_color, t)
        } else {
            let t = (v - 0.5) / 0.5;
            interpolate_color(horizon_color, top_color, t)
        };

        let srgba_base = base_color.to_srgba();
        let r = srgba_base.red + horizon_intensity * (srgba_glow.red - srgba_base.red);
        let g = srgba_base.green + horizon_intensity * (srgba_glow.green - srgba_base.green);
        let b = srgba_base.blue + horizon_intensity * (srgba_glow.blue - srgba_base.blue);

        let idx = y * 4;
        data[idx] = (r * 255.0).clamp(0.0, 255.0) as u8;
        data[idx + 1] = (g * 255.0).clamp(0.0, 255.0) as u8;
        data[idx + 2] = (b * 255.0).clamp(0.0, 255.0) as u8;
        data[idx + 3] = 255;
    }
}

fn interpolate_color(c1: Color, c2: Color, t: f32) -> Color {
    let s1 = c1.to_srgba();
    let s2 = c2.to_srgba();
    Color::Srgba(Srgba::new(
        s1.red + t * (s2.red - s1.red),
        s1.green + t * (s2.green - s1.green),
        s1.blue + t * (s2.blue - s1.blue),
        s1.alpha + t * (s2.alpha - s1.alpha),
    ))
}

fn multiply_color(color: Color, tint: Color) -> Color {
    let color = color.to_srgba();
    let tint = tint.to_srgba();
    Color::srgba(
        color.red * tint.red,
        color.green * tint.green,
        color.blue * tint.blue,
        color.alpha,
    )
}
