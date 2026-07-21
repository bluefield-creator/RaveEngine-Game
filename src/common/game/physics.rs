use avian3d::prelude::*;
use bevy::prelude::*;

#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PhysicsSimulationState {
    #[default]
    Stopped,
    Running,
}

#[derive(Message, Clone, Copy, Debug)]
pub enum PhysicsSimulationAction {
    Play,
    Stop,
    Replay,
}

#[derive(Component, Clone, Copy, Debug)]
pub struct TransformBackup(pub Transform);

pub struct PhysicsSimulationPlugin;

impl Plugin for PhysicsSimulationPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(PhysicsPlugins::default())
            .insert_resource(Gravity(Vec3::new(0.0, -186.9 * 0.28, 0.0)))
            .init_resource::<PhysicsSimulationState>()
            .add_message::<PhysicsSimulationAction>()
            .add_systems(Startup, setup_physics)
            .add_systems(
                Update,
                (
                    handle_physics_simulation_actions,
                    handle_newly_spawned_bricks,
                    sync_brick_physics_changes,
                ),
            );
    }
}

fn setup_physics(
    mut time_physics: ResMut<Time<Physics>>,
    mut state: ResMut<PhysicsSimulationState>,
    server_settings: Option<Res<crate::server::ServerSettings>>,
) {
    if server_settings.is_none() {
        time_physics.pause();
    } else {
        *state = PhysicsSimulationState::Running;
        time_physics.unpause();
    }
}

fn handle_physics_simulation_actions(
    mut actions: MessageReader<PhysicsSimulationAction>,
    mut state: ResMut<PhysicsSimulationState>,
    mut time_physics: ResMut<Time<Physics>>,
    mut commands: Commands,
    mut bricks_query: Query<
        (
            Entity,
            &mut Transform,
            &Name,
            Option<&crate::common::game::bricks::components::BrickShapeComponent>,
            Option<&crate::common::game::bricks::components::BrickPhysics>,
            Option<&TransformBackup>,
        ),
        With<crate::common::game::bricks::components::Brick>,
    >,
) {
    for action in actions.read() {
        match *action {
            PhysicsSimulationAction::Play => {
                if *state == PhysicsSimulationState::Stopped {
                    *state = PhysicsSimulationState::Running;
                    time_physics.unpause();

                    for (entity, transform, _name, shape_opt, phys_opt, backup) in &bricks_query {
                        if backup.is_none() {
                            commands.entity(entity).insert(TransformBackup(*transform));
                        }

                        let (
                            enabled,
                            bounciness,
                            player_can_collide,
                            friction,
                            gravity_scale,
                            mass,
                        ) = if let Some(phys) = phys_opt {
                            (
                                phys.enabled,
                                phys.bounciness,
                                phys.player_can_collide,
                                phys.friction,
                                phys.gravity_scale,
                                phys.mass,
                            )
                        } else {
                            (true, 0.3, true, 0.3, 1.0, 1.0)
                        };

                        let shape = shape_opt
                            .map(|s| s.shape)
                            .unwrap_or(crate::common::game::bricks::components::BrickShape::Block);
                        let collider = match shape {
                            crate::common::game::bricks::components::BrickShape::Block => {
                                Collider::cuboid(4.0 * 0.28, 1.0 * 0.28, 2.0 * 0.28)
                            }
                            crate::common::game::bricks::components::BrickShape::Sphere => {
                                Collider::sphere(1.0 * 0.28)
                            }
                        };

                        let layers = if player_can_collide {
                            CollisionLayers::from_bits(0b0001, 0xFFFF_FFFF)
                        } else {
                            CollisionLayers::from_bits(0b0100, 0xFFFF_FFFD)
                        };

                        if enabled {
                            commands.entity(entity).insert((
                                RigidBody::Dynamic,
                                collider,
                                Friction::new(friction),
                                Restitution::new(bounciness),
                                GravityScale(gravity_scale),
                                Mass(mass),
                                layers,
                                SleepingDisabled,
                            ));
                        } else {
                            commands.entity(entity).insert((
                                RigidBody::Static,
                                collider,
                                Friction::new(friction),
                                Restitution::new(0.0),
                                layers,
                            ));
                        }
                    }
                }
            }
            PhysicsSimulationAction::Stop => {
                if *state == PhysicsSimulationState::Running {
                    *state = PhysicsSimulationState::Stopped;
                    time_physics.pause();

                    for (entity, mut transform, _, _, _, backup) in &mut bricks_query {
                        if let Some(backup_val) = backup {
                            *transform = backup_val.0;
                            commands.entity(entity).remove::<TransformBackup>();
                        }
                        commands.entity(entity).remove::<(
                            RigidBody,
                            Collider,
                            Friction,
                            Restitution,
                            Mass,
                            LinearVelocity,
                            AngularVelocity,
                            GravityScale,
                            CollisionLayers,
                            SleepingDisabled,
                        )>();
                    }
                }
            }
            PhysicsSimulationAction::Replay => {
                if *state == PhysicsSimulationState::Running {
                    for (entity, mut transform, _, _, _, backup) in &mut bricks_query {
                        if let Some(backup_val) = backup {
                            *transform = backup_val.0;
                        }
                        commands.entity(entity).remove::<(
                            RigidBody,
                            Collider,
                            Friction,
                            Restitution,
                            Mass,
                            LinearVelocity,
                            AngularVelocity,
                            GravityScale,
                            CollisionLayers,
                            SleepingDisabled,
                        )>();
                    }
                } else {
                    *state = PhysicsSimulationState::Running;
                    time_physics.unpause();
                }

                for (entity, transform, _name, shape_opt, phys_opt, backup) in &bricks_query {
                    if backup.is_none() {
                        commands.entity(entity).insert(TransformBackup(*transform));
                    }

                    let (enabled, bounciness, player_can_collide, friction, gravity_scale, mass) =
                        if let Some(phys) = phys_opt {
                            (
                                phys.enabled,
                                phys.bounciness,
                                phys.player_can_collide,
                                phys.friction,
                                phys.gravity_scale,
                                phys.mass,
                            )
                        } else {
                            (true, 0.3, true, 0.3, 1.0, 1.0)
                        };

                    let shape = shape_opt
                        .map(|s| s.shape)
                        .unwrap_or(crate::common::game::bricks::components::BrickShape::Block);
                    let collider = match shape {
                        crate::common::game::bricks::components::BrickShape::Block => {
                            Collider::cuboid(4.0 * 0.28, 1.0 * 0.28, 2.0 * 0.28)
                        }
                        crate::common::game::bricks::components::BrickShape::Sphere => {
                            Collider::sphere(1.0 * 0.28)
                        }
                    };

                    let layers = if player_can_collide {
                        CollisionLayers::from_bits(0b0001, 0xFFFF_FFFF)
                    } else {
                        CollisionLayers::from_bits(0b0100, 0xFFFF_FFFD)
                    };

                    if enabled {
                        commands.entity(entity).insert((
                            RigidBody::Dynamic,
                            collider,
                            Friction::new(friction),
                            Restitution::new(bounciness),
                            GravityScale(gravity_scale),
                            Mass(mass),
                            layers,
                            SleepingDisabled,
                        ));
                    } else {
                        commands.entity(entity).insert((
                            RigidBody::Static,
                            collider,
                            Friction::new(friction),
                            Restitution::new(0.0),
                            layers,
                        ));
                    }
                }
            }
        }
    }
}

fn handle_newly_spawned_bricks(
    mut commands: Commands,
    state: Res<PhysicsSimulationState>,
    query: Query<
        (
            Entity,
            &Transform,
            &Name,
            Option<&crate::common::game::bricks::components::BrickShapeComponent>,
            Option<&crate::common::game::bricks::components::BrickPhysics>,
        ),
        (
            With<crate::common::game::bricks::components::Brick>,
            Without<TransformBackup>,
        ),
    >,
) {
    if *state == PhysicsSimulationState::Running {
        for (entity, transform, _name, shape_opt, phys_opt) in &query {
            commands.entity(entity).insert(TransformBackup(*transform));

            let (enabled, bounciness, player_can_collide, friction, gravity_scale, mass) =
                if let Some(phys) = phys_opt {
                    (
                        phys.enabled,
                        phys.bounciness,
                        phys.player_can_collide,
                        phys.friction,
                        phys.gravity_scale,
                        phys.mass,
                    )
                } else {
                    (true, 0.3, true, 0.3, 1.0, 1.0)
                };

            let shape = shape_opt
                .map(|s| s.shape)
                .unwrap_or(crate::common::game::bricks::components::BrickShape::Block);
            let collider = match shape {
                crate::common::game::bricks::components::BrickShape::Block => {
                    Collider::cuboid(4.0 * 0.28, 1.0 * 0.28, 2.0 * 0.28)
                }
                crate::common::game::bricks::components::BrickShape::Sphere => {
                    Collider::sphere(1.0 * 0.28)
                }
            };

            let layers = if player_can_collide {
                CollisionLayers::from_bits(0b0001, 0xFFFF_FFFF)
            } else {
                CollisionLayers::from_bits(0b0100, 0xFFFF_FFFD)
            };

            if enabled {
                commands.entity(entity).insert((
                    RigidBody::Dynamic,
                    collider,
                    Friction::new(friction),
                    Restitution::new(bounciness),
                    GravityScale(gravity_scale),
                    Mass(mass),
                    layers,
                    SleepingDisabled,
                ));
            } else {
                commands.entity(entity).insert((
                    RigidBody::Static,
                    collider,
                    Friction::new(friction),
                    Restitution::new(0.0),
                    layers,
                ));
            }
        }
    }
}

pub fn sync_brick_physics_changes(
    mut commands: Commands,
    state: Res<PhysicsSimulationState>,
    query: Query<
        (
            Entity,
            &crate::common::game::bricks::components::BrickPhysics,
            &crate::common::game::bricks::components::BrickShapeComponent,
            Option<&Collider>,
            Option<&Friction>,
            Option<&Restitution>,
            Option<&GravityScale>,
            Option<&Mass>,
            Option<&RigidBody>,
            Option<&CollisionLayers>,
            Option<&SleepingDisabled>,
        ),
        Or<(
            Changed<crate::common::game::bricks::components::BrickPhysics>,
            Changed<crate::common::game::bricks::components::BrickShapeComponent>,
        )>,
    >,
) {
    if *state != PhysicsSimulationState::Running {
        return;
    }

    for (
        entity,
        physics,
        shape,
        collider,
        friction,
        restitution,
        gravity_scale,
        mass,
        rigid_body,
        layers,
        sleeping_disabled,
    ) in &query
    {
        let mut entity_commands = commands.entity(entity);
        let new_body = if physics.enabled {
            RigidBody::Dynamic
        } else {
            RigidBody::Static
        };
        if rigid_body != Some(&new_body) {
            entity_commands.insert(new_body);
        }
        let collider_matches_shape = collider.is_some_and(|collider| match shape.shape {
            crate::common::game::bricks::components::BrickShape::Block => {
                collider.shape().as_cuboid().is_some()
            }
            crate::common::game::bricks::components::BrickShape::Sphere => {
                collider.shape().as_ball().is_some()
            }
        });
        if !collider_matches_shape {
            let new_collider = match shape.shape {
                crate::common::game::bricks::components::BrickShape::Block => {
                    Collider::cuboid(4.0 * 0.28, 1.0 * 0.28, 2.0 * 0.28)
                }
                crate::common::game::bricks::components::BrickShape::Sphere => {
                    Collider::sphere(1.0 * 0.28)
                }
            };
            entity_commands.insert(new_collider);
        }
        let new_friction = Friction::new(physics.friction);
        if friction != Some(&new_friction) {
            entity_commands.insert(new_friction);
        }
        let new_restitution = Restitution::new(physics.bounciness);
        if restitution != Some(&new_restitution) {
            entity_commands.insert(new_restitution);
        }
        let new_layers = if physics.player_can_collide {
            CollisionLayers::from_bits(0b0001, 0xFFFF_FFFF)
        } else {
            CollisionLayers::from_bits(0b0100, 0xFFFF_FFFD)
        };
        if layers != Some(&new_layers) {
            entity_commands.insert(new_layers);
        }

        if physics.enabled {
            let new_g = GravityScale(physics.gravity_scale);
            if gravity_scale != Some(&new_g) {
                entity_commands.insert(new_g);
            }
            let new_m = Mass(physics.mass);
            if mass != Some(&new_m) {
                entity_commands.insert(new_m);
            }
            if sleeping_disabled.is_none() {
                entity_commands.insert(SleepingDisabled);
            }
        } else if gravity_scale.is_some() || mass.is_some() || sleeping_disabled.is_some() {
            entity_commands.remove::<(GravityScale, Mass, SleepingDisabled)>();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::game::bricks::components::{BrickPhysics, BrickShape, BrickShapeComponent};

    fn physics_app() -> App {
        let mut app = App::new();
        app.insert_resource(PhysicsSimulationState::Running)
            .add_systems(Update, sync_brick_physics_changes);
        app
    }

    #[test]
    fn shape_changes_replace_block_and_sphere_colliders() {
        let mut app = physics_app();
        let entity = app
            .world_mut()
            .spawn((
                BrickPhysics::default(),
                BrickShapeComponent {
                    shape: BrickShape::Block,
                },
            ))
            .id();

        app.update();
        assert!(
            app.world()
                .get::<Collider>(entity)
                .unwrap()
                .shape()
                .as_cuboid()
                .is_some()
        );

        app.world_mut()
            .get_mut::<BrickShapeComponent>(entity)
            .unwrap()
            .shape = BrickShape::Sphere;
        app.update();
        let collider = app.world().get::<Collider>(entity).unwrap();
        assert_eq!(collider.shape().as_ball().unwrap().radius, 0.28);
    }

    #[test]
    fn static_to_dynamic_restores_all_configured_components() {
        let mut app = physics_app();
        let entity = app
            .world_mut()
            .spawn((
                BrickPhysics {
                    enabled: false,
                    bounciness: 0.65,
                    player_can_collide: false,
                    friction: 0.8,
                    gravity_scale: 0.4,
                    mass: 7.5,
                    ..default()
                },
                BrickShapeComponent {
                    shape: BrickShape::Block,
                },
            ))
            .id();
        app.update();
        assert_eq!(
            app.world().get::<RigidBody>(entity),
            Some(&RigidBody::Static)
        );
        assert!(app.world().get::<GravityScale>(entity).is_none());
        assert!(app.world().get::<Mass>(entity).is_none());
        assert!(app.world().get::<SleepingDisabled>(entity).is_none());

        app.world_mut()
            .get_mut::<BrickPhysics>(entity)
            .unwrap()
            .enabled = true;
        app.update();

        assert_eq!(
            app.world().get::<RigidBody>(entity),
            Some(&RigidBody::Dynamic)
        );
        assert_eq!(
            app.world().get::<Friction>(entity),
            Some(&Friction::new(0.8))
        );
        assert_eq!(
            app.world().get::<Restitution>(entity),
            Some(&Restitution::new(0.65))
        );
        assert_eq!(
            app.world().get::<GravityScale>(entity),
            Some(&GravityScale(0.4))
        );
        assert_eq!(app.world().get::<Mass>(entity), Some(&Mass(7.5)));
        assert!(app.world().get::<SleepingDisabled>(entity).is_some());
        assert!(app.world().get::<CollisionLayers>(entity).is_some());
    }
}
