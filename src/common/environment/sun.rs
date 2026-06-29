use bevy::prelude::*;
use bevy::light::CascadeShadowConfigBuilder;

pub fn setup_sun(
    mut commands: Commands,
    mut ambient: ResMut<GlobalAmbientLight>,
) {
    ambient.color = Color::srgb(0.85, 0.88, 1.0);
    ambient.brightness = 350.0;

    commands.spawn((
        DirectionalLight {
            color: Color::srgb(1.0, 0.96, 0.85),
            illuminance: 16000.0,
            shadow_maps_enabled: true,
            contact_shadows_enabled: true,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -0.32, 0.95, 0.0)),
        CascadeShadowConfigBuilder {
            num_cascades: 4,
            minimum_distance: 1.0,
            maximum_distance: 120.0,
            first_cascade_far_bound: 10.0,
            overlap_proportion: 0.2,
        }.build(),
    ));
}