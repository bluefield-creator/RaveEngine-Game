use bevy::prelude::*;
use serde::{Deserialize, Serialize};

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