pub mod auth;
pub mod components;
pub mod messages;

use bevy::prelude::*;
use lightyear::prelude::*;

pub const NETCODE_PROTOCOL_ID: u64 = 1;
pub const NETCODE_KEY_ENV: &str = "VERTIGO_NETCODE_KEY";

const LOOPBACK_DEVELOPMENT_NETCODE_KEY: [u8; 32] = [0xa5; 32];

pub fn parse_netcode_key(value: &str) -> Result<[u8; 32], String> {
    if value.len() != 64 {
        return Err("Netcode key must contain exactly 64 hexadecimal characters".to_string());
    }

    let mut key = [0; 32];
    for (index, byte) in key.iter_mut().enumerate() {
        let offset = index * 2;
        *byte = u8::from_str_radix(&value[offset..offset + 2], 16)
            .map_err(|_| "Netcode key must contain only hexadecimal characters".to_string())?;
    }
    Ok(key)
}

pub fn resolve_netcode_key(
    configured: Option<&str>,
    allow_loopback_development_default: bool,
) -> Result<[u8; 32], String> {
    match configured {
        Some(value) => parse_netcode_key(value),
        None if allow_loopback_development_default => Ok(LOOPBACK_DEVELOPMENT_NETCODE_KEY),
        None => Err(format!(
            "A 32-byte Netcode key is required; set {NETCODE_KEY_ENV} to 64 hexadecimal characters"
        )),
    }
}

pub mod replicon {
    pub use bevy_replicon::prelude::*;
}

#[allow(dead_code)]
pub struct NetPlugin;

impl Plugin for NetPlugin {
    fn build(&self, _app: &mut App) {}
}

#[allow(dead_code)]
pub struct NetworkPlugin;

impl Plugin for NetworkPlugin {
    fn build(&self, app: &mut App) {
        register_protocol(app);
    }
}

pub struct ProtocolPlugin;

impl Plugin for ProtocolPlugin {
    fn build(&self, app: &mut App) {
        register_protocol(app);
    }
}

pub fn register_protocol(app: &mut App) {
    app.register_type::<components::Player>();
    app.register_type::<components::NetworkTransform>();
    app.register_type::<components::PlayersServiceContainer>();
    app.register_type::<components::LightingServiceContainer>();

    app.add_channel::<messages::GameChannel>(ChannelSettings {
        mode: ChannelMode::OrderedReliable(ReliableSettings::default()),
        ..default()
    })
    .add_direction(lightyear::prelude::NetworkDirection::ClientToServer)
    .add_direction(lightyear::prelude::NetworkDirection::ServerToClient);

    app.component::<components::Player>().replicate();
    app.component::<components::NetworkTransform>().replicate();
    app.component::<components::PlayersServiceContainer>()
        .replicate();
    app.component::<components::LightingServiceContainer>()
        .replicate();
    app.component::<crate::common::game::bricks::components::Brick>()
        .replicate();
    app.component::<crate::common::game::bricks::components::BrickShapeComponent>()
        .replicate();
    app.component::<crate::common::game::bricks::components::BrickPhysics>()
        .replicate();
    app.component::<crate::common::game::bricks::components::BrickColor>()
        .replicate();
    app.component::<crate::scripting::ecs::LocalScript>()
        .replicate();
    app.component::<crate::scripting::ecs::ModuleScript>()
        .replicate();

    app.register_message::<messages::PlayerInputMessage>()
        .add_direction(lightyear::prelude::NetworkDirection::ClientToServer);

    app.register_message::<messages::HelloMessage>()
        .add_direction(lightyear::prelude::NetworkDirection::ClientToServer);

    app.register_message::<messages::KickMessage>()
        .add_direction(lightyear::prelude::NetworkDirection::ServerToClient);

    app.register_message::<messages::AuthSuccessMessage>()
        .add_direction(lightyear::prelude::NetworkDirection::ServerToClient);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_32_byte_hex_key() {
        let key =
            parse_netcode_key("000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f")
                .unwrap();

        assert_eq!(key, core::array::from_fn(|index| index as u8));
    }

    #[test]
    fn rejects_invalid_netcode_keys() {
        assert!(parse_netcode_key("abcd").is_err());
        assert!(parse_netcode_key(&"z".repeat(64)).is_err());
    }

    #[test]
    fn development_default_is_loopback_only() {
        assert!(resolve_netcode_key(None, true).is_ok());
        assert!(resolve_netcode_key(None, false).is_err());
    }
}
