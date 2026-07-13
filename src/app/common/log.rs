use bevy::log::LogPlugin;

pub fn setup_app_logging(app_name: &str) -> LogPlugin {
    let rust_log = std::env::var("RUST_LOG").unwrap_or_default();

    let default_filter = match app_name {
        "studio" => "info,wgpu=warn,naga=warn,wgpu_hal=warn,wgpu_core=warn,offset_allocator=off",
        _ => "debug,wgpu=error,bevy_render=error,bevy_ecs=warn,lightyear=debug,lightyear_udp=trace,lightyear_netcode=trace,naga=warn,wgpu_hal=warn,wgpu_core=warn,offset_allocator=off",
    };

    let new_rust_log = if rust_log.is_empty() {
        default_filter.to_string()
    } else if !rust_log.contains("offset_allocator") {
        format!("{rust_log},offset_allocator=off")
    } else {
        rust_log
    };

    unsafe {
        std::env::set_var("VERTIGO_APP", app_name);
        std::env::set_var("RUST_LOG", new_rust_log);
    }

    let level = match app_name {
        "studio" => bevy::log::Level::INFO,
        _ => bevy::log::Level::DEBUG,
    };

    LogPlugin {
        level,
        filter: default_filter.to_string(),
        ..Default::default()
    }
}