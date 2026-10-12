use eframe::egui::{self, Color32, Margin, Rounding, Stroke, Vec2, Visuals};

pub const COLOR_BG_DARK: Color32 = Color32::from_rgb(18, 21, 26);
pub const COLOR_SURFACE: Color32 = Color32::from_rgb(26, 31, 38);
pub const COLOR_SURFACE_ELEVATED: Color32 = Color32::from_rgb(34, 41, 51);
pub const COLOR_BORDER: Color32 = Color32::from_rgb(46, 56, 70);
pub const COLOR_PRIMARY_CYAN: Color32 = Color32::from_rgb(0, 210, 255);
pub const COLOR_PRIMARY_CYAN_HOVER: Color32 = Color32::from_rgb(51, 219, 255);
pub const COLOR_ACCENT_GREEN: Color32 = Color32::from_rgb(16, 185, 129);
pub const COLOR_ACCENT_AMBER: Color32 = Color32::from_rgb(245, 158, 11);
pub const COLOR_ACCENT_RED: Color32 = Color32::from_rgb(239, 68, 68);
pub const COLOR_TEXT_PRIMARY: Color32 = Color32::from_rgb(243, 244, 246);
pub const COLOR_TEXT_SECONDARY: Color32 = Color32::from_rgb(156, 163, 175);
pub const COLOR_TEXT_MUTED: Color32 = Color32::from_rgb(107, 114, 128);

/// Applies a sleek automotive dark theme to egui
pub fn apply_automotive_theme(ctx: &egui::Context) {
    let mut visuals = Visuals::dark();

    visuals.panel_fill = COLOR_BG_DARK;
    visuals.window_fill = COLOR_SURFACE;
    visuals.extreme_bg_color = Color32::from_rgb(13, 15, 19);

    // Default widgets (inactive)
    visuals.widgets.inactive.bg_fill = COLOR_SURFACE;
    visuals.widgets.inactive.bg_stroke = Stroke::new(1.0_f32, COLOR_BORDER);
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0_f32, COLOR_TEXT_PRIMARY);
    visuals.widgets.inactive.rounding = Rounding::same(6.0_f32);

    // Hovered widgets
    visuals.widgets.hovered.bg_fill = COLOR_SURFACE_ELEVATED;
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.5_f32, COLOR_PRIMARY_CYAN);
    visuals.widgets.hovered.fg_stroke = Stroke::new(1.5_f32, COLOR_TEXT_PRIMARY);
    visuals.widgets.hovered.rounding = Rounding::same(6.0_f32);

    // Active / Clicked widgets
    visuals.widgets.active.bg_fill = COLOR_SURFACE_ELEVATED;
    visuals.widgets.active.bg_stroke = Stroke::new(2.0_f32, COLOR_PRIMARY_CYAN_HOVER);
    visuals.widgets.active.fg_stroke = Stroke::new(2.0_f32, COLOR_PRIMARY_CYAN);
    visuals.widgets.active.rounding = Rounding::same(6.0_f32);

    // Open menus / dropdowns
    visuals.widgets.open.bg_fill = COLOR_SURFACE_ELEVATED;
    visuals.widgets.open.bg_stroke = Stroke::new(1.5_f32, COLOR_PRIMARY_CYAN);
    visuals.widgets.open.rounding = Rounding::same(6.0_f32);

    ctx.set_visuals(visuals);

    let mut style = (*ctx.style()).clone();
    style.spacing.item_spacing = Vec2::new(10.0, 10.0);
    style.spacing.window_margin = Margin::same(16.0_f32);
    style.spacing.button_padding = Vec2::new(12.0, 8.0);
    ctx.set_style(style);
}
