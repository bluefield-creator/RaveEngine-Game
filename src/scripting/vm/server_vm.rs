use super::scheduler::{LuaScheduler, SchedulerRef, ScriptRegistry, ScriptRegistryRef};
use crate::scripting::runtime::require::ModuleCacheRef;
use bevy::prelude::*;
use mlua::prelude::*;
use std::sync::{Arc, Mutex};

#[derive(Resource)]
pub struct ServerScriptVM {
    pub lua: Lua,
    pub scheduler: Arc<Mutex<LuaScheduler>>,
    pub registry: Arc<Mutex<ScriptRegistry>>,
}

pub struct WorldRef(pub *mut World);

pub fn world_ptr_from_lua(lua: &Lua) -> *mut World {
    *lua.app_data_ref::<usize>()
        .expect("WorldRef not set in Lua app data") as *mut World
}

#[allow(clippy::missing_safety_doc)]
pub unsafe fn world_from_lua(lua: &Lua) -> Result<&mut World, mlua::Error> {
    Ok(unsafe { &mut *world_ptr_from_lua(lua) })
}

#[allow(clippy::missing_safety_doc)]
pub unsafe fn world_from_lua_shared(lua: &Lua) -> Result<&World, mlua::Error> {
    Ok(unsafe { &*world_ptr_from_lua(lua) })
}

impl Default for ServerScriptVM {
    fn default() -> Self {
        Self::new()
    }
}

impl ServerScriptVM {
    pub fn new() -> Self {
        let lua = Lua::new();

        let scheduler = Arc::new(Mutex::new(LuaScheduler::new()));
        lua.set_app_data(SchedulerRef(scheduler.clone()));

        let registry = Arc::new(Mutex::new(ScriptRegistry {
            connections: std::collections::HashMap::new(),
        }));
        lua.set_app_data(ScriptRegistryRef(registry.clone()));

        let module_cache = Arc::new(Mutex::new(
            crate::scripting::runtime::require::ModuleCache {
                cached_results: std::collections::HashMap::new(),
                loading_modules: std::collections::HashSet::new(),
            },
        ));
        lua.set_app_data(ModuleCacheRef(module_cache));

        crate::scripting::runtime::globals::setup_globals(&lua).unwrap();
        crate::scripting::runtime::require::register_require(&lua).unwrap();

        lua.globals()
            .set(
                "workspace",
                crate::scripting::services::workspace::WorkspaceService,
            )
            .unwrap();
        lua.globals()
            .set(
                "Workspace",
                crate::scripting::services::workspace::WorkspaceService,
            )
            .unwrap();
        lua.globals()
            .set(
                "Players",
                crate::scripting::services::players::PlayersService,
            )
            .unwrap();
        lua.globals()
            .set(
                "Lighting",
                crate::scripting::services::lighting::LightingService,
            )
            .unwrap();
        lua.globals()
            .set(
                "RunService",
                crate::scripting::services::run_service::RunService,
            )
            .unwrap();

        let game_table = lua.create_table().unwrap();
        game_table
            .set(
                "Workspace",
                crate::scripting::services::workspace::WorkspaceService,
            )
            .unwrap();
        game_table
            .set(
                "workspace",
                crate::scripting::services::workspace::WorkspaceService,
            )
            .unwrap();
        game_table
            .set(
                "Players",
                crate::scripting::services::players::PlayersService,
            )
            .unwrap();
        game_table
            .set(
                "Lighting",
                crate::scripting::services::lighting::LightingService,
            )
            .unwrap();
        game_table
            .set(
                "RunService",
                crate::scripting::services::run_service::RunService,
            )
            .unwrap();
        game_table
            .set(
                "GetService",
                lua.create_function(|lua, service_name: String| match service_name.as_str() {
                    "Workspace" | "workspace" => Ok(LuaValue::UserData(lua.create_userdata(
                        crate::scripting::services::workspace::WorkspaceService,
                    )?)),
                    "Players" => Ok(LuaValue::UserData(
                        lua.create_userdata(crate::scripting::services::players::PlayersService)?,
                    )),
                    "Lighting" => Ok(LuaValue::UserData(lua.create_userdata(
                        crate::scripting::services::lighting::LightingService,
                    )?)),
                    "RunService" => Ok(LuaValue::UserData(
                        lua.create_userdata(crate::scripting::services::run_service::RunService)?,
                    )),
                    _ => Err(mlua::Error::RuntimeError(format!(
                        "Service not found: {}",
                        service_name
                    ))),
                })
                .unwrap(),
            )
            .unwrap();
        lua.globals().set("game", game_table).unwrap();

        Self {
            lua,
            scheduler,
            registry,
        }
    }
}
