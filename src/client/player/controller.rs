use bevy::prelude::*;
use avian3d::prelude::*;
use super::{Player, PlayerController, CameraSettings, PlayerCamera};

pub fn player_movement(
    keys: Res<ButtonInput<KeyCode>>,
    mut player_query: Query<
        (
            Entity,
            &mut Transform,
            &mut LinearVelocity,
            &PlayerController,
            &CollidingEntities,
        ),
        With<Player>,
    >,
    camera_query: Query<(&Transform, &CameraSettings), (With<PlayerCamera>, Without<Player>)>,
    spatial_query: SpatialQuery,
) {
    let Some((player_entity, mut player_transform, mut velocity, controller, colliding)) =
        player_query.iter_mut().next()
    else {
        return;
    };
    let Some((camera_transform, camera_settings)) = camera_query.iter().next() else {
        return;
    };

    let in_first_person = camera_settings.distance <= 0.6;

    let mut move_direction = Vec3::ZERO;

    let camera_forward = *camera_transform.forward();
    let camera_right = *camera_transform.right();

    let mut forward_proj = Vec3::new(camera_forward.x, 0.0, camera_forward.z);
    let mut right_proj = Vec3::new(camera_right.x, 0.0, camera_right.z);

    if forward_proj.length_squared() > 0.001 {
        forward_proj = forward_proj.normalize();
    }
    if right_proj.length_squared() > 0.001 {
        right_proj = right_proj.normalize();
    }

    if keys.pressed(KeyCode::KeyW) {
        move_direction += forward_proj;
    }
    if keys.pressed(KeyCode::KeyS) {
        move_direction -= forward_proj;
    }
    if keys.pressed(KeyCode::KeyA) {
        move_direction -= right_proj;
    }
    if keys.pressed(KeyCode::KeyD) {
        move_direction += right_proj;
    }

    if move_direction.length_squared() > 0.001 {
        let move_direction = move_direction.normalize();
        let target_vel = move_direction * controller.move_speed;
        velocity.x = target_vel.x;
        velocity.z = target_vel.z;

        if !in_first_person {
            let target_angle = move_direction.z.atan2(move_direction.x);
            let target_rotation =
                Quat::from_rotation_y(-target_angle + std::f32::consts::FRAC_PI_2);
            player_transform.rotation = player_transform.rotation.lerp(target_rotation, 0.15);
        }
    } else {
        velocity.x = 0.0;
        velocity.z = 0.0;
    }

    let is_grounded = {
        let mut grounded = !colliding.is_empty() && velocity.y.abs() < 0.2;
        if !grounded {
            let ray_origin = player_transform.translation;
            let max_ray_dist = 2.5 * 0.28 + 0.15;
            let filter = SpatialQueryFilter::default().with_excluded_entities([player_entity]);
            if spatial_query.cast_ray(ray_origin, Dir3::NEG_Y, max_ray_dist, true, &filter).is_some() {
                grounded = true;
            }
        }
        grounded
    };
    if keys.pressed(KeyCode::Space) && is_grounded {
        velocity.y = controller.jump_power;
    }
}