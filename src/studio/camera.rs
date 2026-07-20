use bevy::anti_alias::fxaa::Fxaa;
use bevy::camera::Hdr;
use bevy::camera_controller::free_camera::FreeCamera;
use bevy::core_pipeline::prepass::{DepthPrepass, MotionVectorPrepass, NormalPrepass};
use bevy::light::ShadowFilteringMethod;
use bevy::pbr::{ContactShadows, ScreenSpaceAmbientOcclusion};
use bevy::post_process::bloom::Bloom;
use bevy::prelude::*;

pub fn setup_studio(
    mut commands: Commands,
    mut egui_global_settings: ResMut<bevy_egui::EguiGlobalSettings>,
    graphics_settings: Res<crate::studio::ui::GraphicsSettings>,
    ambient: Option<ResMut<GlobalAmbientLight>>,
) {
    egui_global_settings.auto_create_primary_context = false;

    if let Some(mut amb) = ambient {
        amb.color = Color::srgb(0.55, 0.75, 0.95);
        amb.brightness = 320.0;
    }

    let mut camera = commands.spawn((
        Camera3d::default(),
        Camera::default(),
        Projection::Perspective(PerspectiveProjection {
            far: 3000.0,
            fov: 80.0f32.to_radians(),
            ..default()
        }),
        Hdr,
        Msaa::Off,
        bevy::core_pipeline::tonemapping::Tonemapping::TonyMcMapface,
        Transform::from_xyz(-10.0, 10.0, -10.0).looking_at(Vec3::ZERO, Vec3::Y),
        MeshPickingCamera,
        FreeCamera::default(),
        DepthPrepass,
        NormalPrepass,
        bevy::render::occlusion_culling::OcclusionCulling,
        ShadowFilteringMethod::Gaussian,
        bevy::camera::visibility::RenderLayers::from_layers(&[0, 1]),
        bevy_egui::PrimaryEguiContext,
    ));

    camera.insert((MotionVectorPrepass, Fxaa::default()));

    let ssao_val = if graphics_settings.ssao {
        Some(ScreenSpaceAmbientOcclusion::default())
    } else {
        None
    };
    let contact_shadows_val = if graphics_settings.contact_shadows {
        Some(ContactShadows::default())
    } else {
        None
    };
    let bloom_val = if graphics_settings.bloom {
        Some(Bloom::default())
    } else {
        None
    };

    if let Some(ssao) = ssao_val.clone() {
        camera.insert(ssao);
    }
    if let Some(contact) = contact_shadows_val.clone() {
        camera.insert(contact);
    }
    if let Some(bloom) = bloom_val.clone() {
        camera.insert(bloom);
    }

    commands.spawn((Name::new("Workspace"),));

    commands.spawn((
        Name::new("Players"),
        crate::common::net::components::PlayersServiceContainer,
    ));

    commands.spawn((
        Name::new("Lighting"),
        crate::common::net::components::LightingServiceContainer,
    ));
}

pub fn disable_camera_on_ui_interaction(
    mut camera_query: Query<&mut bevy::camera_controller::free_camera::FreeCameraState>,
    mut contexts: bevy_egui::EguiContexts,
    mut picking_settings: ResMut<bevy::picking::PickingSettings>,
    hover_state: Res<crate::studio::tools::HoverState>,
    onboarding_state: Res<State<crate::studio::tools::OnboardingState>>,
    playtest: Option<Res<crate::client::PlaytestState>>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    keys: Res<ButtonInput<KeyCode>>,
) {
    let onboarding_active =
        *onboarding_state.get() != crate::studio::tools::OnboardingState::Inactive;
    let playtesting_active = playtest.map_or(false, |p| p.active);

    let right_mouse_held = mouse_buttons.pressed(MouseButton::Right);
    let movement_keys_held = keys.any_pressed([
        KeyCode::KeyW,
        KeyCode::KeyA,
        KeyCode::KeyS,
        KeyCode::KeyD,
        KeyCode::KeyQ,
        KeyCode::KeyE,
        KeyCode::ArrowUp,
        KeyCode::ArrowDown,
        KeyCode::ArrowLeft,
        KeyCode::ArrowRight,
    ]);
    let camera_moving = right_mouse_held || movement_keys_held;

    if let Ok(ctx) = contexts.ctx_mut() {
        let wants_input = ctx.egui_wants_pointer_input()
            || ctx.egui_wants_keyboard_input()
            || hover_state.is_hovering_ui
            || onboarding_active
            || playtesting_active;
        for mut state in &mut camera_query {
            state.enabled = !wants_input;
        }
        picking_settings.is_enabled = !wants_input && !camera_moving;
    }
}

pub fn sync_primary_egui_camera(
    mut commands: Commands,
    camera_query: Query<(Entity, &Camera), With<Camera3d>>,
    context_query: Query<Entity, With<bevy_egui::PrimaryEguiContext>>,
) {
    let target = camera_query
        .iter()
        .filter(|(_, camera)| camera.is_active)
        .max_by_key(|(_, camera)| camera.order)
        .map(|(entity, _)| entity);
    let Some(target) = target else {
        return;
    };
    for entity in &context_query {
        if entity != target {
            commands
                .entity(entity)
                .remove::<bevy_egui::PrimaryEguiContext>();
        }
    }
    if !context_query.contains(target) {
        commands
            .entity(target)
            .insert(bevy_egui::PrimaryEguiContext);
    }
}

pub fn toggle_editor_camera_active(
    playtest: Option<Res<crate::client::PlaytestState>>,
    mut camera_query: Query<
        &mut Camera,
        (
            With<bevy::camera_controller::free_camera::FreeCamera>,
            Without<crate::client::player::PlayerCamera>,
        ),
    >,
) {
    let playtesting_active = playtest.map_or(false, |p| p.active);
    for mut camera in &mut camera_query {
        camera.is_active = !playtesting_active;
    }
}

pub fn disable_cameras_on_minimization(
    window_query: Query<&Window, With<bevy::window::PrimaryWindow>>,
    mut camera_query: Query<(Entity, &mut Camera)>,
    mut previous_active_states: Local<std::collections::HashMap<Entity, bool>>,
) {
    let Ok(window) = window_query.single() else {
        return;
    };
    let is_minimized = window.width() <= 0.0 || window.height() <= 0.0;

    if is_minimized {
        for (entity, mut camera) in &mut camera_query {
            if camera.is_active {
                previous_active_states.insert(entity, true);
                camera.is_active = false;
            }
        }
    } else {
        for (entity, mut camera) in &mut camera_query {
            if let Some(&prev_state) = previous_active_states.get(&entity) {
                if prev_state && !camera.is_active {
                    camera.is_active = true;
                }
            }
        }
        previous_active_states.clear();
    }
}
