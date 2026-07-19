use bevy::prelude::*;
use std::time::Instant;

#[derive(Resource, Clone)]
pub struct BenchStats {
    pub find_service_calls: u64,
    pub find_service_scan_calls: u64,
    pub find_service_ns: u128,
    pub event_dispatch_ns: u128,
    pub collision_ns: u128,
    pub scheduler_ns: u128,
    pub player_velocity_ns: u128,
    pub hide_visuals_ns: u128,
    pub brick_material_ns: u128,
    pub total_frames: u64,
    pub start_time: Instant,
}

impl Default for BenchStats {
    fn default() -> Self {
        Self {
            find_service_calls: 0,
            find_service_scan_calls: 0,
            find_service_ns: 0,
            event_dispatch_ns: 0,
            collision_ns: 0,
            scheduler_ns: 0,
            player_velocity_ns: 0,
            hide_visuals_ns: 0,
            brick_material_ns: 0,
            total_frames: 0,
            start_time: Instant::now(),
        }
    }
}

#[macro_export]
macro_rules! bench_time {
    ($stats:expr, $field:ident, $block:expr) => {{
        let _t = std::time::Instant::now();
        let result = $block;
        $stats.$field += _t.elapsed().as_nanos() as u128;
        result
    }};
}

pub fn bench_frame_counter(
    mut stats: ResMut<BenchStats>,
) {
    stats.total_frames += 1;
}

pub fn bench_log_system(
    stats: Res<BenchStats>,
    mut last_log: Local<f32>,
    time: Res<Time>,
) {
    let elapsed = time.elapsed_secs();
    if elapsed - *last_log < 2.0 {
        return;
    }
    *last_log = elapsed;

    let dt = elapsed - stats.start_time.elapsed().as_secs_f32().abs();
    let _ = dt;

    info!(
        "BENCH frame={} svc_calls={} svc_scans={} svc_ns={} evt_ns={} col_ns={} sched_ns={} vel_ns={} vis_ns={} mat_ns={}",
        stats.total_frames,
        stats.find_service_calls,
        stats.find_service_scan_calls,
        stats.find_service_ns,
        stats.event_dispatch_ns,
        stats.collision_ns,
        stats.scheduler_ns,
        stats.player_velocity_ns,
        stats.hide_visuals_ns,
        stats.brick_material_ns,
    );
}

pub fn bench_dump_json(stats: Res<BenchStats>) {
    let total_ns = stats.start_time.elapsed().as_nanos() as u128;
    let avg_frame_ns = if stats.total_frames > 0 {
        total_ns / stats.total_frames as u128
    } else {
        0
    };
    println!(
        r#"{{"total_frames":{},"total_ns":{},"avg_frame_ns":{},"find_service_calls":{},"find_service_scan_calls":{},"find_service_ns":{},"event_dispatch_ns":{},"collision_ns":{},"scheduler_ns":{},"player_velocity_ns":{},"hide_visuals_ns":{},"brick_material_ns":{}}}"#,
        stats.total_frames,
        total_ns,
        avg_frame_ns,
        stats.find_service_calls,
        stats.find_service_scan_calls,
        stats.find_service_ns,
        stats.event_dispatch_ns,
        stats.collision_ns,
        stats.scheduler_ns,
        stats.player_velocity_ns,
        stats.hide_visuals_ns,
        stats.brick_material_ns,
    );
}
