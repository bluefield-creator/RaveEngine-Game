use super::ScriptVM;
use bevy::prelude::*;
use mlua::prelude::*;

#[derive(Resource, Deref, DerefMut)]
pub struct ClientScriptVM(pub ScriptVM);

impl ClientScriptVM {
    pub fn new() -> LuaResult<Self> {
        ScriptVM::new().map(Self)
    }
}
