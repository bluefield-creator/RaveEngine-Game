use bevy::light::{CascadeShadowConfigBuilder, DirectionalLightShadowMap};
use bevy::prelude::*;

pub fn setup_sun(mut commands: Commands, mut ambient: ResMut<GlobalAmbientLight>) {
    ambient.color = Color::srgb(0.55, 0.75, 0.95);
    ambient.brightness = 320.0;

    commands.insert_resource(DirectionalLightShadowMap { size: 4096 });

    commands.spawn((
        DirectionalLight {
            color: Color::srgb(1.0, 0.98, 0.92),
            illuminance: 85000.0,
            shadow_maps_enabled: true,
            contact_shadows_enabled: true,
            shadow_depth_bias: 0.02,
            shadow_normal_bias: 0.6,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -1.2, 0.3, 0.0)),
        CascadeShadowConfigBuilder {
            num_cascades: 4,
            minimum_distance: 0.5,
            maximum_distance: 600.0,
            first_cascade_far_bound: 30.0,
            overlap_proportion: 0.2,
        }
        .build(),
    ));
}
