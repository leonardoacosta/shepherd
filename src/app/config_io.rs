use super::App;

impl App {
    pub(super) fn update_config_file<F>(&mut self, error_context: &str, update: F) -> bool
    where
        F: FnOnce(&str) -> String,
    {
        #[cfg(test)]
        if std::env::var_os(crate::config::CONFIG_PATH_ENV_VAR).is_none() {
            return false;
        }

        let path = crate::config::config_path();
        if let Some(parent) = path.parent() {
            if let Err(err) = std::fs::create_dir_all(parent) {
                crate::logging::config_write_failed(&path, error_context, &err.to_string());
                self.state.config_diagnostic =
                    Some(format!("failed to save {error_context}: {err}"));
                self.config_diagnostic_deadline =
                    Some(std::time::Instant::now() + std::time::Duration::from_secs(5));
                return false;
            }
        }

        let content = std::fs::read_to_string(&path).unwrap_or_default();
        let new_content = update(&content);
        if let Err(err) = crate::persist::atomic_write_config(&path, new_content.as_bytes()) {
            crate::logging::config_write_failed(&path, error_context, &err.to_string());
            self.state.config_diagnostic = Some(format!("failed to save {error_context}: {err}"));
            self.config_diagnostic_deadline =
                Some(std::time::Instant::now() + std::time::Duration::from_secs(5));
            return false;
        }

        true
    }

    pub(super) fn mark_onboarding_complete(&mut self) {
        self.update_config_file("onboarding setting", |content| {
            crate::config::upsert_top_level_bool(content, "onboarding", false)
        });
    }

    pub(super) fn save_theme(&mut self, name: &str) {
        if self.update_config_file("theme", |content| {
            let content = crate::config::upsert_section_value(
                content,
                "theme",
                "name",
                &format!("\"{name}\""),
            );
            crate::config::upsert_section_bool(&content, "theme", "auto_switch", false)
        }) {
            self.apply_config_from_disk(false);
        }
    }

    pub(super) fn save_sound(&mut self, enabled: bool) {
        if self.update_config_file("sound setting", |content| {
            crate::config::upsert_section_bool(content, "ui.sound", "enabled", enabled)
        }) {
            self.apply_config_from_disk(false);
        }
    }

    pub(super) fn save_toast_delivery(&mut self, delivery: crate::config::ToastDelivery) {
        let value = match delivery {
            crate::config::ToastDelivery::Off => "\"off\"",
            crate::config::ToastDelivery::Shepherd => "\"shepherd\"",
            crate::config::ToastDelivery::Terminal => "\"terminal\"",
            crate::config::ToastDelivery::System => "\"system\"",
        };
        if self.update_config_file("toast setting", |content| {
            let content =
                crate::config::upsert_section_value(content, "ui.toast", "delivery", value);
            crate::config::remove_section_key(&content, "ui.toast", "enabled")
        }) {
            self.apply_config_from_disk(false);
        }
    }

    pub(super) fn save_agent_border_labels(&mut self, enabled: bool) {
        if self.update_config_file("agent border labels", |content| {
            crate::config::upsert_section_bool(
                content,
                "ui",
                "show_agent_labels_on_pane_borders",
                enabled,
            )
        }) {
            self.apply_config_from_disk(false);
        }
    }

    pub(super) fn save_topbar_enabled(&mut self, enabled: bool) {
        if self.update_config_file("topbar setting", |content| {
            crate::config::upsert_section_bool(content, "ui.topbar", "enabled", enabled)
        }) {
            self.apply_config_from_disk(false);
        }
    }

    pub(super) fn save_dock_enabled(&mut self, enabled: bool) {
        if self.update_config_file("dock setting", |content| {
            crate::config::upsert_section_bool(content, "ui.dock", "enabled", enabled)
        }) {
            self.apply_config_from_disk(false);
            if !enabled {
                self.state.dock_focus = None;
            }
        }
    }

    pub(super) fn save_dock_side(&mut self, side: crate::config::DockSide) {
        let value = match side {
            crate::config::DockSide::Bottom => "\"bottom\"",
            crate::config::DockSide::Right => "\"right\"",
        };
        if self.update_config_file("dock side", |content| {
            crate::config::upsert_section_value(content, "ui.dock", "side", value)
        }) {
            self.apply_config_from_disk(false);
        }
    }

    pub(super) fn save_dock_size(&mut self, size: u16) {
        let size = size.max(crate::config::MIN_DOCK_SIZE);
        if self.update_config_file("dock size", |content| {
            crate::config::upsert_section_value(content, "ui.dock", "size", &size.to_string())
        }) {
            self.apply_config_from_disk(false);
        }
    }

    pub(super) fn save_pane_history_persistence(&mut self, enabled: bool) {
        if self.update_config_file("pane screen history", |content| {
            crate::config::upsert_section_bool(content, "experimental", "pane_history", enabled)
        }) {
            self.apply_config_from_disk(false);
        }
    }

    pub(super) fn save_switch_ascii_input_source_in_prefix(&mut self, enabled: bool) {
        if self.update_config_file("prefix ascii input source", |content| {
            crate::config::upsert_section_bool(
                content,
                "experimental",
                "switch_ascii_input_source_in_prefix",
                enabled,
            )
        }) {
            self.apply_config_from_disk(false);
        }
    }

    pub(super) fn save_agent_panel_sort(&mut self, sort: crate::app::state::AgentPanelSort) {
        let value = match sort {
            crate::app::state::AgentPanelSort::Spaces => {
                crate::config::AgentPanelSortConfig::Spaces.as_str()
            }
            crate::app::state::AgentPanelSort::Priority => {
                crate::config::AgentPanelSortConfig::Priority.as_str()
            }
        };
        if self.update_config_file("agent panel sort", |content| {
            crate::config::upsert_section_value(
                content,
                "ui",
                "agent_panel_sort",
                &format!("\"{value}\""),
            )
        }) {
            self.apply_config_from_disk(false);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_save_is_atomic_comment_preserving_and_live() {
        let _guard = crate::config::test_config_env_lock()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let path = std::env::temp_dir().join(format!(
            "shepherd-settings-save-{}-{}.toml",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        std::fs::write(
            &path,
            concat!(
                "# user preface\n",
                "[ui]\n",
                "mouse_capture = false\n",
                "show_agent_labels_on_pane_borders = false # keep label note\n",
                "\n",
                "[ui.topbar] # keep topbar note\n",
                "enabled = false\n",
                "\n",
                "[ui.dock] # keep dock note\n",
                "enabled = false\n",
                "side = \"bottom\"\n",
                "size = 10\n",
            ),
        )
        .unwrap();
        let previous = std::env::var_os(crate::config::CONFIG_PATH_ENV_VAR);
        std::env::set_var(crate::config::CONFIG_PATH_ENV_VAR, &path);

        let (_api_tx, api_rx) = tokio::sync::mpsc::unbounded_channel();
        let mut app = App::new(
            &crate::config::Config::default(),
            true,
            None,
            api_rx,
            crate::api::EventHub::default(),
        );
        app.save_agent_border_labels(true);
        app.save_topbar_enabled(true);
        app.save_dock_enabled(true);
        app.save_dock_side(crate::config::DockSide::Right);
        app.save_dock_size(7);

        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("# user preface"));
        assert!(content.contains("mouse_capture = false"));
        assert!(content.contains("show_agent_labels_on_pane_borders = true # keep label note"));
        assert!(content.contains("[ui.topbar] # keep topbar note\nenabled = true"));
        assert!(content.contains("[ui.dock] # keep dock note\nenabled = true"));
        assert!(content.contains("side = \"right\""));
        assert!(content.contains("size = 7"));
        assert!(app.state.show_agent_labels_on_pane_borders);
        assert!(app.state.topbar_enabled);
        assert!(app.state.dock_enabled);
        assert_eq!(app.state.dock_side, crate::config::DockSide::Right);
        assert_eq!(app.state.dock_size, 7);

        match previous {
            Some(value) => std::env::set_var(crate::config::CONFIG_PATH_ENV_VAR, value),
            None => std::env::remove_var(crate::config::CONFIG_PATH_ENV_VAR),
        }
        let _ = std::fs::remove_file(path);
    }
}
