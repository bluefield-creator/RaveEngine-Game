use crate::common::net::netcode::{
    netcode_private_key, netcode_protocol_id, parse_hex_key, spawn_netcode_client,
};
use bevy::prelude::*;
use lightyear::prelude::client::*;
use lightyear::prelude::*;
use serde::Deserialize;
use std::net::{IpAddr, SocketAddr};

pub const CONNECT_WARMUP_FRAMES: u32 = 30;

#[derive(Resource)]
pub struct ClientConnectSettings {
    pub ip: IpAddr,
    pub port: u16,
    pub netcode_key: Option<[u8; 32]>,
    pub protocol_id: Option<u64>,
}

impl ClientConnectSettings {
    pub fn server_addr(&self) -> SocketAddr {
        SocketAddr::new(self.ip, self.port)
    }

    pub fn private_key(&self) -> [u8; 32] {
        self.netcode_key.unwrap_or_else(|| netcode_private_key(None))
    }

    pub fn protocol_id(&self) -> u64 {
        self.protocol_id.unwrap_or_else(|| netcode_protocol_id(None))
    }
}

#[derive(Resource, Default)]
pub struct ClientSpawned(pub bool);

#[derive(Deserialize)]
pub struct LaunchInfo {
    pub ip: String,
    pub port: u16,
    pub ukey: String,
    #[serde(default)]
    pub netcode_key: Option<String>,
    #[serde(default)]
    pub protocol_id: Option<u64>,
}

pub fn has_client_ukey(ukey: Option<Res<crate::client::ClientUkey>>) -> bool {
    ukey.is_some_and(|ukey| !ukey.0.is_empty())
}

pub fn spawn_game_client(
    commands: &mut Commands,
    server_addr: SocketAddr,
    private_key: [u8; 32],
    protocol_id: u64,
    extra: impl Bundle,
) -> Option<Entity> {
    let client_id = rand::random::<u64>();
    commands.insert_resource(crate::client::LocalClientId(client_id));
    let entity =
        spawn_netcode_client(commands, server_addr, client_id, private_key, protocol_id, extra)?;
    info!(
        "CLIENT_CONNECT: Spawned Client entity with server_addr: {}, client_id: {}",
        server_addr, client_id
    );
    Some(entity)
}

pub fn setup_client(mut commands: Commands, settings: Res<ClientConnectSettings>) {
    spawn_game_client(
        &mut commands,
        settings.server_addr(),
        settings.private_key(),
        settings.protocol_id(),
        (),
    );
}

fn read_launch_info(launch_path: &std::path::Path) -> Option<LaunchInfo> {
    let content = std::fs::read_to_string(launch_path)
        .or_else(|_| std::fs::read_to_string("launch_info.json"))
        .ok()?;
    serde_json::from_str(&content).ok()
}

pub fn poll_launch_details(
    mut commands: Commands,
    ukey_res: Option<Res<crate::client::ClientUkey>>,
    mut settings: Option<ResMut<ClientConnectSettings>>,
    mut frame_counter: Local<u32>,
) {
    if ukey_res.is_some_and(|ukey| !ukey.0.is_empty()) {
        return;
    }

    *frame_counter += 1;
    if *frame_counter % 10 != 0 {
        return;
    }

    let launch_path = crate::common::assets_path::resolve_launch_info_path();
    let Some(info) = read_launch_info(&launch_path) else {
        return;
    };
    let Ok(ip) = info.ip.parse::<IpAddr>() else {
        return;
    };

    let loaded = ClientConnectSettings {
        ip,
        port: info.port,
        netcode_key: info.netcode_key.as_deref().and_then(parse_hex_key),
        protocol_id: info.protocol_id,
    };
    commands.insert_resource(crate::client::ClientUkey(info.ukey));
    match settings.as_deref_mut() {
        Some(settings) => *settings = loaded,
        None => commands.insert_resource(loaded),
    }
    info!("CLIENT_CONNECT: Successfully loaded connection details from launch_info.json");
    let _ = std::fs::remove_file(&launch_path);
    let _ = std::fs::remove_file("launch_info.json");
}

pub fn initialize_client(
    mut commands: Commands,
    settings: Res<ClientConnectSettings>,
    mut spawned: ResMut<ClientSpawned>,
) {
    if spawned.0 {
        return;
    }
    spawned.0 = true;
    spawn_game_client(
        &mut commands,
        settings.server_addr(),
        settings.private_key(),
        settings.protocol_id(),
        Transport::new(PriorityConfig::default()).with_compression(CompressionConfig::LZ4),
    );
}

pub fn connect_after_warmup(
    mut commands: Commands,
    mut frame_count: Local<u32>,
    client_query: Query<Entity, With<Client>>,
    mut connected: Local<bool>,
) {
    if *connected {
        return;
    }
    *frame_count += 1;
    if *frame_count < CONNECT_WARMUP_FRAMES {
        return;
    }
    for entity in &client_query {
        commands.trigger(Connect { entity });
        info!("CLIENT_CONNECT: Handshake connection triggered after rendering warmup");
    }
    *connected = true;
}
