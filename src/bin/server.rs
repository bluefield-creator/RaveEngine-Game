use bevy::prelude::*;
use bevy::log::LogPlugin;
use bevy::state::app::StatesPlugin;
use RaveEngineLib::server::ServerPlugin;
use RaveEngineLib::common::CommonPlugin;

fn main() {
    let mut port = 5000;
    let mut map_path = "assets/maps/default.vrtx".to_string();

    let args: Vec<String> = std::env::args().collect();
    for i in 0..args.len() {
        if args[i] == "--port" && i + 1 < args.len() {
            if let Ok(p) = args[i + 1].parse::<u16>() {
                port = p;
            }
        }
        if args[i] == "--map" && i + 1 < args.len() {
            map_path = args[i + 1].clone();
        }
    }

    let mut app = App::new();
    app.add_plugins(LogPlugin {
        level: bevy::log::Level::DEBUG,
        filter: "wgpu=error,bevy_render=error,bevy_ecs=warn,lightyear=debug,lightyear_udp=trace,lightyear_netcode=trace".to_string(),
        ..default()
    });
    app.add_plugins(MinimalPlugins);
    app.add_plugins(AssetPlugin::default());
    app.init_asset::<Mesh>();
    app.add_plugins(StatesPlugin);
    app.add_plugins(TransformPlugin);
    app.add_plugins(CommonPlugin);
    app.add_plugins(ServerPlugin {
        map_path,
        port,
    });
    app.run();
}