use crate::app::common::args::CliArgs;
use std::net::{IpAddr, Ipv4Addr};

pub struct ServerAppConfig {
    pub port: u16,
    pub map_path: String,
    pub bind_addr: IpAddr,
    pub netcode_key: [u8; 32],
    pub protocol_id: u64,
    pub allow_unauthenticated: bool,
}

impl ServerAppConfig {
    pub fn from_env_and_args() -> Self {
        Self::from_args(&CliArgs::from_env())
    }

    pub fn from_args(args: &CliArgs) -> Self {
        let map_path = args.value("--map").map(str::to_string).unwrap_or_else(|| {
            crate::common::assets_path::resolve_vrtx_path("assets/maps/default.vrtx")
                .to_string_lossy()
                .into_owned()
        });

        Self {
            port: args.parsed("--port").unwrap_or(5000),
            map_path,
            bind_addr: args
                .parsed("--bind")
                .unwrap_or(IpAddr::V4(Ipv4Addr::LOCALHOST)),
            netcode_key: crate::common::net::netcode::netcode_private_key(
                args.value("--netcode-key"),
            ),
            protocol_id: crate::common::net::netcode::netcode_protocol_id(
                args.parsed("--protocol-id"),
            ),
            allow_unauthenticated: args.has("--allow-unauthenticated"),
        }
    }

    pub fn into_plugin(self) -> crate::server::ServerPlugin {
        crate::server::ServerPlugin {
            map_path: self.map_path,
            port: self.port,
            bind_addr: self.bind_addr,
            netcode_key: self.netcode_key,
            protocol_id: self.protocol_id,
            allow_unauthenticated: self.allow_unauthenticated,
        }
    }
}
