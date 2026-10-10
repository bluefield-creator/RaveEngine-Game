use bevy::prelude::*;
use lightyear::prelude::client::ClientPlugins;
use rave_engine_lib::app::client::config::ClientAppConfig;
use rave_engine_lib::app::client::network_boot::{
    ClientConnectSettings, connect_after_warmup, setup_client,
};
use rave_engine_lib::app::common::log::{VERBOSE_NET_FILTER, configure_logging};
use rave_engine_lib::client::ClientPlugin;
use rave_engine_lib::common::CommonPlugin;

#[bevy_main]
fn main() {
    let log_plugin = configure_logging("client", VERBOSE_NET_FILTER, bevy::log::Level::DEBUG);
    let config = ClientAppConfig::from_env_and_args();

    let mut app = App::new();
    app.add_plugins(
        DefaultPlugins
            .set(log_plugin)
            .set(rave_engine_lib::common::assets_path::asset_plugin())
            .set(rave_engine_lib::app::common::render::render_plugin()),
    );
    app.insert_resource(ClientConnectSettings {
        ip: config.ip,
        port: config.port,
        netcode_key: Some(config.resolved_netcode_key()),
        protocol_id: Some(config.resolved_protocol_id()),
    });
    app.insert_resource(rave_engine_lib::client::ClientUkey(config.ukey));
    app.add_plugins(ClientPlugins {
        tick_duration: core::time::Duration::from_secs_f64(1.0 / 60.0),
    });
    app.add_plugins(CommonPlugin);
    app.add_plugins(ClientPlugin);
    app.add_systems(
        Startup,
        setup_client.after(rave_engine_lib::client::setup_player_assets),
    );
    app.add_systems(Update, connect_after_warmup);
    app.run();
}
