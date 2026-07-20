pub mod core;
pub mod game;
pub mod net;
pub mod ui;

use bevy::prelude::*;

pub struct CommonPlugin;

impl Plugin for CommonPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(game::GamePlugin)
            .add_plugins(net::NetPlugin)
            .add_plugins(ui::UiPlugin)
            .add_plugins(core::CorePlugin)
            .add_plugins(crate::scripting::plugin::ScriptingPlugin);
    }
}
