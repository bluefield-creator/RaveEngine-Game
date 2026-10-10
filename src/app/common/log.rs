use bevy::log::{Level, LogPlugin};

pub const VERBOSE_NET_FILTER: &str = "debug,wgpu=error,bevy_render=error,bevy_ecs=warn,lightyear=debug,lightyear_udp=trace,lightyear_netcode=trace,naga=warn,wgpu_hal=warn,wgpu_core=warn,offset_allocator=off";
pub const QUIET_NET_FILTER: &str = "info,wgpu=error,bevy_render=error,bevy_ecs=warn,lightyear=error,lightyear_udp=error,lightyear_netcode=error,naga=warn,wgpu_hal=warn,wgpu_core=warn,offset_allocator=off";
pub const STUDIO_FILTER: &str =
    "info,wgpu=warn,naga=warn,wgpu_hal=warn,wgpu_core=warn,offset_allocator=off";

pub fn configure_logging(app_name: &str, default_filter: &str, level: Level) -> LogPlugin {
    let rust_log = std::env::var("RUST_LOG").unwrap_or_default();
    let rust_log = if rust_log.is_empty() {
        default_filter.to_string()
    } else if !rust_log.contains("offset_allocator") {
        format!("{rust_log},offset_allocator=off")
    } else {
        rust_log
    };

    unsafe {
        std::env::set_var("VERTIGO_APP", app_name);
        std::env::set_var("RUST_LOG", rust_log);
    }

    LogPlugin {
        level,
        filter: default_filter.to_string(),
        ..Default::default()
    }
}

pub fn setup_app_logging(app_name: &str) -> LogPlugin {
    let default_filter = match app_name {
        "studio" => STUDIO_FILTER,
        _ => QUIET_NET_FILTER,
    };
    configure_logging(app_name, default_filter, Level::INFO)
}
