use bevy::prelude::default;
use bevy::render::RenderPlugin;
use bevy::render::settings::{RenderCreation, WgpuFeatures, WgpuSettings};

pub fn render_plugin() -> RenderPlugin {
    RenderPlugin {
        render_creation: RenderCreation::Automatic(Box::new(WgpuSettings {
            disabled_features: Some(WgpuFeatures::TEXTURE_BINDING_ARRAY),
            ..default()
        })),
        ..default()
    }
}
