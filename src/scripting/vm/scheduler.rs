use bevy::log::*;
use bevy::prelude::{Entity, Resource};
use mlua::prelude::*;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::time::Instant;

#[derive(Resource, Default)]
pub struct ServiceEntities {
    pub workspace: Option<Entity>,
    pub players: Option<Entity>,
    pub lighting: Option<Entity>,
}

pub struct LuaTask {
    pub thread_key: mlua::RegistryKey,
    pub wake_time: Option<Instant>,
}

pub struct LuaScheduler {
    pub tasks: Vec<LuaTask>,
    pub deferred: VecDeque<mlua::RegistryKey>,
}

pub struct SchedulerRef(pub Arc<Mutex<LuaScheduler>>);

pub struct ScriptRegistry {
    pub connections:
        std::collections::HashMap<(Entity, &'static str), Vec<std::sync::Arc<mlua::RegistryKey>>>,
}

#[derive(Clone)]
pub struct ScriptRegistryRef(pub Arc<Mutex<ScriptRegistry>>);

impl Default for LuaScheduler {
    fn default() -> Self {
        Self::new()
    }
}

impl LuaScheduler {
    pub fn new() -> Self {
        Self {
            tasks: Vec::new(),
            deferred: VecDeque::new(),
        }
    }

    pub fn run_tick(scheduler: &Arc<Mutex<Self>>, lua: &mlua::Lua) {
        let now = Instant::now();
        let mut tasks_to_run = Vec::new();
        {
            let mut scheduler = scheduler.lock().expect("Lua scheduler lock poisoned");
            let old_tasks = std::mem::take(&mut scheduler.tasks);
            for task in old_tasks {
                if task.wake_time.is_none_or(|wake| now >= wake) {
                    tasks_to_run.push(task.thread_key);
                } else {
                    scheduler.tasks.push(task);
                }
            }
            tasks_to_run.extend(scheduler.deferred.drain(..));
        }

        let mut requeued = Vec::new();
        for thread_key in tasks_to_run {
            let thread_opt: Result<LuaThread, _> = lua.registry_value(&thread_key);
            if let Ok(thread) = thread_opt {
                match thread.resume::<Option<f32>>(()) {
                    Ok(yielded_val) => {
                        if thread.status() == LuaThreadStatus::Resumable {
                            let wake = yielded_val.and_then(|sec| checked_wake_time(now, sec));
                            requeued.push(LuaTask {
                                thread_key,
                                wake_time: wake,
                            });
                        } else {
                            let _ = lua.remove_registry_value(thread_key);
                        }
                    }
                    Err(e) => {
                        error!("Luau scheduler execution error: {}", e);
                        let _ = lua.remove_registry_value(thread_key);
                    }
                }
            } else {
                let _ = lua.remove_registry_value(thread_key);
            }
        }

        if !requeued.is_empty() {
            scheduler
                .lock()
                .expect("Lua scheduler lock poisoned")
                .tasks
                .extend(requeued);
        }
    }
}

pub(crate) fn checked_wake_time(now: Instant, seconds: f32) -> Option<Instant> {
    if !seconds.is_finite() || seconds < 0.0 {
        return None;
    }
    let duration = std::time::Duration::try_from_secs_f64(seconds as f64).ok()?;
    now.checked_add(duration)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checked_delays_reject_invalid_and_out_of_range_values() {
        let now = Instant::now();
        assert_eq!(checked_wake_time(now, 0.0), Some(now));
        assert!(checked_wake_time(now, 0.25).is_some());
        assert!(checked_wake_time(now, -1.0).is_none());
        assert!(checked_wake_time(now, f32::NAN).is_none());
        assert!(checked_wake_time(now, f32::INFINITY).is_none());
        assert!(checked_wake_time(now, f32::MAX).is_none());
    }

    #[test]
    fn scheduler_allows_resumed_tasks_to_schedule_more_work() {
        let lua = Lua::new();
        crate::scripting::runtime::globals::setup_globals(&lua).unwrap();
        let scheduler = Arc::new(Mutex::new(LuaScheduler::new()));
        lua.set_app_data(SchedulerRef(scheduler.clone()));
        let thread = lua
            .create_thread(
                lua.load("task.defer(function() end)")
                    .into_function()
                    .unwrap(),
            )
            .unwrap();
        let key = lua.create_registry_value(thread).unwrap();
        scheduler.lock().unwrap().tasks.push(LuaTask {
            thread_key: key,
            wake_time: None,
        });

        LuaScheduler::run_tick(&scheduler, &lua);

        assert_eq!(scheduler.lock().unwrap().deferred.len(), 1);
    }
}
