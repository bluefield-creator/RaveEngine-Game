use crate::app::common::args::CliArgs;
use crate::common::net::netcode::parse_hex_key;
use std::net::{IpAddr, Ipv4Addr};

pub struct ClientAppConfig {
    pub ip: IpAddr,
    pub port: u16,
    pub ukey: String,
    pub netcode_key: Option<[u8; 32]>,
    pub protocol_id: Option<u64>,
}

impl ClientAppConfig {
    pub fn from_env_and_args() -> Self {
        Self::from_args(&CliArgs::from_env())
    }

    pub fn from_args(args: &CliArgs) -> Self {
        let env = |name: &str| std::env::var(name).ok();

        let mut ip = env("VERTIGO_SERVER_IP")
            .and_then(|v| v.parse().ok())
            .unwrap_or(IpAddr::V4(Ipv4Addr::LOCALHOST));
        let mut port = env("VERTIGO_SERVER_PORT")
            .and_then(|v| v.parse().ok())
            .unwrap_or(5000);
        let mut ukey = env("VERTIGO_CLIENT_UKEY").unwrap_or_default();

        if let Some(join) = args
            .iter()
            .filter_map(crate::client::uri::parse_client_join_uri)
            .last()
        {
            ip = join.ip;
            port = join.port;
            ukey = join.ukey;
        }
        ip = args.parsed("--ip").unwrap_or(ip);
        port = args.parsed("--port").unwrap_or(port);
        if let Some(value) = args.value("--ukey") {
            ukey = value.to_string();
        }

        let netcode_key = args
            .value("--netcode-key")
            .and_then(parse_hex_key)
            .or_else(|| env("NETCODE_PRIVATE_KEY").as_deref().and_then(parse_hex_key));
        let protocol_id = args.parsed("--protocol-id").or_else(|| {
            env("NETCODE_PROTOCOL_ID").and_then(|v| v.trim().parse().ok())
        });

        Self {
            ip,
            port,
            ukey,
            netcode_key,
            protocol_id,
        }
    }

    pub fn resolved_netcode_key(&self) -> [u8; 32] {
        self.netcode_key
            .unwrap_or_else(|| crate::common::net::netcode::netcode_private_key(None))
    }

    pub fn resolved_protocol_id(&self) -> u64 {
        self.protocol_id
            .unwrap_or_else(|| crate::common::net::netcode::netcode_protocol_id(None))
    }
}
