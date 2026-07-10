pub mod vrtx;
pub mod performance;

use bevy::prelude::*;

pub struct CorePlugin;

impl Plugin for CorePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(performance::PerformancePlugin);
    }
}