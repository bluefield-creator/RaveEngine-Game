use bevy::prelude::*;

#[derive(Component)]
pub struct Brick;

#[derive(Component, Clone, Copy, Debug, Reflect)]
#[reflect(Component)]
pub struct BrickPhysics {
    pub enabled: bool,
    pub bounciness: f32,
}

impl Default for BrickPhysics {
    fn default() -> Self {
        Self {
            enabled: true,
            bounciness: 0.3,
        }
    }
}