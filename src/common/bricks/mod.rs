pub mod components;
pub mod studs;
pub mod data;

use bevy::prelude::*;
use bevy::pbr::{ExtendedMaterial, MaterialPlugin};

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct ColorKey(pub u32, pub u32, pub u32, pub u32);

impl ColorKey {
    pub fn from_color(color: Color) -> Self {
        let srgba = color.to_srgba();
        Self(
            srgba.red.to_bits(),
            srgba.green.to_bits(),
            srgba.blue.to_bits(),
            srgba.alpha.to_bits(),
        )
    }
}

#[derive(Resource, Default)]
pub struct BrickMaterialCache {
    pub studs_materials: std::collections::HashMap<ColorKey, Handle<ExtendedMaterial<StandardMaterial, studs::StudsExtension>>>,
    pub plain_materials: std::collections::HashMap<ColorKey, Handle<StandardMaterial>>,
    pub block_mesh: Option<Handle<Mesh>>,
    pub sphere_mesh: Option<Handle<Mesh>>,
}

impl BrickMaterialCache {
    pub fn get_block_mesh(&mut self, meshes: &mut Assets<Mesh>) -> Handle<Mesh> {
        self.block_mesh.get_or_insert_with(|| {
            meshes.add(Cuboid::new(4.0 * 0.28, 1.0 * 0.28, 2.0 * 0.28))
        }).clone()
    }

    pub fn get_sphere_mesh(&mut self, meshes: &mut Assets<Mesh>) -> Handle<Mesh> {
        self.sphere_mesh.get_or_insert_with(|| {
            meshes.add(Sphere::new(1.0 * 0.28))
        }).clone()
    }

    pub fn get_studs_material(
        &mut self,
        color: Color,
        studs_assets: &studs::StudsAssets,
        studs_materials: &mut Assets<ExtendedMaterial<StandardMaterial, studs::StudsExtension>>,
    ) -> Handle<ExtendedMaterial<StandardMaterial, studs::StudsExtension>> {
        let key = ColorKey::from_color(color);
        self.studs_materials.entry(key).or_insert_with(|| {
            studs_materials.add(ExtendedMaterial {
                base: StandardMaterial {
                    base_color: color,
                    perceptual_roughness: 0.9,
                    ..default()
                },
                extension: studs::StudsExtension {
                    stud_texture: studs_assets.stud.clone(),
                    inlet_texture: studs_assets.inlet.clone(),
                },
            })
        }).clone()
    }

    pub fn get_plain_material(
        &mut self,
        color: Color,
        materials: &mut Assets<StandardMaterial>,
    ) -> Handle<StandardMaterial> {
        let key = ColorKey::from_color(color);
        self.plain_materials.entry(key).or_insert_with(|| {
            materials.add(StandardMaterial {
                base_color: color,
                perceptual_roughness: 0.9,
                ..default()
            })
        }).clone()
    }
}

#[derive(Resource)]
pub struct StaticMeshCombiner {
    pub combined_entities: Vec<Entity>,
    pub baked_bricks: std::collections::HashSet<Entity>,
    pub dirty: bool,
}

impl Default for StaticMeshCombiner {
    fn default() -> Self {
        Self {
            combined_entities: Vec::new(),
            baked_bricks: std::collections::HashSet::new(),
            dirty: true,
        }
    }
}

pub struct BricksPlugin;

impl Plugin for BricksPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<components::BrickPhysics>()
            .register_type::<components::Brick>()
            .register_type::<components::BrickShapeComponent>()
            .register_type::<components::BrickColor>()
            .init_resource::<data::BrickSpawnerCount>()
            .init_resource::<BrickMaterialCache>()
            .init_resource::<StaticMeshCombiner>();

        if app.is_plugin_added::<bevy::render::RenderPlugin>() {
            app.add_plugins(MaterialPlugin::<ExtendedMaterial<StandardMaterial, studs::StudsExtension>>::default())
                .add_systems(Startup, studs::setup_studs)
                .add_systems(Update, (
                    studs::configure_studs_samplers,
                    optimize_bricks_system,
                    optimize_brick_visibility,
                    detect_static_brick_changes,
                    rebuild_static_mesh_combinations,
                    links_optimizer_system,
                ));
        }
    }
}

pub fn optimize_bricks_system(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut studs_materials: ResMut<Assets<ExtendedMaterial<StandardMaterial, studs::StudsExtension>>>,
    studs_assets: Option<Res<studs::StudsAssets>>,
    mut cache: ResMut<BrickMaterialCache>,
    query: Query<
        (
            Entity,
            &components::BrickShapeComponent,
            &components::BrickColor,
            &components::BrickPhysics,
            Option<&Mesh3d>,
            Option<&MeshMaterial3d<StandardMaterial>>,
            Option<&MeshMaterial3d<ExtendedMaterial<StandardMaterial, studs::StudsExtension>>>,
        ),
        Or<(Changed<components::BrickShapeComponent>, Changed<components::BrickColor>)>,
    >,
) {
    let Some(studs_assets) = studs_assets else { return; };
    for (entity, shape_comp, brick_color, physics, mesh_opt, plain_opt, studs_opt) in &query {
        if !physics.enabled {
            continue;
        }

        let expected_mesh = match shape_comp.shape {
            components::BrickShape::Block => cache.get_block_mesh(&mut meshes),
            components::BrickShape::Sphere => cache.get_sphere_mesh(&mut meshes),
        };

        let mut needs_mesh_update = true;
        if let Some(mesh) = mesh_opt {
            if mesh.0 == expected_mesh {
                needs_mesh_update = false;
            }
        }
        if needs_mesh_update {
            commands.entity(entity).insert(Mesh3d(expected_mesh));
        }

        if studs_opt.is_some() {
            let expected_mat = cache.get_studs_material(brick_color.color, &studs_assets, &mut studs_materials);
            commands.entity(entity).insert(MeshMaterial3d(expected_mat));
        } else if plain_opt.is_some() {
            let expected_mat = cache.get_plain_material(brick_color.color, &mut materials);
            commands.entity(entity).insert(MeshMaterial3d(expected_mat));
        } else {
            let expected_mat = cache.get_studs_material(brick_color.color, &studs_assets, &mut studs_materials);
            commands.entity(entity).insert(MeshMaterial3d(expected_mat));
        }
    }
}

fn links_optimizer_system() {}

pub fn optimize_brick_visibility(
    mut commands: Commands,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut studs_materials: ResMut<Assets<ExtendedMaterial<StandardMaterial, crate::common::bricks::studs::StudsExtension>>>,
    studs_assets: Option<Res<crate::common::bricks::studs::StudsAssets>>,
    mut cache: ResMut<BrickMaterialCache>,
    camera_query: Query<&Transform, With<Camera3d>>,
    mut bricks_query: Query<(
        Entity,
        &GlobalTransform,
        &components::BrickColor,
        Option<&MeshMaterial3d<StandardMaterial>>,
        Option<&MeshMaterial3d<ExtendedMaterial<StandardMaterial, crate::common::bricks::studs::StudsExtension>>>,
    ), With<components::Brick>>,
) {


    let Some(studs_assets) = studs_assets else { return; };
    let Some(camera_transform) = camera_query.iter().next() else { return; };
    let camera_pos = camera_transform.translation;

    for (entity, global_transform, brick_color, plain_opt, studs_opt) in &mut bricks_query {
        let brick_pos = global_transform.translation();
        let dist_sq = camera_pos.distance_squared(brick_pos);
        let is_far = dist_sq > 50.0 * 50.0;

        if is_far {
            if studs_opt.is_some() {
                let plain_mat = cache.get_plain_material(brick_color.color, &mut materials);
                commands.entity(entity)
                    .remove::<MeshMaterial3d<ExtendedMaterial<StandardMaterial, crate::common::bricks::studs::StudsExtension>>>()
                    .insert(MeshMaterial3d(plain_mat));
            }
            
        } else {
            if plain_opt.is_some() || (plain_opt.is_none() && studs_opt.is_none()) {
                let studs_mat = cache.get_studs_material(brick_color.color, &studs_assets, &mut studs_materials);
                commands.entity(entity)
                    .remove::<MeshMaterial3d<StandardMaterial>>()
                    .insert(MeshMaterial3d(studs_mat));
            }
        }
    }
}

pub fn detect_static_brick_changes(
    mut combiner: ResMut<StaticMeshCombiner>,
    added_bricks: Query<Entity, Added<components::Brick>>,
    changed_physics: Query<Entity, (With<components::Brick>, Changed<components::BrickPhysics>)>,
    changed_color: Query<Entity, (With<components::Brick>, Changed<components::BrickColor>)>,
    changed_shape: Query<Entity, (With<components::Brick>, Changed<components::BrickShapeComponent>)>,
    changed_transform: Query<Entity, (With<components::Brick>, Changed<Transform>)>,
    removed_bricks: RemovedComponents<components::Brick>,
) {
    let mut any_change = false;
    if !added_bricks.is_empty() { any_change = true; }
    if !changed_physics.is_empty() { any_change = true; }
    if !changed_color.is_empty() { any_change = true; }
    if !changed_shape.is_empty() { any_change = true; }
    if !changed_transform.is_empty() { any_change = true; }
    if !removed_bricks.is_empty() { any_change = true; }

    if any_change {
        combiner.dirty = true;
    }
}

pub fn rebuild_static_mesh_combinations(
    mut commands: Commands,
    mut combiner: ResMut<StaticMeshCombiner>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut studs_materials: ResMut<Assets<ExtendedMaterial<StandardMaterial, studs::StudsExtension>>>,
    studs_assets: Option<Res<studs::StudsAssets>>,
    mut cache: ResMut<BrickMaterialCache>,
    bricks_query: Query<(
        Entity,
        &GlobalTransform,
        &components::BrickShapeComponent,
        &components::BrickColor,
        &components::BrickPhysics,
    ), With<components::Brick>>,
) {
    if !combiner.dirty {
        return;
    }
    combiner.dirty = false;

    let Some(studs_assets) = studs_assets else { return; };

    for entity in &combiner.combined_entities {
        commands.entity(*entity).despawn();
    }
    combiner.combined_entities.clear();

    let mut static_bricks = Vec::new();
    for (entity, global_transform, shape_comp, brick_color, physics) in &bricks_query {
        if !physics.enabled {
            static_bricks.push((entity, global_transform, shape_comp.shape, brick_color.color));
        }
    }

    if static_bricks.is_empty() {
        combiner.baked_bricks.clear();
        return;
    }

    let current_static_set: std::collections::HashSet<Entity> = static_bricks.iter().map(|(e, _, _, _)| *e).collect();
    for old_baked in &combiner.baked_bricks {
        if !current_static_set.contains(old_baked) {
            if let Ok((_, _, _, brick_color, _)) = bricks_query.get(*old_baked) {
                let studs_mat = cache.get_studs_material(brick_color.color, &studs_assets, &mut studs_materials);
                commands.entity(*old_baked).insert(MeshMaterial3d(studs_mat));
            }
        }
    }

    combiner.baked_bricks = current_static_set;

    let mut groups: std::collections::HashMap<(components::BrickShape, ColorKey), Vec<(Entity, &GlobalTransform)>> = std::collections::HashMap::new();
    for &(entity, global_transform, shape, color) in &static_bricks {
        let key = (shape, ColorKey::from_color(color));
        groups.entry(key).or_default().push((entity, global_transform));
    }

    for ((shape, color_key), bricks) in groups {
        let template_mesh_handle = match shape {
            components::BrickShape::Block => cache.get_block_mesh(&mut meshes),
            components::BrickShape::Sphere => cache.get_sphere_mesh(&mut meshes),
        };

        let Some(template_mesh) = meshes.get(&template_mesh_handle) else {
            continue;
        };

        let Some(bevy::render::mesh::VertexAttributeValues::Float32x3(template_positions)) = template_mesh.attribute(Mesh::ATTRIBUTE_POSITION) else {
            continue;
        };
        let template_normals = match template_mesh.attribute(Mesh::ATTRIBUTE_NORMAL) {
            Some(bevy::render::mesh::VertexAttributeValues::Float32x3(norms)) => Some(norms),
            _ => None,
        };
        let template_uvs = match template_mesh.attribute(Mesh::ATTRIBUTE_UV_0) {
            Some(bevy::render::mesh::VertexAttributeValues::Float32x2(uvs)) => Some(uvs),
            _ => None,
        };
        let template_indices = template_mesh.indices();

        let mut combined_positions: Vec<[f32; 3]> = Vec::new();
        let mut combined_normals: Vec<[f32; 3]> = Vec::new();
        let mut combined_uvs: Vec<[f32; 2]> = Vec::new();
        let mut combined_indices_vec = Vec::new();

        for &(brick_entity, global_transform) in &bricks {
            let affine = global_transform.to_matrix();
            let (_, rot, _) = global_transform.to_scale_rotation_translation();

            commands.entity(brick_entity).remove::<MeshMaterial3d<ExtendedMaterial<StandardMaterial, studs::StudsExtension>>>();
            commands.entity(brick_entity).remove::<MeshMaterial3d<StandardMaterial>>();

            let offset = combined_positions.len() as u32;

            for pos in template_positions {
                let transformed_pos = affine.transform_point3(Vec3::from(*pos));
                combined_positions.push(transformed_pos.into());
            }

            if let Some(normals) = template_normals {
                for norm in normals {
                    let transformed_norm = rot * Vec3::from(*norm);
                    combined_normals.push(transformed_norm.into());
                }
            }

            if let Some(uvs) = template_uvs {
                combined_uvs.extend(uvs);
            }

            if let Some(indices) = template_indices {
                match indices {
                    bevy::render::mesh::Indices::U16(idx_vec) => {
                        for idx in idx_vec {
                            combined_indices_vec.push(*idx as u32 + offset);
                        }
                    }
                    bevy::render::mesh::Indices::U32(idx_vec) => {
                        for idx in idx_vec {
                            combined_indices_vec.push(*idx + offset);
                        }
                    }
                }
            }
        }

        let mut combined_mesh = Mesh::new(
            bevy::render::render_resource::PrimitiveTopology::TriangleList,
            bevy::asset::RenderAssetUsages::default(),
        );

        combined_mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, combined_positions);
        if !combined_normals.is_empty() {
            combined_mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, combined_normals);
        }
        if !combined_uvs.is_empty() {
            combined_mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, combined_uvs);
        }
        combined_mesh.insert_indices(bevy::render::mesh::Indices::U32(combined_indices_vec));

        let combined_mesh_handle = meshes.add(combined_mesh);

        let r = f32::from_bits(color_key.0);
        let g = f32::from_bits(color_key.1);
        let b = f32::from_bits(color_key.2);
        let a = f32::from_bits(color_key.3);
        let color = Color::Srgba(Srgba::new(r, g, b, a));

        let studs_mat_handle = cache.get_studs_material(color, &studs_assets, &mut studs_materials);

        let combined_entity = commands.spawn((
            Mesh3d(combined_mesh_handle),
            MeshMaterial3d(studs_mat_handle),
            Transform::IDENTITY,
            GlobalTransform::IDENTITY,
            Visibility::Inherited,
            Name::new(format!("CombinedStaticBricks_{:?}_{:?}", shape, color_key)),
        )).id();

        combiner.combined_entities.push(combined_entity);
    }
}