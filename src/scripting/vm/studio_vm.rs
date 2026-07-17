use bevy::prelude::*;
use mlua::prelude::*;
use std::sync::{Arc, Mutex};
use super::scheduler::{LuaScheduler, SchedulerRef, ScriptRegistryRef, ScriptRegistry};
use crate::scripting::runtime::require::ModuleCacheRef;

#[derive(Resource)]
pub struct StudioScriptVM {
    pub lua: Lua,
    pub scheduler: Arc<Mutex<LuaScheduler>>,
    pub registry: Arc<Mutex<ScriptRegistry>>,
}

impl StudioScriptVM {
    pub fn new() -> Self {
        let lua = Lua::new();

        let scheduler = Arc::new(Mutex::new(LuaScheduler::new()));
        lua.set_app_data(SchedulerRef(scheduler.clone()));

        let registry = Arc::new(Mutex::new(ScriptRegistry {
            connections: std::collections::HashMap::new(),
        }));
        lua.set_app_data(ScriptRegistryRef(registry.clone()));

        let module_cache = Arc::new(Mutex::new(crate::scripting::runtime::require::ModuleCache {
            cached_results: std::collections::HashMap::new(),
            loading_modules: std::collections::HashSet::new(),
        }));
        lua.set_app_data(ModuleCacheRef(module_cache));

        crate::scripting::runtime::globals::setup_globals(&lua).unwrap();
        crate::scripting::runtime::require::register_require(&lua).unwrap();

        Self {
            lua,
            scheduler,
            registry,
        }
    }
}