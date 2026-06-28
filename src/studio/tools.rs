use bevy::prelude::*;
use bevy::window::{CursorIcon, SystemCursorIcon};
use crate::common::components::Brick;
use crate::studio::gizmos::ToolGizmo;

#[derive(Default, States, Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum ToolState {
    #[default]
    None,
    Move,
    Size,
    Rotate,
}

#[derive(Resource, Default)]
pub struct Selection {
    pub entity: Option<Entity>,
}

#[derive(Resource, Default)]
pub struct DragState {
    pub active: bool,
    pub gizmo_entity: Option<Entity>,
}

#[derive(Resource, Default)]
pub struct HoverState {
    pub hovered_gizmo: Option<Entity>,
}

pub fn select_brick(
    mut clicks: MessageReader<Pointer<Click>>,
    bricks: Query<Entity, With<Brick>>,
    gizmos: Query<Entity, With<ToolGizmo>>,
    mut selection: ResMut<Selection>,
) {
    for click in clicks.read() {
        let target = click.event_target();
        if bricks.get(target).is_ok() {
            selection.entity = Some(target);
        } else if gizmos.get(target).is_err() {
            selection.entity = None;
        }
    }
}

pub fn handle_drag_start(
    mut drags: MessageReader<Pointer<DragStart>>,
    gizmos: Query<&ToolGizmo>,
    mut drag_state: ResMut<DragState>,
) {
    for drag in drags.read() {
        let target = drag.event_target();
        if gizmos.get(target).is_ok() {
            drag_state.active = true;
            drag_state.gizmo_entity = Some(target);
        }
    }
}

pub fn handle_drag(
    mut drags: MessageReader<Pointer<Drag>>,
    gizmos: Query<&ToolGizmo>,
    mut bricks: Query<&mut Transform, With<Brick>>,
    drag_state: Res<DragState>,
    camera_query: Query<(&Camera, &GlobalTransform)>,
) {
    if !drag_state.active { return; }
    
    let Some(gizmo_entity) = drag_state.gizmo_entity else { return };
    let Ok(gizmo) = gizmos.get(gizmo_entity) else { return };
    let Ok(mut brick_transform) = bricks.get_mut(gizmo.target) else { return };
    let Some((camera, camera_transform)) = camera_query.iter().next() else { return };

    for drag in drags.read() {
        let delta = drag.delta;
        let center_world = brick_transform.translation;

        if gizmo.tool == ToolState::Rotate {
            let axis_world = brick_transform.rotation.mul_vec3(gizmo.axis);
            let view_dir = camera_transform.forward().as_vec3();
            let tangent_3d = view_dir.cross(axis_world).normalize_or_zero();
            let tangent_world = center_world + tangent_3d;

            if let (Ok(c), Ok(t)) = (
                camera.world_to_viewport(camera_transform, center_world),
                camera.world_to_viewport(camera_transform, tangent_world)
            ) {
                let screen_vec = t - c;
                let pixel_len = screen_vec.length();
                let screen_dir = screen_vec.normalize_or_zero();
                
                let mut drag_world = 0.0;
                if pixel_len > 0.0 {
                    drag_world = delta.dot(screen_dir) / pixel_len;
                }
                
                let amount = drag_world / 3.5;
                let rot = Quat::from_axis_angle(gizmo.axis, amount);
                brick_transform.rotate_local(rot);
            }
        } else {
            let axis_world = brick_transform.rotation.mul_vec3(gizmo.axis);
            let tip_world = center_world + axis_world;

            if let (Ok(c), Ok(t)) = (
                camera.world_to_viewport(camera_transform, center_world),
                camera.world_to_viewport(camera_transform, tip_world)
            ) {
                let screen_vec = t - c;
                let pixel_len = screen_vec.length();
                let screen_dir = screen_vec.normalize_or_zero();
                
                let mut amount_world = 0.0;
                if pixel_len > 0.0 {
                    amount_world = delta.dot(screen_dir) / pixel_len;
                }

                match gizmo.tool {
                    ToolState::Move => {
                        let translation_delta = brick_transform.rotation.mul_vec3(gizmo.axis) * amount_world;
                        brick_transform.translation += translation_delta;
                    }
                    ToolState::Size => {
                        let axis_abs = gizmo.axis.abs();
                        let base_extents = Vec3::new(2.0, 0.5, 2.0);
                        let base_dimension = axis_abs * base_extents * 2.0;
                        let base_dim_scalar = base_dimension.length();

                        let delta_scale = if base_dim_scalar > 0.0 {
                            amount_world / base_dim_scalar
                        } else {
                            0.0
                        };

                        let amount_3d = axis_abs * delta_scale;
                        let new_scale = (brick_transform.scale + amount_3d).max(Vec3::splat(0.1));
                        let actual_delta_scale = new_scale - brick_transform.scale;
                        
                        brick_transform.scale = new_scale;
                        
                        let translation_delta = gizmo.axis * actual_delta_scale * base_extents;
                        let final_translation_delta = brick_transform.rotation.mul_vec3(translation_delta);
                        brick_transform.translation += final_translation_delta;
                    }
                    _ => {}
                }
            }
        }
    }
}

pub fn handle_drag_end(
    mut drags: MessageReader<Pointer<DragEnd>>,
    mut drag_state: ResMut<DragState>,
) {
    for _ in drags.read() {
        drag_state.active = false;
        drag_state.gizmo_entity = None;
    }
}

pub fn handle_hover(
    mut over_events: MessageReader<Pointer<Over>>,
    mut out_events: MessageReader<Pointer<Out>>,
    gizmos: Query<&ToolGizmo>,
    mut hover_state: ResMut<HoverState>,
) {
    for over in over_events.read() {
        let target = over.event_target();
        if gizmos.get(target).is_ok() {
            hover_state.hovered_gizmo = Some(target);
        }
    }
    for out in out_events.read() {
        let target = out.event_target();
        if Some(target) == hover_state.hovered_gizmo {
            hover_state.hovered_gizmo = None;
        }
    }
}

pub fn update_cursor(
    mut commands: Commands,
    drag_state: Res<DragState>,
    hover_state: Res<HoverState>,
    windows: Query<Entity, With<Window>>,
) {
    let Ok(window_entity) = windows.single() else { return };
    if drag_state.active {
        commands.entity(window_entity).insert(CursorIcon::from(SystemCursorIcon::Grabbing));
    } else if hover_state.hovered_gizmo.is_some() {
        commands.entity(window_entity).insert(CursorIcon::from(SystemCursorIcon::Grab));
    } else {
        commands.entity(window_entity).remove::<CursorIcon>();
    }
}