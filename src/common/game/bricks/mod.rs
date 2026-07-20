pub mod components;
pub mod studs;
pub mod data;

use bevy::prelude::*;
use bevy::pbr::{ExtendedMaterial, MaterialPlugin};

#[derive(Resource, Default)]
pub struct BrickMaterialCache {
    pub studs_materials: std::collections::HashMap<[u32; 4], Handle<ExtendedMaterial<StandardMaterial, studs::StudsExtension>>>,
    pub plain_materials: std::collections::HashMap<[u32; 4], Handle<StandardMaterial>>,
    pub block_mesh: Option<Handle<Mesh>>,
    pub sphere_mesh: Option<Handle<Mesh>>,
}

pub struct BricksPlugin;

impl Plugin for BricksPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<components::BrickPhysics>()
            .register_type::<components::Brick>()
            .register_type::<components::BrickShapeComponent>()
            .register_type::<components::BrickColor>()
            .init_resource::<data::BrickSpawnerCount>()
            .init_resource::<BrickMaterialCache>();

        if app.is_plugin_added::<bevy::render::RenderPlugin>() {
            app.add_plugins(MaterialPlugin::<ExtendedMaterial<StandardMaterial, studs::StudsExtension>>::default())
                .add_systems(Startup, studs::setup_studs)
                .add_systems(Update, (
                    studs::configure_studs_samplers,
                    update_brick_meshes_on_shape_change,
                ));
        }
    }
}

pub fn update_brick_meshes_on_shape_change(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut cache: ResMut<BrickMaterialCache>,
    query: Query<(Entity, &components::BrickShapeComponent), Changed<components::BrickShapeComponent>>,
) {
    for (entity, brick_shape_comp) in &query {
        match brick_shape_comp.shape {
            components::BrickShape::Block => {
                if cache.block_mesh.is_none() {
                    cache.block_mesh = Some(meshes.add(Cuboid::new(4.0 * 0.28, 1.0 * 0.28, 2.0 * 0.28)));
                }
                commands.entity(entity).insert(Mesh3d(cache.block_mesh.clone().unwrap()));
            }
            components::BrickShape::Sphere => {
                if cache.sphere_mesh.is_none() {
                    cache.sphere_mesh = Some(meshes.add(Sphere::new(1.0 * 0.28)));
                }
                commands.entity(entity).insert(Mesh3d(cache.sphere_mesh.clone().unwrap()));
            }
        }
    }
}

