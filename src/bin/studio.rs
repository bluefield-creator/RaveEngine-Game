use bevy::prelude::*;
use RaveEngineLib::studio::StudioPlugin;
use RaveEngineLib::common::CommonPlugin;

fn main() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins);
    app.add_plugins(CommonPlugin);
    app.add_plugins(StudioPlugin);
    app.run();
}