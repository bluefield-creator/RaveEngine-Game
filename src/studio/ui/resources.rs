use bevy::prelude::*;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InsertKind {
    Part,
    Sphere,
    ServerScript,
    LocalScript,
    ModuleScript,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EditorAction {
    NewProject,
    Open,
    Save,
    SaveAs,
    Exit,
    Undo,
    Redo,
    Cut,
    Copy,
    Paste,
    Duplicate,
    Rename,
    Delete,
    SelectAll,
    Insert(InsertKind, Option<Entity>),
    ToggleExplorer,
    ToggleProperties,
    ToggleScriptEditor,
    FrameSelected,
    ResetCamera,
    ResetLayout,
    ToggleSimulation,
    TogglePlaytest,
    StopTesting,
}

#[derive(Resource, Default)]
pub struct EditorActionQueue(pub Vec<EditorAction>);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PendingDocumentAction {
    NewProject,
    Open,
    Exit,
}

#[derive(Resource, Default)]
pub struct DocumentState {
    pub dirty: bool,
    pub pending: Option<PendingDocumentAction>,
    pub error: Option<String>,
}

#[derive(Resource)]
pub struct EditorLayoutState {
    pub explorer_visible: bool,
    pub properties_visible: bool,
    pub script_editor_visible: bool,
    pub dock_width: f32,
    pub explorer_height: f32,
}

impl Default for EditorLayoutState {
    fn default() -> Self {
        Self {
            explorer_visible: true,
            properties_visible: true,
            script_editor_visible: true,
            dock_width: 220.0,
            explorer_height: 180.0,
        }
    }
}

#[derive(Resource, Default)]
pub struct ExplorerState {
    pub search: String,
    pub rename_entity: Option<Entity>,
    pub rename_buffer: String,
}

#[derive(Resource, Default)]
pub struct CopiedEntityBuffer {
    pub transform: Option<Transform>,
    pub mesh: Option<Mesh3d>,
    pub material: Option<MeshMaterial3d<StandardMaterial>>,
    pub studs_material: Option<
        MeshMaterial3d<
            bevy::pbr::ExtendedMaterial<
                StandardMaterial,
                crate::common::game::bricks::studs::StudsExtension,
            >,
        >,
    >,
    pub name: Option<String>,
    pub is_brick: bool,
    pub shape: crate::common::game::bricks::components::BrickShape,
    pub physics: Option<crate::common::game::bricks::components::BrickPhysics>,
    pub script: Option<CopiedScript>,
    pub trees: Vec<EditorNodeSnapshot>,
}

#[derive(Clone)]
pub enum CopiedScript {
    Server {
        name: String,
        code: String,
        enabled: bool,
    },
    Local {
        name: String,
        code: String,
        enabled: bool,
    },
    Module {
        name: String,
        code: String,
    },
}

#[derive(Clone, Debug)]
pub enum EditorItemSnapshot {
    Part(crate::common::game::bricks::data::BrickData),
    Server {
        name: String,
        code: String,
        enabled: bool,
    },
    Local {
        name: String,
        code: String,
        enabled: bool,
    },
    Module {
        name: String,
        code: String,
    },
}

#[derive(Clone, Debug)]
pub struct EditorNodeSnapshot {
    pub item: EditorItemSnapshot,
    pub parent: Option<Entity>,
    pub children: Vec<EditorNodeSnapshot>,
}

pub fn selected_root_entities(
    selected: &[Entity],
    query: &Query<
        (
            Entity,
            &Name,
            Option<&ChildOf>,
            Option<&Children>,
            Option<&crate::common::game::bricks::components::Brick>,
            Option<&crate::scripting::ecs::ServerScript>,
            Option<&crate::scripting::ecs::LocalScript>,
            Option<&crate::scripting::ecs::ModuleScript>,
        ),
        Without<Camera3d>,
    >,
) -> Vec<Entity> {
    let selected: std::collections::HashSet<_> = selected.iter().copied().collect();
    selected
        .iter()
        .copied()
        .filter(|entity| {
            let mut current = *entity;
            while let Ok((_, _, Some(parent), _, _, _, _, _)) = query.get(current) {
                current = parent.parent();
                if selected.contains(&current) {
                    return false;
                }
            }
            true
        })
        .collect()
}

pub fn capture_editor_snapshot(
    entity: Entity,
    explorer: &Query<
        (
            Entity,
            &Name,
            Option<&ChildOf>,
            Option<&Children>,
            Option<&crate::common::game::bricks::components::Brick>,
            Option<&crate::scripting::ecs::ServerScript>,
            Option<&crate::scripting::ecs::LocalScript>,
            Option<&crate::scripting::ecs::ModuleScript>,
        ),
        Without<Camera3d>,
    >,
    entities: &Query<
        (
            Entity,
            &mut Transform,
            &Name,
            Option<&ChildOf>,
            Option<&Children>,
            Option<&crate::common::game::bricks::components::Brick>,
            Option<&mut crate::common::game::bricks::components::BrickShapeComponent>,
            &GlobalTransform,
            Option<&Mesh3d>,
            Option<&MeshMaterial3d<StandardMaterial>>,
            Option<
                &MeshMaterial3d<
                    bevy::pbr::ExtendedMaterial<
                        StandardMaterial,
                        crate::common::game::bricks::studs::StudsExtension,
                    >,
                >,
            >,
            Option<&mut crate::common::game::bricks::components::BrickPhysics>,
        ),
        Without<Camera3d>,
    >,
) -> Option<EditorNodeSnapshot> {
    let (_, name, parent, children, brick, server, local, module) = explorer.get(entity).ok()?;
    let item = if brick.is_some() {
        EditorItemSnapshot::Part(crate::common::game::bricks::data::capture_brick_data(
            entity, entities,
        )?)
    } else if let Some(script) = server {
        EditorItemSnapshot::Server {
            name: name.to_string(),
            code: script.code.clone(),
            enabled: script.enabled,
        }
    } else if let Some(script) = local {
        EditorItemSnapshot::Local {
            name: name.to_string(),
            code: script.code.clone(),
            enabled: script.enabled,
        }
    } else if let Some(script) = module {
        EditorItemSnapshot::Module {
            name: name.to_string(),
            code: script.code.clone(),
        }
    } else {
        return None;
    };
    let children = children
        .into_iter()
        .flat_map(|children| children.iter())
        .filter_map(|child| capture_editor_snapshot(child, explorer, entities))
        .collect();
    Some(EditorNodeSnapshot {
        item,
        parent: parent.map(|parent| parent.parent()),
        children,
    })
}

pub fn spawn_editor_snapshot(
    commands: &mut Commands,
    snapshot: &EditorNodeSnapshot,
    parent: Option<Entity>,
) -> Entity {
    let entity = match &snapshot.item {
        EditorItemSnapshot::Part(data) => {
            let mut data = data.clone();
            data.parent = None;
            crate::common::game::bricks::data::spawn_from_data(commands, &data)
        }
        EditorItemSnapshot::Server {
            name,
            code,
            enabled,
        } => commands
            .spawn((
                Name::new(name.clone()),
                crate::scripting::ecs::ServerScript {
                    code: code.clone(),
                    enabled: *enabled,
                    started: false,
                },
            ))
            .id(),
        EditorItemSnapshot::Local {
            name,
            code,
            enabled,
        } => commands
            .spawn((
                Name::new(name.clone()),
                crate::scripting::ecs::LocalScript {
                    code: code.clone(),
                    enabled: *enabled,
                    started: false,
                },
                lightyear::prelude::Replicate::default(),
            ))
            .id(),
        EditorItemSnapshot::Module { name, code } => commands
            .spawn((
                Name::new(name.clone()),
                crate::scripting::ecs::ModuleScript { code: code.clone() },
                lightyear::prelude::Replicate::default(),
            ))
            .id(),
    };
    if let Some(parent) = parent {
        commands.entity(parent).add_child(entity);
    }
    for child in &snapshot.children {
        spawn_editor_snapshot(commands, child, Some(entity));
    }
    entity
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
    pub scripts: Vec<crate::common::core::vrtx::VrtxScript>,
    pub gravity: Option<Vec3>,
    pub players_service: Option<crate::studio::tools::PlayersService>,
    pub lighting_service: Option<crate::common::game::environment::lighting::LightingService>,
}

#[derive(Component)]
pub struct InEditorPlaytestClient;

#[derive(Resource, Default)]
pub struct ActiveScriptEditor {
    pub entity: Option<Entity>,
    pub open: bool,
    pub buffer: String,
    pub error: Option<String>,
    pub open_entities: Vec<Entity>,
}

pub enum FileDialogResult {
    BrowseSavePath(std::path::PathBuf),
    OpenFile(std::path::PathBuf),
    SaveAs(std::path::PathBuf),
    Cancel,
}

#[derive(Resource)]
pub struct FileDialogState {
    pub tx: std::sync::mpsc::Sender<FileDialogResult>,
    pub rx: std::sync::Mutex<std::sync::mpsc::Receiver<FileDialogResult>>,
    pub is_open: std::sync::atomic::AtomicBool,
}

impl Default for FileDialogState {
    fn default() -> Self {
        let (tx, rx) = std::sync::mpsc::channel();
        Self {
            tx,
            rx: std::sync::Mutex::new(rx),
            is_open: std::sync::atomic::AtomicBool::new(false),
        }
    }
}

pub fn cleanup_play_processes_on_exit(
    events: MessageReader<AppExit>,
    mut play_processes: ResMut<PlayInClientProcesses>,
) {
    if !events.is_empty() {
        crate::app::server::bootstrap::SHUTDOWN_SERVER
            .store(true, std::sync::atomic::Ordering::Relaxed);
        if let Some(mut child) = play_processes.client_process.take() {
            let _ = child.kill();
        }
    }
}

pub fn handle_file_dialog_results(
    mut commands: Commands,
    file_dialog_state: Res<FileDialogState>,
    mut onboarding_data: ResMut<crate::studio::ui::panels::onboarding::OnboardingData>,
    mut graphics_settings: ResMut<crate::common::core::performance::GraphicsSettings>,
    mut gravity: Option<ResMut<avian3d::prelude::Gravity>>,
    mut camera_transform_query: Query<&mut Transform, With<Camera3d>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut studs_materials: ResMut<
        Assets<
            bevy::pbr::ExtendedMaterial<
                StandardMaterial,
                crate::common::game::bricks::studs::StudsExtension,
            >,
        >,
    >,
    entities_query: Query<
        (
            Entity,
            Option<&crate::common::game::bricks::components::Brick>,
        ),
        Without<Camera3d>,
    >,
    explorer_query: Query<
        (
            Entity,
            Option<&crate::scripting::ecs::ServerScript>,
            Option<&crate::scripting::ecs::LocalScript>,
            Option<&crate::scripting::ecs::ModuleScript>,
        ),
        Without<Camera3d>,
    >,
    save_query: Query<
        (
            Entity,
            &Transform,
            &Name,
            Option<&ChildOf>,
            Option<&Children>,
            Option<&crate::common::game::bricks::components::Brick>,
            Option<&crate::common::game::bricks::components::BrickShapeComponent>,
            &GlobalTransform,
            Option<&Mesh3d>,
            Option<&MeshMaterial3d<StandardMaterial>>,
            Option<
                &MeshMaterial3d<
                    bevy::pbr::ExtendedMaterial<
                        StandardMaterial,
                        crate::common::game::bricks::studs::StudsExtension,
                    >,
                >,
            >,
            Option<&crate::common::game::bricks::components::BrickPhysics>,
        ),
        Without<Camera3d>,
    >,
    save_explorer_query: Query<
        (
            Entity,
            &Name,
            Option<&ChildOf>,
            Option<&Children>,
            Option<&crate::common::game::bricks::components::Brick>,
            Option<&crate::scripting::ecs::ServerScript>,
            Option<&crate::scripting::ecs::LocalScript>,
            Option<&crate::scripting::ecs::ModuleScript>,
        ),
        Without<Camera3d>,
    >,
    mut document: ResMut<DocumentState>,
) {
    let rx = file_dialog_state.rx.lock().unwrap();
    while let Ok(result) = rx.try_recv() {
        match result {
            FileDialogResult::BrowseSavePath(path) => {
                onboarding_data.save_path = path.display().to_string();
                file_dialog_state
                    .is_open
                    .store(false, std::sync::atomic::Ordering::Relaxed);
            }
            FileDialogResult::OpenFile(path) => {
                let open_path_str = path.display().to_string();
                if let Ok(state) =
                    crate::common::core::vrtx::VrtxFileState::load_from_file(&open_path_str)
                {
                    document.dirty = false;
                    onboarding_data.save_path = open_path_str;
                    for (entity, brick_opt) in &entities_query {
                        if brick_opt.is_some() {
                            commands.entity(entity).try_despawn();
                        }
                    }
                    for (entity, s_opt, l_opt, m_opt) in &explorer_query {
                        if s_opt.is_some() || l_opt.is_some() || m_opt.is_some() {
                            commands.entity(entity).try_despawn();
                        }
                    }
                    graphics_settings.ssao = state.settings.ssao;
                    graphics_settings.contact_shadows = state.settings.contact_shadows;
                    graphics_settings.bloom = state.settings.bloom;
                    if let Some(ref mut g) = gravity {
                        g.0 = state.gravity;
                    }
                    if let Some(mut cam_t) = camera_transform_query.iter_mut().next() {
                        *cam_t = state.camera_transform;
                    }
                    let version = state.version;
                    let mut named_entities = std::collections::HashMap::new();
                    let mut node_entities = std::collections::HashMap::new();
                    let mut pending_parents = Vec::new();
                    for brick in state.bricks {
                        let layers = if brick.player_can_collide {
                            avian3d::prelude::CollisionLayers::from_bits(0b0001, 0xFFFF_FFFF)
                        } else {
                            avian3d::prelude::CollisionLayers::from_bits(0b0100, 0xFFFF_FFFD)
                        };
                        let new_id = commands
                            .spawn((
                                brick.transform,
                                crate::common::game::bricks::components::Brick,
                                crate::common::game::bricks::components::BrickShapeComponent {
                                    shape: brick.shape,
                                },
                                crate::common::game::bricks::components::BrickPhysics {
                                    enabled: brick.physics_enabled,
                                    bounciness: brick.bounciness,
                                    player_can_collide: brick.player_can_collide,
                                    friction: brick.friction,
                                    gravity_scale: brick.gravity_scale,
                                    mass: brick.mass,
                                },
                                crate::common::game::bricks::components::BrickColor {
                                    color: brick.color,
                                },
                                layers,
                                Pickable::default(),
                                Name::new(brick.name.clone()),
                            ))
                            .id();
                        named_entities.entry(brick.name).or_insert(new_id);
                        node_entities.insert(brick.node_id, new_id);
                        if let Some(parent) = brick.parent_node_id {
                            pending_parents.push((new_id, parent));
                        }
                    }
                    for script in state.scripts {
                        let mut cmd = commands.spawn(Name::new(script.name));
                        match script.script_type {
                            0 => {
                                cmd.insert(crate::scripting::ecs::ServerScript {
                                    code: script.code,
                                    enabled: script.enabled,
                                    started: false,
                                });
                            }
                            1 => {
                                cmd.insert((
                                    crate::scripting::ecs::LocalScript {
                                        code: script.code,
                                        enabled: script.enabled,
                                        started: false,
                                    },
                                    lightyear::prelude::Replicate::default(),
                                ));
                            }
                            _ => {
                                cmd.insert((
                                    crate::scripting::ecs::ModuleScript { code: script.code },
                                    lightyear::prelude::Replicate::default(),
                                ));
                            }
                        }
                        let new_script_entity = cmd.id();
                        node_entities.insert(script.node_id, new_script_entity);
                        if let Some(parent) = script.parent_node_id {
                            pending_parents.push((new_script_entity, parent));
                        } else if version < 6 {
                            if let Some(ref p_name) = script.parent_name {
                                if let Some(&parent_entity) = named_entities.get(p_name) {
                                    commands.entity(parent_entity).add_child(new_script_entity);
                                }
                            }
                        }
                    }
                    for (child, parent_id) in pending_parents {
                        if let Some(parent) = node_entities.get(&parent_id) {
                            commands.entity(*parent).add_child(child);
                        }
                    }
                }
                file_dialog_state
                    .is_open
                    .store(false, std::sync::atomic::Ordering::Relaxed);
            }
            FileDialogResult::SaveAs(path) => {
                let save_path_str = path.display().to_string();
                onboarding_data.save_path = save_path_str.clone();
                let node_ids: std::collections::HashMap<Entity, u64> = save_explorer_query
                    .iter()
                    .filter(|(_, _, _, _, brick, server, local, module)| {
                        brick.is_some() || server.is_some() || local.is_some() || module.is_some()
                    })
                    .enumerate()
                    .map(|(index, (entity, _, _, _, _, _, _, _))| (entity, index as u64))
                    .collect();

                let mut bricks_data = Vec::new();
                for (
                    entity,
                    transform,
                    name,
                    child_of,
                    _,
                    brick_opt,
                    shape_opt,
                    _,
                    _,
                    mat_opt,
                    studs_mat_opt,
                    phys_opt,
                ) in &save_query
                {
                    if brick_opt.is_some() {
                        let shape = shape_opt
                            .as_ref()
                            .map(|s| s.shape)
                            .unwrap_or(crate::common::game::bricks::components::BrickShape::Block);
                        let mut current_color = Color::Srgba(Srgba::new(0.84, 0.24, 0.16, 1.0));
                        if let Some(studs_mat_handle) = studs_mat_opt {
                            if let Some(mat) = studs_materials.get(&studs_mat_handle.0) {
                                current_color = mat.base.base_color;
                            }
                        } else if let Some(mat_handle) = mat_opt {
                            if let Some(mat) = materials.get(&mat_handle.0) {
                                current_color = mat.base_color;
                            }
                        }
                        let (
                            physics_enabled,
                            bounciness,
                            player_can_collide,
                            friction,
                            gravity_scale,
                            mass,
                        ) = if let Some(phys) = phys_opt {
                            (
                                phys.enabled,
                                phys.bounciness,
                                phys.player_can_collide,
                                phys.friction,
                                phys.gravity_scale,
                                phys.mass,
                            )
                        } else {
                            (true, 0.3, true, 0.3, 1.0, 1.0)
                        };
                        bricks_data.push(crate::common::core::vrtx::VrtxBrick {
                            node_id: node_ids
                                .get(&entity)
                                .copied()
                                .unwrap_or(bricks_data.len() as u64),
                            parent_node_id: child_of
                                .and_then(|parent| node_ids.get(&parent.parent()).copied()),
                            name: name.to_string(),
                            transform: *transform,
                            shape,
                            color: current_color,
                            physics_enabled,
                            bounciness,
                            player_can_collide,
                            friction,
                            gravity_scale,
                            mass,
                        });
                    }
                }

                let mut scripts_data = Vec::new();
                for (entity, name, child_of_opt, _, _, s_opt, l_opt, m_opt) in &save_explorer_query
                {
                    let mut script_type_opt = None;
                    let mut code = String::new();
                    let mut enabled = true;
                    if let Some(s) = s_opt {
                        script_type_opt = Some(0);
                        code = s.code.clone();
                        enabled = s.enabled;
                    } else if let Some(l) = l_opt {
                        script_type_opt = Some(1);
                        code = l.code.clone();
                        enabled = l.enabled;
                    } else if let Some(m) = m_opt {
                        script_type_opt = Some(2);
                        code = m.code.clone();
                    }
                    if let Some(script_type) = script_type_opt {
                        let mut parent_name = None;
                        if let Some(child_of) = child_of_opt {
                            if let Ok((_, p_name, _, _, _, _, _, _)) =
                                save_explorer_query.get(child_of.parent())
                            {
                                parent_name = Some(p_name.to_string());
                            }
                        }
                        scripts_data.push(crate::common::core::vrtx::VrtxScript {
                            node_id: node_ids
                                .get(&entity)
                                .copied()
                                .unwrap_or((bricks_data.len() + scripts_data.len()) as u64),
                            parent_node_id: child_of_opt
                                .and_then(|parent| node_ids.get(&parent.parent()).copied()),
                            name: name.to_string(),
                            script_type,
                            code,
                            parent_name,
                            enabled,
                        });
                    }
                }

                let gravity_val = if let Some(g) = gravity.as_ref() {
                    g.0
                } else {
                    Vec3::new(0.0, -186.9 * 0.28, 0.0)
                };
                let cam_transform = if let Some(cam_t) = camera_transform_query.iter().next() {
                    *cam_t
                } else {
                    Transform::IDENTITY
                };
                let state = crate::common::core::vrtx::VrtxFileState {
                    version: crate::common::core::vrtx::CURRENT_VRTX_VERSION,
                    gravity: gravity_val,
                    settings: crate::common::core::vrtx::VrtxSettings {
                        ssao: graphics_settings.ssao,
                        contact_shadows: graphics_settings.contact_shadows,
                        bloom: graphics_settings.bloom,
                    },
                    camera_transform: cam_transform,
                    bricks: bricks_data,
                    scripts: scripts_data,
                };
                match state.save_to_file(&save_path_str) {
                    Ok(()) => document.dirty = false,
                    Err(error) => {
                        error!("Failed to save project to '{}': {}", save_path_str, error);
                        document.error = Some(format!("Could not save project: {error}"));
                    }
                }
                file_dialog_state
                    .is_open
                    .store(false, std::sync::atomic::Ordering::Relaxed);
            }
            FileDialogResult::Cancel => {
                file_dialog_state
                    .is_open
                    .store(false, std::sync::atomic::Ordering::Relaxed);
            }
        }
    }
}

pub fn update_studio_window_title(
    document: Res<DocumentState>,
    onboarding: Res<crate::studio::ui::panels::onboarding::OnboardingData>,
    mut windows: Query<&mut Window, With<bevy::window::PrimaryWindow>>,
) {
    if !document.is_changed() && !onboarding.is_changed() {
        return;
    }
    let name = std::path::Path::new(&onboarding.save_path)
        .file_stem()
        .and_then(|name| name.to_str())
        .unwrap_or("Untitled");
    let dirty = if document.dirty { "*" } else { "" };
    if let Some(mut window) = windows.iter_mut().next() {
        window.title = format!("{name}{dirty} — RaveEngine Studio");
    }
}

pub fn track_document_changes(
    mut document: ResMut<DocumentState>,
    changed: Query<
        (),
        (
            Without<Camera3d>,
            Or<(
                Changed<Name>,
                Changed<Transform>,
                Changed<crate::common::game::bricks::components::BrickShapeComponent>,
                Changed<crate::common::game::bricks::components::BrickPhysics>,
                Changed<crate::scripting::ecs::ServerScript>,
                Changed<crate::scripting::ecs::LocalScript>,
                Changed<crate::scripting::ecs::ModuleScript>,
            )>,
        ),
    >,
    mut initialized: Local<bool>,
) {
    if !*initialized {
        *initialized = true;
        return;
    }
    if !changed.is_empty() {
        document.dirty = true;
    }
}
