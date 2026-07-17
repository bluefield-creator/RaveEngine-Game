use mlua::prelude::*;

pub fn compile_code(lua: &Lua, code: &str, name: &str) -> Result<LuaFunction, mlua::Error> {
    lua.load(code).set_name(name).into_function()
}