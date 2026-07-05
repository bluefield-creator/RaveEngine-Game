use bevy::prelude::*;
use bevy::input::mouse::{MouseMotion, MouseWheel};
use bevy::window::{CursorGrabMode, CursorOptions};
use avian3d::prelude::{SpatialQuery, SpatialQueryFilter};
use super::{PlayerCamera, CameraSettings};
use crate::client::LocalPlayer;

pub fn update_camera(
    mut mouse_motion: MessageReader<MouseMotion>,
    mut mouse_wheel: MessageReader<MouseWheel>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    mut player_query: Query<(Entity, &mut Transform, Option<&Children>), With<LocalPlayer>>,
    mut camera_query: Query<(&mut Transform, &mut CameraSettings), (With<PlayerCamera>, Without<LocalPlayer>)>,
    spatial_query: SpatialQuery,
    mut window_query: Query<&mut CursorOptions, With<bevy::window::PrimaryWindow>>,
    mut visibility_query: Query<&mut Visibility>,
) {
    let Some((player_entity, mut player_transform, children_opt)) = player_query.iter_mut().next() else {
        return;
    };
    let Some((mut camera_transform, mut settings)) = camera_query.iter_mut().next() else {
        return;
    };

    let mut rotation_move = Vec2::ZERO;
    for event in mouse_motion.read() {
        rotation_move += event.delta;
    }

    for event in mouse_wheel.read() {
        settings.distance = (settings.distance - event.y * 0.5).clamp(0.5, 40.0);
    }

    let in_first_person = settings.distance <= 0.6;

    if let Some(mut cursor_opts) = window_query.iter_mut().next() {
        if in_first_person || mouse_buttons.pressed(MouseButton::Right) {
            cursor_opts.visible = false;
            cursor_opts.grab_mode = CursorGrabMode::Locked;
            settings.yaw -= rotation_move.x * 0.005;
            settings.pitch = (settings.pitch - rotation_move.y * 0.005).clamp(-1.4, 1.4);
        } else {
            cursor_opts.visible = true;
            cursor_opts.grab_mode = CursorGrabMode::None;
        }
    }

    if in_first_person {
        player_transform.rotation = Quat::from_rotation_y(settings.yaw);
    }

    if let Some(children) = children_opt {
        for child in children.iter() {
            if let Ok(mut visibility) = visibility_query.get_mut(child) {
                if in_first_person {
                    *visibility = Visibility::Hidden;
                } else {
                    *visibility = Visibility::Inherited;
                }
            }
        }
    }

    let target_translation = player_transform.translation + settings.target_offset;
    let rotation = Quat::from_rotation_y(settings.yaw) * Quat::from_rotation_x(settings.pitch);
    let camera_offset = rotation.mul_vec3(Vec3::new(0.0, 0.0, settings.distance));

    let ray_dir = camera_offset.normalize_or_zero();
    let mut final_distance = settings.distance;

    if !in_first_person {
        if let Ok(dir3) = Dir3::new(ray_dir) {
            let filter = SpatialQueryFilter::default().with_excluded_entities([player_entity]);
            if let Some(hit) = spatial_query.cast_ray(
                target_translation,
                dir3,
                settings.distance,
                true,
                &filter,
            ) {
                final_distance = (hit.distance - 0.15).max(0.2);
            }
        }
    } else {
        final_distance = 0.0;
    }

    let final_offset = rotation.mul_vec3(Vec3::new(0.0, 0.0, final_distance));
    camera_transform.translation = target_translation + final_offset;
    camera_transform.rotation = rotation;
}