pub mod clouds;
pub mod horizon;
pub mod lighting;
pub mod sky;
pub mod sun;

use bevy::prelude::*;

pub struct EnvironmentPlugin;

impl Plugin for EnvironmentPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<lighting::LightingService>();

        if app.is_plugin_added::<bevy::render::RenderPlugin>() {
            app.add_systems(
                Startup,
                (
                    sky::setup_sky,
                    clouds::setup_clouds,
                    sun::setup_sun,
                    horizon::setup_horizon,
                ),
            )
            .add_systems(
                Update,
                (
                    sky::sync_sky_dome,
                    clouds::animate_and_wrap_clouds,
                    lighting::sync_lighting_service,
                    lighting::update_lighting_system,
                ),
            );
        }
    }
}
