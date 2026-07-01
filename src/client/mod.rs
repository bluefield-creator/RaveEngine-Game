pub mod player;

use bevy::prelude::*;
use bevy::pbr::ExtendedMaterial;
use avian3d::prelude::Physics;
use avian3d::schedule::PhysicsTime;
use crate::common::bricks::components::Brick;
use crate::common::bricks::studs::{setup_studs, StudsAssets, StudsExtension};
use crate::common::physics::PhysicsSimulationState;

#[derive(Component)]
struct ClientPhysicsInitializer;

pub struct ClientPlugin;

impl Plugin for ClientPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(player::PlayerPlugin)
            .add_systems(Startup, (
                setup_client_world.after(setup_studs),
                setup_physics_initializer,
            ))
            .add_systems(PreUpdate, initialize_client_physics);
    }
}

fn setup_physics_initializer(mut commands: Commands) {
    commands.spawn(ClientPhysicsInitializer);
}

fn initialize_client_physics(
    mut time_physics: ResMut<Time<Physics>>,
    mut state: ResMut<PhysicsSimulationState>,
    mut commands: Commands,
    query: Query<Entity, With<ClientPhysicsInitializer>>,
) {
    *state = PhysicsSimulationState::Running;
    time_physics.unpause();
    for entity in &query {
        commands.entity(entity).despawn();
    }
}

fn setup_client_world(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut studs_materials: ResMut<Assets<ExtendedMaterial<StandardMaterial, StudsExtension>>>,
    studs_assets: Res<StudsAssets>,
) {
    let baseplate_transform = Transform::from_xyz(0.0, -0.14, 0.0).with_scale(Vec3::new(25.0, 1.0, 50.0));
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(4.0 * 0.28, 1.0 * 0.28, 2.0 * 0.28))),
        MeshMaterial3d(studs_materials.add(ExtendedMaterial {
            base: StandardMaterial {
                base_color: Color::srgb(0.28, 0.62, 0.32),
                perceptual_roughness: 0.95,
                reflectance: 0.08,
                metallic: 0.0,
                ..default()
            },
            extension: StudsExtension {
                stud_texture: studs_assets.stud.clone(),
                inlet_texture: studs_assets.inlet.clone(),
            },
        })),
        baseplate_transform,
        Brick,
        Name::new("Baseplate"),
        avian3d::prelude::RigidBody::Static,
        avian3d::prelude::Collider::cuboid(4.0 * 0.28, 1.0 * 0.28, 2.0 * 0.28),
        avian3d::prelude::Friction::new(0.3),
        avian3d::prelude::Restitution::new(0.0),
        crate::common::physics::TransformBackup(baseplate_transform),
    ));
}