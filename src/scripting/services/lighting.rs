use bevy::prelude::*;
use mlua::prelude::*;

#[derive(Clone, Copy)]
pub struct LightingService;

impl LuaUserData for LightingService {
    fn add_methods<M: LuaUserDataMethods<Self>>(methods: &mut M) {
        methods.add_meta_method(LuaMetaMethod::Index, |lua, _, key: String| {
            let world = unsafe { crate::scripting::vm::server_vm::world_from_lua(lua)? };

            match key.as_str() {
                "ClassName" => Ok(LuaValue::String(lua.create_string("Lighting")?)),
                "Name" => Ok(LuaValue::String(lua.create_string("Lighting")?)),
                "TimeOfDay" => {
                    let tod = world.get_resource::<crate::common::game::environment::lighting::LightingService>()
                        .map(|l| l.time_of_day)
                        .unwrap_or(12.0);
                    Ok(LuaValue::Number(tod as f64))
                }
                _ => {
                    if let Some(lighting_entity) = crate::scripting::userdata::instance::find_service_entity(world, "Lighting") {
                        let instance = crate::scripting::userdata::instance::Instance { entity: lighting_entity };
                        let instance_userdata = lua.create_userdata(instance)?;
                        let metatable: LuaUserDataMetatable = instance_userdata.metatable()?;
                        let index_fn: LuaFunction = metatable.get("__index")?;
                        index_fn.call::<LuaValue>((instance_userdata, key))
                    } else {
                        Ok(LuaValue::Nil)
                    }
                }
            }
        });

        methods.add_meta_method(LuaMetaMethod::NewIndex, |lua, _, (key, value): (String, LuaValue)| {
            let world = unsafe { crate::scripting::vm::server_vm::world_from_lua(lua)? };

            if key.as_str() == "TimeOfDay" {
                let opt_val = match value {
                    LuaValue::Number(n) => Some(n),
                    LuaValue::Integer(i) => Some(i as f64),
                    _ => None,
                };
                if let Some(val) = opt_val
                    && let Some(mut service) = world.get_resource_mut::<crate::common::game::environment::lighting::LightingService>() {
                        service.time_of_day = val as f32;
                        if let Ok(mut shared) = crate::studio::tools::SHARED_LIGHTING_SERVICE.write() {
                            *shared = val as f32;
                        }
                    }
            }
            Ok(())
        });
    }
}
