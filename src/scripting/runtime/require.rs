use crate::common::core::sync::LockExt;
use crate::scripting::userdata::instance::Instance;
use bevy::prelude::*;
use mlua::prelude::*;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

pub struct ModuleCache {
    pub cached_results: HashMap<Entity, LuaValue>,
    pub loading_modules: HashSet<Entity>,
}

pub struct ModuleCacheRef(pub Arc<Mutex<ModuleCache>>);

pub fn register_require(lua: &Lua) -> Result<(), mlua::Error> {
    let require_fn = lua.create_function(|lua, value: LuaValue| {
        let not_a_module = || {
            mlua::Error::RuntimeError(
                "require expects an Instance representing a ModuleScript".to_string(),
            )
        };
        let entity = match value {
            LuaValue::UserData(ref ud) => ud.borrow::<Instance>().map_err(|_| not_a_module())?.entity,
            _ => return Err(not_a_module()),
        };

        let world = unsafe { &*crate::scripting::vm::world_ptr(lua)? };
        let code = world
            .get::<crate::scripting::ecs::ModuleScript>(entity)
            .map(|module| module.code.clone())
            .ok_or_else(|| {
                mlua::Error::RuntimeError("Provided Instance is not a ModuleScript".to_string())
            })?;

        let cache_ref = crate::scripting::vm::app_data::<ModuleCacheRef>(lua)?.0.clone();
        {
            let mut cache = cache_ref.lock_or_recover();
            if let Some(val) = cache.cached_results.get(&entity) {
                return Ok(val.clone());
            }
            if !cache.loading_modules.insert(entity) {
                return Err(mlua::Error::RuntimeError(
                    "Cyclic require dependency detected for ModuleScript".to_string(),
                ));
            }
        }

        let chunk_name = format!("ModuleScript[{entity}]");
        let result =
            crate::scripting::vm::compiler::compile_script(lua, &code, &chunk_name, entity)
                .and_then(|func| {
                    crate::scripting::vm::sandbox::set_caller_frame(lua, chunk_name, None);
                    func.call::<LuaValue>(Instance { entity })
                });

        let mut cache = cache_ref.lock_or_recover();
        cache.loading_modules.remove(&entity);
        let res = result?;
        cache.cached_results.insert(entity, res.clone());
        Ok(res)
    })?;

    lua.globals().set("require", require_fn)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scripting::testing::{eval, test_vm, test_world};
    use crate::scripting::userdata::instance::Instance;

    fn spawn_module(world: &mut World, name: &str, code: &str) -> Entity {
        world
            .spawn((
                Name::new(name.to_string()),
                crate::scripting::ecs::ModuleScript {
                    code: code.to_string(),
                },
            ))
            .id()
    }

    fn require_of(
        vm: &crate::scripting::vm::server_vm::ServerScriptVM,
        entity: Entity,
    ) -> mlua::Result<LuaValue> {
        let require: LuaFunction = vm.lua.globals().get("require").unwrap();
        let inst = vm.lua.create_userdata(Instance { entity }).unwrap();
        require.call(inst)
    }

    #[test]
    fn module_results_are_cached() {
        let mut world = test_world();
        let module = spawn_module(
            &mut world,
            "M",
            "_G.runs = (_G.runs or 0) + 1 return { value = 42 }",
        );
        let vm = test_vm(&mut world);

        let inst = vm.lua.create_userdata(Instance { entity: module }).unwrap();
        let require: LuaFunction = vm.lua.globals().get("require").unwrap();
        require.call::<()>(inst.clone()).unwrap();
        require.call::<()>(inst.clone()).unwrap();

        assert_eq!(vm.lua.globals().get::<i32>("runs").unwrap(), 1);
        vm.lua.globals().set("m", inst).unwrap();
        let same: bool = eval(&vm, "return require(m) == require(m)");
        assert!(same);
    }

    #[test]
    fn require_returns_first_return_value() {
        let mut world = test_world();
        let module = spawn_module(&mut world, "M", "return 1, 2, 3");
        let vm = test_vm(&mut world);

        let res = require_of(&vm, module).unwrap();
        assert_eq!(res, LuaValue::Integer(1));
    }

    #[test]
    fn require_rejects_non_module_instances() {
        let mut world = test_world();
        let part = crate::scripting::testing::spawn_brick(&mut world, "Part");
        let vm = test_vm(&mut world);

        let err = require_of(&vm, part).unwrap_err();
        assert!(err.to_string().contains("not a ModuleScript"), "got: {err}");
    }

    #[test]
    fn require_rejects_non_instance_values() {
        let mut world = test_world();
        let vm = test_vm(&mut world);
        let ok: bool = eval(&vm, "return pcall(require, 123)");
        assert!(!ok);
    }

    #[test]
    fn failed_modules_are_not_cached_and_can_be_retried() {
        let mut world = test_world();
        let module = spawn_module(&mut world, "M", "error('first attempt fails')");
        let vm = test_vm(&mut world);

        assert!(require_of(&vm, module).is_err());

        world
            .entity_mut(module)
            .insert(crate::scripting::ecs::ModuleScript {
                code: "return 'fixed'".to_string(),
            });
        let res = require_of(&vm, module).unwrap();
        assert_eq!(
            res,
            LuaValue::String(vm.lua.create_string("fixed").unwrap())
        );
    }

    #[test]
    fn module_environments_are_isolated() {
        let mut world = test_world();
        let module = spawn_module(&mut world, "M", "module_side_effect = true return 'ok'");
        let vm = test_vm(&mut world);

        require_of(&vm, module).unwrap();

        assert_eq!(
            vm.lua
                .globals()
                .get::<LuaValue>("module_side_effect")
                .unwrap(),
            LuaValue::Nil
        );
    }

    #[test]
    fn module_sees_globals_through_environment() {
        let mut world = test_world();
        let module = spawn_module(&mut world, "M", "return task ~= nil and Vector3 ~= nil");
        let vm = test_vm(&mut world);
        let res = require_of(&vm, module).unwrap();
        assert_eq!(res, LuaValue::Boolean(true));
    }

    #[test]
    fn cyclic_require_is_rejected() {
        let mut world = test_world();
        let folder = world.spawn(Name::new("F")).id();
        let a = spawn_module(
            &mut world,
            "A",
            "return require(script.Parent:FindFirstChild('B'))",
        );
        let b = spawn_module(
            &mut world,
            "B",
            "return require(script.Parent:FindFirstChild('A'))",
        );
        world.entity_mut(folder).add_child(a);
        world.entity_mut(folder).add_child(b);

        let vm = test_vm(&mut world);
        let err = require_of(&vm, a).unwrap_err();
        assert!(err.to_string().contains("Cyclic require"), "got: {err}");
    }
}
