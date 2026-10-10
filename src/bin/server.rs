use RaveEngineLib::app::common::args::CliArgs;
use RaveEngineLib::app::common::log::configure_logging;
use RaveEngineLib::app::server::config::ServerAppConfig;
use RaveEngineLib::common::CommonPlugin;
#[cfg(feature = "bench")]
use RaveEngineLib::common::net::components::NetworkTransform;
#[cfg(feature = "bench")]
use RaveEngineLib::common::net::components::Player;
#[cfg(feature = "bench")]
use avian3d::prelude::*;
use bevy::app::ScheduleRunnerPlugin;
use bevy::prelude::*;
use bevy::state::app::StatesPlugin;

#[cfg(feature = "bench")]
const DEFAULT_LOG_FILTER: &str = "info,wgpu=error,bevy_render=error,bevy_ecs=warn,lightyear=error,naga=warn,wgpu_hal=warn,wgpu_core=warn,offset_allocator=off";
#[cfg(not(feature = "bench"))]
const DEFAULT_LOG_FILTER: &str = RaveEngineLib::app::common::log::VERBOSE_NET_FILTER;

#[cfg(feature = "bench")]
const BENCH_SCENARIOS: [&str; 4] = ["server", "client", "studio", "bricks"];

#[cfg(feature = "bench")]
fn spawn_bench_players(mut commands: Commands) {
    for i in 0..10u64 {
        commands.spawn((
            Name::new(format!("BenchPlayer{}", i)),
            Player {
                client_id: 1000 + i,
                speed: 16.0 * 0.28,
                jump_power: 50.0 * 0.28,
                username: format!("BenchPlayer{}", i),
                ..default()
            },
            Transform::from_xyz(i as f32 * 2.0, 5.0 + i as f32 * 0.5, 0.0),
            NetworkTransform {
                translation: Vec3::new(i as f32 * 2.0, 5.0 + i as f32 * 0.5, 0.0),
                rotation: Quat::IDENTITY,
                scale: Vec3::ONE,
                velocity: Vec3::ZERO,
            },
            RigidBody::Dynamic,
            Collider::capsule(1.0 * 0.28, 3.0 * 0.28),
            CollisionLayers::from_bits(0b0010, 0b0011),
            LockedAxes::ROTATION_LOCKED,
            CollidingEntities::default(),
            SleepingDisabled,
            lightyear::prelude::Replicate::default(),
        ));
    }
    info!("BENCH: Spawned 10 simulated players");
}

#[cfg(feature = "bench")]
fn bench_move_players(
    mut query: Query<(&mut Transform, &mut LinearVelocity, &Player)>,
    mut frame: Local<u64>,
) {
    *frame += 1;
    let t = *frame as f32 * 0.05;
    for (mut transform, mut lin_vel, player) in &mut query {
        let idx = (player.client_id - 1000) as f32;
        let dx = (t + idx).sin() * 3.0;
        let dz = (t + idx * 1.5).cos() * 3.0;
        lin_vel.x = dx * 0.28;
        lin_vel.z = dz * 0.28;
        if *frame % 60 == idx as u64 {
            lin_vel.y = player.jump_power;
        }
        transform.rotation = Quat::from_rotation_y(t + idx);
    }
}

fn main() {
    let log_plugin = configure_logging("server", DEFAULT_LOG_FILTER, bevy::log::Level::DEBUG);
    let args = CliArgs::from_env();

    #[cfg(feature = "bench")]
    let bench_mode = args.has("--benchmark");
    #[cfg(feature = "bench")]
    let bench_scenario = args.value("--bench-scenario").unwrap_or("server").to_string();
    #[cfg(feature = "bench")]
    if bench_mode && !BENCH_SCENARIOS.contains(&bench_scenario.as_str()) {
        eprintln!("unsupported benchmark scenario: {bench_scenario}");
        std::process::exit(2);
    }
    #[cfg(feature = "bench")]
    let run_server = !bench_mode || bench_scenario == "server";
    #[cfg(not(feature = "bench"))]
    let run_server = true;

    let run_loop =
        MinimalPlugins.set(ScheduleRunnerPlugin::run_loop(std::time::Duration::from_millis(16)));

    let mut app = App::new();
    app.add_plugins(log_plugin);
    #[cfg(feature = "bench")]
    if bench_mode {
        app.add_plugins(MinimalPlugins);
    } else {
        app.add_plugins(run_loop);
    }
    #[cfg(not(feature = "bench"))]
    app.add_plugins(run_loop);
    app.add_plugins(RaveEngineLib::common::assets_path::asset_plugin());
    app.init_asset::<Mesh>();
    app.add_plugins(StatesPlugin);
    app.add_plugins(TransformPlugin);
    app.add_plugins(CommonPlugin);
    if run_server {
        app.add_plugins(ServerAppConfig::from_args(&args).into_plugin());
    }

    #[cfg(feature = "bench")]
    if bench_mode {
        let bench_frames: u64 = args.parsed("--bench-frames").unwrap_or(500);
        let bench_warmup: u64 = args.parsed("--bench-warmup").unwrap_or(100);
        app.world_mut()
            .resource_mut::<RaveEngineLib::common::core::bench::BenchStats>()
            .configure(&bench_scenario, bench_warmup, bench_frames);
        match bench_scenario.as_str() {
            "server" => {
                app.add_systems(Startup, spawn_bench_players);
                app.add_systems(Update, bench_move_players);
            }
            "client" => RaveEngineLib::client::add_client_benchmark(&mut app),
            "studio" => RaveEngineLib::studio::add_studio_benchmark(&mut app),
            _ => RaveEngineLib::common::game::bricks::add_bricks_benchmark(&mut app),
        }
        info!(
            "BENCH: Running {} with {} warmup and {} measured frames",
            bench_scenario, bench_warmup, bench_frames
        );
    }

    app.run();
}
