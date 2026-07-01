use bevy::prelude::*;
use RaveEngineLib::common::CommonPlugin;
use RaveEngineLib::client::ClientPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(CommonPlugin)
        .add_plugins(ClientPlugin)
        .run();
}