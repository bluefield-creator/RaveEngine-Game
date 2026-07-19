pub mod vrtx;
pub mod performance;
#[cfg(feature = "bench")]
pub mod bench;

use bevy::prelude::*;

pub struct CorePlugin;

impl Plugin for CorePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(performance::PerformancePlugin);
        #[cfg(feature = "bench")]
        {
            app.init_resource::<bench::BenchStats>();
            app.add_systems(Update, (
                bench::bench_frame_counter,
                bench::bench_log_system,
            ));
        }
    }
}