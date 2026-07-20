use bevy::anti_alias::fxaa::Fxaa;
use bevy::camera::Hdr;
use bevy::camera_controller::free_camera::FreeCamera;
use bevy::core_pipeline::prepass::{DepthPrepass, MotionVectorPrepass, NormalPrepass};
use bevy::light::ShadowFilteringMethod;
use bevy::pbr::ScreenSpaceAmbientOcclusion;
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
    let bloom_val = if graphics_settings.bloom {
        Some(Bloom::default())
    } else {
        None
    };

    if let Some(ssao) = ssao_val.clone() {
        camera.insert(ssao);
    }
    camera.insert(crate::common::core::performance::contact_shadow_settings(
        &graphics_settings,
    ));
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
    let playtesting_active = playtest.is_some_and(|p| p.active);

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
        let disable_camera =
            ctx.egui_wants_keyboard_input() || onboarding_active || playtesting_active;
        let disable_picking = ctx.egui_wants_pointer_input()
            || ctx.egui_wants_keyboard_input()
            || hover_state.is_hovering_ui
            || onboarding_active
            || playtesting_active;
        for mut state in &mut camera_query {
            state.enabled = !disable_camera;
        }
        picking_settings.is_enabled = !disable_picking && !camera_moving;
    }
}

pub fn sync_playtest_camera(
    playtest: Option<Res<crate::client::PlaytestState>>,
    mut editor_query: Query<
        (&mut Transform, &mut Projection),
        (
            With<bevy::camera_controller::free_camera::FreeCamera>,
            Without<crate::client::player::PlayerCamera>,
        ),
    >,
    player_query: Query<
        (&Transform, &Projection),
        (
            With<crate::client::player::PlayerCamera>,
            Without<bevy::camera_controller::free_camera::FreeCamera>,
        ),
    >,
    mut saved_editor_view: Local<Option<(Transform, Projection)>>,
) {
    let playtesting_active = playtest.is_some_and(|p| p.active);
    let Ok((mut editor_transform, mut editor_projection)) = editor_query.single_mut() else {
        return;
    };

    if playtesting_active {
        let Some((player_transform, player_projection)) = player_query.iter().next() else {
            return;
        };
        if saved_editor_view.is_none() {
            *saved_editor_view = Some((*editor_transform, editor_projection.clone()));
        }
        *editor_transform = *player_transform;
        *editor_projection = player_projection.clone();
    } else if let Some((transform, projection)) = saved_editor_view.take() {
        *editor_transform = transform;
        *editor_projection = projection;
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
            if let Some(&prev_state) = previous_active_states.get(&entity)
                && prev_state
                && !camera.is_active
            {
                camera.is_active = true;
            }
        }
        previous_active_states.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn follows_playtest_camera_and_restores_editor_view() {
        let mut app = App::new();
        app.insert_resource(crate::client::PlaytestState { active: true })
            .add_systems(Update, sync_playtest_camera);

        let editor_transform = Transform::from_xyz(-10.0, 10.0, -10.0);
        let player_transform = Transform::from_xyz(2.0, 3.0, 4.0);
        let editor = app
            .world_mut()
            .spawn((
                FreeCamera::default(),
                editor_transform,
                Projection::Perspective(PerspectiveProjection {
                    fov: 80.0f32.to_radians(),
                    ..default()
                }),
            ))
            .id();
        app.world_mut().spawn((
            crate::client::player::PlayerCamera,
            player_transform,
            Projection::Perspective(PerspectiveProjection {
                fov: 70.0f32.to_radians(),
                ..default()
            }),
        ));

        app.update();

        let followed_transform = app.world().get::<Transform>(editor).unwrap();
        assert_eq!(*followed_transform, player_transform);
        let Projection::Perspective(followed_projection) =
            app.world().get::<Projection>(editor).unwrap()
        else {
            panic!("expected perspective projection");
        };
        assert_eq!(followed_projection.fov, 70.0f32.to_radians());

        app.world_mut()
            .resource_mut::<crate::client::PlaytestState>()
            .active = false;
        app.update();

        let restored_transform = app.world().get::<Transform>(editor).unwrap();
        assert_eq!(*restored_transform, editor_transform);
        let Projection::Perspective(restored_projection) =
            app.world().get::<Projection>(editor).unwrap()
        else {
            panic!("expected perspective projection");
        };
        assert_eq!(restored_projection.fov, 80.0f32.to_radians());
    }
}
