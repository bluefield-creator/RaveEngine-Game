pub mod play_camera;
pub mod controller;
pub mod loader;
pub mod animation;
pub mod model;

use bevy::prelude::*;

#[derive(Component)]
pub struct Player;

#[derive(Component)]
pub struct PlayerCamera;

#[derive(Component)]
pub struct PlayerController {
    pub move_speed: f32,
    pub jump_power: f32,
}

#[derive(Component)]
pub struct CameraSettings {
    pub yaw: f32,
    pub pitch: f32,
    pub distance: f32,
    pub current_distance: f32,
    pub target_offset: Vec3,
}

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<animation::PlayerAnimationGraphLoaded>()
            .init_resource::<animation::AvatarAnimationsRetargeted>()
            .add_systems(
                Update,
                (
                    crate::client::player::play_camera::update_camera,
                    animation::add_missing_animation_players,
                    animation::build_avatar_animation_graph,
                    animation::retarget_avatar_clips,
                    animation::init_player_animations,
                    animation::track_player_velocities,
                    animation::animate_player,
                ).run_if(crate::client::is_playtesting),
            );
    }
}