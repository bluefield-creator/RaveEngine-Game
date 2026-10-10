use crate::common::core::sync::LockExt;
use crate::scripting::ecs::{LocalScript, ModuleScript, RunnableScript, ServerScript};
use crate::scripting::userdata::instance::Instance;
use crate::scripting::vm::ScriptVM;
use crate::scripting::vm::client_vm::ClientScriptVM;
use crate::scripting::vm::scheduler::{
    LuaScheduler, PreviousCollisions, ScriptRegistry, ServiceEntities, resume_and_schedule,
    scheduler_has_work,
};
use crate::scripting::vm::server_vm::ServerScriptVM;
use avian3d::prelude::CollidingEntities;
use bevy::prelude::*;
use mlua::prelude::*;
use std::collections::HashSet;
use std::ops::Deref;
use std::sync::{Arc, Mutex};

type Callback = (Arc<mlua::RegistryKey>, LuaFunction, LuaValue);

trait VmResource: Resource + Deref<Target = ScriptVM> {}

impl<T: Resource + Deref<Target = ScriptVM>> VmResource for T {}

pub struct ScriptingPlugin;

impl Plugin for ScriptingPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ServiceEntities>()
            .init_resource::<PreviousCollisions>()
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
                )
                    .chain(),
            )
            .add_systems(
                PostUpdate,
                (cache_service_entities, sweep_stale_connections),
            );
    }
}

fn with_vm<V: VmResource>(world: &mut World, f: impl FnOnce(&ScriptVM, &mut World)) {
    let Some(vm) = world.remove_resource::<V>() else {
        return;
    };
    vm.attach_world(world);
    f(&vm, world);
    world.insert_resource(vm);
}

fn instance_value(lua: &Lua, entity: Entity) -> LuaResult<LuaValue> {
    lua.create_userdata(Instance { entity })
        .map(LuaValue::UserData)
}

fn spawn_and_run_callback(
    lua: &Lua,
    scheduler: &Mutex<LuaScheduler>,
    func: LuaFunction,
    arg: LuaValue,
    callback_key: &Arc<mlua::RegistryKey>,
    source: &str,
) {
    {
        let sched = scheduler.lock_or_recover();
        if sched.active_callbacks.contains(callback_key) || !sched.has_capacity() {
            return;
        }
    }
    let Ok(thread) = lua.create_thread(func) else {
        return;
    };
    if let Err(e) = resume_and_schedule(
        lua,
        scheduler,
        thread,
        arg,
        source.to_string(),
        None,
        Some(callback_key),
    ) {
        error!("Luau callback runtime error: {}", e);
        crate::scripting::output::push_error("Event", e.to_string());
    }
}

fn fire_callbacks(vm: &ScriptVM, callbacks: Vec<Callback>, source: &str) {
    for (key, func, arg) in callbacks {
        spawn_and_run_callback(&vm.lua, &vm.scheduler, func, arg, &key, source);
    }
}

fn start_script(
    lua: &Lua,
    scheduler: &Mutex<LuaScheduler>,
    entity: Entity,
    code: String,
    kind: &str,
) {
    let chunk_name = format!("{kind}[{entity}]");
    if !scheduler.lock_or_recover().has_capacity() {
        warn!("Luau {kind} rejected: pending task limit reached");
        return;
    }
    let func =
        match crate::scripting::vm::compiler::compile_script(lua, &code, &chunk_name, entity) {
            Ok(func) => func,
            Err(e) => {
                error!("Luau {kind} compile error: {}", e);
                crate::scripting::output::push_error(&chunk_name, format!("Compile error: {}", e));
                return;
            }
        };

    crate::scripting::output::record_script_start();

    let thread = match lua.create_thread(func) {
        Ok(thread) => thread,
        Err(e) => {
            error!("Failed to create thread for {kind}: {}", e);
            crate::scripting::output::push_error(
                &chunk_name,
                format!("Failed to create thread: {}", e),
            );
            return;
        }
    };
    if let Err(e) = resume_and_schedule(lua, scheduler, thread, (), chunk_name.clone(), None, None)
    {
        error!("Luau {kind} runtime error: {}", e);
        crate::scripting::output::push_error(&chunk_name, e.to_string());
    }
}

fn discover_and_run_scripts<S: RunnableScript, V: VmResource>(world: &mut World) {
    let mut scripts_to_run = Vec::new();
    let mut query = world.query_filtered::<(Entity, &mut S), Changed<S>>();
    for (entity, mut script) in query.iter_mut(world) {
        if script.needs_start() {
            scripts_to_run.push((entity, script.start()));
        }
    }

    if scripts_to_run.is_empty() {
        return;
    }

    with_vm::<V>(world, |vm, _| {
        for (entity, code) in scripts_to_run {
            start_script(&vm.lua, &vm.scheduler, entity, code, S::KIND);
        }
    });
}

pub fn discover_and_run_server_scripts(world: &mut World) {
    discover_and_run_scripts::<ServerScript, ServerScriptVM>(world);
}

pub fn discover_and_run_local_scripts(world: &mut World) {
    discover_and_run_scripts::<LocalScript, ClientScriptVM>(world);
}

fn collect_signal_callbacks(
    lua: &Lua,
    registry: &ScriptRegistry,
    key: (Entity, &'static str),
    make_arg: impl Fn(&Lua) -> mlua::Result<LuaValue>,
    out: &mut Vec<Callback>,
) {
    let Some(keys) = registry.connections.get(&key) else {
        return;
    };
    for registry_key in keys {
        if let Ok(func) = lua.registry_value::<LuaFunction>(&**registry_key) {
            if let Ok(arg) = make_arg(lua) {
                out.push((registry_key.clone(), func, arg));
            }
        }
    }
}

fn ordered_pair(a: Entity, b: Entity) -> (Entity, Entity) {
    if a < b { (a, b) } else { (b, a) }
}

pub fn detect_touched_collisions(world: &mut World, mut current: Local<HashSet<(Entity, Entity)>>) {
    let removed: Vec<Entity> = world.removed::<CollidingEntities>().collect();
    if !removed.is_empty() {
        let Some(mut history) = world.get_resource_mut::<PreviousCollisions>() else {
            return;
        };
        for entity in removed {
            history.0.retain(|&(a, b)| a != entity && b != entity);
        }
    }

    current.clear();
    let mut changed_entities: Vec<Entity> = Vec::new();
    let mut query = world.query_filtered::<
        (Entity, &CollidingEntities),
        Or<(Changed<CollidingEntities>, Added<CollidingEntities>)>,
    >();
    for (entity, colliding) in query.iter(world) {
        current.extend(colliding.iter().map(|other| ordered_pair(entity, *other)));
        changed_entities.push(entity);
    }

    let began: Vec<(Entity, Entity)> = {
        let Some(mut history) = world.get_resource_mut::<PreviousCollisions>() else {
            return;
        };
        for entity in &changed_entities {
            history.0.retain(|&(a, b)| a != *entity && b != *entity);
        }
        let began = current.difference(&history.0).copied().collect();
        history.0.extend(current.drain());
        began
    };

    if began.is_empty() {
        return;
    }

    dispatch_touched::<ServerScriptVM>(world, &began);
    dispatch_touched::<ClientScriptVM>(world, &began);
}

fn dispatch_touched<V: VmResource>(world: &mut World, began: &[(Entity, Entity)]) {
    with_vm::<V>(world, |vm, _| {
        let mut callbacks = Vec::new();
        {
            let registry = vm.registry.lock_or_recover();
            for &(a, b) in began {
                for (target, other) in [(a, b), (b, a)] {
                    collect_signal_callbacks(
                        &vm.lua,
                        &registry,
                        (target, "Touched"),
                        |lua| instance_value(lua, other),
                        &mut callbacks,
                    );
                }
            }
        }
        fire_callbacks(vm, callbacks, "Touched");
    });
}

pub fn detect_player_added_events(world: &mut World) {
    let joined: Vec<Entity> = world
        .query_filtered::<Entity, Added<crate::common::net::components::Player>>()
        .iter(world)
        .collect();

    if joined.is_empty() {
        return;
    }

    dispatch_player_added::<ServerScriptVM>(world, &joined);
    dispatch_player_added::<ClientScriptVM>(world, &joined);
}

fn dispatch_player_added<V: VmResource>(world: &mut World, joined: &[Entity]) {
    with_vm::<V>(world, |vm, world| {
        let mut callbacks = Vec::new();
        {
            let registry = vm.registry.lock_or_recover();
            let players_entity =
                crate::scripting::userdata::instance::find_service_entity(world, "Players")
                    .unwrap_or(Entity::PLACEHOLDER);
            let keys = registry.connections.get(&(players_entity, "PlayerAdded"));
            for key in keys.into_iter().flatten() {
                let Ok(func) = vm.lua.registry_value::<LuaFunction>(&**key) else {
                    continue;
                };
                for &entity in joined {
                    if let Ok(arg) = instance_value(&vm.lua, entity) {
                        callbacks.push((key.clone(), func.clone(), arg));
                    }
                }
            }
        }
        fire_callbacks(vm, callbacks, "PlayerAdded");
    });
}

fn has_run_service_connections(registry: &ScriptRegistry, workspace_entity: Option<Entity>) -> bool {
    let Some(workspace_entity) = workspace_entity else {
        return false;
    };
    ["Heartbeat", "Stepped"]
        .into_iter()
        .any(|event| registry.connections.contains_key(&(workspace_entity, event)))
}

fn cached_workspace_entity(world: &World) -> Option<Entity> {
    world
        .get_resource::<ServiceEntities>()
        .and_then(|cache| cache.workspace)
        .or_else(|| crate::scripting::userdata::instance::find_service_entity(world, "Workspace"))
}

pub fn trigger_run_service_events(world: &mut World) {
    let delta_secs = world.resource::<Time>().delta_secs();
    run_service_tick::<ServerScriptVM>(world, delta_secs);
    run_service_tick::<ClientScriptVM>(world, delta_secs);
}

fn run_service_tick<V: VmResource>(world: &mut World, delta_secs: f32) {
    let workspace_entity = cached_workspace_entity(world);
    let has_work = world.get_resource::<V>().is_some_and(|vm| {
        has_run_service_connections(&vm.registry.lock_or_recover(), workspace_entity)
            || scheduler_has_work(&vm.scheduler)
    });
    if !has_work {
        return;
    }

    with_vm::<V>(world, |vm, _| {
        let mut callbacks = Vec::new();
        {
            let registry = vm.registry.lock_or_recover();
            let workspace_entity = workspace_entity.unwrap_or(Entity::PLACEHOLDER);
            for event in ["Heartbeat", "Stepped"] {
                collect_signal_callbacks(
                    &vm.lua,
                    &registry,
                    (workspace_entity, event),
                    |_| Ok(LuaValue::Number(delta_secs as f64)),
                    &mut callbacks,
                );
            }
        }
        fire_callbacks(vm, callbacks, "RunService");
        crate::scripting::vm::scheduler::run_scheduler_tick(&vm.scheduler, &vm.lua);
    });
}

fn cache_service_entities(mut cache: ResMut<ServiceEntities>, query: Query<(Entity, &Name)>) {
    let cache = cache.as_mut();
    let mut slots = [
        (&mut cache.workspace, "Workspace"),
        (&mut cache.players, "Players"),
        (&mut cache.lighting, "Lighting"),
        (&mut cache.asset_service, "AssetService"),
    ];

    for (slot, service) in slots.iter_mut() {
        if slot.is_some_and(
            |entity| !matches!(query.get(entity), Ok((_, name)) if name.as_str() == *service),
        ) {
            **slot = None;
        }
    }

    if slots.iter().all(|(slot, _)| slot.is_some()) {
        return;
    }

    for (entity, name) in &query {
        for (slot, service) in slots.iter_mut() {
            if slot.is_none() && name.as_str() == *service {
                **slot = Some(entity);
            }
        }
    }
}

fn sweep_vm_connections<V: VmResource>(world: &mut World) {
    let has_connections = world
        .get_resource::<V>()
        .is_some_and(|vm| !vm.registry.lock_or_recover().connections.is_empty());
    if !has_connections {
        return;
    }
    with_vm::<V>(world, |vm, world| {
        crate::scripting::vm::scheduler::sweep_stale_connections(&vm.lua, &vm.registry, world);
    });
}

fn sweep_stale_connections(world: &mut World) {
    sweep_vm_connections::<ServerScriptVM>(world);
    sweep_vm_connections::<ClientScriptVM>(world);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::net::components::{Player, PlayersServiceContainer};
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

    fn script_app() -> App {
        let mut app = App::new();
        app.add_plugins(ScriptingPlugin);
        app.world_mut()
            .insert_resource(bevy::time::Time::<()>::default());
        app.world_mut().insert_resource(ServerScriptVM::new().unwrap());
        app.world_mut().insert_resource(ClientScriptVM::new().unwrap());
        app
    }

    fn server_global<T: FromLua>(app: &App, name: &str) -> T {
        app.world()
            .resource::<ServerScriptVM>()
            .lua
            .globals()
            .get(name)
            .unwrap()
    }

    fn client_global<T: FromLua>(app: &App, name: &str) -> T {
        app.world()
            .resource::<ClientScriptVM>()
            .lua
            .globals()
            .get(name)
            .unwrap()
    }

    #[test]
    fn server_scripts_discover_run_and_restart_on_edit() {
        let mut app = script_app();
        let entity = app
            .world_mut()
            .spawn((
                Name::new("Counter"),
                ServerScript {
                    code: "_G.runs = (_G.runs or 0) + 1".to_string(),
                    ..default()
                },
            ))
            .id();

        app.update();
        assert_eq!(server_global::<i32>(&app, "runs"), 1);

        app.world_mut().entity_mut(entity).insert(ServerScript {
            code: "_G.runs = (_G.runs or 0) + 2".to_string(),
            ..default()
        });
        app.update();
        assert_eq!(server_global::<i32>(&app, "runs"), 3);

        app.update();
        assert_eq!(server_global::<i32>(&app, "runs"), 3);
    }

    #[test]
    fn disabled_scripts_do_not_run() {
        let mut app = script_app();
        app.world_mut().spawn((
            Name::new("Off"),
            ServerScript {
                code: "_G.should_not_run = true".to_string(),
                enabled: false,
                ..default()
            },
        ));

        app.update();
        let value = server_global::<LuaValue>(&app, "should_not_run");
        assert_eq!(value, LuaValue::Nil);
    }

    #[test]
    fn local_scripts_run_on_the_client_vm() {
        let mut app = script_app();
        app.world_mut().spawn((
            Name::new("L"),
            LocalScript {
                code: "_G.client_ran = true".to_string(),
                ..default()
            },
            lightyear::prelude::Replicate::default(),
        ));

        app.update();
        assert!(client_global::<bool>(&app, "client_ran"));
        let value = server_global::<LuaValue>(&app, "client_ran");
        assert_eq!(value, LuaValue::Nil);
    }

    #[test]
    fn scripts_error_without_crashing_the_engine() {
        let mut app = script_app();
        app.world_mut().spawn((
            Name::new("Bad"),
            ServerScript {
                code: "error('script exploded')".to_string(),
                ..default()
            },
        ));
        app.update();
        app.world_mut().spawn((
            Name::new("Good"),
            ServerScript {
                code: "_G.good_ran = true".to_string(),
                ..default()
            },
        ));
        app.update();
        assert!(server_global::<bool>(&app, "good_ran"));
    }

    #[test]
    fn player_added_fires_for_joining_players() {
        let mut app = script_app();
        app.world_mut()
            .spawn((Name::new("Players"), PlayersServiceContainer));
        app.world_mut().spawn((
            Name::new("Connector"),
            ServerScript {
                code: r#"
                    Players.PlayerAdded:Connect(function(p)
                        _G.added = (_G.added or 0) + 1
                        _G.last_added = p.Name
                    end)
                "#
                .to_string(),
                ..default()
            },
        ));

        app.update();
        assert_eq!(server_global::<LuaValue>(&app, "added"), LuaValue::Nil);

        app.world_mut().spawn((
            Name::new("Bob"),
            Player {
                client_id: 1,
                ..default()
            },
        ));
        app.update();
        assert_eq!(server_global::<i32>(&app, "added"), 1);
        assert_eq!(server_global::<String>(&app, "last_added"), "Bob");

        app.update();
        assert_eq!(server_global::<i32>(&app, "added"), 1);
    }

    #[test]
    fn touched_fires_once_per_contact() {
        let mut app = script_app();
        app.world_mut().spawn((
            Name::new("Sensor"),
            ServerScript {
                code: r#"
                    _G.p = Instance.new("Part")
                    _G.count = 0
                    _G.p.Touched:Connect(function()
                        _G.count = _G.count + 1
                    end)
                "#
                .to_string(),
                ..default()
            },
        ));
        app.update();

        let part = {
            let ud: mlua::AnyUserData = server_global(&app, "p");
            ud.borrow::<crate::scripting::userdata::instance::Instance>()
                .unwrap()
                .entity
        };
        let other = app.world_mut().spawn(Name::new("Other")).id();

        app.world_mut().entity_mut(part).insert(CollidingEntities(
            bevy::ecs::entity::EntityHashSet::from([other]),
        ));
        app.update();
        assert_eq!(server_global::<i32>(&app, "count"), 1);

        app.update();
        assert_eq!(server_global::<i32>(&app, "count"), 1);

        app.world_mut()
            .entity_mut(part)
            .remove::<CollidingEntities>();
        app.update();

        app.world_mut().entity_mut(part).insert(CollidingEntities(
            bevy::ecs::entity::EntityHashSet::from([other]),
        ));
        app.update();
        assert_eq!(server_global::<i32>(&app, "count"), 2);
    }

    #[test]
    fn run_service_events_fire_every_frame() {
        let mut app = script_app();
        app.world_mut().spawn(Name::new("Workspace"));
        app.world_mut().spawn((
            Name::new("Connector"),
            ServerScript {
                code: r#"
                    RunService.Heartbeat:Connect(function(dt)
                        _G.hb = (_G.hb or 0) + 1
                        _G.last_dt = dt
                    end)
                    RunService.Stepped:Connect(function(dt)
                        _G.stepped = (_G.stepped or 0) + 1
                    end)
                "#
                .to_string(),
                ..default()
            },
        ));

        app.update();
        assert_eq!(server_global::<i32>(&app, "hb"), 1);
        assert_eq!(server_global::<i32>(&app, "stepped"), 1);
        let dt: f64 = server_global(&app, "last_dt");
        assert!(dt >= 0.0, "Heartbeat must pass the frame delta");

        app.update();
        assert_eq!(server_global::<i32>(&app, "hb"), 2);
    }

    #[test]
    fn stale_connections_swept_end_to_end() {
        let mut app = script_app();
        app.world_mut().spawn((
            Name::new("Sensor"),
            ServerScript {
                code: r#"
                    _G.p = Instance.new("Part")
                    _G.p.Touched:Connect(function() end)
                "#
                .to_string(),
                ..default()
            },
        ));
        app.update();

        let part = {
            let ud: mlua::AnyUserData = server_global(&app, "p");
            ud.borrow::<crate::scripting::userdata::instance::Instance>()
                .unwrap()
                .entity
        };
        assert!(
            app.world()
                .resource::<ServerScriptVM>()
                .registry
                .lock()
                .unwrap()
                .connections
                .contains_key(&(part, "Touched"))
        );

        app.world_mut().entity_mut(part).despawn();
        app.update();
        assert!(
            !app.world()
                .resource::<ServerScriptVM>()
                .registry
                .lock()
                .unwrap()
                .connections
                .contains_key(&(part, "Touched"))
        );
    }
}
