use super::types::*;
use imgui::Condition;

const MAX_ROWS: usize = 500;

pub(super) struct TranslationTool {
    pub filter: String,
    pub only_missing: bool,
}

impl TranslationTool {
    pub(super) fn new() -> TranslationTool {
        TranslationTool {
            filter: String::new(),
            only_missing: false,
        }
    }
}

pub(super) fn window(ui: &imgui::Ui, open: &mut bool, tool: &mut TranslationTool) {
    if !*open {
        return;
    }
    ui.window("Translations")
        .opened(open)
        .size([760.0, 520.0], Condition::FirstUseEver)
        .build(|| {
            let lang = ::i18n::language();
            let shown = if lang.is_empty() {
                "en".to_string()
            } else {
                lang
            };
            ui.text(format!("Language: {shown}"));
            for code in ::i18n::languages() {
                ui.same_line();
                if ui.small_button(code) {
                    ::i18n::set_language(code);
                }
            }
            ui.input_text("Filter##tr", &mut tool.filter).build();
            ui.checkbox("Only missing##tr", &mut tool.only_missing);
            let needle = tool.filter.trim().to_lowercase();
            let keys = ::i18n::keys();
            ui.text(format!("{} keys", keys.len()));
            ui.separator();
            ui.child_window("##tr_keys").build(|| {
                ui.columns(3, "##tr_cols", true);
                for h in ["Key", shown.as_str(), "en"] {
                    ui.text(h);
                    ui.next_column();
                }
                ui.separator();
                let mut rows = 0;
                for key in &keys {
                    if !needle.is_empty() && !key.to_lowercase().contains(&needle) {
                        continue;
                    }
                    let current = ::i18n::lookup(&shown, key);
                    if tool.only_missing && (shown == "en" || current.is_some()) {
                        continue;
                    }
                    if rows >= MAX_ROWS {
                        break;
                    }
                    rows += 1;
                    ui.text(key.as_str());
                    ui.next_column();
                    match current {
                        Some(t) => ui.text(t),
                        None => ui.text_colored([1.0, 0.4, 0.3, 1.0], "missing"),
                    }
                    ui.next_column();
                    ui.text(::i18n::lookup("en", key).unwrap_or_else(|| key.clone()));
                    ui.next_column();
                }
                ui.columns(1, "##tr_colsend", false);
                if rows >= MAX_ROWS {
                    ui.text_disabled(format!("first {MAX_ROWS} rows, narrow the filter"));
                }
            });
        });
}