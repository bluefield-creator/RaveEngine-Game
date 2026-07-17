use bevy::prelude::*;
use serde::{Serialize, Deserialize};

fn default_true() -> bool {
    true
}

#[derive(Component, Reflect, Clone, Serialize, Deserialize)]
#[reflect(Component)]
pub struct ServerScript {
    pub code: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(skip)]
    pub started: bool,
}

impl Default for ServerScript {
    fn default() -> Self {
        Self {
            code: "".to_string(),
            enabled: true,
            started: false,
        }
    }
}

#[derive(Component, Reflect, Clone, Serialize, Deserialize)]
#[reflect(Component)]
pub struct LocalScript {
    pub code: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(skip)]
    pub started: bool,
}

impl Default for LocalScript {
    fn default() -> Self {
        Self {
            code: "".to_string(),
            enabled: true,
            started: false,
        }
    }
}

#[derive(Component, Reflect, Default, Clone, Serialize, Deserialize)]
#[reflect(Component)]
pub struct ModuleScript {
    pub code: String,
}