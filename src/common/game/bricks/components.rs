use bevy::prelude::*;
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Reflect, Default, Serialize, Deserialize)]
pub enum BrickShape {
    #[default]
    Block,
    Sphere,
}

#[derive(Component, Clone, Copy, Debug, Reflect, Default, Serialize, Deserialize)]
#[reflect(Component)]
pub struct Brick;

#[derive(Component, Clone, Copy, Debug, Reflect, Default, Serialize, Deserialize)]
#[reflect(Component)]
pub struct BrickShapeComponent {
    pub shape: BrickShape,
}

#[derive(Component, Clone, Copy, Debug, Reflect, Serialize, Deserialize)]
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

#[derive(Component, Clone, Copy, Debug, Reflect, Serialize, Deserialize)]
#[reflect(Component)]
pub struct BrickColor {
    pub color: Color,
}

impl Default for BrickColor {
    fn default() -> Self {
        Self {
            color: Color::Srgba(Srgba::new(0.84, 0.24, 0.16, 1.0)),
        }
    }
}