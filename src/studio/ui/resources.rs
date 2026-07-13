use bevy::prelude::*;

#[derive(Resource, Default)]
pub struct CopiedEntityBuffer {
    pub transform: Option<Transform>,
    pub mesh: Option<Mesh3d>,
    pub material: Option<MeshMaterial3d<StandardMaterial>>,
    pub studs_material: Option<MeshMaterial3d<bevy::pbr::ExtendedMaterial<StandardMaterial, crate::common::game::bricks::studs::StudsExtension>>>,
    pub name: Option<String>,
    pub is_brick: bool,
    pub shape: crate::common::game::bricks::components::BrickShape,
    pub physics: Option<crate::common::game::bricks::components::BrickPhysics>,
}

#[derive(Resource, Default)]
pub struct HierarchyDraggedEntity {
    pub entity: Option<Entity>,
}

#[derive(Resource, Default)]
pub struct SettingsWindow {
    pub open: bool,
}

#[derive(Resource, Default)]
pub struct PlayInClientProcesses {
    pub client_process: Option<std::process::Child>,
}

#[derive(Resource, Default)]
pub struct PlaytestBackup {
    pub bricks: Vec<crate::common::game::bricks::data::BrickData>,
}

#[derive(Component)]
pub struct InEditorPlaytestClient;

pub fn cleanup_play_processes_on_exit(
    events: MessageReader<AppExit>,
    mut play_processes: ResMut<PlayInClientProcesses>,
) {
    if !events.is_empty() {
        crate::app::server::bootstrap::SHUTDOWN_SERVER.store(true, std::sync::atomic::Ordering::Relaxed);
        if let Some(mut child) = play_processes.client_process.take() {
            let _ = child.kill();
        }
    }
}