use bevy::prelude::*;
use bevy::core_pipeline::prepass::{DepthPrepass, MotionVectorPrepass, NormalPrepass};
use bevy::anti_alias::fxaa::Fxaa;
use bevy::camera_controller::free_camera::FreeCamera;
use bevy::camera::Hdr;
use bevy::post_process::bloom::Bloom;
use bevy::pbr::{ScreenSpaceAmbientOcclusion, ContactShadows};
use bevy::winit::{WinitSettings, UpdateMode};
use bevy::window::{PrimaryWindow, WindowMode};
use std::time::Duration;

#[derive(Component)]
pub struct GizmoCamera;

#[derive(Component)]
pub struct PreviousTransform(pub Transform);

pub fn setup_studio(
    mut commands: Commands,
    mut egui_global_settings: ResMut<bevy_egui::EguiGlobalSettings>,
    graphics_settings: Res<crate::studio::ui::GraphicsSettings>,
) {
    egui_global_settings.auto_create_primary_context = false;

    commands.spawn((
        PointLight {
            intensity: 1500.0,
            shadow_maps_enabled: true,
            contact_shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(4.0, 8.0, 4.0),
    ));
    
    let mut camera = commands.spawn((
        Camera3d::default(),
        Projection::Perspective(PerspectiveProjection {
            far: 3000.0,
            fov: 80.0f32.to_radians(),
            ..default()
        }),
        Hdr,
        Msaa::Off,
        bevy::core_pipeline::tonemapping::Tonemapping::None,
        Transform::from_xyz(-10.0, 10.0, -10.0).looking_at(Vec3::ZERO, Vec3::Y),
        MeshPickingCamera,
        FreeCamera::default(),
        DepthPrepass,
        NormalPrepass,
    ));

    camera.insert((
        MotionVectorPrepass,
        Fxaa::default(),
        DistanceFog {
            color: Color::srgb(0.70, 0.90, 1.00),
            falloff: FogFalloff::Linear {
                start: 400.0,
                end: 1100.0,
            },
            ..default()
        },
    ));

    if graphics_settings.ssao {
        camera.insert(ScreenSpaceAmbientOcclusion::default());
    }
    if graphics_settings.contact_shadows {
        camera.insert(ContactShadows::default());
    }
    if graphics_settings.bloom {
        camera.insert(Bloom::default());
    }

    commands.spawn((
        Camera3d::default(),
        Camera {
            order: 1,
            clear_color: ClearColorConfig::None,
            ..default()
        },
        Hdr,
        Msaa::Off,
        bevy::core_pipeline::tonemapping::Tonemapping::None,
        bevy::camera::visibility::RenderLayers::layer(1),
        bevy_egui::PrimaryEguiContext,
        GizmoCamera,
    ));
}

pub fn apply_graphics_settings(
    settings: Res<crate::studio::ui::GraphicsSettings>,
    mut commands: Commands,
    camera_query: Query<Entity, (With<Camera3d>, Without<GizmoCamera>)>,
) {
    if !settings.is_changed() {
        return;
    }
    for entity in &camera_query {
        if settings.ssao {
            commands.entity(entity).insert(ScreenSpaceAmbientOcclusion::default());
        } else {
            commands.entity(entity).remove::<ScreenSpaceAmbientOcclusion>();
        }

        if settings.contact_shadows {
            commands.entity(entity).insert(ContactShadows::default());
        } else {
            commands.entity(entity).remove::<ContactShadows>();
        }

        if settings.bloom {
            commands.entity(entity).insert(Bloom::default());
        } else {
            commands.entity(entity).remove::<Bloom>();
        }
    }
}

pub fn disable_camera_on_ui_interaction(
    mut camera_query: Query<&mut bevy::camera_controller::free_camera::FreeCameraState>,
    mut contexts: bevy_egui::EguiContexts,
    mut picking_settings: ResMut<bevy::picking::PickingSettings>,
    hover_state: Res<crate::studio::tools::HoverState>,
    onboarding_state: Res<State<crate::studio::tools::OnboardingState>>,
) {
    let onboarding_active = *onboarding_state.get() != crate::studio::tools::OnboardingState::Inactive;

    if let Ok(ctx) = contexts.ctx_mut() {
        let wants_input = ctx.egui_wants_pointer_input() || ctx.egui_wants_keyboard_input() || hover_state.is_hovering_ui || onboarding_active;
        for mut state in &mut camera_query {
            state.enabled = !wants_input;
        }
        picking_settings.is_enabled = !wants_input;
    }
}

pub fn sync_gizmo_camera(
    camera_query: Query<(&Transform, &Projection), (With<Camera3d>, Without<GizmoCamera>)>,
    mut gizmo_camera: Query<(&mut Transform, &mut Projection), With<GizmoCamera>>,
) {
    if let Some((main_trans, main_proj)) = camera_query.iter().next() {
        if let Some((mut gizmo_trans, mut gizmo_proj)) = gizmo_camera.iter_mut().next() {
            *gizmo_trans = *main_trans;
            *gizmo_proj = main_proj.clone();
        }
    }
}

pub fn manage_winit_performance(
    mut winit_settings: ResMut<WinitSettings>,
    drag_state: Res<crate::studio::tools::DragState>,
    part_drag_state: Res<crate::studio::tools::PartDragState>,
    physics_state: Res<crate::common::physics::PhysicsSimulationState>,
    camera_query: Query<(Entity, &Transform), (With<Camera3d>, Without<GizmoCamera>)>,
    windows: Query<&Window, With<PrimaryWindow>>,
    time: Res<Time>,
    mut prev_transforms: Query<&mut PreviousTransform>,
    mut commands: Commands,
    mut last_mouse_position: Local<Option<Vec2>>,
    mut last_mouse_movement_time: Local<f32>,
) {
    let current_time = time.elapsed_secs();
    
    let mut is_hovered = false;
    let mut is_fullscreen = false;
    
    if let Ok(window) = windows.single() {
        if !matches!(window.mode, WindowMode::Windowed) {
            is_fullscreen = true;
        }
        if let Some(cursor_pos) = window.cursor_position() {
            is_hovered = true;
            if let Some(last_pos) = *last_mouse_position {
                if cursor_pos.distance_squared(last_pos) > 0.0001 {
                    *last_mouse_position = Some(cursor_pos);
                    *last_mouse_movement_time = current_time;
                }
            } else {
                *last_mouse_position = Some(cursor_pos);
                *last_mouse_movement_time = current_time;
            }
        } else {
            *last_mouse_position = None;
        }
    }

    let time_since_last_move = current_time - *last_mouse_movement_time;
    let is_mouse_active = is_hovered && (time_since_last_move < 30.0);

    let mut is_active = drag_state.active
        || part_drag_state.active
        || *physics_state == crate::common::physics::PhysicsSimulationState::Running
        || is_fullscreen
        || is_mouse_active;

    for (entity, transform) in &camera_query {
        if let Ok(mut prev) = prev_transforms.get_mut(entity) {
            let dist_sq = transform.translation.distance_squared(prev.0.translation);
            let rot_diff = transform.rotation.dot(prev.0.rotation).abs();
            if dist_sq > 0.00001 || rot_diff < 0.99999 {
                is_active = true;
            }
            prev.0 = *transform;
        } else {
            commands.entity(entity).insert(PreviousTransform(*transform));
            is_active = true;
        }
    }

    if is_active {
        winit_settings.focused_mode = UpdateMode::Continuous;
    } else {
        winit_settings.focused_mode = UpdateMode::reactive(Duration::from_millis(16));
    }
}