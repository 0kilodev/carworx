use eframe::egui::{self, Button, Color32, RichText, Rounding, Sense, Stroke, Ui, Vec2};

use super::theme::*;

/// Render a primary accent button (cyan bold)
pub fn primary_button(ui: &mut Ui, text: &str) -> egui::Response {
    let btn = Button::new(RichText::new(text).color(Color32::BLACK).strong())
        .fill(COLOR_PRIMARY_CYAN)
        .rounding(Rounding::same(6.0_f32))
        .min_size(Vec2::new(120.0, 34.0));
    ui.add(btn)
}

/// Render a secondary button (subtle dark surface with border)
pub fn secondary_button(ui: &mut Ui, text: &str) -> egui::Response {
    let btn = Button::new(RichText::new(text).color(COLOR_TEXT_PRIMARY))
        .fill(COLOR_SURFACE_ELEVATED)
        .stroke(Stroke::new(1.0_f32, COLOR_BORDER))
        .rounding(Rounding::same(6.0_f32))
        .min_size(Vec2::new(100.0, 34.0));
    ui.add(btn)
}

/// Render a danger button (red accent)
pub fn danger_button(ui: &mut Ui, text: &str) -> egui::Response {
    let btn = Button::new(RichText::new(text).color(Color32::WHITE).strong())
        .fill(COLOR_ACCENT_RED)
        .rounding(Rounding::same(6.0_f32))
        .min_size(Vec2::new(100.0, 34.0));
    ui.add(btn)
}

/// Render an interactive action card button for landing pages & dashboards
pub fn action_card_button(
    ui: &mut Ui,
    icon: &str,
    title: &str,
    description: &str,
    accent_color: Color32,
    width: f32,
) -> egui::Response {
    let desired_size = Vec2::new(width, 140.0);
    let (rect, response) = ui.allocate_exact_size(desired_size, Sense::click());

    if ui.is_rect_visible(rect) {
        let is_hovered = response.hovered();
        let bg_fill = if is_hovered {
            COLOR_SURFACE_ELEVATED
        } else {
            COLOR_SURFACE
        };
        let border_stroke = if is_hovered {
            Stroke::new(2.0_f32, accent_color)
        } else {
            Stroke::new(1.0_f32, COLOR_BORDER)
        };

        // Draw card background
        ui.painter().rect_filled(rect, Rounding::same(10.0_f32), bg_fill);
        ui.painter().rect_stroke(rect, Rounding::same(10.0_f32), border_stroke);

        // Top accent line if hovered
        if is_hovered {
            let top_line = egui::Rect::from_min_size(rect.min, Vec2::new(rect.width(), 3.0));
            ui.painter().rect_filled(top_line, Rounding::same(2.0_f32), accent_color);
        }

        // Child UI for text content
        let mut child_ui = ui.new_child(egui::UiBuilder::default().max_rect(rect.shrink(16.0)));

        child_ui.horizontal(|ui| {
            // Icon
            ui.label(RichText::new(icon).size(32.0));
            ui.add_space(8.0);
            ui.vertical(|ui| {
                // Title
                ui.label(
                    RichText::new(title)
                        .color(if is_hovered { accent_color } else { COLOR_TEXT_PRIMARY })
                        .size(18.0)
                        .strong(),
                );
                ui.add_space(4.0);
                // Description
                ui.label(
                    RichText::new(description)
                        .color(COLOR_TEXT_SECONDARY)
                        .size(13.0),
                );
            });
        });
    }

    response
}
