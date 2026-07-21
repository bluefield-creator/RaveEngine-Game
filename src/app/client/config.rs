use std::net::IpAddr;

pub struct ClientAppConfig {
    pub ip: IpAddr,
    pub port: u16,
    pub ukey: String,
    pub netcode_key: [u8; 32],
}

impl ClientAppConfig {
    pub fn from_env_and_args() -> Self {
        let mut ip = IpAddr::V4(std::net::Ipv4Addr::new(127, 0, 0, 1));
        let mut port = 5000;
        let mut ukey = "".to_string();
        let mut configured_key = std::env::var(crate::common::net::NETCODE_KEY_ENV).ok();

        if let Ok(env_ukey) = std::env::var("VERTIGO_CLIENT_UKEY") {
            ukey = env_ukey;
        }
        if let Ok(ip_str) = std::env::var("VERTIGO_SERVER_IP")
            && let Ok(parsed_ip) = ip_str.parse::<IpAddr>()
        {
            ip = parsed_ip;
        }
        if let Ok(port_str) = std::env::var("VERTIGO_SERVER_PORT")
            && let Ok(parsed_port) = port_str.parse::<u16>()
        {
            port = parsed_port;
        }

        let args: Vec<String> = std::env::args().collect();
        for i in 0..args.len() {
            if args[i] == "--port"
                && i + 1 < args.len()
                && let Ok(p) = args[i + 1].parse::<u16>()
            {
                port = p;
            }
            if args[i] == "--ip"
                && i + 1 < args.len()
                && let Ok(ip_addr) = args[i + 1].parse::<IpAddr>()
            {
                ip = ip_addr;
            }
            if args[i] == "--ukey" && i + 1 < args.len() {
                ukey = args[i + 1].clone();
            }
            if args[i] == "--netcode-key" && i + 1 < args.len() {
                configured_key = Some(args[i + 1].clone());
            }
        }

        let netcode_key =
            crate::common::net::resolve_netcode_key(configured_key.as_deref(), ip.is_loopback())
                .unwrap_or_else(|error| panic!("Invalid client Netcode configuration: {error}"));

        Self {
            ip,
            port,
            ukey,
            netcode_key,
        }
    }
}
