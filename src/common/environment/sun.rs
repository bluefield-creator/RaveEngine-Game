use bevy::prelude::*;
use bevy::light::{CascadeShadowConfigBuilder, DirectionalLightShadowMap};

pub fn setup_sun(
    mut commands: Commands,
    mut ambient: ResMut<GlobalAmbientLight>,
) {
    ambient.color = Color::srgb(0.85, 0.88, 1.0);
    ambient.brightness = 1000.0;

    commands.insert_resource(DirectionalLightShadowMap { size: 4096 });

    commands.spawn((
        DirectionalLight {
            color: Color::srgb(1.0, 0.96, 0.85),
            illuminance: 48000.0,
            shadow_maps_enabled: true,
            contact_shadows_enabled: true,
            shadow_depth_bias: 0.1,
            shadow_normal_bias: 1.2,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -0.32, 0.95, 0.0)),
        CascadeShadowConfigBuilder {
            num_cascades: 4,
            minimum_distance: 0.5,
            maximum_distance: 300.0,
            first_cascade_far_bound: 15.0,
            overlap_proportion: 0.2,
        }.build(),
    ));
}