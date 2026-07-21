use crate::common::core::vrtx::VrtxFileState;
use crate::common::game::bricks::components::{
    Brick, BrickColor, BrickPhysics, BrickShape, BrickShapeComponent,
};
use crate::common::net::components::NetworkTransform;
use crate::server::ServerSettings;
use avian3d::prelude::*;
use bevy::prelude::*;
use lightyear::prelude::Replicate;

fn validate_hierarchy(state: &VrtxFileState) -> Result<(), &'static str> {
    if state.version < 6 {
        return Ok(());
    }
    let parents: std::collections::HashMap<_, _> = state
        .bricks
        .iter()
        .map(|item| (item.node_id, item.parent_node_id))
        .chain(
            state
                .scripts
                .iter()
                .map(|item| (item.node_id, item.parent_node_id)),
        )
        .collect();
    if parents.len() != state.bricks.len() + state.scripts.len() {
        return Err("map contains duplicate node IDs");
    }
    for (&id, &parent) in &parents {
        if parent == Some(id) {
            return Err("map contains a self-parenting node");
        }
        if parent.is_some_and(|parent| !parents.contains_key(&parent)) {
            return Err("map contains a dangling parent ID");
        }
        let mut seen = std::collections::HashSet::new();
        let mut current = Some(id);
        while let Some(node) = current {
            if !seen.insert(node) {
                return Err("map hierarchy contains a cycle");
            }
            current = parents.get(&node).copied().flatten();
        }
    }
    Ok(())
}

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

    if let Some(state) = loaded_state.filter(|state| {
        validate_hierarchy(state)
            .inspect_err(|error| error!("Failed to load map hierarchy: {error}"))
            .is_ok()
    }) {
        let version = state.version;
        let mut node_id_to_entity = std::collections::HashMap::new();
        let mut pending_parents: Vec<(Entity, u64)> = Vec::new();
        let mut pending_parent_names: Vec<(Entity, String)> = Vec::new();
        let mut name_to_entity = std::collections::HashMap::new();
        let mut ambiguous_names = std::collections::HashSet::new();
        for brick in state.bricks {
            let node_id = brick.node_id;
            let parent_node_id = brick.parent_node_id;
            let name = brick.name.clone();
            let entity = spawn_brick_entity(&mut commands, brick);
            node_id_to_entity.insert(node_id, entity);
            if name_to_entity.insert(name.clone(), entity).is_some() {
                ambiguous_names.insert(name);
            }
            if let Some(parent_id) = parent_node_id {
                pending_parents.push((entity, parent_id));
            }
        }
        for script in state.scripts {
            let node_id = script.node_id;
            let parent_node_id = script.parent_node_id;
            let parent_name = script.parent_name.clone();
            let name = script.name.clone();
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
            node_id_to_entity.insert(node_id, new_script_entity);
            if name_to_entity
                .insert(name.clone(), new_script_entity)
                .is_some()
            {
                ambiguous_names.insert(name);
            }
            if let Some(parent_id) = parent_node_id {
                pending_parents.push((new_script_entity, parent_id));
            } else if version < 6
                && let Some(parent_name) = parent_name
            {
                pending_parent_names.push((new_script_entity, parent_name));
            }
        }
        for (child, parent_id) in pending_parents {
            if let Some(&parent) = node_id_to_entity.get(&parent_id) {
                commands.entity(parent).add_child(child);
            }
        }
        for (child, parent_name) in pending_parent_names {
            if !ambiguous_names.contains(&parent_name)
                && let Some(&parent) = name_to_entity.get(&parent_name)
            {
                commands.entity(parent).add_child(child);
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::core::vrtx::{VrtxBrick, VrtxScript, VrtxSettings};

    fn state(bricks: Vec<VrtxBrick>, scripts: Vec<VrtxScript>) -> VrtxFileState {
        VrtxFileState {
            version: 7,
            gravity: Vec3::ZERO,
            settings: VrtxSettings::default(),
            lighting: default(),
            camera_transform: Transform::IDENTITY,
            bricks,
            scripts,
        }
    }

    fn brick(id: u64, parent: Option<u64>) -> VrtxBrick {
        VrtxBrick {
            node_id: id,
            parent_node_id: parent,
            name: format!("brick-{id}"),
            transform: Transform::IDENTITY,
            shape: BrickShape::Block,
            color: Color::WHITE,
            physics_enabled: false,
            bounciness: 0.3,
            player_can_collide: true,
            friction: 0.3,
            gravity_scale: 1.0,
            mass: 1.0,
        }
    }

    #[test]
    fn rejects_duplicate_and_invalid_hierarchy_ids() {
        let duplicate = state(vec![brick(1, None), brick(1, None)], vec![]);
        assert_eq!(
            validate_hierarchy(&duplicate),
            Err("map contains duplicate node IDs")
        );

        let dangling = state(vec![brick(1, Some(2))], vec![]);
        assert_eq!(
            validate_hierarchy(&dangling),
            Err("map contains a dangling parent ID")
        );

        let cycle = state(vec![brick(1, Some(2)), brick(2, Some(1))], vec![]);
        assert_eq!(
            validate_hierarchy(&cycle),
            Err("map hierarchy contains a cycle")
        );

        let script = VrtxScript {
            node_id: 2,
            parent_node_id: Some(1),
            name: "nested".into(),
            script_type: 0,
            code: String::new(),
            parent_name: None,
            enabled: true,
        };
        assert!(validate_hierarchy(&state(vec![brick(1, None)], vec![script])).is_ok());
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
