use crate::common::core::vrtx::VrtxFileState;
use crate::common::game::bricks::components::{
    Brick, BrickColor, BrickPhysics, BrickShape, BrickShapeComponent,
};
use crate::common::net::components::NetworkTransform;
use crate::server::ServerSettings;
use avian3d::prelude::*;
use bevy::prelude::*;
use lightyear::prelude::Replicate;

pub fn load_fallback_map(commands: &mut Commands) {
    commands.spawn((
        Transform::from_xyz(0.0, -0.14, 0.0).with_scale(Vec3::new(25.0, 1.0, 50.0)),
        Name::new("Baseplate"),
        Brick,
        BrickShapeComponent {
            shape: BrickShape::Block,
        },
        BrickPhysics {
            enabled: false,
            locked: true,
            bounciness: 0.3,
            player_can_collide: true,
            friction: 0.3,
            gravity_scale: 1.0,
            mass: 1.0,
        },
        BrickColor {
            color: Color::srgb(0.28, 0.62, 0.32),
        },
        NetworkTransform {
            translation: Vec3::new(0.0, -0.14, 0.0),
            rotation: Quat::IDENTITY,
            scale: Vec3::new(25.0, 1.0, 50.0),
        },
        RigidBody::Static,
        Collider::cuboid(4.0 * 0.28, 1.0 * 0.28, 2.0 * 0.28),
        CollisionLayers::from_bits(0b0001, 0xFFFF_FFFF),
        Friction::new(0.3),
        Restitution::new(0.3),
        Replicate::default(),
    ));

    commands.spawn((
        Transform::from_xyz(0.0, 0.14, 0.0),
        Name::new("Part0"),
        Brick,
        BrickShapeComponent {
            shape: BrickShape::Block,
        },
        BrickPhysics {
            enabled: true,
            ..default()
        },
        BrickColor {
            color: Color::srgb(0.84, 0.24, 0.16),
        },
        NetworkTransform {
            translation: Vec3::new(0.0, 0.14, 0.0),
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE,
        },
        RigidBody::Dynamic,
        Collider::cuboid(4.0 * 0.28, 1.0 * 0.28, 2.0 * 0.28),
        CollisionLayers::from_bits(0b0001, 0xFFFF_FFFF),
        Friction::new(0.3),
        Restitution::new(0.3),
        SleepingDisabled,
        Replicate::default(),
    ));
}

pub fn load_map(mut commands: Commands, settings: Res<ServerSettings>) {
    let mut loaded = false;
    info!("Loading map: {}", settings.map_path);

    let loaded_state = VrtxFileState::load_from_file(&settings.map_path).ok();

    if let Some(state) = loaded_state {
        let mut node_id_to_entity = std::collections::HashMap::new();
        let mut pending_parents: Vec<(Entity, u64)> = Vec::new();
        let mut name_to_entity = std::collections::HashMap::new();
        for brick in state.bricks {
            let node_id = brick.node_id;
            let parent_node_id = brick.parent_node_id;
            let name = brick.name.clone();
            let entity = spawn_brick_entity(&mut commands, brick);
            node_id_to_entity.insert(node_id, entity);
            name_to_entity.insert(name, entity);
            if let Some(parent_id) = parent_node_id {
                pending_parents.push((entity, parent_id));
            }
        }
        for (child, parent_id) in pending_parents {
            if let Some(&parent) = node_id_to_entity.get(&parent_id) {
                commands.entity(parent).add_child(child);
            }
        }
        for script in state.scripts {
            let mut cmd = commands.spawn(Name::new(script.name));
            match script.script_type {
                0 => {
                    cmd.insert(crate::scripting::ecs::ServerScript {
                        code: script.code,
                        enabled: script.enabled,
                        started: false,
                    });
                }
                1 => {
                    cmd.insert((
                        crate::scripting::ecs::LocalScript {
                            code: script.code,
                            enabled: script.enabled,
                            started: false,
                        },
                        lightyear::prelude::Replicate::default(),
                    ));
                }
                _ => {
                    cmd.insert((
                        crate::scripting::ecs::ModuleScript { code: script.code },
                        lightyear::prelude::Replicate::default(),
                    ));
                }
            }
            let new_script_entity = cmd.id();
            let mut parented = false;
            if let Some(parent_id) = script.parent_node_id
                && let Some(&parent_entity) = node_id_to_entity.get(&parent_id)
            {
                commands.entity(parent_entity).add_child(new_script_entity);
                parented = true;
            }
            if !parented
                && let Some(ref p_name) = script.parent_name
                && let Some(&parent_entity) = name_to_entity.get(p_name)
            {
                commands.entity(parent_entity).add_child(new_script_entity);
            }
        }
        loaded = true;
        info!("Map loaded successfully");
    }

    if !loaded {
        info!("Failed to load map, spawning fallback map instead");
        load_fallback_map(&mut commands);
    }
}

pub fn spawn_brick_entity(
    commands: &mut Commands,
    brick: crate::common::core::vrtx::VrtxBrick,
) -> Entity {
    let collider = match brick.shape {
        BrickShape::Block => Collider::cuboid(4.0 * 0.28, 1.0 * 0.28, 2.0 * 0.28),
        BrickShape::Sphere => Collider::sphere(1.0 * 0.28),
    };

    let body_type = if brick.physics_enabled {
        RigidBody::Dynamic
    } else {
        RigidBody::Static
    };

    let layers = if brick.player_can_collide {
        CollisionLayers::from_bits(0b0001, 0xFFFF_FFFF)
    } else {
        CollisionLayers::from_bits(0b0100, 0xFFFF_FFFD)
    };

    commands
        .spawn((
            brick.transform,
            Name::new(brick.name.clone()),
            Brick,
            BrickShapeComponent { shape: brick.shape },
            BrickPhysics {
                enabled: brick.physics_enabled,
                locked: false,
                bounciness: brick.bounciness,
                player_can_collide: brick.player_can_collide,
                friction: brick.friction,
                gravity_scale: brick.gravity_scale,
                mass: brick.mass,
            },
            BrickColor { color: brick.color },
            NetworkTransform {
                translation: brick.transform.translation,
                rotation: brick.transform.rotation,
                scale: brick.transform.scale,
            },
            body_type,
            collider,
            layers,
        ))
        .insert((
            Friction::new(brick.friction),
            Restitution::new(brick.bounciness),
            GravityScale(brick.gravity_scale),
            Mass(brick.mass),
            SleepingDisabled,
            Replicate::default(),
        ))
        .id()
}
