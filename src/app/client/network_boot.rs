use bevy::prelude::*;
use lightyear::prelude::client::*;
use lightyear::prelude::*;
use serde::Deserialize;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};

#[derive(Resource)]
pub struct ClientConnectSettings {
    pub ip: IpAddr,
    pub port: u16,
    pub netcode_key: [u8; 32],
}

#[derive(Resource, Default)]
pub struct ClientSpawned(pub bool);

#[derive(Deserialize)]
pub struct LaunchInfo {
    pub ip: String,
    pub port: u16,
    pub ukey: String,
    #[serde(default)]
    pub netcode_key: String,
}

pub fn poll_launch_details(
    mut commands: Commands,
    ukey_res: Option<Res<crate::client::ClientUkey>>,
    mut settings: Option<ResMut<ClientConnectSettings>>,
) {
    if let Some(ukey) = &ukey_res
        && !ukey.0.is_empty()
    {
        return;
    }

    if let Ok(file_content) = std::fs::read_to_string("launch_info.json")
        && let Ok(info) = serde_json::from_str::<LaunchInfo>(&file_content)
        && let Ok(parsed_ip) = info.ip.parse::<IpAddr>()
    {
        let configured_key = if info.netcode_key.is_empty() {
            std::env::var(crate::common::net::NETCODE_KEY_ENV).ok()
        } else {
            Some(info.netcode_key)
        };
        let Ok(netcode_key) = crate::common::net::resolve_netcode_key(
            configured_key.as_deref(),
            parsed_ip.is_loopback(),
        ) else {
            error!("CLIENT_CONNECT: Invalid Netcode key in launch configuration");
            return;
        };
        commands.insert_resource(crate::client::ClientUkey(info.ukey));
        if let Some(ref mut s) = settings {
            s.ip = parsed_ip;
            s.port = info.port;
            s.netcode_key = netcode_key;
        } else {
            commands.insert_resource(ClientConnectSettings {
                ip: parsed_ip,
                port: info.port,
                netcode_key,
            });
        }
        info!("CLIENT_CONNECT: Successfully loaded connection details from launch_info.json");
        let _ = std::fs::remove_file("launch_info.json");
    }
}

pub fn initialize_client(
    mut commands: Commands,
    settings: Res<ClientConnectSettings>,
    ukey_res: Option<Res<crate::client::ClientUkey>>,
    mut spawned: ResMut<ClientSpawned>,
) {
    if spawned.0 {
        return;
    }
    let Some(ukey) = ukey_res else {
        return;
    };
    if ukey.0.is_empty() {
        return;
    }

    let server_addr = SocketAddr::new(settings.ip, settings.port);
    let client_id = rand::random::<u64>();

    commands.insert_resource(crate::client::LocalClientId(client_id));

    let auth = Authentication::Manual {
        server_addr,
        client_id,
        private_key: settings.netcode_key,
        protocol_id: crate::common::net::NETCODE_PROTOCOL_ID,
    };

    let netcode_config = NetcodeConfig {
        client_timeout_secs: 15,
        ..default()
    };

    let netcode_client = match NetcodeClient::new(auth, netcode_config) {
        Ok(c) => c,
        Err(e) => {
            error!("Failed to create network client: {e}");
            return;
        }
    };

    commands.spawn((
        Client::default(),
        UdpIo::default(),
        netcode_client,
        LocalAddr(SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), 0)),
        PeerAddr(server_addr),
    ));

    spawned.0 = true;
    info!(
        "CLIENT_CONNECT: Spawned Client entity with server_addr: {}, client_id: {}",
        server_addr, client_id
    );
}

pub fn trigger_delayed_connect(
    mut commands: Commands,
    mut frame_count: Local<u32>,
    client_query: Query<Entity, With<Client>>,
    mut connected: Local<bool>,
    ukey_res: Option<Res<crate::client::ClientUkey>>,
) {
    if *connected {
        return;
    }
    let Some(ukey) = ukey_res else {
        trace!("CLIENT_CONNECT: Waiting for ClientUkey resource to be inserted...");
        return;
    };
    if ukey.0.is_empty() {
        trace!("CLIENT_CONNECT: Waiting for valid ClientUkey details...");
        return;
    }
    *frame_count += 1;
    if *frame_count >= 30 {
        for entity in &client_query {
            commands.trigger(Connect { entity });
            info!("CLIENT_CONNECT: Handshake connection triggered after rendering warmup");
        }
        *connected = true;
    }
}
