use eframe::egui::{Color32, Frame, Margin, Rounding, Stroke, Ui};

use super::theme::*;

/// Render contents inside a framed card with rounded corners and border
pub fn card_frame<R>(ui: &mut Ui, add_contents: impl FnOnce(&mut Ui) -> R) -> R {
    Frame::none()
        .fill(COLOR_SURFACE)
        .stroke(Stroke::new(1.0_f32, COLOR_BORDER))
        .rounding(Rounding::same(8.0_f32))
        .inner_margin(Margin::same(16.0_f32))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            add_contents(ui)
        })
        .inner
}

/// Render an elevated card with optional colored border accent
pub fn elevated_card_frame<R>(
    ui: &mut Ui,
    border_color: Color32,
    add_contents: impl FnOnce(&mut Ui) -> R,
) -> R {
    Frame::none()
        .fill(COLOR_SURFACE_ELEVATED)
        .stroke(Stroke::new(1.0_f32, border_color))
        .rounding(Rounding::same(8.0_f32))
        .inner_margin(Margin::same(16.0_f32))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            add_contents(ui)
        })
        .inner
}
