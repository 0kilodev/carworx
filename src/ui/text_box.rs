use eframe::egui::{self, Margin, RichText, TextEdit, Ui, Vec2};

use super::theme::*;

/// Render a styled text edit box with hint
pub fn text_box(ui: &mut Ui, value: &mut String, hint: &str) -> egui::Response {
    let edit = TextEdit::singleline(value)
        .hint_text(RichText::new(hint).color(COLOR_TEXT_MUTED))
        .text_color(COLOR_TEXT_PRIMARY)
        .margin(Margin::symmetric(10.0_f32, 8.0_f32))
        .min_size(Vec2::new(120.0, 32.0));
    ui.add(edit)
}

/// Render a labeled text input with clear typography
pub fn labeled_text_box(ui: &mut Ui, label: &str, value: &mut String, hint: &str) -> egui::Response {
    ui.vertical(|ui| {
        ui.label(RichText::new(label).color(COLOR_TEXT_SECONDARY).size(13.0).strong());
        ui.add_space(2.0);
        let resp = text_box(ui, value, hint);
        ui.add_space(4.0);
        resp
    })
    .inner
}

/// Render a search box with search icon indicator
pub fn search_box(ui: &mut Ui, value: &mut String, hint: &str) -> egui::Response {
    ui.horizontal(|ui| {
        ui.label(RichText::new("🔍").size(14.0));
        let edit = TextEdit::singleline(value)
            .hint_text(RichText::new(hint).color(COLOR_TEXT_MUTED))
            .margin(Margin::symmetric(8.0_f32, 6.0_f32))
            .min_size(Vec2::new(160.0, 30.0));
        ui.add(edit)
    })
    .inner
}
