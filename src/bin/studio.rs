#[cfg(target_os = "android")] //Hey android, die!!
fn main() {}

#[cfg(not(target_os = "android"))]
mod desktop_studio {
    use RaveEngineLib::app::common::log::{STUDIO_FILTER, configure_logging};
    use RaveEngineLib::common::CommonPlugin;
    use RaveEngineLib::studio::StudioPlugin;
    use RaveEngineLib::studio::auth::StoredCredentials;
    use bevy::prelude::*;

    fn auth_code_from_uri(arg: &str) -> Option<String> {
        let lower = arg.to_lowercase();
        if !lower.starts_with("vertexia-studio://") && !lower.starts_with("vertexia://") {
            return None;
        }
        let query = arg.split('?').nth(1)?;
        query
            .split('&')
            .filter_map(|pair| pair.split_once('='))
            .filter(|(key, _)| *key == "code")
            .map(|(_, value)| value.to_string())
            .last()
    }

    fn exchange_auth_code(code: &str) -> Option<StoredCredentials> {
        let url = format!(
            "{}/api/v1/studio/exchange",
            RaveEngineLib::studio::auth::website_base()
        );
        let client =
            RaveEngineLib::common::net::api::blocking_client(std::time::Duration::from_secs(10))
                .ok()?;
        let resp = client
            .post(&url)
            .json(&serde_json::json!({ "code": code }))
            .send()
            .ok()?;
        if !resp.status().is_success() {
            return None;
        }
        let body: serde_json::Value = serde_json::from_str(&resp.text().ok()?).ok()?;
        Some(StoredCredentials {
            username: body.get("username")?.as_str()?.to_string(),
            uid: body.get("uid")?.as_i64()? as i32,
            ukey: body.get("ukey")?.as_str()?.to_string(),
        })
    }

    fn handle_auth_uri(code: &str) -> ! {
        match exchange_auth_code(code) {
            Some(creds) => {
                let _ = RaveEngineLib::studio::auth::save_studio_credentials(&creds);
                println!(
                    "VERTEXIA Studio authenticated as {} (uid {}) via URI",
                    creds.username, creds.uid
                );
                std::process::exit(0);
            }
            None => {
                eprintln!("Failed to exchange studio auth code via URI");
                std::process::exit(1);
            }
        }
    }

    pub fn main() {
        for arg in std::env::args().skip(1) {
            let lower = arg.to_lowercase();
            if lower == "--no-clouds" || lower == "--disable-clouds" {
                unsafe {
                    std::env::set_var("VERTIGO_NO_CLOUDS", "1");
                }
            }
            if let Some(code) = auth_code_from_uri(&arg).filter(|code| !code.is_empty()) {
                handle_auth_uri(&code);
            }
        }

        let log_plugin = configure_logging("studio", STUDIO_FILTER, bevy::log::Level::INFO);
        App::new()
            .insert_resource(bevy_egui::EguiGlobalSettings {
                auto_create_primary_context: false,
                ..default()
            })
            .add_plugins(
                DefaultPlugins
                    .set(log_plugin)
                    .set(RaveEngineLib::common::assets_path::asset_plugin())
                    .set(RaveEngineLib::app::common::render::render_plugin()),
            )
            .add_plugins(lightyear::prelude::client::ClientPlugins {
                tick_duration: core::time::Duration::from_secs_f64(1.0 / 60.0),
            })
            .add_plugins(CommonPlugin)
            .add_plugins(RaveEngineLib::client::ClientPlugin)
            .add_plugins(StudioPlugin)
            .run();
    }
}

#[cfg(not(target_os = "android"))]
fn main() {
    desktop_studio::main();
}
