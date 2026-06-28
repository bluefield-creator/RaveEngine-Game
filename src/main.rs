mod common;
mod studio;

use bevy::prelude::*;
use bevy::pbr::DefaultOpaqueRendererMethod;
use bevy::light::DirectionalLightShadowMap;
use bevy_egui::EguiPlugin;

fn main() {
    App::new()
        .insert_resource(DefaultOpaqueRendererMethod::forward())
        .insert_resource(DirectionalLightShadowMap { size: 4096 })
        .add_plugins(DefaultPlugins)
        .add_plugins(EguiPlugin::default())
        .add_plugins(common::CommonPlugin)
        .add_plugins(studio::StudioPlugin)
        .run();
}