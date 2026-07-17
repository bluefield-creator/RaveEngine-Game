use bevy::prelude::*;
use mlua::prelude::*;

pub fn setup_globals(lua: &Lua) -> Result<(), mlua::Error> {
    let task_table = lua.create_table()?;

    let wait_fn: LuaFunction = lua.load("function(seconds) return coroutine.yield(seconds or 0) end").eval()?;
    task_table.set("wait", wait_fn)?;

    let spawn_fn = lua.create_function(|lua, val: LuaValue| {
        match val {
            LuaValue::Function(f) => {
                let thread = lua.create_thread(f)?;
                thread.resume::<()>(())?;
            }
            LuaValue::Thread(t) => {
                t.resume::<()>(())?;
            }
            _ => return Err(mlua::Error::RuntimeError("task.spawn expects function or thread".to_string())),
        }
        Ok(())
    })?;
    task_table.set("spawn", spawn_fn)?;

    let defer_fn = lua.create_function(|lua, f: LuaFunction| {
        let scheduler_ref = lua.app_data_ref::<crate::scripting::vm::scheduler::SchedulerRef>().unwrap();
        let mut scheduler = scheduler_ref.0.lock().unwrap();
        let thread = lua.create_thread(f)?;
        let key = lua.create_registry_value(thread)?;
        scheduler.deferred.push_back(key);
        Ok(())
    })?;
    task_table.set("defer", defer_fn)?;

    let delay_fn = lua.create_function(|lua, (seconds, f): (f32, LuaFunction)| {
        let scheduler_ref = lua.app_data_ref::<crate::scripting::vm::scheduler::SchedulerRef>().unwrap();
        let mut scheduler = scheduler_ref.0.lock().unwrap();
        let thread = lua.create_thread(f)?;
        let key = lua.create_registry_value(thread)?;
        scheduler.tasks.push(crate::scripting::vm::scheduler::LuaTask {
            thread_key: key,
            wake_time: Some(std::time::Instant::now() + std::time::Duration::from_secs_f64(seconds as f64)),
        });
        Ok(())
    })?;
    task_table.set("delay", delay_fn)?;

    lua.globals().set("task", task_table)?;

    let print_fn = lua.create_function(|_, val: String| {
        info!("LUA_PRINT: {}", val);
        Ok(())
    })?;
    lua.globals().set("print", print_fn)?;

    let warn_fn = lua.create_function(|_, val: String| {
        warn!("LUA_WARN: {}", val);
        Ok(())
    })?;
    lua.globals().set("warn", warn_fn)?;

    let error_fn = lua.create_function(|_, val: String| {
        error!("LUA_ERROR: {}", val);
        Err::<(), _>(mlua::Error::RuntimeError(val))
    })?;
    lua.globals().set("error", error_fn)?;

    let vector3_class = lua.create_table()?;
    vector3_class.set("new", lua.create_function(|_, (x, y, z): (f32, f32, f32)| {
        Ok(crate::scripting::userdata::vector3::Vector3(Vec3::new(x, y, z)))
    })?)?;
    lua.globals().set("Vector3", vector3_class)?;

    let color3_class = lua.create_table()?;
    color3_class.set("new", lua.create_function(|_, (r, g, b): (f32, f32, f32)| {
        Ok(crate::scripting::userdata::color3::Color3(Color::Srgba(Srgba::new(r, g, b, 1.0))))
    })?)?;
    color3_class.set("fromRGB", lua.create_function(|_, (r, g, b): (f32, f32, f32)| {
        Ok(crate::scripting::userdata::color3::Color3(Color::Srgba(Srgba::new(r / 255.0, g / 255.0, b / 255.0, 1.0))))
    })?)?;
    lua.globals().set("Color3", color3_class)?;

    let cframe_class = lua.create_table()?;
    cframe_class.set("new", lua.create_function(|_, (x, y, z): (f32, f32, f32)| {
        Ok(crate::scripting::userdata::cframe::CFrame {
            position: Vec3::new(x, y, z),
            rotation: Quat::IDENTITY,
        })
    })?)?;
    lua.globals().set("CFrame", cframe_class)?;

    Ok(())
}