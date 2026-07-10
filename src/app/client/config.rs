use std::net::IpAddr;

pub struct ClientAppConfig {
    pub ip: IpAddr,
    pub port: u16,
    pub ukey: String,
}

impl ClientAppConfig {
    pub fn from_env_and_args() -> Self {
        let mut ip = IpAddr::V4(std::net::Ipv4Addr::new(127, 0, 0, 1));
        let mut port = 5000;
        let mut ukey = "".to_string();

        if let Ok(env_ukey) = std::env::var("VERTIGO_CLIENT_UKEY") {
            ukey = env_ukey;
        }
        if let Ok(ip_str) = std::env::var("VERTIGO_SERVER_IP") {
            if let Ok(parsed_ip) = ip_str.parse::<IpAddr>() {
                ip = parsed_ip;
            }
        }
        if let Ok(port_str) = std::env::var("VERTIGO_SERVER_PORT") {
            if let Ok(parsed_port) = port_str.parse::<u16>() {
                port = parsed_port;
            }
        }

        let args: Vec<String> = std::env::args().collect();
        for i in 0..args.len() {
            if args[i] == "--port" && i + 1 < args.len() {
                if let Ok(p) = args[i + 1].parse::<u16>() {
                    port = p;
                }
            }
            if args[i] == "--ip" && i + 1 < args.len() {
                if let Ok(ip_addr) = args[i + 1].parse::<IpAddr>() {
                    ip = ip_addr;
                }
            }
            if args[i] == "--ukey" && i + 1 < args.len() {
                ukey = args[i + 1].clone();
            }
        }

        Self { ip, port, ukey }
    }
}