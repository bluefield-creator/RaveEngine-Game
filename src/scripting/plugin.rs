use crate::scripting::ecs::{LocalScript, ModuleScript, ServerScript};
use crate::scripting::vm::client_vm::ClientScriptVM;
use crate::scripting::vm::server_vm::ServerScriptVM;
use avian3d::prelude::CollidingEntities;
use bevy::prelude::*;
use mlua::prelude::*;
use std::ops::Deref;

struct RemovedVm<T: Resource> {
    world: *mut World,
    vm: Option<T>,
}

impl<T: Resource> RemovedVm<T> {
    fn take(world: &mut World) -> Option<Self> {
        world.remove_resource().map(|vm| Self {
            world,
            vm: Some(vm),
        })
    }
}

impl<T: Resource> Deref for RemovedVm<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.vm.as_ref().unwrap()
    }
}

impl<T: Resource> Drop for RemovedVm<T> {
    fn drop(&mut self) {
        let vm = self.vm.take().unwrap();
        unsafe { &mut *self.world }.insert_resource(vm);
    }
}

pub struct ScriptingPlugin;

impl Plugin for ScriptingPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<crate::scripting::vm::scheduler::ServiceEntities>()
            .register_type::<ServerScript>()
            .register_type::<LocalScript>()
            .register_type::<ModuleScript>()
            .add_systems(
                Update,
                (
                    discover_and_run_server_scripts,
                    discover_and_run_local_scripts,
                    detect_touched_collisions,
                    detect_player_added_events,
                    trigger_run_service_events,
                    server_scheduler_system,
                    client_scheduler_system,
                )
                    .chain(),
            )
            .add_systems(PostUpdate, cache_service_entities);
    }
}

fn spawn_and_run_callback(
    lua: &Lua,
    scheduler: &std::sync::Arc<std::sync::Mutex<crate::scripting::vm::scheduler::LuaScheduler>>,
    func: LuaFunction,
    arg: LuaValue,
) {
    if let Ok(thread) = lua.create_thread(func) {
        match thread.resume::<Option<f32>>(arg) {
            Ok(yielded_val) => {
                if thread.status() == LuaThreadStatus::Resumable {
                    let wake = yielded_val.and_then(|seconds| {
                        crate::scripting::vm::scheduler::checked_wake_time(
                            std::time::Instant::now(),
                            seconds,
                        )
                    });
                    let mut sched = scheduler.lock().expect("Lua scheduler lock poisoned");
                    if let Ok(key) = lua.create_registry_value(thread) {
                        sched.tasks.push(crate::scripting::vm::scheduler::LuaTask {
                            thread_key: key,
                            wake_time: wake,
                        });
                    }
                }
            }
            Err(e) => {
                error!("Luau callback runtime error: {}", e);
            }
        }
    }
}

fn registered_functions(
    lua: &Lua,
    registry: &std::sync::Arc<std::sync::Mutex<crate::scripting::vm::scheduler::ScriptRegistry>>,
    entity: Entity,
    event: &'static str,
) -> Vec<LuaFunction> {
    let keys = registry
        .lock()
        .expect("ScriptRegistry lock poisoned")
        .connections
        .get(&(entity, event))
        .cloned()
        .unwrap_or_default();
    keys.iter()
        .filter_map(|key| lua.registry_value::<LuaFunction>(key).ok())
        .collect()
}

fn entered_contacts(
    previous: &mut std::collections::HashSet<(Entity, Entity)>,
    collisions: impl IntoIterator<Item = (Entity, Entity)>,
) -> Vec<(Entity, Entity)> {
    let current = collisions
        .into_iter()
        .map(|(entity, other)| {
            if entity < other {
                (entity, other)
            } else {
                (other, entity)
            }
        })
        .collect::<std::collections::HashSet<_>>();
    let entered = current.difference(previous).copied().collect();
    *previous = current;
    entered
}

fn dispatch_touched(
    lua: &Lua,
    scheduler: &std::sync::Arc<std::sync::Mutex<crate::scripting::vm::scheduler::LuaScheduler>>,
    registry: &std::sync::Arc<std::sync::Mutex<crate::scripting::vm::scheduler::ScriptRegistry>>,
    contacts: &[(Entity, Entity)],
) {
    for &(entity, other) in contacts {
        for (target, argument) in [(entity, other), (other, entity)] {
            for function in registered_functions(lua, registry, target, "Touched") {
                if let Ok(instance) =
                    lua.create_userdata(crate::scripting::userdata::instance::Instance {
                        entity: argument,
                    })
                {
                    spawn_and_run_callback(lua, scheduler, function, LuaValue::UserData(instance));
                }
            }
        }
    }
}

pub fn server_scheduler_system(world: &mut World) {
    if let Some(server_vm) = RemovedVm::<ServerScriptVM>::take(world) {
        let _access =
            crate::scripting::vm::server_vm::WorldAccess::new(&server_vm.lua, world as *mut World);
        crate::scripting::vm::scheduler::LuaScheduler::run_tick(
            &server_vm.scheduler,
            &server_vm.lua,
        );
    }
}

pub fn client_scheduler_system(world: &mut World) {
    if let Some(client_vm) = RemovedVm::<ClientScriptVM>::take(world) {
        let _access =
            crate::scripting::vm::server_vm::WorldAccess::new(&client_vm.lua, world as *mut World);
        crate::scripting::vm::scheduler::LuaScheduler::run_tick(
            &client_vm.scheduler,
            &client_vm.lua,
        );
    }
}

pub fn discover_and_run_server_scripts(world: &mut World) {
    let mut scripts_to_run = Vec::new();
    let mut query = world.query::<(Entity, &mut ServerScript)>();
    for (entity, mut script) in query.iter_mut(world) {
        if !script.started && script.enabled {
            scripts_to_run.push((entity, script.code.clone()));
            script.started = true;
        }
    }

    if scripts_to_run.is_empty() {
        return;
    }

    if let Some(server_vm) = RemovedVm::<ServerScriptVM>::take(world) {
        let _access =
            crate::scripting::vm::server_vm::WorldAccess::new(&server_vm.lua, world as *mut World);

        for (entity, code) in scripts_to_run {
            match crate::scripting::vm::compiler::compile_code(
                &server_vm.lua,
                &code,
                "ServerScript",
            ) {
                Ok(func) => {
                    let script_env = match server_vm.lua.create_table() {
                        Ok(t) => t,
                        Err(_) => continue,
                    };
                    let meta = match server_vm.lua.create_table() {
                        Ok(m) => m,
                        Err(_) => continue,
                    };
                    let globals = server_vm.lua.globals();
                    let _ = meta.set("__index", globals);
                    let _ = script_env.set_metatable(Some(meta));

                    let script_instance = crate::scripting::userdata::instance::Instance { entity };
                    let _ = script_env.set("script", script_instance);

                    let _ = func.set_environment(script_env);

                    match server_vm.lua.create_thread(func) {
                        Ok(thread) => match thread.resume::<Option<f32>>(()) {
                            Ok(yielded_val) => {
                                if thread.status() == LuaThreadStatus::Resumable {
                                    let wake = yielded_val.and_then(|seconds| {
                                        crate::scripting::vm::scheduler::checked_wake_time(
                                            std::time::Instant::now(),
                                            seconds,
                                        )
                                    });
                                    let mut scheduler = server_vm
                                        .scheduler
                                        .lock()
                                        .expect("Lua scheduler lock poisoned");
                                    if let Ok(key) = server_vm.lua.create_registry_value(thread) {
                                        scheduler.tasks.push(
                                            crate::scripting::vm::scheduler::LuaTask {
                                                thread_key: key,
                                                wake_time: wake,
                                            },
                                        );
                                    }
                                }
                            }
                            Err(e) => {
                                error!("Luau script runtime error: {}", e);
                            }
                        },
                        Err(e) => {
                            error!("Failed to create thread for ServerScript: {}", e);
                        }
                    }
                }
                Err(e) => {
                    error!("Luau script compile error: {}", e);
                }
            }
        }
    }
}

pub fn discover_and_run_local_scripts(world: &mut World) {
    let mut scripts_to_run = Vec::new();
    let mut query = world.query::<(Entity, &mut LocalScript)>();
    for (entity, mut script) in query.iter_mut(world) {
        if !script.started && script.enabled {
            scripts_to_run.push((entity, script.code.clone()));
            script.started = true;
        }
    }

    if scripts_to_run.is_empty() {
        return;
    }

    if let Some(client_vm) = RemovedVm::<ClientScriptVM>::take(world) {
        let _access =
            crate::scripting::vm::server_vm::WorldAccess::new(&client_vm.lua, world as *mut World);

        for (entity, code) in scripts_to_run {
            match crate::scripting::vm::compiler::compile_code(&client_vm.lua, &code, "LocalScript")
            {
                Ok(func) => {
                    let script_env = match client_vm.lua.create_table() {
                        Ok(t) => t,
                        Err(_) => continue,
                    };
                    let meta = match client_vm.lua.create_table() {
                        Ok(m) => m,
                        Err(_) => continue,
                    };
                    let globals = client_vm.lua.globals();
                    let _ = meta.set("__index", globals);
                    let _ = script_env.set_metatable(Some(meta));

                    let script_instance = crate::scripting::userdata::instance::Instance { entity };
                    let _ = script_env.set("script", script_instance);

                    let _ = func.set_environment(script_env);

                    match client_vm.lua.create_thread(func) {
                        Ok(thread) => match thread.resume::<Option<f32>>(()) {
                            Ok(yielded_val) => {
                                if thread.status() == LuaThreadStatus::Resumable {
                                    let wake = yielded_val.and_then(|seconds| {
                                        crate::scripting::vm::scheduler::checked_wake_time(
                                            std::time::Instant::now(),
                                            seconds,
                                        )
                                    });
                                    let mut scheduler = client_vm
                                        .scheduler
                                        .lock()
                                        .expect("Lua scheduler lock poisoned");
                                    if let Ok(key) = client_vm.lua.create_registry_value(thread) {
                                        scheduler.tasks.push(
                                            crate::scripting::vm::scheduler::LuaTask {
                                                thread_key: key,
                                                wake_time: wake,
                                            },
                                        );
                                    }
                                }
                            }
                            Err(e) => {
                                error!("Luau local script runtime error: {}", e);
                            }
                        },
                        Err(e) => {
                            error!("Failed to create thread for LocalScript: {}", e);
                        }
                    }
                }
                Err(e) => {
                    error!("Luau local script compile error: {}", e);
                }
            }
        }
    }
}

pub fn detect_touched_collisions(
    world: &mut World,
    mut previous_contacts: Local<std::collections::HashSet<(Entity, Entity)>>,
) {
    let mut collisions = Vec::new();
    let mut query = world.query::<(Entity, &CollidingEntities)>();
    for (entity, colliding) in query.iter(world) {
        for other in colliding.iter() {
            collisions.push((entity, *other));
        }
    }

    let entered = entered_contacts(&mut previous_contacts, collisions);
    if entered.is_empty() {
        return;
    }

    if let Some(server_vm) = RemovedVm::<ServerScriptVM>::take(world) {
        let _access =
            crate::scripting::vm::server_vm::WorldAccess::new(&server_vm.lua, world as *mut World);

        dispatch_touched(
            &server_vm.lua,
            &server_vm.scheduler,
            &server_vm.registry,
            &entered,
        );
    }

    if let Some(client_vm) = RemovedVm::<ClientScriptVM>::take(world) {
        let _access =
            crate::scripting::vm::server_vm::WorldAccess::new(&client_vm.lua, world as *mut World);

        dispatch_touched(
            &client_vm.lua,
            &client_vm.scheduler,
            &client_vm.registry,
            &entered,
        );
    }
}

pub fn detect_player_added_events(
    world: &mut World,
    mut last_players: Local<std::collections::HashSet<Entity>>,
) {
    let mut current_players = std::collections::HashSet::new();
    let mut query = world.query::<(Entity, &crate::common::net::components::Player)>();
    for (entity, _) in query.iter(world) {
        current_players.insert(entity);
    }

    let joined: Vec<Entity> = current_players.difference(&last_players).cloned().collect();
    *last_players = current_players;

    if joined.is_empty() {
        return;
    }

    if let Some(server_vm) = RemovedVm::<ServerScriptVM>::take(world) {
        let _access =
            crate::scripting::vm::server_vm::WorldAccess::new(&server_vm.lua, world as *mut World);

        let players_entity =
            crate::scripting::userdata::instance::find_service_entity(world, "Players")
                .unwrap_or(Entity::PLACEHOLDER);
        for function in registered_functions(
            &server_vm.lua,
            &server_vm.registry,
            players_entity,
            "PlayerAdded",
        ) {
            for &entity in &joined {
                if let Ok(instance) = server_vm
                    .lua
                    .create_userdata(crate::scripting::userdata::instance::Instance { entity })
                {
                    spawn_and_run_callback(
                        &server_vm.lua,
                        &server_vm.scheduler,
                        function.clone(),
                        LuaValue::UserData(instance),
                    );
                }
            }
        }
    }

    if let Some(client_vm) = RemovedVm::<ClientScriptVM>::take(world) {
        let _access =
            crate::scripting::vm::server_vm::WorldAccess::new(&client_vm.lua, world as *mut World);

        let players_entity =
            crate::scripting::userdata::instance::find_service_entity(world, "Players")
                .unwrap_or(Entity::PLACEHOLDER);
        for function in registered_functions(
            &client_vm.lua,
            &client_vm.registry,
            players_entity,
            "PlayerAdded",
        ) {
            for &entity in &joined {
                if let Ok(instance) = client_vm
                    .lua
                    .create_userdata(crate::scripting::userdata::instance::Instance { entity })
                {
                    spawn_and_run_callback(
                        &client_vm.lua,
                        &client_vm.scheduler,
                        function.clone(),
                        LuaValue::UserData(instance),
                    );
                }
            }
        }
    }
}

pub fn trigger_run_service_events(world: &mut World) {
    let delta_secs = {
        let time = world.resource::<Time>();
        time.delta_secs()
    };

    if let Some(server_vm) = RemovedVm::<ServerScriptVM>::take(world) {
        let _access =
            crate::scripting::vm::server_vm::WorldAccess::new(&server_vm.lua, world as *mut World);

        let workspace_entity =
            crate::scripting::userdata::instance::find_service_entity(world, "Workspace")
                .unwrap_or(Entity::PLACEHOLDER);
        for event in ["Heartbeat", "Stepped"] {
            for function in
                registered_functions(&server_vm.lua, &server_vm.registry, workspace_entity, event)
            {
                spawn_and_run_callback(
                    &server_vm.lua,
                    &server_vm.scheduler,
                    function,
                    LuaValue::Number(delta_secs as f64),
                );
            }
        }
    }

    if let Some(client_vm) = RemovedVm::<ClientScriptVM>::take(world) {
        let _access =
            crate::scripting::vm::server_vm::WorldAccess::new(&client_vm.lua, world as *mut World);

        let workspace_entity =
            crate::scripting::userdata::instance::find_service_entity(world, "Workspace")
                .unwrap_or(Entity::PLACEHOLDER);
        for event in ["Heartbeat", "Stepped"] {
            for function in
                registered_functions(&client_vm.lua, &client_vm.registry, workspace_entity, event)
            {
                spawn_and_run_callback(
                    &client_vm.lua,
                    &client_vm.scheduler,
                    function,
                    LuaValue::Number(delta_secs as f64),
                );
            }
        }
    }
}

fn cache_service_entities(
    mut cache: ResMut<crate::scripting::vm::scheduler::ServiceEntities>,
    query: Query<(Entity, &Name)>,
) {
    if cache.workspace.is_some_and(
        |entity| !matches!(query.get(entity), Ok((_, name)) if name.as_str() == "Workspace"),
    ) {
        cache.workspace = None;
    }
    if cache.players.is_some_and(
        |entity| !matches!(query.get(entity), Ok((_, name)) if name.as_str() == "Players"),
    ) {
        cache.players = None;
    }
    if cache.lighting.is_some_and(
        |entity| !matches!(query.get(entity), Ok((_, name)) if name.as_str() == "Lighting"),
    ) {
        cache.lighting = None;
    }

    if cache.workspace.is_some() && cache.players.is_some() && cache.lighting.is_some() {
        return;
    }

    for (entity, name) in &query {
        match name.as_str() {
            "Workspace" if cache.workspace.is_none() => cache.workspace = Some(entity),
            "Players" if cache.players.is_none() => cache.players = Some(entity),
            "Lighting" if cache.lighting.is_none() => cache.lighting = Some(entity),
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scripting::vm::scheduler::ServiceEntities;

    fn cache_app() -> App {
        let mut app = App::new();
        app.init_resource::<ServiceEntities>()
            .add_systems(Update, cache_service_entities);
        app
    }

    #[test]
    fn discovers_service_entities() {
        let mut app = cache_app();
        let workspace = app.world_mut().spawn(Name::new("Workspace")).id();
        let players = app.world_mut().spawn(Name::new("Players")).id();
        let lighting = app.world_mut().spawn(Name::new("Lighting")).id();

        app.update();

        let cache = app.world().resource::<ServiceEntities>();
        assert_eq!(cache.workspace, Some(workspace));
        assert_eq!(cache.players, Some(players));
        assert_eq!(cache.lighting, Some(lighting));
    }

    #[test]
    fn refreshes_renamed_and_recreated_services() {
        let mut app = cache_app();
        let old_workspace = app.world_mut().spawn(Name::new("Workspace")).id();
        let old_players = app.world_mut().spawn(Name::new("Players")).id();
        app.world_mut().spawn(Name::new("Lighting"));
        app.update();

        app.world_mut()
            .entity_mut(old_workspace)
            .insert(Name::new("Renamed"));
        app.world_mut().despawn(old_players);
        let workspace = app.world_mut().spawn(Name::new("Workspace")).id();
        let players = app.world_mut().spawn(Name::new("Players")).id();
        app.update();

        let cache = app.world().resource::<ServiceEntities>();
        assert_eq!(cache.workspace, Some(workspace));
        assert_eq!(cache.players, Some(players));
    }

    #[test]
    fn touched_contacts_only_enter_on_new_edges_and_reset_when_empty() {
        let mut world = World::new();
        let first = world.spawn_empty().id();
        let second = world.spawn_empty().id();
        let pair = if first < second {
            (first, second)
        } else {
            (second, first)
        };
        let mut previous = std::collections::HashSet::new();

        assert_eq!(
            entered_contacts(&mut previous, [(second, first), (first, second)]),
            vec![pair]
        );
        assert!(entered_contacts(&mut previous, [(first, second)]).is_empty());
        assert!(entered_contacts(&mut previous, []).is_empty());
        assert_eq!(
            entered_contacts(&mut previous, [(first, second)]),
            vec![pair]
        );
    }
}
