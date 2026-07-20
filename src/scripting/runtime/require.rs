use bevy::prelude::*;
use mlua::prelude::*;
use crate::scripting::userdata::instance::Instance;
use std::sync::{Arc, Mutex};
use std::collections::{HashMap, HashSet};

pub struct ModuleCache {
    pub cached_results: HashMap<Entity, LuaValue>,
    pub loading_modules: HashSet<Entity>,
}

pub struct ModuleCacheRef(pub Arc<Mutex<ModuleCache>>);

pub fn register_require(lua: &Lua) -> Result<(), mlua::Error> {
    let require_fn = lua.create_function(|lua, value: LuaValue| {
        let instance = match value {
            LuaValue::UserData(ref ud) => {
                if let Ok(inst) = ud.borrow::<Instance>() {
                    inst
                } else {
                    return Err(mlua::Error::RuntimeError("require expects an Instance representing a ModuleScript".to_string()));
                }
            }
            _ => return Err(mlua::Error::RuntimeError("require expects an Instance representing a ModuleScript".to_string())),
        };

        let world = unsafe { crate::scripting::vm::server_vm::world_from_lua_shared(lua)? };

        let module_comp = world.get::<crate::scripting::ecs::ModuleScript>(instance.entity)
            .ok_or_else(|| mlua::Error::RuntimeError("Provided Instance is not a ModuleScript".to_string()))?;

        let cache_ref = lua.app_data_ref::<crate::scripting::runtime::require::ModuleCacheRef>()
            .ok_or_else(|| mlua::Error::RuntimeError("ModuleCacheRef not set".into()))?;
        {
            let mut cache = cache_ref.0.lock().expect("ModuleCache lock poisoned");
            if let Some(val) = cache.cached_results.get(&instance.entity) {
                return Ok(val.clone());
            }
            if cache.loading_modules.contains(&instance.entity) {
                return Err(mlua::Error::RuntimeError("Cyclic require dependency detected for ModuleScript".to_string()));
            }
            cache.loading_modules.insert(instance.entity);
        }

        let code = module_comp.code.clone();
        let func = match crate::scripting::vm::compiler::compile_code(lua, &code, "ModuleScript") {
            Ok(f) => f,
            Err(e) => {
                let mut cache = cache_ref.0.lock().expect("ModuleCache lock poisoned");
                cache.loading_modules.remove(&instance.entity);
                return Err(e);
            }
        };

        let res = match func.call::<LuaValue>(()) {
            Ok(v) => v,
            Err(e) => {
                let mut cache = cache_ref.0.lock().expect("ModuleCache lock poisoned");
                cache.loading_modules.remove(&instance.entity);
                return Err(e);
            }
        };

        {
            let mut cache = cache_ref.0.lock().expect("ModuleCache lock poisoned");
            cache.loading_modules.remove(&instance.entity);
            cache.cached_results.insert(instance.entity, res.clone());
        }

        Ok(res)
    })?;

    lua.globals().set("require", require_fn)?;
    Ok(())
}