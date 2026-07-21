use crate::common::game::bricks::data::BrickSpawnerCount;
use crate::common::game::bricks::data::spawn_brick;
use crate::common::game::bricks::studs::StudsAssets;
use crate::studio::tools::{Selection, SnapConfig, ToolState};
use bevy::pbr::ExtendedMaterial;
use bevy::prelude::*;
use bevy_egui::egui;

fn style_top_bar_menu(ui: &mut egui::Ui) {
    let widgets = &mut ui.style_mut().visuals.widgets;
    for visuals in [
        &mut widgets.inactive,
        &mut widgets.hovered,
        &mut widgets.active,
    ] {
        visuals.weak_bg_fill = egui::Color32::TRANSPARENT;
        visuals.bg_fill = egui::Color32::TRANSPARENT;
        visuals.bg_stroke = egui::Stroke::NONE;
    }
}

#[allow(deprecated)]
fn top_bar_menu<R>(
    ui: &mut egui::Ui,
    id_source: &'static str,
    label: &'static str,
    add_contents: impl FnOnce(&mut egui::Ui) -> R,
) {
    let id = ui.make_persistent_id(id_source);
    let hovered = ui.data(|data| data.get_temp::<bool>(id)).unwrap_or(false);
    let color = if hovered {
        egui::Color32::from_rgb(80, 160, 240)
    } else {
        egui::Color32::from_rgb(60, 60, 60)
    };
    let response = ui
        .scope(|ui| {
            style_top_bar_menu(ui);
            egui::menu::menu_button(
                ui,
                egui::RichText::new(label).color(color).size(13.0),
                add_contents,
            )
        })
        .inner
        .response;
    ui.data_mut(|data| data.insert_temp(id, response.hovered()));
}

#[allow(deprecated)]
pub fn draw_top_bar(
    ui: &mut egui::Ui,
    next_tool: &mut NextState<ToolState>,
    current_tool: &State<ToolState>,
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    studs_materials: &mut ResMut<
        Assets<
            ExtendedMaterial<StandardMaterial, crate::common::game::bricks::studs::StudsExtension>,
        >,
    >,
    studs_assets: &StudsAssets,
    count: &mut ResMut<BrickSpawnerCount>,
    snap_config: &mut ResMut<SnapConfig>,
    move_tex: egui::TextureId,
    rotate_tex: egui::TextureId,
    scale_tex: egui::TextureId,
    add_tex: egui::TextureId,
    play_tex: egui::TextureId,
    playc_tex: egui::TextureId,
    stopp_tex: egui::TextureId,
    diagnostics: &Res<bevy::diagnostic::DiagnosticsStore>,
    camera_transform: Option<&Transform>,
    _action_writer: &mut MessageWriter<crate::studio::tools::UndoRedoAction>,
    history: &mut ResMut<crate::studio::tools::UndoRedoHistory>,
    physics_state: crate::common::game::physics::PhysicsSimulationState,
    physics_action_writer: &mut MessageWriter<
        crate::common::game::physics::PhysicsSimulationAction,
    >,
    settings_window: &mut ResMut<crate::studio::ui::SettingsWindow>,
    graphics_settings: &mut crate::studio::ui::GraphicsSettings,
    gravity: &mut Option<ResMut<avian3d::prelude::Gravity>>,
    camera_transform_query: &mut Query<&mut Transform, With<Camera3d>>,
    entities_query: &mut Query<
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
                    ExtendedMaterial<
                        StandardMaterial,
                        crate::common::game::bricks::studs::StudsExtension,
                    >,
                >,
            >,
            Option<&mut crate::common::game::bricks::components::BrickPhysics>,
            Option<&crate::common::game::bricks::components::BrickColor>,
        ),
        Without<Camera3d>,
    >,
    onboarding_data: &mut crate::studio::ui::panels::onboarding::OnboardingData,
    _play_processes: &mut ResMut<crate::studio::ui::resources::PlayInClientProcesses>,
    playtest_state: &mut ResMut<crate::client::PlaytestState>,
    playtest_backup: &mut ResMut<crate::studio::ui::resources::PlaytestBackup>,
    playtest_client_query: &Query<
        Entity,
        With<crate::studio::ui::resources::InEditorPlaytestClient>,
    >,
    selection: &Selection,
    explorer_query: &Query<
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
    onboarding_active: bool,
    players_service: &mut Option<ResMut<crate::studio::tools::PlayersService>>,
    lighting_service: &mut Option<
        ResMut<crate::common::game::environment::lighting::LightingService>,
    >,
    file_dialog_state: &crate::studio::ui::resources::FileDialogState,
    actions: &mut crate::studio::ui::resources::EditorActionQueue,
    layout: &crate::studio::ui::resources::EditorLayoutState,
    document: &mut crate::studio::ui::resources::DocumentState,
) {
    ui.style_mut().interaction.selectable_labels = false;

    egui::Frame::NONE
        .inner_margin(egui::Margin::symmetric(12, 6))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing = egui::vec2(16.0, 0.0);

                let file_id = ui.make_persistent_id("file_menu_btn");
                let is_file_hovered = ui.data(|d| d.get_temp::<bool>(file_id)).unwrap_or(false);
                let file_text_color = if is_file_hovered {
                    egui::Color32::from_rgb(80, 160, 240)
                } else {
                    egui::Color32::from_rgb(60, 60, 60)
                };

                let mut is_hovered = false;
                ui.scope(|ui| {
                    style_top_bar_menu(ui);

                    let file_button_res = egui::menu::menu_button(ui, egui::RichText::new("File").color(file_text_color).size(13.0), |ui| {
                        if ui.button("New Project\tCtrl+N").clicked() {
                            actions.0.push(crate::studio::ui::resources::EditorAction::NewProject);
                            ui.close();
                        }
                        if ui.button("Open…\tCtrl+O").clicked() {
                            actions.0.push(crate::studio::ui::resources::EditorAction::Open);
                            ui.close();
                        }
                        ui.separator();
                        let save_enabled = !onboarding_data.save_path.is_empty();
                        if ui.add_enabled(save_enabled, egui::Button::new("Save\tCtrl+S")).clicked() {
                            let node_ids: std::collections::HashMap<Entity, u64> = explorer_query.iter()
                                .filter(|(_, _, _, _, brick, server, local, module)| brick.is_some() || server.is_some() || local.is_some() || module.is_some())
                                .enumerate().map(|(index, (entity, _, _, _, _, _, _, _))| (entity, index as u64)).collect();
                            let mut bricks_data = Vec::new();
                            for (entity, transform, name, child_of, _, brick_opt, shape_opt, _, _, mat_opt, studs_mat_opt, phys_opt, _) in entities_query.iter() {
                                if brick_opt.is_some() {
                                    let shape = shape_opt.as_ref().map(|s| s.shape).unwrap_or(crate::common::game::bricks::components::BrickShape::Block);
                                    let mut current_color = Color::Srgba(Srgba::new(0.84, 0.24, 0.16, 1.0));
                                    if let Some(studs_mat_handle) = studs_mat_opt {
                                        if let Some(mat) = studs_materials.get(&studs_mat_handle.0) {
                                            current_color = mat.base.base_color;
                                        }
                                    } else if let Some(mat_handle) = mat_opt
                                        && let Some(mat) = materials.get(&mat_handle.0) {
                                            current_color = mat.base_color;
                                        }
                                    let (physics_enabled, bounciness, player_can_collide, friction, gravity_scale, mass) = if let Some(phys) = phys_opt {
                                        (phys.enabled, phys.bounciness, phys.player_can_collide, phys.friction, phys.gravity_scale, phys.mass)
                                    } else {
                                        (true, 0.3, true, 0.3, 1.0, 1.0)
                                    };
                                    bricks_data.push(crate::common::core::vrtx::VrtxBrick {
                                        node_id: node_ids.get(&entity).copied().unwrap_or(bricks_data.len() as u64),
                                        parent_node_id: child_of.and_then(|parent| node_ids.get(&parent.parent()).copied()),
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
                            for (entity, name, child_of_opt, _, _, s_opt, l_opt, m_opt) in explorer_query.iter() {
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
                                    if let Some(child_of) = child_of_opt
                                        && let Ok((_, p_name, _, _, _, _, _, _)) = explorer_query.get(child_of.parent()) {
                                            parent_name = Some(p_name.to_string());
                                        }
                                    scripts_data.push(crate::common::core::vrtx::VrtxScript {
                                        node_id: node_ids.get(&entity).copied().unwrap_or((bricks_data.len() + scripts_data.len()) as u64),
                                        parent_node_id: child_of_opt.and_then(|parent| node_ids.get(&parent.parent()).copied()),
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
                                settings: crate::common::core::vrtx::VrtxSettings::from_graphics(
                                    graphics_settings,
                                ),
                                lighting: lighting_service
                                    .as_ref()
                                    .map(|service| (**service).clone())
                                    .unwrap_or_default(),
                                camera_transform: cam_transform,
                                bricks: bricks_data,
                                scripts: scripts_data,
                            };
                            match state.save_to_file(&onboarding_data.save_path) {
                                Ok(()) => document.dirty = false,
                                Err(error) => document.error = Some(format!("Could not save project: {error}")),
                            }
                            ui.close_menu();
                        }

                        let is_open = file_dialog_state.is_open.load(std::sync::atomic::Ordering::Relaxed);
                        if ui.add_enabled(!is_open, egui::Button::new("Save As…\tCtrl+Shift+S")).clicked() {
                            file_dialog_state.is_open.store(true, std::sync::atomic::Ordering::Relaxed);
                            let tx = file_dialog_state.tx.clone();
                            std::thread::spawn(move || {
                                if let Some(path) = rfd::FileDialog::new()
                                    .add_filter("Rave Project", &["vrtx"])
                                    .set_directory(std::env::current_dir().unwrap_or_default())
                                    .save_file() {
                                    let _ = tx.send(crate::studio::ui::resources::FileDialogResult::SaveAs(path));
                                } else {
                                    let _ = tx.send(crate::studio::ui::resources::FileDialogResult::Cancel);
                                }
                            });
                            ui.close_menu();
                        }
                        ui.separator();
                        if ui.button("Exit").clicked() {
                            actions.0.push(crate::studio::ui::resources::EditorAction::Exit);
                            ui.close();
                        }
                    });
                    is_hovered = file_button_res.response.hovered();
                });
                ui.data_mut(|d| d.insert_temp(file_id, is_hovered));

                top_bar_menu(ui, "edit_menu_btn", "Edit", |ui| {
                    for (label, action, enabled) in [
                        ("Undo\tCtrl+Z", crate::studio::ui::resources::EditorAction::Undo, !history.undo_stack.is_empty()),
                        ("Redo\tCtrl+Y", crate::studio::ui::resources::EditorAction::Redo, !history.redo_stack.is_empty()),
                        ("Cut\tCtrl+X", crate::studio::ui::resources::EditorAction::Cut, !selection.entities.is_empty()),
                        ("Copy\tCtrl+C", crate::studio::ui::resources::EditorAction::Copy, !selection.entities.is_empty()),
                        ("Paste\tCtrl+V", crate::studio::ui::resources::EditorAction::Paste, true),
                        ("Duplicate\tCtrl+D", crate::studio::ui::resources::EditorAction::Duplicate, !selection.entities.is_empty()),
                        ("Rename\tF2", crate::studio::ui::resources::EditorAction::Rename, selection.entities.len() == 1),
                        ("Delete\tDelete", crate::studio::ui::resources::EditorAction::Delete, !selection.entities.is_empty()),
                        ("Select All\tCtrl+A", crate::studio::ui::resources::EditorAction::SelectAll, true),
                    ] {
                        if ui.add_enabled(enabled, egui::Button::new(label)).clicked() { actions.0.push(action); ui.close(); }
                    }
                });
                top_bar_menu(ui, "insert_menu_btn", "Insert", |ui| {
                    for (label, kind) in [("Part", crate::studio::ui::resources::InsertKind::Part), ("Sphere", crate::studio::ui::resources::InsertKind::Sphere), ("Script", crate::studio::ui::resources::InsertKind::ServerScript), ("LocalScript", crate::studio::ui::resources::InsertKind::LocalScript), ("ModuleScript", crate::studio::ui::resources::InsertKind::ModuleScript)] {
                        if ui.button(label).clicked() { actions.0.push(crate::studio::ui::resources::EditorAction::Insert(kind, selection.entity)); ui.close(); }
                    }
                });
                top_bar_menu(ui, "view_menu_btn", "View", |ui| {
                    if ui.selectable_label(layout.explorer_visible, "Explorer").clicked() { actions.0.push(crate::studio::ui::resources::EditorAction::ToggleExplorer); }
                    if ui.selectable_label(layout.properties_visible, "Properties").clicked() { actions.0.push(crate::studio::ui::resources::EditorAction::ToggleProperties); }
                    if ui.selectable_label(layout.script_editor_visible, "Script Editor").clicked() { actions.0.push(crate::studio::ui::resources::EditorAction::ToggleScriptEditor); }
                    ui.separator();
                    if ui.add_enabled(!selection.entities.is_empty(), egui::Button::new("Frame Selected\tF")).clicked() { actions.0.push(crate::studio::ui::resources::EditorAction::FrameSelected); ui.close(); }
                    if ui.button("Reset Camera").clicked() { actions.0.push(crate::studio::ui::resources::EditorAction::ResetCamera); ui.close(); }
                    if ui.button("Reset Layout").clicked() { actions.0.push(crate::studio::ui::resources::EditorAction::ResetLayout); ui.close(); }
                });
                top_bar_menu(ui, "test_menu_btn", "Test", |ui| {
                    let simulation = if physics_state == crate::common::game::physics::PhysicsSimulationState::Running { "Stop Simulation\tF6" } else { "Play Simulation\tF6" };
                    if ui.button(simulation).clicked() { ui.data_mut(|data| data.insert_temp(egui::Id::new("trigger_simulation"), true)); ui.close(); }
                    let playtest = if playtest_state.active { "Stop Playtest\tShift+F5" } else { "Play in Studio\tF5" };
                    if ui.button(playtest).clicked() { ui.data_mut(|data| data.insert_temp(egui::Id::new("trigger_playtest"), true)); ui.close(); }
                });

                let settings_id = ui.make_persistent_id("settings_menu_btn");
                let is_hovered_settings = ui.data(|d| d.get_temp::<bool>(settings_id)).unwrap_or(false);
                let text_color = if is_hovered_settings {
                    egui::Color32::from_rgb(80, 160, 240)
                } else {
                    egui::Color32::from_rgb(60, 60, 60)
                };

                let settings_btn = ui.add(egui::Label::new(egui::RichText::new("Settings").color(text_color).size(13.0)).sense(egui::Sense::click()));
                ui.data_mut(|d| d.insert_temp(settings_id, settings_btn.hovered()));

                if settings_btn.clicked() {
                    settings_window.open = !settings_window.open;
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let fps = if let Some(diag) = diagnostics.get(&bevy::diagnostic::FrameTimeDiagnosticsPlugin::FPS) {
                        diag.smoothed().unwrap_or_default()
                    } else {
                        0.0
                    };
                    ui.label(
                        egui::RichText::new(format!("FPS: {:.0}", fps))
                            .color(egui::Color32::from_rgb(100, 100, 100))
                            .size(13.0)
                    );
                });
            });
        });

    let (bottom_sep, _) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), 1.0), egui::Sense::hover());
    ui.painter()
        .rect_filled(bottom_sep, 0.0, egui::Color32::from_rgb(212, 212, 212));

    egui::Frame::NONE
        .inner_margin(egui::Margin::symmetric(12, 8))
        .show(ui, |ui| {
            ui.add_enabled_ui(!onboarding_active, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(4.0, 0.0);

                    let is_move = *current_tool.get() == ToolState::Move;
                    if ribbonbutton(ui, Some(move_tex), "Move", is_move).clicked() {
                        if is_move {
                            next_tool.set(ToolState::None);
                        } else {
                            next_tool.set(ToolState::Move);
                        }
                    }

                    let is_rotate = *current_tool.get() == ToolState::Rotate;
                    if ribbonbutton(ui, Some(rotate_tex), "Rotate", is_rotate).clicked() {
                        if is_rotate {
                            next_tool.set(ToolState::None);
                        } else {
                            next_tool.set(ToolState::Rotate);
                        }
                    }

                    let is_scale = *current_tool.get() == ToolState::Size;
                    if ribbonbutton(ui, Some(scale_tex), "Scale", is_scale).clicked() {
                        if is_scale {
                            next_tool.set(ToolState::None);
                        } else {
                            next_tool.set(ToolState::Size);
                        }
                    }

                    ui.add_space(8.0);
                    let (sep_rect_snap, _) = ui.allocate_exact_size(egui::vec2(1.0, 56.0), egui::Sense::hover());
                    ui.painter().rect_filled(sep_rect_snap, 0.0, egui::Color32::from_rgb(212, 212, 212));
                    ui.add_space(8.0);

                    ui.vertical(|ui| {
                        ui.add_space(6.0);
                        let mut enabled = snap_config.enabled;
                        if ui.checkbox(&mut enabled, "Snap").changed() {
                            snap_config.enabled = enabled;
                            if enabled {
                                snap_config.distance = 1.0;
                            }
                        }
                        if snap_config.enabled {
                            ui.add_space(2.0);
                            ui.horizontal(|ui| {
                                ui.add(egui::DragValue::new(&mut snap_config.distance)
                                    .speed(0.1)
                                    .range(0.01..=1000.0)
                                    .suffix(" stud")
                                );
                            });
                        }
                    });

                    ui.add_space(8.0);
                    let (sep_rect, _) = ui.allocate_exact_size(egui::vec2(1.0, 56.0), egui::Sense::hover());
                    ui.painter().rect_filled(sep_rect, 0.0, egui::Color32::from_rgb(212, 212, 212));
                    ui.add_space(8.0);

                    let add_btn = ribbonbutton(ui, Some(add_tex), "Add", false);
                    let popup_id = ui.make_persistent_id("add_part_popup");
                    if add_btn.clicked() {
                        ui.memory_mut(|mem| mem.toggle_popup(popup_id));
                    }

                    let mut search_query = ui.data_mut(|d| d.get_temp::<String>(popup_id).unwrap_or_default());

                    let original_window_fill = ui.visuals().window_fill;
                    let original_window_stroke = ui.visuals().window_stroke;

                    ui.visuals_mut().window_fill = egui::Color32::from_rgb(255, 255, 255);
                    ui.visuals_mut().window_stroke = egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(212, 212, 212));

                    let _: Option<()> = egui::popup_below_widget(
                        ui,
                        popup_id,
                        &add_btn,
                        egui::PopupCloseBehavior::CloseOnClickOutside,
                        |ui: &mut egui::Ui| {
                            ui.visuals_mut().widgets.hovered.bg_fill = egui::Color32::from_rgb(224, 238, 249);
                            ui.visuals_mut().widgets.active.bg_fill = egui::Color32::from_rgb(204, 232, 255);
                            ui.visuals_mut().widgets.inactive.bg_fill = egui::Color32::from_rgb(255, 255, 255);
                            ui.visuals_mut().widgets.noninteractive.bg_fill = egui::Color32::from_rgb(255, 255, 255);

                            ui.set_min_width(150.0);
                            ui.horizontal(|ui| {
                                ui.label("🔍"); 
                                let text_edit_res = ui.text_edit_singleline(&mut search_query);
                                if text_edit_res.changed() {
                                    ui.data_mut(|d| d.insert_temp(popup_id, search_query.clone()));
                                }
                            });
                            ui.separator();

                            let items = [
                                ("Block", crate::common::game::bricks::components::BrickShape::Block),
                                ("Sphere", crate::common::game::bricks::components::BrickShape::Sphere),
                            ];
                            for (item, shape) in items {
                                if item.to_lowercase().contains(&search_query.to_lowercase())
                                    && ui.button(item).clicked() {
                                        let mut spawn_pos = Vec3::new(0.0, 0.14, 0.0);
                                        if let Some(cam_t) = camera_transform {
                                            let camera_pos = cam_t.translation;
                                            let camera_forward = cam_t.forward();

                                            let mut found_hit = false;
                                            if camera_forward.y.abs() > 0.001 {
                                                let resting_y = 0.5 * 0.28;
                                                let t = (resting_y - camera_pos.y) / camera_forward.y;
                                                if t > 0.0 && t < 100.0 {
                                                    let hit_pos = camera_pos + camera_forward * t;
                                                    if hit_pos.x.abs() <= 25.0 && hit_pos.z.abs() <= 25.0 {
                                                        spawn_pos = hit_pos;
                                                        found_hit = true;
                                                    }
                                                }
                                            }
                                            if !found_hit {
                                                spawn_pos = camera_pos + camera_forward * (10.0 * 0.28);
                                            }
                                        }

                                        if snap_config.enabled && snap_config.distance > 0.0 {
                                            let snap_interval = snap_config.distance * 0.28;
                                            let half_ext = match shape {
                                                crate::common::game::bricks::components::BrickShape::Block => Vec3::new(2.0 * 0.28, 0.5 * 0.28, 1.0 * 0.28),
                                                crate::common::game::bricks::components::BrickShape::Sphere => Vec3::new(1.0 * 0.28, 1.0 * 0.28, 1.0 * 0.28),
                                            };
                                            spawn_pos.x = ((spawn_pos.x - half_ext.x) / snap_interval).round() * snap_interval + half_ext.x;
                                            spawn_pos.z = ((spawn_pos.z - half_ext.z) / snap_interval).round() * snap_interval + half_ext.x;
                                            spawn_pos.y = ((spawn_pos.y - half_ext.y) / snap_interval).round() * snap_interval + half_ext.y;
                                            if spawn_pos.y < half_ext.y {
                                                spawn_pos.y = half_ext.y;
                                            }
                                        }

                                        let new_entity = spawn_brick(commands, meshes, studs_materials, studs_assets, count, spawn_pos, shape);

                                        let default_name = match shape {
                                            crate::common::game::bricks::components::BrickShape::Block => format!("Part{}", count.count - 1),
                                            crate::common::game::bricks::components::BrickShape::Sphere => format!("Sphere{}", count.count - 1),
                                        };

                                        let default_mesh = match shape {
                                            crate::common::game::bricks::components::BrickShape::Block => Some(Mesh3d(meshes.add(Cuboid::new(4.0 * 0.28, 1.0 * 0.28, 2.0 * 0.28)))),
                                            crate::common::game::bricks::components::BrickShape::Sphere => Some(Mesh3d(meshes.add(Sphere::new(1.0 * 0.28)))),
                                        };

                                        let data = crate::common::game::bricks::data::BrickData {
                                            transform: Transform::from_translation(spawn_pos),
                                            name: default_name,
                                            is_brick: true,
                                            shape,
                                            mesh: default_mesh,
                                            standard_material: None,
                                            studs_material: Some(MeshMaterial3d(studs_materials.add(ExtendedMaterial {
                                                base: StandardMaterial {
                                                    base_color: Color::srgb(0.84, 0.24, 0.16),
                                                    perceptual_roughness: 0.95,
                                                    reflectance: 0.1,
                                                    ..default()
                                                },
                                                extension: crate::common::game::bricks::studs::StudsExtension {
                                                    stud_texture: studs_assets.stud.clone(),
                                                    inlet_texture: studs_assets.inlet.clone(),
                                                },
                                            }))),
                                            parent: None,
                                            physics: Some(crate::common::game::bricks::components::BrickPhysics::default()),
                                            color: None,
                                        };

                                        history.push_command(crate::studio::tools::UndoCommand::Spawn {
                                            entity: new_entity,
                                            data,
                                        });

                                        ui.memory_mut(|mem| mem.close_popup(popup_id));
                                    }
                            }

                            let script_items = [
                                ("Script", 0),
                                ("LocalScript", 1),
                                ("ModuleScript", 2),
                            ];
                            for (item, script_type) in script_items {
                                if item.to_lowercase().contains(&search_query.to_lowercase())
                                    && ui.button(item).clicked() {
                                        let new_entity = match script_type {
                                            0 => commands.spawn((
                                                Name::new(item),
                                                crate::scripting::ecs::ServerScript {
                                                    code: "print(\"Hello World from Server!\")\n".to_string(),
                                                    enabled: true,
                                                    started: false,
                                                },
                                            )).id(),
                                            1 => commands.spawn((
                                                Name::new(item),
                                                crate::scripting::ecs::LocalScript {
                                                    code: "print(\"Hello World from Local!\")\n".to_string(),
                                                    enabled: true,
                                                    started: false,
                                                },
                                                lightyear::prelude::Replicate::default(),
                                            )).id(),
                                            _ => commands.spawn((
                                                Name::new(item),
                                                crate::scripting::ecs::ModuleScript {
                                                    code: "local module = {}\nreturn module\n".to_string(),
                                                },
                                                lightyear::prelude::Replicate::default(),
                                            )).id(),
                                        };

                                        if let Some(parent) = selection.entity {
                                            commands.entity(parent).add_child(new_entity);
                                        }

                                        ui.memory_mut(|mem| mem.close_popup(popup_id));
                                    }
                            }
                        },
                    );

                    ui.add_space(8.0);
                    ui.visuals_mut().window_fill = original_window_fill;
                    ui.visuals_mut().window_stroke = original_window_stroke;

                    ui.add_space(8.0);
                    let (sep_rect_play, _) = ui.allocate_exact_size(egui::vec2(1.0, 56.0), egui::Sense::hover());
                    ui.painter().rect_filled(sep_rect_play, 0.0, egui::Color32::from_rgb(212, 212, 212));
                    ui.add_space(8.0);

                    let is_playing = physics_state == crate::common::game::physics::PhysicsSimulationState::Running;
                    let play_btn_tex = if is_playing { stopp_tex } else { play_tex };
                    let play_btn_label = if is_playing { "Stop" } else { "Play" };

                    let trigger_simulation = ui.data_mut(|data| data.remove_temp::<bool>(egui::Id::new("trigger_simulation"))).unwrap_or(false);
                    if ribbonbutton(ui, Some(play_btn_tex), play_btn_label, is_playing).clicked() || trigger_simulation {
                        if is_playing {
                            physics_action_writer.write(crate::common::game::physics::PhysicsSimulationAction::Stop);
                        } else {
                            physics_action_writer.write(crate::common::game::physics::PhysicsSimulationAction::Play);
                        }
                    }

                    let playtesting_active = playtest_state.active;
                    let playc_btn_label = if playtesting_active { "Stop Playtest" } else { "Play in Studio" };
                    let playc_btn_tex = if playtesting_active { stopp_tex } else { playc_tex };

                    let trigger_playtest = ui.data_mut(|data| data.remove_temp::<bool>(egui::Id::new("trigger_playtest"))).unwrap_or(false);
                    if ribbonbutton(ui, Some(playc_btn_tex), playc_btn_label, playtesting_active).clicked() || trigger_playtest {
                        if playtesting_active {
                            playtest_state.active = false;

                            crate::app::server::bootstrap::SHUTDOWN_SERVER.store(true, std::sync::atomic::Ordering::Relaxed);

                            if let Some(client_entity) = playtest_client_query.iter().next() {
                                commands.trigger(lightyear::prelude::client::Disconnect { entity: client_entity });
                                commands.entity(client_entity).despawn();
                            }

                            for (entity, _, _name, _, _, brick_opt, _, _, _, _, _, _, _) in entities_query.iter() {
                                let name_str = _name.as_str();
                                if brick_opt.is_some() || name_str == "Player" || name_str == "LocalPlayer" || name_str.starts_with("Player_") {
                                    commands.entity(entity).despawn();
                                }
                            }

                            for snapshot in playtest_backup.snapshots.drain(..) {
                                crate::studio::ui::resources::spawn_editor_snapshot(
                                    commands,
                                    &snapshot,
                                    snapshot.parent,
                                );
                            }

                            if let Some(gravity_val) = playtest_backup.gravity.take()
                                && let Some(g) = gravity {
                                    g.0 = gravity_val;
                                }
                            if let Some(ps_val) = playtest_backup.players_service.take() {
                                if let Some(ps) = players_service {
                                    **ps = ps_val.clone();
                                }
                                if let Ok(mut shared) = crate::studio::tools::SHARED_PLAYERS_SERVICE.write() {
                                    *shared = ps_val;
                                }
                            }
                            if let Some(ls_val) = playtest_backup.lighting_service.take() {
                                if let Some(ls) = lighting_service {
                                    **ls = ls_val.clone();
                                }
                                if let Ok(mut shared) = crate::studio::tools::SHARED_LIGHTING_SERVICE.write() {
                                    *shared = ls_val.time_of_day;
                                }
                            }
                        } else {
                            playtest_state.active = true;

                            if let Some(g) = gravity.as_ref() {
                                playtest_backup.gravity = Some(g.0);
                            } else {
                                playtest_backup.gravity = None;
                            }
                            if let Some(ps) = players_service.as_ref() {
                                playtest_backup.players_service = Some((**ps).clone());
                            } else {
                                playtest_backup.players_service = None;
                            }
                            if let Some(ls) = lighting_service.as_ref() {
                                playtest_backup.lighting_service = Some((**ls).clone());
                            } else {
                                playtest_backup.lighting_service = None;
                            }

                            let roots: Vec<_> = explorer_query
                                .iter()
                                .filter_map(|(entity, _, parent, _, brick, server, local, module)| {
                                    (parent.is_none()
                                        && (brick.is_some()
                                            || server.is_some()
                                            || local.is_some()
                                            || module.is_some()))
                                    .then_some(entity)
                                })
                                .collect();
                            playtest_backup.snapshots = roots
                                .iter()
                                .filter_map(|entity| {
                                    crate::studio::ui::resources::capture_editor_snapshot(
                                        *entity,
                                        explorer_query,
                                        entities_query,
                                    )
                                })
                                .collect();
                            let (playtest_bricks, playtest_scripts) =
                                crate::studio::ui::resources::snapshots_to_vrtx(
                                    &playtest_backup.snapshots,
                                );
                            let temp_map_path = "temp_play.vrtx".to_string();
                            let state = crate::common::core::vrtx::VrtxFileState {
                                version: crate::common::core::vrtx::CURRENT_VRTX_VERSION,
                                gravity: Vec3::new(0.0, -186.9 * 0.28, 0.0),
                                settings: crate::common::core::vrtx::VrtxSettings::from_graphics(
                                    graphics_settings,
                                ),
                                lighting: lighting_service
                                    .as_ref()
                                    .map(|service| (**service).clone())
                                    .unwrap_or_default(),
                                camera_transform: Transform::IDENTITY,
                                bricks: playtest_bricks,
                                scripts: playtest_scripts,
                            };

                            if state.save_to_file(&temp_map_path).is_ok() {
                                for entity in roots {
                                    commands.entity(entity).despawn();
                                }
                                crate::app::server::bootstrap::SHUTDOWN_SERVER.store(false, std::sync::atomic::Ordering::Relaxed);
                                let netcode_key = rand::random::<[u8; 32]>();

                                let server_app = crate::app::server::bootstrap::RaveServerApp::new(
                                    crate::app::server::config::ServerAppConfig {
                                        port: 5000,
                                        map_path: temp_map_path,
                                        bind_ip: std::net::IpAddr::V4(std::net::Ipv4Addr::LOCALHOST),
                                        netcode_key,
                                        embedded_server: true,
                                    }
                                );
                                std::thread::spawn(move || {
                                    server_app.run();
                                });

                                std::thread::sleep(std::time::Duration::from_millis(100));

                                let server_addr = std::net::SocketAddr::new(
                                    std::net::IpAddr::V4(std::net::Ipv4Addr::new(127, 0, 0, 1)),
                                    5000,
                                );
                                let client_id = rand::random::<u64>();

                                commands.insert_resource(crate::client::LocalClientId(client_id));
                                commands.insert_resource(crate::client::ClientUkey("studio_play_local_key".to_string()));

                                let auth = lightyear::prelude::Authentication::Manual {
                                    server_addr,
                                    client_id,
                                    private_key: netcode_key,
                                    protocol_id: crate::common::net::NETCODE_PROTOCOL_ID,
                                };

                                let netcode_config = lightyear::prelude::client::NetcodeConfig {
                                    client_timeout_secs: 15,
                                    ..default()
                                };

                                let netcode_client = match lightyear::prelude::client::NetcodeClient::new(auth, netcode_config) {
                                    Ok(c) => c,
                                    Err(e) => {
                                        error!("Failed to create playtest network client: {e}");
                                        return;
                                    }
                                };

                                let client_entity = commands.spawn((
                                    lightyear::prelude::client::Client::default(),
                                    lightyear::prelude::UdpIo::default(),
                                    netcode_client,
                                    lightyear::prelude::LocalAddr(std::net::SocketAddr::new(
                                        std::net::IpAddr::V4(std::net::Ipv4Addr::new(0, 0, 0, 0)),
                                        0,
                                    )),
                                    lightyear::prelude::PeerAddr(server_addr),
                                    crate::studio::ui::resources::InEditorPlaytestClient,
                                )).id();

                                commands.trigger(lightyear::prelude::client::Connect { entity: client_entity });
                            } else {
                                playtest_state.active = false;
                                playtest_backup.snapshots.clear();
                            }
                        }
                    }
                });
            });
        });

    let (bottom_sep, _) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), 1.0), egui::Sense::hover());
    ui.painter()
        .rect_filled(bottom_sep, 0.0, egui::Color32::from_rgb(180, 180, 180));
}

#[allow(deprecated)]
fn ribbonbutton(
    ui: &mut egui::Ui,
    icon: Option<egui::TextureId>,
    label: &str,
    selected: bool,
) -> egui::Response {
    let width = if label.len() > 8 { 88.0 } else { 56.0 };
    let size = egui::vec2(width, 56.0);
    let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click());

    if selected {
        ui.painter()
            .rect_filled(rect, 4.0, egui::Color32::from_rgb(204, 232, 255));
        ui.painter().rect_stroke(
            rect,
            4.0,
            egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(153, 209, 255)),
            egui::StrokeKind::Inside,
        );
    } else if response.hovered() {
        ui.painter()
            .rect_filled(rect, 4.0, egui::Color32::from_rgb(224, 238, 249));
        ui.painter().rect_stroke(
            rect,
            4.0,
            egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(190, 220, 240)),
            egui::StrokeKind::Inside,
        );
    }

    ui.scope_builder(egui::UiBuilder::new().max_rect(rect), |ui| {
        ui.vertical_centered(|ui| {
            if label.is_empty() {
                if let Some(texture_id) = icon {
                    ui.add_space(14.0);
                    ui.add(egui::Image::new((texture_id, egui::vec2(28.0, 28.0))));
                }
            } else {
                if let Some(texture_id) = icon {
                    ui.add_space(5.0);
                    ui.add(egui::Image::new((texture_id, egui::vec2(28.0, 28.0))));
                    ui.add_space(1.0);
                } else {
                    ui.add_space(18.0);
                }
                let text_color = egui::Color32::from_rgb(20, 20, 20);
                ui.label(egui::RichText::new(label).color(text_color).size(11.0));
            }
        });
    });

    response
}
