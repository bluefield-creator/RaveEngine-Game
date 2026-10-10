use RaveEngineLib::app::client::config::ClientAppConfig;
use RaveEngineLib::app::client::network_boot::{
    ClientConnectSettings, connect_after_warmup, setup_client,
};
use RaveEngineLib::app::common::args::CliArgs;
use RaveEngineLib::app::common::log::{VERBOSE_NET_FILTER, configure_logging};
use RaveEngineLib::client::ClientPlugin;
use RaveEngineLib::common::CommonPlugin;
use bevy::prelude::*;
use lightyear::prelude::client::ClientPlugins;

#[bevy_main]
fn main() {
    let log_plugin = configure_logging("client", VERBOSE_NET_FILTER, bevy::log::Level::DEBUG);
    let _ = RaveEngineLib::client::uri::register_client_uri_scheme();

    let args = CliArgs::from_env();
    if args.has("--no-clouds") || args.has("--disable-clouds") {
        unsafe {
            std::env::set_var("VERTIGO_NO_CLOUDS", "1");
        }
    }
    let config = ClientAppConfig::from_args(&args);

    let mut app = App::new();
    app.add_plugins(
        DefaultPlugins
            .set(log_plugin)
            .set(RaveEngineLib::common::assets_path::asset_plugin())
            .set(RaveEngineLib::app::common::render::render_plugin()),
    );
    app.insert_resource(ClientConnectSettings {
        ip: config.ip,
        port: config.port,
        netcode_key: Some(config.resolved_netcode_key()),
        protocol_id: Some(config.resolved_protocol_id()),
    });
    app.insert_resource(RaveEngineLib::client::ClientUkey(config.ukey));
    app.add_plugins(ClientPlugins {
        tick_duration: core::time::Duration::from_secs_f64(1.0 / 60.0),
    });
    app.add_plugins(CommonPlugin);
    app.add_plugins(ClientPlugin);
    app.add_systems(
        Startup,
        setup_client.after(RaveEngineLib::client::setup_player_assets),
    );
    app.add_systems(Update, connect_after_warmup);
    app.run();
}
