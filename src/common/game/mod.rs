pub mod bricks;
pub mod environment;
pub mod physics;

use bevy::prelude::*;

pub struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(bricks::BricksPlugin)
           .add_plugins(environment::EnvironmentPlugin)
           .add_plugins(physics::PhysicsSimulationPlugin);
    }
}