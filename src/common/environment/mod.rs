pub mod sky;
pub mod clouds;
pub mod sun;
pub mod horizon;

use bevy::prelude::*;

pub struct EnvironmentPlugin;

impl Plugin for EnvironmentPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, (
            sky::setup_sky,
            clouds::setup_clouds,
            sun::setup_sun,
            horizon::setup_horizon,
        ))
        .add_systems(Update, (
            sky::sync_sky_dome,
            clouds::animate_and_wrap_clouds,
        ));
    }
}