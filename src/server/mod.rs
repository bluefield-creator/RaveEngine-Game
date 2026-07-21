use avian3d::prelude::*;
use bevy::prelude::*;
use lightyear::prelude::server::*;
use lightyear::prelude::*;
use std::net::SocketAddr;
use std::time::Duration;

pub mod map;
pub mod player;

#[derive(Resource)]
pub struct ServerSettings {
    pub map_path: String,
    pub port: u16,
    pub bind_addr: SocketAddr,
    pub netcode_key: [u8; 32],
    pub embedded_server: bool,
}

pub struct ServerPlugin {
    pub map_path: String,
    pub port: u16,
    pub bind_ip: std::net::IpAddr,
    pub netcode_key: [u8; 32],
    pub embedded_server: bool,
}

impl Plugin for ServerPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(ServerSettings {
            map_path: self.map_path.clone(),
            port: self.port,
            bind_addr: SocketAddr::new(self.bind_ip, self.port),
            netcode_key: self.netcode_key,
            embedded_server: self.embedded_server,
        })
        .insert_resource(Gravity(Vec3::new(0.0, -186.9 * 0.28, 0.0)))
        .add_plugins(server::ServerPlugins {
            tick_duration: Duration::from_secs_f64(1.0 / 60.0),
        })
        .add_plugins(crate::common::net::ProtocolPlugin)
        .add_systems(Startup, (setup_server, map::load_map))
        .add_systems(
            Update,
            (
                player::handle_player_inputs,
                player::handle_hello_messages,
                player::complete_authentication_tasks,
                player::sync_players_service_properties,
            ),
        )
        .add_systems(PostUpdate, player::sync_transforms_to_network)
        .add_observer(player::handle_new_client)
        .add_observer(player::handle_client_disconnect);
    }
}

fn setup_server(mut commands: Commands, mut settings: ResMut<ServerSettings>) {
    info!("Starting setup_server system on port: {}", settings.port);
    let bind_addr = settings.bind_addr;
    settings.bind_addr = bind_addr;

    let netcode_config = NetcodeConfig {
        protocol_id: crate::common::net::NETCODE_PROTOCOL_ID,
        private_key: settings.netcode_key,
        client_timeout_secs: 15,
        ..Default::default()
    };

    let server_entity = commands
        .spawn((
            Server::default(),
            ServerUdpIo::default(),
            LocalAddr(bind_addr),
            NetcodeServer::new(netcode_config),
        ))
        .id();

    commands.trigger(Start {
        entity: server_entity,
    });
    info!("Server entity spawned and Start trigger dispatched");

    commands.insert_resource(crate::scripting::vm::server_vm::ServerScriptVM::new());

    commands.spawn((Name::new("Workspace"),));

    commands.spawn((
        Name::new("Players"),
        crate::common::net::components::PlayersServiceContainer,
        Replicate::default(),
    ));

    commands.spawn((
        Name::new("Lighting"),
        crate::common::net::components::LightingServiceContainer,
        Replicate::default(),
    ));
}
