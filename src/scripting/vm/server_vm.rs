use super::ScriptVM;
use bevy::prelude::*;
use mlua::prelude::*;

pub struct WorldRef(pub *mut World);
unsafe impl Send for WorldRef {}
unsafe impl Sync for WorldRef {}

#[derive(Resource, Deref, DerefMut)]
pub struct ServerScriptVM(pub ScriptVM);

impl ServerScriptVM {
    pub fn new() -> LuaResult<Self> {
        ScriptVM::new().map(Self)
    }
}
