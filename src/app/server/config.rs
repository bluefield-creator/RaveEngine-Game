use std::net::{IpAddr, Ipv4Addr};

pub struct ServerAppConfig {
    pub port: u16,
    pub map_path: String,
    pub bind_ip: IpAddr,
    pub netcode_key: [u8; 32],
    pub embedded_server: bool,
}

impl ServerAppConfig {
    pub fn from_env_and_args() -> Self {
        let mut port = 5000; //default
        let mut map_path = "assets/maps/temp_playtest.vrtx".to_string();
        let mut bind_ip = IpAddr::V4(Ipv4Addr::UNSPECIFIED);
        let mut configured_key = std::env::var(crate::common::net::NETCODE_KEY_ENV).ok();

        if let Ok(value) = std::env::var("VERTIGO_SERVER_BIND_IP")
            && let Ok(value) = value.parse()
        {
            bind_ip = value;
        }

        let args: Vec<String> = std::env::args().collect();
        for i in 0..args.len() {
            if args[i] == "--port"
                && i + 1 < args.len()
                && let Ok(p) = args[i + 1].parse::<u16>()
            {
                port = p;
            }
            if args[i] == "--map" && i + 1 < args.len() {
                map_path = args[i + 1].clone();
            }
            if args[i] == "--bind-ip"
                && i + 1 < args.len()
                && let Ok(value) = args[i + 1].parse()
            {
                bind_ip = value;
            }
            if args[i] == "--netcode-key" && i + 1 < args.len() {
                configured_key = Some(args[i + 1].clone());
            }
        }

        let netcode_key = crate::common::net::resolve_netcode_key(
            configured_key.as_deref(),
            bind_ip.is_loopback(),
        )
        .unwrap_or_else(|error| panic!("Invalid server Netcode configuration: {error}"));

        Self {
            port,
            map_path,
            bind_ip,
            netcode_key,
            embedded_server: false,
        }
    }
}
