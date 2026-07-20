use bevy::light::{CascadeShadowConfigBuilder, DirectionalLightShadowMap};
use bevy::prelude::*;

fn studio_cascade_config() -> bevy::light::CascadeShadowConfig {
    CascadeShadowConfigBuilder {
        num_cascades: 4,
        minimum_distance: 0.5,
        maximum_distance: 250.0,
        first_cascade_far_bound: 12.0,
        overlap_proportion: 0.3,
    }
    .build()
}

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
            shadow_normal_bias: DirectionalLight::DEFAULT_SHADOW_NORMAL_BIAS,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -1.2, 0.3, 0.0)),
        studio_cascade_config(),
    ));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uses_four_overlapping_cascades_with_tight_near_coverage() {
        let config = studio_cascade_config();

        assert_eq!(config.bounds.len(), 4);
        assert_eq!(config.bounds[0], 12.0);
        assert!((config.bounds[3] - 250.0).abs() < 0.001);
        assert_eq!(config.overlap_proportion, 0.3);
    }
}
