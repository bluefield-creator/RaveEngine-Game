use bevy::prelude::*;
use bevy::state::app::StatesPlugin;
use crate::app::server::config::ServerAppConfig;
use crate::common::CommonPlugin;
use crate::server::ServerPlugin;
use crate::app::common::log::setup_app_logging;

pub struct RaveServerApp {
    config: ServerAppConfig,
}

impl RaveServerApp {
    pub fn new(config: ServerAppConfig) -> Self {
        Self { config }
    }

    pub fn run(self) {
        let log_plugin = setup_app_logging("server");

        let mut app = App::new();
        app.add_plugins(log_plugin);
        app.add_plugins(MinimalPlugins);
        app.add_plugins(AssetPlugin::default());
        app.init_asset::<Mesh>();
        app.add_plugins(StatesPlugin);
        app.add_plugins(TransformPlugin);
        app.add_plugins(CommonPlugin);
        app.add_plugins(ServerPlugin {
            map_path: self.config.map_path,
            port: self.config.port,
        });
        app.run();
    }
}