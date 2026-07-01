use bevy_egui::egui;

pub fn draw_settings_window(
    ctx: &egui::Context,
    open: &mut bool,
    graphics_settings: &mut crate::studio::ui::GraphicsSettings,
) {
    egui::Window::new("Settings")
        .open(open)
        .pivot(egui::Align2::CENTER_CENTER)
        .default_pos(ctx.content_rect().center())
        .default_size(egui::vec2(280.0, 180.0))
        .resizable(false)
        .collapsible(false)
        .show(ctx, |ui| {
            ui.label(egui::RichText::new("Graphics Settings").strong().size(14.0));
            ui.add_space(8.0);

            ui.checkbox(&mut graphics_settings.ssao, "Screen Space Ambient Occlusion (SSAO)");
            ui.checkbox(&mut graphics_settings.contact_shadows, "Contact Shadows");
            ui.checkbox(&mut graphics_settings.bloom, "Bloom");

            ui.add_space(8.0);
            ui.label(egui::RichText::new("Note: Disabling SSAO and Contact Shadows is highly recommended on Integrated Graphics to minimize GPU usage.")
                .size(11.0)
                .color(egui::Color32::from_rgb(100, 100, 100)));
        });
}