pub mod components;
pub mod studs;
pub mod data;

use bevy::prelude::*;
use bevy::pbr::{ExtendedMaterial, MaterialPlugin};

pub struct BricksPlugin;

impl Plugin for BricksPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(MaterialPlugin::<ExtendedMaterial<StandardMaterial, studs::StudsExtension>>::default())
            .init_resource::<data::BrickSpawnerCount>()
            .register_type::<components::BrickPhysics>()
            .add_systems(Startup, studs::setup_studs)
            .add_systems(Update, studs::configure_studs_samplers);
    }
}