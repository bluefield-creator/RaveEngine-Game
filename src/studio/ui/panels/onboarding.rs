use bevy::prelude::*;
use bevy_egui::egui;
use bevy::pbr::ExtendedMaterial;
use crate::common::bricks::data::spawn_brick;

pub fn draw_onboarding(
    ctx: &egui::Context,
    next_onboarding_state: &mut ResMut<NextState<crate::studio::tools::OnboardingState>>,
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    studs_materials: &mut Assets<ExtendedMaterial<StandardMaterial, crate::common::bricks::studs::StudsExtension>>,
    studs_assets: &crate::common::bricks::studs::StudsAssets,
    count: &mut crate::common::bricks::data::BrickSpawnerCount,
    thumb_empty_tex: egui::TextureId,
    thumb_baseplate_tex: egui::TextureId,
) {
    let center = ctx.available_rect().center();

    egui::Window::new("Welcome to VERTEXIA")
        .pivot(egui::Align2::CENTER_CENTER)
        .fixed_pos(center)
        .resizable(false)
        .collapsible(false)
        .title_bar(false)
        .frame(egui::Frame::NONE
            .fill(egui::Color32::from_rgb(255, 255, 255))
            .corner_radius(8.0)
            .inner_margin(egui::Margin::same(24))
            .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(212, 212, 212)))
        )
        .show(ctx, |ui| {
            ui.set_max_width(550.0);
            ui.vertical_centered(|ui| {
                ui.label(egui::RichText::new("Welcome!").size(28.0).strong().color(egui::Color32::from_rgb(20, 20, 20)));
                ui.add_space(8.0);
                ui.label(egui::RichText::new("Welcome to the VERTEXIA studio! Pick a template to start off, or begin with an empty place.")
                    .size(14.0)
                    .color(egui::Color32::from_rgb(100, 100, 100)));
            });
            ui.add_space(24.0);

            ui.columns(2, |columns| {
                columns[0].vertical_centered(|ui| {
                    let card_id = ui.make_persistent_id("empty_card");
                    let is_hovered = ui.rect_contains_pointer(ui.max_rect());

                    let frame = egui::Frame::NONE
                        .fill(if is_hovered { egui::Color32::from_rgb(235, 242, 252) } else { egui::Color32::from_rgb(245, 246, 247) })
                        .corner_radius(12.0)
                        .stroke(egui::Stroke::new(1.0, if is_hovered { egui::Color32::from_rgb(80, 160, 240) } else { egui::Color32::from_rgb(220, 220, 220) }))
                        .inner_margin(egui::Margin::same(16));

                    let inner_res = frame.show(ui, |ui| {
                        ui.set_min_height(250.0);
                        ui.add_space(8.0);
                        ui.add(egui::Image::new((thumb_empty_tex, egui::vec2(220.0, 137.5))).corner_radius(12.0));
                        ui.add_space(12.0);
                        ui.label(egui::RichText::new("Empty").size(18.0).strong().color(egui::Color32::from_rgb(20, 20, 20)));
                        ui.add_space(4.0);
                        ui.label(egui::RichText::new("An empty world. The sky's the limit...")
                            .size(12.0)
                            .color(egui::Color32::from_rgb(120, 120, 120)));
                    });

                    let response = ui.interact(inner_res.response.rect, card_id, egui::Sense::click());
                    if response.hovered() {
                        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                    }
                    if response.clicked() {
                        next_onboarding_state.set(crate::studio::tools::OnboardingState::Inactive);
                    }
                });

                columns[1].vertical_centered(|ui| {
                    let card_id = ui.make_persistent_id("baseplate_card");
                    let is_hovered = ui.rect_contains_pointer(ui.max_rect());

                    let frame = egui::Frame::NONE
                        .fill(if is_hovered { egui::Color32::from_rgb(235, 242, 252) } else { egui::Color32::from_rgb(245, 246, 247) })
                        .corner_radius(12.0)
                        .stroke(egui::Stroke::new(1.0, if is_hovered { egui::Color32::from_rgb(80, 160, 240) } else { egui::Color32::from_rgb(220, 220, 220) }))
                        .inner_margin(egui::Margin::same(16));

                    let inner_res = frame.show(ui, |ui| {
                        ui.set_min_height(250.0);
                        ui.add_space(8.0);
                        ui.add(egui::Image::new((thumb_baseplate_tex, egui::vec2(220.0, 137.5))).corner_radius(12.0));
                        ui.add_space(12.0);
                        ui.label(egui::RichText::new("Baseplate").size(18.0).strong().color(egui::Color32::from_rgb(20, 20, 20)));
                        ui.add_space(4.0);
                        ui.label(egui::RichText::new("A small floating baseplate to get ya started!")
                            .size(12.0)
                            .color(egui::Color32::from_rgb(120, 120, 120)));
                    });

                    let response = ui.interact(inner_res.response.rect, card_id, egui::Sense::click());
                    if response.hovered() {
                        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                    }
                    if response.clicked() {
                        next_onboarding_state.set(crate::studio::tools::OnboardingState::Inactive);
                        
                        commands.spawn((
                            Mesh3d(meshes.add(Cuboid::new(4.0 * 0.28, 1.0 * 0.28, 2.0 * 0.28))),
                            MeshMaterial3d(studs_materials.add(ExtendedMaterial {
                                base: StandardMaterial {
                                    base_color: Color::srgb(0.28, 0.62, 0.32),
                                    perceptual_roughness: 0.95,
                                    reflectance: 0.08,
                                    metallic: 0.0,
                                    ..default()
                                },
                                extension: crate::common::bricks::studs::StudsExtension {
                                    stud_texture: studs_assets.stud.clone(),
                                    inlet_texture: studs_assets.inlet.clone(),
                                },
                            })),
                            Transform::from_xyz(0.0, -0.14, 0.0).with_scale(Vec3::new(25.0, 1.0, 50.0)),
                            crate::common::bricks::components::Brick,
                            Pickable::default(),
                            Name::new("Baseplate"),
                        ));

                        spawn_brick(commands, meshes, studs_materials, studs_assets, count, Vec3::new(0.0, 0.14, 0.0));
                    }
                });
            });
        });
}