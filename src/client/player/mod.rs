pub mod camera;
pub mod controller;
pub mod loader;

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
    pub target_offset: Vec3,
}

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            camera::update_camera,
        );
    }
}