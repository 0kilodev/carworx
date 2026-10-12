use eframe::egui::{ComboBox, RichText, Ui};

use super::theme::*;

/// Render a styled dropdown / combo-box selector
/// Returns `true` if the selected item changed.
pub fn dropdown_select<T: AsRef<str>>(
    ui: &mut Ui,
    id_salt: impl std::hash::Hash,
    label: &str,
    selected_index: &mut Option<usize>,
    options: &[T],
    placeholder: &str,
    width: f32,
) -> bool {
    let mut changed = false;

    ui.vertical(|ui| {
        if !label.is_empty() {
            ui.label(RichText::new(label).color(COLOR_TEXT_SECONDARY).size(13.0).strong());
            ui.add_space(2.0);
        }

        let current_text = match *selected_index {
            Some(idx) if idx < options.len() => options[idx].as_ref().to_string(),
            _ => placeholder.to_string(),
        };

        let is_selected = selected_index.is_some();
        let display_text = if is_selected {
            RichText::new(current_text).color(COLOR_TEXT_PRIMARY)
        } else {
            RichText::new(current_text).color(COLOR_TEXT_MUTED)
        };

        ComboBox::from_id_salt(id_salt)
            .width(width)
            .selected_text(display_text)
            .show_ui(ui, |ui| {
                for (idx, opt) in options.iter().enumerate() {
                    let opt_str = opt.as_ref();
                    let is_item_active = *selected_index == Some(idx);

                    let item_label = if is_item_active {
                        RichText::new(format!("✓ {}", opt_str)).color(COLOR_PRIMARY_CYAN).strong()
                    } else {
                        RichText::new(opt_str).color(COLOR_TEXT_PRIMARY)
                    };

                    if ui.selectable_label(is_item_active, item_label).clicked() {
                        if *selected_index != Some(idx) {
                            *selected_index = Some(idx);
                            changed = true;
                        }
                    }
                }
            });

        ui.add_space(4.0);
    });

    changed
}
