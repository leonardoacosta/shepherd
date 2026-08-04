use crossterm::event::{KeyCode, KeyEvent, MouseButton, MouseEvent, MouseEventKind};
use ratatui::layout::Rect;

use crate::{
    app::{
        state::{AppState, DisplayRowId, ExperimentSetting, SettingsSection, THEME_NAMES},
        App, Mode,
    },
    config::ToastDelivery,
};

#[derive(Debug, Clone, PartialEq, Eq)]
// The shared `Save` verb is semantic: these actions persist settings.
#[allow(clippy::enum_variant_names)]
pub(super) enum SettingsAction {
    SaveTheme(String),
    SaveSound(bool),
    SaveToastDelivery(ToastDelivery),
    SaveAgentBorderLabels(bool),
    SaveTopbarEnabled(bool),
    SaveDockEnabled(bool),
    SaveDockSide(crate::config::DockSide),
    SaveDockSize(u16),
    OpenDock {
        plugin_id: String,
        entrypoint: String,
    },
    SavePaneHistory(bool),
    SaveSwitchAsciiInputSourceInPrefix(bool),
    InstallRecommendedIntegrations,
}

/// Map an Experiments row index to the toggle action that flips it.
fn experiment_toggle_action(state: &AppState, idx: usize) -> Option<SettingsAction> {
    match ExperimentSetting::ALL.get(idx).copied()? {
        ExperimentSetting::PaneHistory => Some(SettingsAction::SavePaneHistory(
            !ExperimentSetting::PaneHistory.enabled(state),
        )),
        ExperimentSetting::SwitchAsciiInputSourceInPrefix => {
            Some(SettingsAction::SaveSwitchAsciiInputSourceInPrefix(
                !ExperimentSetting::SwitchAsciiInputSourceInPrefix.enabled(state),
            ))
        }
    }
}

impl App {
    pub(crate) fn handle_settings_key(&mut self, key: KeyEvent) {
        let previous_section = self.state.settings.section;
        if let Some(action) = update_settings_state(&mut self.state, key) {
            self.apply_settings_action(action);
        }
        self.after_settings_section_change(previous_section);
    }

    pub(super) fn apply_settings_action(&mut self, action: SettingsAction) {
        match action {
            SettingsAction::SaveTheme(name) => self.save_theme(&name),
            SettingsAction::SaveSound(enabled) => self.save_sound(enabled),
            SettingsAction::SaveToastDelivery(delivery) => self.save_toast_delivery(delivery),
            SettingsAction::SaveAgentBorderLabels(enabled) => {
                self.save_agent_border_labels(enabled)
            }
            SettingsAction::SaveTopbarEnabled(enabled) => self.save_topbar_enabled(enabled),
            SettingsAction::SaveDockEnabled(enabled) => self.save_dock_enabled(enabled),
            SettingsAction::SaveDockSide(side) => self.save_dock_side(side),
            SettingsAction::SaveDockSize(size) => self.save_dock_size(size),
            SettingsAction::OpenDock {
                plugin_id,
                entrypoint,
            } => self.open_dock_from_settings(plugin_id, entrypoint),
            SettingsAction::SavePaneHistory(enabled) => self.save_pane_history_persistence(enabled),
            SettingsAction::SaveSwitchAsciiInputSourceInPrefix(enabled) => {
                self.save_switch_ascii_input_source_in_prefix(enabled)
            }
            SettingsAction::InstallRecommendedIntegrations => {
                self.install_recommended_integrations()
            }
        }
    }

    pub(super) fn after_settings_section_change(&mut self, previous: SettingsSection) {
        if previous != SettingsSection::Integrations
            && self.state.settings.section == SettingsSection::Integrations
        {
            self.refresh_integration_recommendations();
        }
        if previous != SettingsSection::Display
            && self.state.settings.section == SettingsSection::Display
        {
            let preferred = self
                .state
                .display_rows()
                .get(self.state.settings.list.selected)
                .map(|row| row.id.clone());
            if let Err(err) = self.refresh_installed_plugins() {
                self.state.settings.display_message = Some(bounded_display_message(format!(
                    "failed to load plugin registry: {err}"
                )));
            }
            self.state.normalize_display_selection(preferred.as_ref());
        }
    }

    fn open_dock_from_settings(&mut self, plugin_id: String, entrypoint: String) {
        let response = self.handle_plugin_pane_open(
            "settings.dock.open".to_string(),
            crate::api::schema::PluginPaneOpenParams {
                plugin_id,
                entrypoint,
                placement: Some(crate::api::schema::PluginPanePlacement::Dock),
                width: None,
                height: None,
                workspace_id: None,
                target_pane_id: None,
                direction: None,
                cwd: None,
                focus: false,
                env: std::collections::HashMap::new(),
            },
        );
        let parsed = serde_json::from_str::<serde_json::Value>(&response).ok();
        if let Some(message) = parsed
            .as_ref()
            .and_then(|value| value.get("error"))
            .and_then(|error| error.get("message"))
            .and_then(serde_json::Value::as_str)
        {
            self.state.settings.display_message = Some(bounded_display_message(format!(
                "dock open failed: {message}"
            )));
            self.state.normalize_display_selection(None);
            return;
        }
        if parsed
            .as_ref()
            .and_then(|value| value.get("result"))
            .is_none()
        {
            self.state.settings.display_message =
                Some("dock open failed: invalid response".to_string());
            return;
        }

        self.state.settings.display_message = None;
        if let Some(ws_idx) = self.state.active {
            if let Some(pane_id) = self
                .state
                .dock_backing_tab_idx(ws_idx)
                .and_then(|tab_idx| self.state.workspaces[ws_idx].tabs.get(tab_idx))
                .map(|tab| tab.root_pane)
            {
                self.state.focus_dock_pane(ws_idx, pane_id);
            }
        }
        super::modal::leave_modal(&mut self.state);
    }
}

fn bounded_display_message(message: String) -> String {
    const MAX_CHARS: usize = 120;
    let mut chars = message.chars();
    let bounded = chars.by_ref().take(MAX_CHARS).collect::<String>();
    if chars.next().is_some() {
        format!("{bounded}…")
    } else {
        bounded
    }
}

fn display_action(state: &AppState, row: &DisplayRowId, size_delta: i8) -> Option<SettingsAction> {
    match row {
        DisplayRowId::AgentBorderLabels => Some(SettingsAction::SaveAgentBorderLabels(
            !state.agent_border_labels_enabled(),
        )),
        DisplayRowId::TopbarEnabled => {
            Some(SettingsAction::SaveTopbarEnabled(!state.topbar_enabled))
        }
        DisplayRowId::DockEnabled => Some(SettingsAction::SaveDockEnabled(!state.dock_enabled)),
        DisplayRowId::DockSide => Some(SettingsAction::SaveDockSide(match state.dock_side {
            crate::config::DockSide::Bottom => crate::config::DockSide::Right,
            crate::config::DockSide::Right => crate::config::DockSide::Bottom,
        })),
        DisplayRowId::DockSize => {
            let size = if size_delta < 0 {
                state
                    .dock_size
                    .saturating_sub(1)
                    .max(crate::config::MIN_DOCK_SIZE)
            } else {
                state.dock_size.saturating_add(1)
            };
            (size != state.dock_size).then_some(SettingsAction::SaveDockSize(size))
        }
        DisplayRowId::DockCandidate {
            plugin_id,
            entrypoint,
        } => Some(SettingsAction::OpenDock {
            plugin_id: plugin_id.clone(),
            entrypoint: entrypoint.clone(),
        }),
        DisplayRowId::TopbarRows
        | DisplayRowId::Status
        | DisplayRowId::DesktopOnly
        | DisplayRowId::Diagnostic => None,
    }
}

fn normalize_theme_name(name: &str) -> String {
    name.to_lowercase().replace([' ', '_'], "-")
}

fn current_theme_index(theme_name: &str) -> usize {
    let normalized = normalize_theme_name(theme_name);
    THEME_NAMES
        .iter()
        .position(|name| normalize_theme_name(name) == normalized)
        .unwrap_or(0)
}

fn toast_delivery_index(delivery: ToastDelivery) -> usize {
    match delivery {
        ToastDelivery::Off => 0,
        ToastDelivery::Shepherd => 1,
        ToastDelivery::Terminal => 2,
        ToastDelivery::System => 3,
    }
}

fn toast_delivery_for_index(idx: usize) -> ToastDelivery {
    match idx {
        0 => ToastDelivery::Off,
        1 => ToastDelivery::Shepherd,
        2 => ToastDelivery::Terminal,
        _ => ToastDelivery::System,
    }
}

fn preview_selected_theme(state: &mut AppState) {
    use crate::app::state::Palette;

    let name = THEME_NAMES[state.settings.list.selected];
    if let Some(mut palette) = Palette::from_name(name) {
        if let Some(custom) = &state.theme_runtime.custom {
            palette = palette.with_overrides(custom);
        }
        if let Some(accent) = &state.theme_runtime.legacy_accent {
            palette.accent = crate::config::parse_color(accent);
        }
        state.palette = palette;
        state.theme_name = name.to_string();
    }
}

fn cancel_settings(state: &mut AppState) {
    if let Some(palette) = state.settings.original_palette.take() {
        state.palette = palette;
    }
    if let Some(theme_name) = state.settings.original_theme.take() {
        state.theme_name = theme_name;
    }
    super::modal::leave_modal(state);
}

fn integrations_need_install(state: &AppState) -> bool {
    state
        .integration_recommendations
        .iter()
        .any(crate::integration::IntegrationRecommendation::needs_install)
}

fn apply_settings(state: &mut AppState) -> Option<SettingsAction> {
    match state.settings.section {
        SettingsSection::Theme => {
            let theme_name = state.theme_name.clone();
            state.settings.original_palette = None;
            state.settings.original_theme = None;
            super::modal::leave_modal(state);
            Some(SettingsAction::SaveTheme(theme_name))
        }
        SettingsSection::Integrations if integrations_need_install(state) => {
            Some(SettingsAction::InstallRecommendedIntegrations)
        }
        SettingsSection::Integrations => None,
        _ => {
            super::modal::leave_modal(state);
            None
        }
    }
}

pub(super) fn update_settings_state(state: &mut AppState, key: KeyEvent) -> Option<SettingsAction> {
    match state.settings.section {
        SettingsSection::Theme => match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                let previous = state.settings.list.selected;
                state.settings.list.move_prev();
                if state.settings.list.selected != previous {
                    preview_selected_theme(state);
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                let previous = state.settings.list.selected;
                state.settings.list.move_next(THEME_NAMES.len());
                if state.settings.list.selected != previous {
                    preview_selected_theme(state);
                }
            }
            KeyCode::Tab | KeyCode::Right | KeyCode::Char('l') => {
                state.settings.section = SettingsSection::Sound;
                state.settings.list.selected = usize::from(!state.sound_enabled());
            }
            KeyCode::BackTab | KeyCode::Left | KeyCode::Char('h') => {
                state.settings.section = SettingsSection::Experiments;
                state.settings.list.selected = 0;
            }
            _ => match super::modal::modal_action_from_key(&key, super::modal::SETTINGS_ACTIONS) {
                Some(super::modal::ModalAction::Apply) => return apply_settings(state),
                Some(super::modal::ModalAction::Close) => cancel_settings(state),
                _ => {}
            },
        },
        SettingsSection::Sound => match key.code {
            KeyCode::Up | KeyCode::Char('k') | KeyCode::Down | KeyCode::Char('j') => {
                state.settings.list.selected = 1 - state.settings.list.selected.min(1);
            }
            KeyCode::Enter | KeyCode::Char(' ') => {
                let enabled = state.settings.list.selected == 0;
                return Some(SettingsAction::SaveSound(enabled));
            }
            KeyCode::Tab | KeyCode::Right | KeyCode::Char('l') => {
                state.settings.section = SettingsSection::Toast;
                state.settings.list.selected = toast_delivery_index(state.toast_delivery());
            }
            KeyCode::BackTab | KeyCode::Left | KeyCode::Char('h') => {
                state.settings.section = SettingsSection::Theme;
                state.settings.list.selected = current_theme_index(&state.theme_name);
            }
            _ => {
                if let Some(super::modal::ModalAction::Close) =
                    super::modal::modal_action_from_key(&key, super::modal::SETTINGS_ACTIONS)
                {
                    cancel_settings(state);
                }
            }
        },
        SettingsSection::Toast => match key.code {
            KeyCode::Up | KeyCode::Char('k') => state.settings.list.move_prev(),
            KeyCode::Down | KeyCode::Char('j') => state.settings.list.move_next(4),
            KeyCode::Enter | KeyCode::Char(' ') => {
                let delivery = toast_delivery_for_index(state.settings.list.selected);
                return Some(SettingsAction::SaveToastDelivery(delivery));
            }
            KeyCode::BackTab | KeyCode::Left | KeyCode::Char('h') => {
                state.settings.section = SettingsSection::Sound;
                state.settings.list.selected = usize::from(!state.sound_enabled());
            }
            KeyCode::Tab | KeyCode::Right | KeyCode::Char('l') => {
                state.settings.section = SettingsSection::Display;
                state.normalize_display_selection(None);
            }
            _ => {
                if let Some(super::modal::ModalAction::Close) =
                    super::modal::modal_action_from_key(&key, super::modal::SETTINGS_ACTIONS)
                {
                    cancel_settings(state);
                }
            }
        },
        SettingsSection::Display => match key.code {
            KeyCode::Up | KeyCode::Char('k') => state.move_display_selection(-1),
            KeyCode::Down | KeyCode::Char('j') => state.move_display_selection(1),
            KeyCode::Enter | KeyCode::Char(' ') => {
                let row = state
                    .display_rows()
                    .get(state.settings.list.selected)
                    .map(|row| row.id.clone());
                return row.and_then(|row| display_action(state, &row, 1));
            }
            KeyCode::Char('-') => {
                let row = state
                    .display_rows()
                    .get(state.settings.list.selected)
                    .map(|row| row.id.clone());
                return row.and_then(|row| display_action(state, &row, -1));
            }
            KeyCode::Char('+') | KeyCode::Char('=') => {
                let row = state
                    .display_rows()
                    .get(state.settings.list.selected)
                    .map(|row| row.id.clone());
                return row.and_then(|row| display_action(state, &row, 1));
            }
            KeyCode::BackTab | KeyCode::Left | KeyCode::Char('h') => {
                state.settings.section = SettingsSection::Toast;
                state.settings.list.selected = toast_delivery_index(state.toast_delivery());
            }
            KeyCode::Tab | KeyCode::Right | KeyCode::Char('l') => {
                state.settings.section = SettingsSection::Integrations;
                state.settings.list.selected = 0;
            }
            _ => {
                if let Some(super::modal::ModalAction::Close) =
                    super::modal::modal_action_from_key(&key, super::modal::SETTINGS_ACTIONS)
                {
                    cancel_settings(state);
                }
            }
        },
        SettingsSection::Experiments => match key.code {
            KeyCode::Up | KeyCode::Char('k') => state.settings.list.move_prev(),
            KeyCode::Down | KeyCode::Char('j') => {
                state.settings.list.move_next(ExperimentSetting::ALL.len())
            }
            KeyCode::Enter | KeyCode::Char(' ') => {
                return experiment_toggle_action(state, state.settings.list.selected);
            }
            KeyCode::BackTab | KeyCode::Left | KeyCode::Char('h') => {
                state.settings.section = SettingsSection::Integrations;
                state.settings.list.selected = 0;
            }
            KeyCode::Tab | KeyCode::Right | KeyCode::Char('l') => {
                state.settings.section = SettingsSection::Theme;
                state.settings.list.selected = current_theme_index(&state.theme_name);
            }
            _ => {
                if let Some(super::modal::ModalAction::Close) =
                    super::modal::modal_action_from_key(&key, super::modal::SETTINGS_ACTIONS)
                {
                    cancel_settings(state);
                }
            }
        },
        SettingsSection::Integrations => match key.code {
            KeyCode::Enter | KeyCode::Char(' ') if integrations_need_install(state) => {
                return Some(SettingsAction::InstallRecommendedIntegrations);
            }
            KeyCode::BackTab | KeyCode::Left | KeyCode::Char('h') => {
                state.settings.section = SettingsSection::Display;
                state.normalize_display_selection(None);
            }
            KeyCode::Tab | KeyCode::Right | KeyCode::Char('l') => {
                state.settings.section = SettingsSection::Experiments;
                state.settings.list.selected = 0;
            }
            _ => match super::modal::modal_action_from_key(&key, super::modal::SETTINGS_ACTIONS) {
                Some(super::modal::ModalAction::Apply) => return apply_settings(state),
                Some(super::modal::ModalAction::Close) => cancel_settings(state),
                _ => {}
            },
        },
    }

    None
}

pub(crate) fn open_settings(state: &mut AppState) {
    open_settings_at(state, SettingsSection::Theme);
}

pub(crate) fn open_settings_at(state: &mut AppState, section: SettingsSection) {
    state.integration_install_messages.clear();
    state.settings.display_message = None;
    state.settings.original_palette = Some(state.palette.clone());
    state.settings.original_theme = Some(state.theme_name.clone());
    state.settings.section = section;
    state.settings.list.selected = match section {
        SettingsSection::Theme => current_theme_index(&state.theme_name),
        SettingsSection::Sound => usize::from(!state.sound_enabled()),
        SettingsSection::Toast => toast_delivery_index(state.toast_delivery()),
        SettingsSection::Display => 0,
        SettingsSection::Experiments => 0,
        SettingsSection::Integrations => 0,
    };
    if section == SettingsSection::Display {
        state.normalize_display_selection(None);
    }
    state.mode = Mode::Settings;
}

impl AppState {
    fn settings_popup_rect(&self) -> Rect {
        crate::ui::centered_popup_rect(
            self.screen_rect(),
            crate::ui::SETTINGS_POPUP_WIDTH,
            crate::ui::settings_popup_height(self),
        )
        .unwrap_or_default()
    }

    fn settings_inner_rect(&self) -> Rect {
        let popup = self.settings_popup_rect();
        Rect::new(
            popup.x + 1,
            popup.y + 1,
            popup.width.saturating_sub(2),
            popup.height.saturating_sub(2),
        )
    }

    fn settings_tab_at(&self, col: u16, row: u16) -> Option<SettingsSection> {
        let inner = self.settings_inner_rect();
        let tab_y = inner.y + 1;
        if row != tab_y {
            return None;
        }
        let mut x = inner.x;
        for section in SettingsSection::ALL {
            let badge_width = if self.settings_section_has_badge(*section) {
                2
            } else {
                0
            };
            let width = section.label().len() as u16 + 2 + badge_width;
            if col >= x && col < x + width {
                return Some(*section);
            }
            x += width + 1;
        }
        None
    }

    pub(crate) fn settings_content_rect(&self) -> Rect {
        let inner = self.settings_inner_rect();
        crate::ui::modal_stack_areas(inner, 3, 2, 0, 1).content
    }

    fn settings_list_index_at(&self, col: u16, row: u16) -> Option<usize> {
        let area = self.settings_content_rect();
        if row < area.y || row >= area.y + area.height || col < area.x || col >= area.x + area.width
        {
            return None;
        }

        match self.settings.section {
            SettingsSection::Theme => {
                let max_visible = area.height as usize;
                let scroll = if self.settings.list.selected >= max_visible {
                    self.settings.list.selected - max_visible + 1
                } else {
                    0
                };
                let idx = scroll + (row - area.y) as usize;
                (idx < THEME_NAMES.len()).then_some(idx)
            }
            SettingsSection::Sound => {
                let list_y = area.y + 3;
                if row >= list_y && row < list_y + 2 {
                    Some((row - list_y) as usize)
                } else {
                    None
                }
            }
            SettingsSection::Toast => {
                let list_y = area.y + 3;
                if row >= list_y && row < list_y + 8 {
                    Some(((row - list_y) / 2) as usize)
                } else {
                    None
                }
            }
            SettingsSection::Display => {
                let list_y = area.y + 3;
                let visible_rows = area.height.saturating_sub(3) as usize;
                let start = self.display_scroll_start(visible_rows);
                if row >= list_y && (row - list_y) < visible_rows as u16 {
                    let idx = start + (row - list_y) as usize;
                    (idx < self.display_rows().len()).then_some(idx)
                } else {
                    None
                }
            }
            SettingsSection::Experiments => {
                let list_y = area.y + 3;
                if row >= list_y && row < list_y + ExperimentSetting::ALL.len() as u16 {
                    Some((row - list_y) as usize)
                } else {
                    None
                }
            }
            SettingsSection::Integrations => None,
        }
    }

    pub(super) fn handle_settings_mouse(&mut self, mouse: MouseEvent) -> Option<SettingsAction> {
        match mouse.kind {
            MouseEventKind::Down(MouseButton::Left) => {
                if let Some(section) = self.settings_tab_at(mouse.column, mouse.row) {
                    self.settings.section = section;
                    self.settings.list.select(match section {
                        SettingsSection::Theme => current_theme_index(&self.theme_name),
                        SettingsSection::Sound => usize::from(!self.sound_enabled()),
                        SettingsSection::Toast => toast_delivery_index(self.toast_delivery()),
                        SettingsSection::Display => 0,
                        SettingsSection::Experiments => 0,
                        SettingsSection::Integrations => 0,
                    });
                    if section == SettingsSection::Display {
                        self.normalize_display_selection(None);
                    }
                    return None;
                }
                if let Some(idx) = self.settings_list_index_at(mouse.column, mouse.row) {
                    let display_row = (self.settings.section == SettingsSection::Display)
                        .then(|| self.display_rows().get(idx).cloned())
                        .flatten();
                    if display_row.as_ref().is_some_and(|row| !row.selectable) {
                        return None;
                    }
                    self.settings.list.select(idx);
                    return match self.settings.section {
                        SettingsSection::Theme => {
                            preview_selected_theme(self);
                            None
                        }
                        SettingsSection::Sound => {
                            let enabled = idx == 0;
                            Some(SettingsAction::SaveSound(enabled))
                        }
                        SettingsSection::Toast => {
                            let delivery = toast_delivery_for_index(idx);
                            Some(SettingsAction::SaveToastDelivery(delivery))
                        }
                        SettingsSection::Display => {
                            let row = display_row?;
                            let size_delta = if row.id == DisplayRowId::DockSize {
                                crate::ui::display_size_delta_at(
                                    self.settings_content_rect(),
                                    mouse.column,
                                    mouse.row,
                                    self.display_scroll_start(
                                        self.settings_content_rect().height.saturating_sub(3)
                                            as usize,
                                    ),
                                    idx,
                                    self.dock_size,
                                )?
                            } else {
                                1
                            };
                            display_action(self, &row.id, size_delta)
                        }
                        SettingsSection::Experiments => experiment_toggle_action(self, idx),
                        SettingsSection::Integrations => None,
                    };
                }

                let inner = self.settings_inner_rect();
                let show_primary = crate::ui::settings_show_primary_action(self);
                let (apply, close) =
                    crate::ui::settings_button_rects(inner, self.settings.section, show_primary);
                let mut buttons = vec![(close, super::modal::ModalAction::Close)];
                if let Some(apply) = apply {
                    buttons.insert(0, (apply, super::modal::ModalAction::Apply));
                }
                match super::modal::modal_action_from_buttons(mouse.column, mouse.row, &buttons) {
                    Some(super::modal::ModalAction::Apply) => apply_settings(self),
                    Some(super::modal::ModalAction::Close) => {
                        cancel_settings(self);
                        None
                    }
                    _ => {
                        cancel_settings(self);
                        None
                    }
                }
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseEventKind};

    use super::super::{app_for_mouse_test, mouse, state_with_workspaces};
    use super::*;

    fn installed_dock_plugin(
        plugin_id: &str,
        name: &str,
        pane_count: usize,
    ) -> crate::api::schema::InstalledPluginInfo {
        let panes = (0..pane_count)
            .map(|idx| {
                serde_json::json!({
                    "id": format!("dock-{idx:02}"),
                    "title": format!("Dock {idx:02}"),
                    "placement": "dock",
                    "command": ["test-command"]
                })
            })
            .collect::<Vec<_>>();
        serde_json::from_value(serde_json::json!({
            "plugin_id": plugin_id,
            "name": name,
            "version": "0.1.0",
            "manifest_path": "/tmp/shepherd-plugin.toml",
            "plugin_root": "/tmp",
            "enabled": true,
            "panes": panes
        }))
        .expect("test plugin")
    }

    #[test]
    fn settings_display_keyboard_and_mouse_share_dynamic_row_identity() {
        let mut app = app_for_mouse_test();
        app.state.workspaces = vec![crate::workspace::Workspace::test_new("display")];
        app.state.active = Some(0);
        app.state.dock_enabled = true;
        let plugin = installed_dock_plugin("example.display", "Display Plugin", 1);
        app.state
            .installed_plugins
            .insert(plugin.plugin_id.clone(), plugin);
        open_settings_at(&mut app.state, SettingsSection::Display);

        let rows = app.state.display_rows();
        let side_idx = rows
            .iter()
            .position(|row| row.id == DisplayRowId::DockSide)
            .expect("dock side row");
        app.state.settings.list.selected = side_idx;
        assert_eq!(
            update_settings_state(
                &mut app.state,
                KeyEvent::new(KeyCode::Enter, KeyModifiers::empty())
            ),
            Some(SettingsAction::SaveDockSide(crate::config::DockSide::Right))
        );

        app.state.settings.list.selected = side_idx;
        let area = app.state.settings_content_rect();
        let start = app
            .state
            .display_scroll_start(area.height.saturating_sub(3) as usize);
        let action = app.state.handle_settings_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            area.x + 5,
            area.y + 3 + (side_idx - start) as u16,
        ));
        assert_eq!(
            action,
            Some(SettingsAction::SaveDockSide(crate::config::DockSide::Right))
        );
        assert_eq!(
            app.state.display_rows()[app.state.settings.list.selected].id,
            DisplayRowId::DockSide
        );
    }

    #[test]
    fn settings_display_rows_scroll_clamp_and_explain_mobile_layout() {
        let mut state = state_with_workspaces(&["display"]);
        state.dock_enabled = true;
        state.view.layout = crate::app::state::ViewLayout::Mobile;
        let plugin = installed_dock_plugin("example.many", "Many Docks", 20);
        state
            .installed_plugins
            .insert(plugin.plugin_id.clone(), plugin);
        open_settings_at(&mut state, SettingsSection::Display);

        let preferred = DisplayRowId::DockCandidate {
            plugin_id: "example.many".to_string(),
            entrypoint: "dock-19".to_string(),
        };
        state.normalize_display_selection(Some(&preferred));
        assert_eq!(
            state.display_rows()[state.settings.list.selected].id,
            preferred
        );
        assert!(state.display_scroll_start(5) > 0);
        assert!(state
            .display_rows()
            .iter()
            .any(|row| row.id == DisplayRowId::DesktopOnly
                && row.label.contains("desktop layout only")));

        state.installed_plugins.clear();
        state.normalize_display_selection(Some(&preferred));
        assert!(state.display_rows()[state.settings.list.selected].selectable);
        assert_ne!(
            state.display_rows()[state.settings.list.selected].id,
            preferred
        );
    }

    #[test]
    fn settings_display_size_buttons_share_minimum_clamp() {
        let mut app = app_for_mouse_test();
        app.state.workspaces = vec![crate::workspace::Workspace::test_new("display")];
        app.state.active = Some(0);
        app.state.dock_size = crate::config::MIN_DOCK_SIZE;
        open_settings_at(&mut app.state, SettingsSection::Display);
        let size_idx = app
            .state
            .display_rows()
            .iter()
            .position(|row| row.id == DisplayRowId::DockSize)
            .expect("dock size row");
        app.state.settings.list.selected = size_idx;

        assert_eq!(
            update_settings_state(
                &mut app.state,
                KeyEvent::new(KeyCode::Char('-'), KeyModifiers::empty())
            ),
            None
        );
        assert_eq!(
            update_settings_state(
                &mut app.state,
                KeyEvent::new(KeyCode::Char('+'), KeyModifiers::empty())
            ),
            Some(SettingsAction::SaveDockSize(
                crate::config::MIN_DOCK_SIZE + 1
            ))
        );

        let area = app.state.settings_content_rect();
        let start = app
            .state
            .display_scroll_start(area.height.saturating_sub(3) as usize);
        let minus_x = area.x + 3 + "dock size: ".len() as u16;
        let action = app.state.handle_settings_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            minus_x,
            area.y + 3 + (size_idx - start) as u16,
        ));
        assert_eq!(action, None, "minimum dock size must not decrement");
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn dock_open_from_settings_preserves_main_tab_and_focuses_auxiliary_pane() {
        let mut app = app_for_mouse_test();
        app.state.workspaces = vec![crate::workspace::Workspace::test_new("display")];
        app.state.active = Some(0);
        app.state.selected = 0;
        app.state.ensure_test_terminals();
        app.state.dock_enabled = true;
        let mut plugin = installed_dock_plugin("example.display", "Display Plugin", 1);
        plugin.panes[0].command = vec!["sh".into(), "-c".into(), "sleep 60".into()];
        app.state
            .installed_plugins
            .insert(plugin.plugin_id.clone(), plugin);
        open_settings_at(&mut app.state, SettingsSection::Display);
        let main_tab = app.state.workspaces[0].active_tab;

        app.open_dock_from_settings("example.display".into(), "dock-00".into());

        assert_eq!(app.state.mode, Mode::Terminal);
        assert_eq!(app.state.workspaces[0].active_tab, main_tab);
        assert_eq!(app.state.visible_tab_indices(0), vec![main_tab]);
        let dock_tab = app.state.dock_backing_tab_idx(0).expect("dock backing tab");
        let dock_pane = app.state.workspaces[0].tabs[dock_tab].root_pane;
        assert!(app.state.dock_focus.as_ref().is_some_and(|focus| {
            focus.workspace_id == app.state.workspaces[0].id && focus.pane_id == dock_pane
        }));
        assert_eq!(app.terminal_runtimes.len(), 1);

        for (_, runtime) in app.terminal_runtimes.drain() {
            runtime.shutdown();
        }
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn dock_open_reconciles_stale_runtime_before_occupancy() {
        let mut app = app_for_mouse_test();
        let mut workspace = crate::workspace::Workspace::test_new("display-stale-dock");
        let stale_tab = workspace.test_add_tab(Some("stale-dock"));
        let stale_pane = workspace.tabs[stale_tab].root_pane;
        app.state.workspaces = vec![workspace];
        app.state.active = Some(0);
        app.state.selected = 0;
        app.state.ensure_test_terminals();
        app.state.dock_enabled = true;
        app.state
            .reserve_dock_pane(0, stale_pane)
            .expect("stale dock reservation");
        app.state.plugin_panes.insert(
            stale_pane,
            crate::app::state::PluginPaneRecord {
                plugin_id: "example.stale".into(),
                entrypoint: "old".into(),
            },
        );
        let mut plugin = installed_dock_plugin("example.display", "Display Plugin", 1);
        plugin.panes[0].command = vec!["sh".into(), "-c".into(), "sleep 60".into()];
        app.state
            .installed_plugins
            .insert(plugin.plugin_id.clone(), plugin);
        open_settings_at(&mut app.state, SettingsSection::Display);

        app.open_dock_from_settings("example.display".into(), "dock-00".into());

        assert_eq!(app.state.mode, Mode::Terminal);
        assert_eq!(app.state.workspaces[0].tabs.len(), 2);
        let live_tab = app.state.dock_backing_tab_idx(0).expect("replacement dock");
        let live_pane = app.state.workspaces[0].tabs[live_tab].root_pane;
        assert_ne!(live_pane, stale_pane);
        assert!(!app.state.plugin_panes.contains_key(&stale_pane));
        assert_eq!(app.terminal_runtimes.len(), 1);

        for (_, runtime) in app.terminal_runtimes.drain() {
            runtime.shutdown();
        }
    }

    #[cfg(unix)]
    #[test]
    fn dock_open_from_settings_failure_stays_visible_and_leaks_nothing() {
        let mut app = app_for_mouse_test();
        app.state.workspaces = vec![crate::workspace::Workspace::test_new("display")];
        app.state.active = Some(0);
        app.state.selected = 0;
        app.state.ensure_test_terminals();
        app.state.dock_enabled = true;
        let mut plugin = installed_dock_plugin("example.display", "Display Plugin", 1);
        plugin.enabled = false;
        app.state
            .installed_plugins
            .insert(plugin.plugin_id.clone(), plugin);
        open_settings_at(&mut app.state, SettingsSection::Display);

        app.open_dock_from_settings("example.display".into(), "dock-00".into());

        assert_eq!(app.state.mode, Mode::Settings);
        assert_eq!(app.state.workspaces[0].tabs.len(), 1);
        assert!(app.state.dock_panes.is_empty());
        assert_eq!(app.terminal_runtimes.len(), 0);
        assert!(app
            .state
            .settings
            .display_message
            .as_deref()
            .is_some_and(|message| message.contains("dock open failed")));
    }

    #[test]
    fn settings_cancel_restores_previewed_theme_from_other_sections() {
        let mut state = state_with_workspaces(&["test"]);
        let original_palette = state.palette.clone();
        let original_theme = state.theme_name.clone();

        open_settings(&mut state);
        update_settings_state(
            &mut state,
            KeyEvent::new(KeyCode::Down, KeyModifiers::empty()),
        );
        assert_ne!(state.theme_name, original_theme);

        update_settings_state(
            &mut state,
            KeyEvent::new(KeyCode::Tab, KeyModifiers::empty()),
        );
        assert_eq!(
            state.settings.section,
            crate::app::state::SettingsSection::Sound
        );

        update_settings_state(
            &mut state,
            KeyEvent::new(KeyCode::Esc, KeyModifiers::empty()),
        );

        assert_eq!(state.mode, Mode::Terminal);
        assert_eq!(state.theme_name, original_theme);
        assert_eq!(state.palette.accent, original_palette.accent);
        assert_eq!(state.palette.panel_bg, original_palette.panel_bg);
    }

    #[test]
    fn settings_sound_toggle_returns_save_action() {
        let mut state = state_with_workspaces(&["test"]);
        open_settings(&mut state);
        state.settings.section = crate::app::state::SettingsSection::Sound;
        state.settings.list.selected = 0;

        let action = update_settings_state(
            &mut state,
            KeyEvent::new(KeyCode::Enter, KeyModifiers::empty()),
        );

        assert_eq!(action, Some(SettingsAction::SaveSound(true)));
        assert!(!state.sound.enabled);
        assert_eq!(state.mode, Mode::Settings);
    }

    #[test]
    fn settings_experiments_toggles_pane_history() {
        let mut state = state_with_workspaces(&["test"]);
        state.pane_history_persistence = false;
        open_settings_at(&mut state, SettingsSection::Experiments);

        let action = update_settings_state(
            &mut state,
            KeyEvent::new(KeyCode::Enter, KeyModifiers::empty()),
        );

        assert_eq!(action, Some(SettingsAction::SavePaneHistory(true)));
        assert_eq!(state.mode, Mode::Settings);
    }

    #[test]
    fn settings_experiments_down_then_toggle_switches_ascii_input_source() {
        let mut state = state_with_workspaces(&["test"]);
        state.switch_ascii_input_source_in_prefix = false;
        open_settings_at(&mut state, SettingsSection::Experiments);

        update_settings_state(
            &mut state,
            KeyEvent::new(KeyCode::Down, KeyModifiers::empty()),
        );
        assert_eq!(state.settings.list.selected, 1);

        let action = update_settings_state(
            &mut state,
            KeyEvent::new(KeyCode::Enter, KeyModifiers::empty()),
        );

        assert_eq!(
            action,
            Some(SettingsAction::SaveSwitchAsciiInputSourceInPrefix(true))
        );
        assert_eq!(state.mode, Mode::Settings);
    }

    #[test]
    fn settings_tab_cycle_places_experiments_last() {
        let mut state = state_with_workspaces(&["test"]);
        open_settings_at(&mut state, SettingsSection::Display);

        update_settings_state(
            &mut state,
            KeyEvent::new(KeyCode::Tab, KeyModifiers::empty()),
        );
        assert_eq!(state.settings.section, SettingsSection::Integrations);

        update_settings_state(
            &mut state,
            KeyEvent::new(KeyCode::Tab, KeyModifiers::empty()),
        );
        assert_eq!(state.settings.section, SettingsSection::Experiments);

        update_settings_state(
            &mut state,
            KeyEvent::new(KeyCode::Tab, KeyModifiers::empty()),
        );
        assert_eq!(state.settings.section, SettingsSection::Theme);

        update_settings_state(
            &mut state,
            KeyEvent::new(KeyCode::BackTab, KeyModifiers::empty()),
        );
        assert_eq!(state.settings.section, SettingsSection::Experiments);

        update_settings_state(
            &mut state,
            KeyEvent::new(KeyCode::BackTab, KeyModifiers::empty()),
        );
        assert_eq!(state.settings.section, SettingsSection::Integrations);

        update_settings_state(
            &mut state,
            KeyEvent::new(KeyCode::BackTab, KeyModifiers::empty()),
        );
        assert_eq!(state.settings.section, SettingsSection::Display);
    }

    #[test]
    fn integrations_enter_does_nothing_when_nothing_needs_install() {
        let mut state = state_with_workspaces(&["test"]);
        open_settings_at(&mut state, SettingsSection::Integrations);

        let enter_action = update_settings_state(
            &mut state,
            KeyEvent::new(KeyCode::Enter, KeyModifiers::empty()),
        );
        assert_eq!(enter_action, None);

        let space_action = update_settings_state(
            &mut state,
            KeyEvent::new(KeyCode::Char(' '), KeyModifiers::empty()),
        );
        assert_eq!(space_action, None);
    }

    #[test]
    fn settings_hover_does_not_change_selection() {
        let mut app = app_for_mouse_test();
        open_settings(&mut app.state);
        app.state.settings.list.select(0);

        let area = app.state.settings_content_rect();
        app.handle_mouse(mouse(MouseEventKind::Moved, area.x + 2, area.y + 2));

        assert_eq!(app.state.settings.list.selected, 0);
    }

    #[test]
    fn settings_mouse_click_toggles_pane_history() {
        let mut app = app_for_mouse_test();
        app.state.pane_history_persistence = false;
        open_settings_at(&mut app.state, SettingsSection::Experiments);

        let area = app.state.settings_content_rect();
        let action = app.state.handle_settings_mouse(mouse(
            MouseEventKind::Down(crossterm::event::MouseButton::Left),
            area.x + 2,
            area.y + 3,
        ));

        assert_eq!(action, Some(SettingsAction::SavePaneHistory(true)));
        assert_eq!(app.state.settings.list.selected, 0);
    }

    #[test]
    fn settings_mouse_click_toggles_switch_ascii_input_source_row() {
        let mut app = app_for_mouse_test();
        app.state.switch_ascii_input_source_in_prefix = false;
        open_settings_at(&mut app.state, SettingsSection::Experiments);

        let area = app.state.settings_content_rect();
        let action = app.state.handle_settings_mouse(mouse(
            MouseEventKind::Down(crossterm::event::MouseButton::Left),
            area.x + 2,
            area.y + 4,
        ));

        assert_eq!(
            action,
            Some(SettingsAction::SaveSwitchAsciiInputSourceInPrefix(true))
        );
        assert_eq!(app.state.settings.list.selected, 1);
    }

    #[test]
    fn integration_update_badge_only_tracks_outdated_recommendations() {
        let mut state = state_with_workspaces(&["test"]);
        state.integration_recommendations = vec![integration_recommendation(
            crate::integration::IntegrationStatusKind::NotInstalled,
            true,
        )];
        assert!(!state.integration_updates_available());

        state.integration_recommendations = vec![integration_recommendation(
            crate::integration::IntegrationStatusKind::NotInstalled,
            false,
        )];
        assert!(!state.integration_updates_available());

        state.integration_recommendations = vec![integration_recommendation(
            crate::integration::IntegrationStatusKind::Current,
            true,
        )];
        assert!(!state.integration_updates_available());

        state.integration_recommendations = vec![integration_recommendation(
            crate::integration::IntegrationStatusKind::Outdated,
            true,
        )];
        assert!(state.integration_updates_available());
    }

    #[test]
    fn settings_tab_hit_area_includes_integration_update_badge() {
        let mut state = state_with_workspaces(&["test"]);
        state.integration_recommendations = vec![integration_recommendation(
            crate::integration::IntegrationStatusKind::Outdated,
            true,
        )];
        open_settings(&mut state);

        let inner = state.settings_inner_rect();
        let tab_y = inner.y + 1;
        let integrations_idx = SettingsSection::ALL
            .iter()
            .position(|section| *section == SettingsSection::Integrations)
            .expect("integrations section should be present");
        let integrations_x = inner.x
            + SettingsSection::ALL[..integrations_idx]
                .iter()
                .map(|section| {
                    let badge_width = if state.settings_section_has_badge(*section) {
                        2
                    } else {
                        0
                    };
                    section.label().len() as u16 + 3 + badge_width
                })
                .sum::<u16>();
        let dotted_width = SettingsSection::Integrations.label().len() as u16 + 4;

        assert_eq!(
            state.settings_tab_at(integrations_x + dotted_width - 1, tab_y),
            Some(SettingsSection::Integrations)
        );
    }

    fn integration_recommendation(
        state: crate::integration::IntegrationStatusKind,
        available: bool,
    ) -> crate::integration::IntegrationRecommendation {
        crate::integration::IntegrationRecommendation {
            target: crate::api::schema::IntegrationTarget::Claude,
            label: "claude",
            command: "claude",
            available,
            path: std::path::PathBuf::from("/tmp/shepherd-test-integration"),
            state,
        }
    }
}
