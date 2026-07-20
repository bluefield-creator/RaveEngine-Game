use crate::common::core::performance::{AmbientOcclusionQuality, AntiAliasing, ShadowQuality};
use bevy_egui::egui;

pub fn draw_settings_window(
    ctx: &egui::Context,
    open: &mut bool,
    graphics_settings: &mut crate::studio::ui::GraphicsSettings,
) -> bool {
    let previous = graphics_settings.clone();
    if graphics_settings.ssao
        && matches!(
            graphics_settings.anti_aliasing,
            AntiAliasing::Msaa2 | AntiAliasing::Msaa4 | AntiAliasing::Msaa8
        )
    {
        graphics_settings.anti_aliasing = AntiAliasing::Fxaa;
    }
    egui::Window::new("Settings")
        .open(open)
        .pivot(egui::Align2::CENTER_CENTER)
        .default_pos(ctx.content_rect().center())
        .default_size(egui::vec2(390.0, 460.0))
        .resizable(false)
        .collapsible(false)
        .show(ctx, |ui| {
            ui.label(
                egui::RichText::new("Lighting & Shadows")
                    .strong()
                    .size(14.0),
            );
            ui.add_space(6.0);
            egui::Grid::new("lighting_graphics_settings")
                .num_columns(2)
                .spacing([16.0, 8.0])
                .show(ui, |ui| {
                    ui.label("Shadow Quality");
                    egui::ComboBox::from_id_salt("shadow_quality")
                        .selected_text(match graphics_settings.shadow_quality {
                            ShadowQuality::Low => "Low",
                            ShadowQuality::Medium => "Medium",
                            ShadowQuality::High => "High",
                            ShadowQuality::Ultra => "Ultra",
                        })
                        .show_ui(ui, |ui| {
                            ui.selectable_value(
                                &mut graphics_settings.shadow_quality,
                                ShadowQuality::Low,
                                "Low (512)",
                            );
                            ui.selectable_value(
                                &mut graphics_settings.shadow_quality,
                                ShadowQuality::Medium,
                                "Medium (1024)",
                            );
                            ui.selectable_value(
                                &mut graphics_settings.shadow_quality,
                                ShadowQuality::High,
                                "High (2048)",
                            );
                            ui.selectable_value(
                                &mut graphics_settings.shadow_quality,
                                ShadowQuality::Ultra,
                                "Ultra (4096)",
                            );
                        });
                    ui.end_row();

                    ui.label("Soft Shadows");
                    ui.checkbox(&mut graphics_settings.soft_shadows, "");
                    ui.end_row();

                    ui.label("Contact Shadows");
                    ui.checkbox(&mut graphics_settings.contact_shadows, "");
                    ui.end_row();

                    ui.label("Contact Distance");
                    ui.add_enabled(
                        graphics_settings.contact_shadows,
                        egui::DragValue::new(&mut graphics_settings.contact_shadow_length)
                            .speed(0.05)
                            .range(0.05..=2.0),
                    );
                    ui.end_row();
                });

            ui.add_space(12.0);
            ui.separator();
            ui.add_space(8.0);
            ui.label(egui::RichText::new("Post Processing").strong().size(14.0));
            ui.add_space(6.0);
            egui::Grid::new("post_process_settings")
                .num_columns(2)
                .spacing([16.0, 8.0])
                .show(ui, |ui| {
                    ui.label("Ambient Occlusion");
                    ui.checkbox(&mut graphics_settings.ssao, "");
                    ui.end_row();

                    ui.label("AO Quality");
                    ui.add_enabled_ui(graphics_settings.ssao, |ui| {
                        egui::ComboBox::from_id_salt("ssao_quality")
                            .selected_text(match graphics_settings.ssao_quality {
                                AmbientOcclusionQuality::Low => "Low",
                                AmbientOcclusionQuality::Medium => "Medium",
                                AmbientOcclusionQuality::High => "High",
                                AmbientOcclusionQuality::Ultra => "Ultra",
                            })
                            .show_ui(ui, |ui| {
                                ui.selectable_value(
                                    &mut graphics_settings.ssao_quality,
                                    AmbientOcclusionQuality::Low,
                                    "Low",
                                );
                                ui.selectable_value(
                                    &mut graphics_settings.ssao_quality,
                                    AmbientOcclusionQuality::Medium,
                                    "Medium",
                                );
                                ui.selectable_value(
                                    &mut graphics_settings.ssao_quality,
                                    AmbientOcclusionQuality::High,
                                    "High",
                                );
                                ui.selectable_value(
                                    &mut graphics_settings.ssao_quality,
                                    AmbientOcclusionQuality::Ultra,
                                    "Ultra",
                                );
                            });
                    });
                    ui.end_row();

                    ui.label("Bloom");
                    ui.checkbox(&mut graphics_settings.bloom, "");
                    ui.end_row();

                    ui.label("Bloom Strength");
                    ui.add_enabled(
                        graphics_settings.bloom,
                        egui::Slider::new(&mut graphics_settings.bloom_intensity, 0.0..=0.3),
                    );
                    ui.end_row();

                    ui.label("Exposure");
                    ui.add(
                        egui::DragValue::new(&mut graphics_settings.exposure_ev100)
                            .speed(0.1)
                            .range(5.0..=16.0)
                            .suffix(" EV"),
                    );
                    ui.end_row();

                    ui.label("Anti-aliasing");
                    egui::ComboBox::from_id_salt("anti_aliasing")
                        .selected_text(match graphics_settings.anti_aliasing {
                            AntiAliasing::Off => "Off",
                            AntiAliasing::Fxaa => "FXAA",
                            AntiAliasing::Msaa2 => "MSAA 2×",
                            AntiAliasing::Msaa4 => "MSAA 4×",
                            AntiAliasing::Msaa8 => "MSAA 8×",
                        })
                        .show_ui(ui, |ui| {
                            ui.selectable_value(
                                &mut graphics_settings.anti_aliasing,
                                AntiAliasing::Off,
                                "Off",
                            );
                            ui.selectable_value(
                                &mut graphics_settings.anti_aliasing,
                                AntiAliasing::Fxaa,
                                "FXAA",
                            );
                            ui.add_enabled_ui(!graphics_settings.ssao, |ui| {
                                ui.selectable_value(
                                    &mut graphics_settings.anti_aliasing,
                                    AntiAliasing::Msaa2,
                                    "MSAA 2×",
                                );
                                ui.selectable_value(
                                    &mut graphics_settings.anti_aliasing,
                                    AntiAliasing::Msaa4,
                                    "MSAA 4×",
                                );
                                ui.selectable_value(
                                    &mut graphics_settings.anti_aliasing,
                                    AntiAliasing::Msaa8,
                                    "MSAA 8×",
                                );
                            });
                        });
                    ui.end_row();
                });

            if graphics_settings.ssao {
                ui.add_space(6.0);
                ui.label(
                    egui::RichText::new("SSAO uses FXAA because it is incompatible with MSAA.")
                        .size(11.0)
                        .color(egui::Color32::from_rgb(100, 100, 100)),
                );
            }

            ui.add_space(12.0);
            ui.label(
                egui::RichText::new(
                    "Higher shadow, AO, and MSAA settings substantially increase GPU cost.",
                )
                .size(11.0)
                .color(egui::Color32::from_rgb(100, 100, 100)),
            );
        });
    *graphics_settings != previous
}
