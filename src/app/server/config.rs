pub struct ServerAppConfig {
    pub port: u16,
    pub map_path: String,
}

impl ServerAppConfig {
    pub fn from_env_and_args() -> Self {
        let mut port = 5000; //default
        let mut map_path = "assets/maps/temp_playtest.vrtx".to_string();

        let args: Vec<String> = std::env::args().collect();
        for i in 0..args.len() {
            if args[i] == "--port" && i + 1 < args.len() {
                if let Ok(p) = args[i + 1].parse::<u16>() {
                    port = p;
                }
            }
            if args[i] == "--map" && i + 1 < args.len() {
                map_path = args[i + 1].clone();
            }
        }

        Self { port, map_path }
    }
}