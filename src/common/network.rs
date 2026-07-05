use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use lightyear::prelude::*;

pub struct GameChannel;

#[derive(Message, Serialize, Deserialize, Debug, Clone)]
pub struct PlayerInputMessage {
    pub w: bool,
    pub a: bool,
    pub s: bool,
    pub d: bool,
    pub jump: bool,
    pub yaw: f32,
    pub in_first_person: bool,
}

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
    app.register_type::<crate::common::components::Player>();
    app.register_type::<crate::common::components::NetworkTransform>();

    app.add_channel::<GameChannel>(ChannelSettings {
        mode: ChannelMode::OrderedReliable(ReliableSettings::default()),
        ..default()
    })
    .add_direction(lightyear::prelude::NetworkDirection::ClientToServer);

    app.component::<crate::common::components::Player>().replicate();
    app.component::<crate::common::components::NetworkTransform>().replicate();
    app.component::<crate::common::bricks::components::Brick>().replicate();
    app.component::<crate::common::bricks::components::BrickShapeComponent>().replicate();
    app.component::<crate::common::bricks::components::BrickPhysics>().replicate();
    app.component::<crate::common::bricks::components::BrickColor>().replicate();

    app.register_message::<PlayerInputMessage>()
        .add_direction(lightyear::prelude::NetworkDirection::ClientToServer);
}