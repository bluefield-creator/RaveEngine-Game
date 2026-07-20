use bevy::anti_alias::fxaa::Fxaa;
use bevy::camera::Exposure;
use bevy::camera_controller::free_camera::FreeCamera;
use bevy::light::{DirectionalLightShadowMap, ShadowFilteringMethod};
use bevy::pbr::{
    ContactShadows, ScreenSpaceAmbientOcclusion, ScreenSpaceAmbientOcclusionQualityLevel,
};
use bevy::post_process::bloom::Bloom;
use bevy::prelude::*;
use bevy::window::{PrimaryWindow, WindowMode};
use bevy::winit::{UpdateMode, WinitSettings};
use std::time::Duration;

#[derive(Component)]
pub struct PreviousTransform(pub Transform);

#[derive(Resource, Clone, PartialEq)]
pub struct GraphicsSettings {
    pub ssao: bool,
    pub ssao_quality: AmbientOcclusionQuality,
    pub contact_shadows: bool,
    pub contact_shadow_length: f32,
    pub bloom: bool,
    pub bloom_intensity: f32,
    pub shadow_quality: ShadowQuality,
    pub soft_shadows: bool,
    pub anti_aliasing: AntiAliasing,
    pub exposure_ev100: f32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AmbientOcclusionQuality {
    Low,
    Medium,
    High,
    Ultra,
}

impl AmbientOcclusionQuality {
    pub fn from_u8(value: u8) -> Self {
        match value {
            0 => Self::Low,
            1 => Self::Medium,
            3 => Self::Ultra,
            _ => Self::High,
        }
    }

    pub fn as_u8(self) -> u8 {
        match self {
            Self::Low => 0,
            Self::Medium => 1,
            Self::High => 2,
            Self::Ultra => 3,
        }
    }

    fn bevy(self) -> ScreenSpaceAmbientOcclusionQualityLevel {
        match self {
            Self::Low => ScreenSpaceAmbientOcclusionQualityLevel::Low,
            Self::Medium => ScreenSpaceAmbientOcclusionQualityLevel::Medium,
            Self::High => ScreenSpaceAmbientOcclusionQualityLevel::High,
            Self::Ultra => ScreenSpaceAmbientOcclusionQualityLevel::Ultra,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ShadowQuality {
    Low,
    Medium,
    High,
    Ultra,
}

impl ShadowQuality {
    pub fn from_u8(value: u8) -> Self {
        match value {
            0 => Self::Low,
            1 => Self::Medium,
            3 => Self::Ultra,
            _ => Self::High,
        }
    }

    pub fn as_u8(self) -> u8 {
        match self {
            Self::Low => 0,
            Self::Medium => 1,
            Self::High => 2,
            Self::Ultra => 3,
        }
    }

    fn map_size(self) -> usize {
        match self {
            Self::Low => 512,
            Self::Medium => 1024,
            Self::High => 2048,
            Self::Ultra => 4096,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AntiAliasing {
    Off,
    Fxaa,
    Msaa2,
    Msaa4,
    Msaa8,
}

impl AntiAliasing {
    pub fn from_u8(value: u8) -> Self {
        match value {
            0 => Self::Off,
            1 => Self::Fxaa,
            2 => Self::Msaa2,
            4 => Self::Msaa8,
            _ => Self::Msaa4,
        }
    }

    pub fn as_u8(self) -> u8 {
        match self {
            Self::Off => 0,
            Self::Fxaa => 1,
            Self::Msaa2 => 2,
            Self::Msaa4 => 3,
            Self::Msaa8 => 4,
        }
    }
}

impl Default for GraphicsSettings {
    fn default() -> Self {
        Self {
            ssao: true,
            ssao_quality: AmbientOcclusionQuality::High,
            contact_shadows: true,
            contact_shadow_length: 0.45,
            bloom: true,
            bloom_intensity: 0.05,
            shadow_quality: ShadowQuality::High,
            soft_shadows: true,
            anti_aliasing: AntiAliasing::Fxaa,
            exposure_ev100: 13.5,
        }
    }
}

pub(crate) fn contact_shadow_settings(settings: &GraphicsSettings) -> ContactShadows {
    ContactShadows {
        linear_steps: if settings.contact_shadows { 24 } else { 0 },
        thickness: 0.1,
        length: settings.contact_shadow_length,
    }
}

pub struct PerformancePlugin;

impl Plugin for PerformancePlugin {
    fn build(&self, app: &mut App) {
        if app.is_plugin_added::<bevy::render::RenderPlugin>() {
            app.insert_resource(WinitSettings::desktop_app())
                .init_resource::<GraphicsSettings>()
                .add_systems(Update, (apply_graphics_settings, manage_winit_performance));
        }
    }
}

pub fn apply_graphics_settings(
    settings: Res<GraphicsSettings>,
    mut commands: Commands,
    camera_query: Query<(Entity, &Camera), With<Camera3d>>,
    mut shadow_map: Option<ResMut<DirectionalLightShadowMap>>,
) {
    if !settings.is_changed() {
        return;
    }
    for (entity, camera) in &camera_query {
        if camera.order != 0 {
            continue;
        }
        if settings.ssao {
            commands.entity(entity).insert(ScreenSpaceAmbientOcclusion {
                quality_level: settings.ssao_quality.bevy(),
                ..default()
            });
        } else {
            commands
                .entity(entity)
                .remove::<ScreenSpaceAmbientOcclusion>();
        }

        commands
            .entity(entity)
            .insert(contact_shadow_settings(&settings));

        if settings.bloom {
            commands.entity(entity).insert(Bloom {
                intensity: settings.bloom_intensity,
                ..default()
            });
        } else {
            commands.entity(entity).remove::<Bloom>();
        }

        commands.entity(entity).insert((
            Exposure {
                ev100: settings.exposure_ev100,
            },
            if settings.soft_shadows {
                ShadowFilteringMethod::Gaussian
            } else {
                ShadowFilteringMethod::Hardware2x2
            },
        ));

        let anti_aliasing = if settings.ssao
            && matches!(
                settings.anti_aliasing,
                AntiAliasing::Msaa2 | AntiAliasing::Msaa4 | AntiAliasing::Msaa8
            ) {
            AntiAliasing::Fxaa
        } else {
            settings.anti_aliasing
        };
        match anti_aliasing {
            AntiAliasing::Off => {
                commands.entity(entity).insert(Msaa::Off).remove::<Fxaa>();
            }
            AntiAliasing::Fxaa => {
                commands.entity(entity).insert((Msaa::Off, Fxaa::default()));
            }
            AntiAliasing::Msaa2 => {
                commands
                    .entity(entity)
                    .insert(Msaa::Sample2)
                    .remove::<Fxaa>();
            }
            AntiAliasing::Msaa4 => {
                commands
                    .entity(entity)
                    .insert(Msaa::Sample4)
                    .remove::<Fxaa>();
            }
            AntiAliasing::Msaa8 => {
                commands
                    .entity(entity)
                    .insert(Msaa::Sample8)
                    .remove::<Fxaa>();
            }
        }
    }

    if let Some(ref mut shadow_map) = shadow_map {
        shadow_map.size = settings.shadow_quality.map_size();
    }
}

pub fn manage_winit_performance(
    mut winit_settings: ResMut<WinitSettings>,
    selection: Option<Res<crate::studio::tools::Selection>>,
    drag_state: Option<Res<crate::studio::tools::DragState>>,
    part_drag_state: Option<Res<crate::studio::tools::PartDragState>>,
    physics_state: Option<Res<crate::common::game::physics::PhysicsSimulationState>>,
    camera_query: Query<(Entity, &Transform), (With<Camera3d>, With<FreeCamera>)>,
    windows: Query<&Window, With<PrimaryWindow>>,
    time: Res<Time>,
    mut prev_transforms: Query<&mut PreviousTransform>,
    mut commands: Commands,
    mut last_mouse_position: Local<Option<Vec2>>,
    mut last_mouse_movement_time: Local<f32>,
) {
    if selection.is_none() {
        winit_settings.focused_mode = UpdateMode::Continuous;
        winit_settings.unfocused_mode = UpdateMode::Continuous;
        return;
    }

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

    let mut is_active = is_fullscreen || is_mouse_active;

    if let Some(ds) = drag_state
        && ds.active
    {
        is_active = true;
    }
    if let Some(pds) = part_drag_state
        && pds.active
    {
        is_active = true;
    }
    if let Some(ps) = physics_state
        && *ps == crate::common::game::physics::PhysicsSimulationState::Running
    {
        is_active = true;
    }

    for (entity, transform) in &camera_query {
        if let Ok(mut prev) = prev_transforms.get_mut(entity) {
            let dist_sq = transform.translation.distance_squared(prev.0.translation);
            let rot_diff = transform.rotation.dot(prev.0.rotation).abs();
            if dist_sq > 0.00001 || rot_diff < 0.99999 {
                is_active = true;
            }
            prev.0 = *transform;
        } else {
            commands
                .entity(entity)
                .insert(PreviousTransform(*transform));
            is_active = true;
        }
    }

    if is_active {
        winit_settings.focused_mode = UpdateMode::Continuous;
    } else {
        winit_settings.focused_mode = UpdateMode::reactive(Duration::from_millis(16));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn disables_contact_shadows_without_changing_the_render_layout() {
        let settings = GraphicsSettings {
            contact_shadows: false,
            contact_shadow_length: 1.25,
            ..Default::default()
        };

        let contact_shadows = contact_shadow_settings(&settings);

        assert_eq!(contact_shadows.linear_steps, 0);
        assert_eq!(contact_shadows.length, 1.25);
    }

    #[test]
    fn enables_contact_shadows_through_the_uniform() {
        let settings = GraphicsSettings::default();

        let contact_shadows = contact_shadow_settings(&settings);

        assert_eq!(contact_shadows.linear_steps, 24);
        assert_eq!(contact_shadows.length, settings.contact_shadow_length);
    }
}
