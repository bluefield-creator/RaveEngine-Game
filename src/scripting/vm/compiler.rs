use crate::scripting::vm::sandbox::MAX_SCRIPT_CODE_BYTES;
use mlua::prelude::*;

pub fn compile_code(lua: &Lua, code: &str, name: &str) -> Result<LuaFunction, mlua::Error> {
    if code.len() > MAX_SCRIPT_CODE_BYTES {
        return Err(mlua::Error::RuntimeError(format!(
            "script '{name}' exceeds the size limit of {MAX_SCRIPT_CODE_BYTES} bytes"
        )));
    }
    lua.load(code).set_name(name).into_function()
}

pub fn compile_script(
    lua: &Lua,
    code: &str,
    name: &str,
    entity: bevy::prelude::Entity,
) -> Result<LuaFunction, mlua::Error> {
    let func = compile_code(lua, code, name)?;
    let meta = lua.create_table()?;
    meta.set("__index", lua.globals())?;
    let script_env = lua.create_table()?;
    script_env.set_metatable(Some(meta))?;
    script_env.set(
        "script",
        crate::scripting::userdata::instance::Instance { entity },
    )?;
    func.set_environment(script_env)?;
    Ok(func)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scripting::testing::{eval, test_vm, test_world};

    #[test]
    fn compiles_valid_code() {
        let mut world = test_world();
        let vm = test_vm(&mut world);
        let func = compile_code(&vm.lua, "return 42", "t").unwrap();
        assert_eq!(func.call::<i32>(()).unwrap(), 42);
    }

    #[test]
    fn rejects_invalid_syntax() {
        let mut world = test_world();
        let vm = test_vm(&mut world);
        let err = compile_code(&vm.lua, "this is not lua", "Broken").unwrap_err();
        assert!(
            err.to_string().contains("Broken"),
            "trace should name the chunk: {err}"
        );
    }

    #[test]
    fn rejects_runtime_errors_at_call_time() {
        let mut world = test_world();
        let vm = test_vm(&mut world);
        let func = compile_code(&vm.lua, "error('kaboom')", "Boomer").unwrap();
        let err = func.call::<()>(()).unwrap_err();
        assert!(err.to_string().contains("kaboom"));
    }

    #[test]
    fn eval_roundtrip() {
        let mut world = test_world();
        let vm = test_vm(&mut world);
        let v: f64 = eval(&vm, "return 3 * 3");
        assert_eq!(v, 9.0);
    }
}
