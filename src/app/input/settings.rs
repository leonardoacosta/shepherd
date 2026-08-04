use crossterm::event::{KeyCode, KeyEvent, MouseButton, MouseEvent, MouseEventKind};
use ratatui::layout::Rect;

use crate::{
    api::schema::IntegrationTarget,
    app::{
        state::{
            AgentPanelSort, AppState, BehaviorRowId, DisplayRowId, ExperimentSetting,
            SettingsSection, THEME_NAMES,
        },
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
    SavePaneBorders(bool),
    SavePaneGaps(bool),
    SaveHideTabBarWhenSingleTab(bool),
    SaveAgentPanelSort(AgentPanelSort),
    SaveTopbarEnabled(bool),
    SaveDockEnabled(bool),
    SaveDockSide(crate::config::DockSide),
    SaveDockSize(u16),
    OpenDock {
        plugin_id: String,
        entrypoint: String,
    },
    SaveConfirmClose(bool),
    SavePromptNewTabName(bool),
    SavePromptNewWorkspaceName(bool),
    SaveCopyOnSelect(bool),
    SaveMouseScrollLines(usize),
    SavePaneHistory(bool),
    SaveSwitchAsciiInputSourceInPrefix(bool),
    InstallRecommendedIntegrations,
    InstallIntegrationTarget(IntegrationTarget),
    UpdateIntegrationTarget(IntegrationTarget),
    UninstallIntegrationTarget(IntegrationTarget),
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
            SettingsAction::SavePaneBorders(enabled) => self.save_pane_borders(enabled),
            SettingsAction::SavePaneGaps(enabled) => self.save_pane_gaps(enabled),
            SettingsAction::SaveHideTabBarWhenSingleTab(enabled) => {
                self.save_hide_tab_bar_when_single_tab(enabled)
            }
            SettingsAction::SaveAgentPanelSort(sort) => self.save_agent_panel_sort(sort),
            SettingsAction::SaveTopbarEnabled(enabled) => self.save_topbar_enabled(enabled),
            SettingsAction::SaveDockEnabled(enabled) => self.save_dock_enabled(enabled),
            SettingsAction::SaveDockSide(side) => self.save_dock_side(side),
            SettingsAction::SaveDockSize(size) => self.save_dock_size(size),
            SettingsAction::OpenDock {
                plugin_id,
                entrypoint,
            } => self.open_dock_from_settings(plugin_id, entrypoint),
            SettingsAction::SaveConfirmClose(enabled) => self.save_confirm_close(enabled),
            SettingsAction::SavePromptNewTabName(enabled) => self.save_prompt_new_tab_name(enabled),
            SettingsAction::SavePromptNewWorkspaceName(enabled) => {
                self.save_prompt_new_workspace_name(enabled)
            }
            SettingsAction::SaveCopyOnSelect(enabled) => self.save_copy_on_select(enabled),
            SettingsAction::SaveMouseScrollLines(lines) => self.save_mouse_scroll_lines(lines),
            SettingsAction::SavePaneHistory(enabled) => self.save_pane_history_persistence(enabled),
            SettingsAction::SaveSwitchAsciiInputSourceInPrefix(enabled) => {
                self.save_switch_ascii_input_source_in_prefix(enabled)
            }
            SettingsAction::InstallRecommendedIntegrations => {
                self.install_recommended_integrations()
            }
            SettingsAction::InstallIntegrationTarget(target) => {
                self.install_integration_target(target)
            }
            SettingsAction::UpdateIntegrationTarget(target) => {
                self.update_integration_target(target)
            }
            SettingsAction::UninstallIntegrationTarget(target) => {
                self.uninstall_integration_target(target)
            }
        }
    }

    pub(super) fn after_settings_section_change(&mut self, previous: SettingsSection) {
        if previous == SettingsSection::Integrations
            && self.state.settings.section != SettingsSection::Integrations
        {
            // Leaving Integrations closes any open sub-dialog rather than
            // leaving it stranded for the next visit.
            let manager = &mut self.state.settings.integration_manager;
            manager.action_menu_target = None;
            manager.action_menu_selected = 0;
            manager.pending_uninstall = None;
            manager.history_detail_open = false;
            manager.history_scroll = 0;
        }
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
        if previous != SettingsSection::Behavior
            && self.state.settings.section == SettingsSection::Behavior
        {
            self.state.normalize_behavior_selection(None);
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
        DisplayRowId::PaneBorders => Some(SettingsAction::SavePaneBorders(!state.pane_borders)),
        DisplayRowId::PaneGaps => Some(SettingsAction::SavePaneGaps(!state.pane_gaps)),
        DisplayRowId::HideSingleTabBar => Some(SettingsAction::SaveHideTabBarWhenSingleTab(
            !state.hide_tab_bar_when_single_tab,
        )),
        DisplayRowId::AgentSort => Some(SettingsAction::SaveAgentPanelSort(
            match state.agent_panel_sort {
                AgentPanelSort::Spaces => AgentPanelSort::Priority,
                AgentPanelSort::Priority => AgentPanelSort::Spaces,
            },
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

/// Lower bound for the mouse scroll speed stepper; mirrors `DockSize`'s
/// min-only clamp (see `display_action`'s `DockSize` arm).
const MIN_MOUSE_SCROLL_LINES: usize = crate::app::state::MIN_MOUSE_SCROLL_LINES;

fn behavior_action(state: &AppState, row: BehaviorRowId, size_delta: i8) -> Option<SettingsAction> {
    match row {
        BehaviorRowId::ConfirmClose => Some(SettingsAction::SaveConfirmClose(!state.confirm_close)),
        BehaviorRowId::PromptNewTabName => Some(SettingsAction::SavePromptNewTabName(
            !state.prompt_new_tab_name,
        )),
        BehaviorRowId::PromptNewWorkspaceName => Some(SettingsAction::SavePromptNewWorkspaceName(
            !state.prompt_new_workspace_name,
        )),
        BehaviorRowId::CopyOnSelect => {
            Some(SettingsAction::SaveCopyOnSelect(!state.copy_on_select))
        }
        BehaviorRowId::ScrollLines => {
            let current = state.mouse_scroll_lines;
            let next = if size_delta < 0 {
                current.saturating_sub(1).max(MIN_MOUSE_SCROLL_LINES)
            } else {
                current.saturating_add(1)
            };
            (next != current).then_some(SettingsAction::SaveMouseScrollLines(next))
        }
    }
}

// ---------------------------------------------------------------------------
// Integrations: per-target action menu, uninstall confirmation, and
// scrollable operation history. Opening/selecting/canceling are pure
// `AppState` mutations; only the final confirmed mutation (install/update/
// uninstall) becomes a `SettingsAction` so `App` can run real I/O.
// ---------------------------------------------------------------------------

fn integration_actions_for(
    state: &AppState,
    target: IntegrationTarget,
) -> Vec<crate::integration::IntegrationAction> {
    state
        .integration_recommendations
        .iter()
        .find(|item| item.target == target)
        .map(crate::integration::IntegrationRecommendation::available_actions)
        .unwrap_or_default()
}

/// Open the action-choice menu for the currently selected target row. A
/// no-op while another operation is running or the target offers no action
/// (unsupported/not found — its row already explains why).
fn open_integration_action_menu(state: &mut AppState) {
    if state.settings.integration_manager.operation_in_flight {
        return;
    }
    let Some(recommendation) = state
        .integration_recommendations
        .get(state.settings.list.selected)
    else {
        return;
    };
    if recommendation.available_actions().is_empty() {
        return;
    }
    state.settings.integration_manager.action_menu_target = Some(recommendation.target);
    state.settings.integration_manager.action_menu_selected = 0;
}

fn cancel_integration_action_menu(state: &mut AppState) {
    state.settings.integration_manager.action_menu_target = None;
    state.settings.integration_manager.action_menu_selected = 0;
}

/// Confirm the highlighted action-menu choice. Install/update become an
/// immediate `SettingsAction`; uninstall instead opens the explicit
/// confirmation step rather than mutating right away.
fn confirm_integration_action_menu(state: &mut AppState) -> Option<SettingsAction> {
    let target = state.settings.integration_manager.action_menu_target?;
    let actions = integration_actions_for(state, target);
    let chosen = actions
        .get(state.settings.integration_manager.action_menu_selected)
        .copied();
    cancel_integration_action_menu(state);
    match chosen? {
        crate::integration::IntegrationAction::Install => {
            Some(SettingsAction::InstallIntegrationTarget(target))
        }
        crate::integration::IntegrationAction::Update => {
            Some(SettingsAction::UpdateIntegrationTarget(target))
        }
        crate::integration::IntegrationAction::Uninstall => {
            state.settings.integration_manager.pending_uninstall = Some(target);
            None
        }
    }
}

fn cancel_integration_uninstall(state: &mut AppState) {
    state.settings.integration_manager.pending_uninstall = None;
}

/// Cancel is mutation-free; confirming clears the pending target and hands
/// back the one `SettingsAction` that actually touches disk.
fn confirm_integration_uninstall(state: &mut AppState) -> Option<SettingsAction> {
    let target = state
        .settings
        .integration_manager
        .pending_uninstall
        .take()?;
    Some(SettingsAction::UninstallIntegrationTarget(target))
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
                state.settings.section = SettingsSection::Behavior;
                state.normalize_behavior_selection(None);
            }
            _ => {
                if let Some(super::modal::ModalAction::Close) =
                    super::modal::modal_action_from_key(&key, super::modal::SETTINGS_ACTIONS)
                {
                    cancel_settings(state);
                }
            }
        },
        SettingsSection::Behavior => match key.code {
            KeyCode::Up | KeyCode::Char('k') => state.move_behavior_selection(-1),
            KeyCode::Down | KeyCode::Char('j') => state.move_behavior_selection(1),
            KeyCode::Enter | KeyCode::Char(' ') => {
                let row = state
                    .behavior_rows()
                    .get(state.settings.list.selected)
                    .map(|row| row.id);
                return row.and_then(|row| behavior_action(state, row, 1));
            }
            KeyCode::Char('-') => {
                let row = state
                    .behavior_rows()
                    .get(state.settings.list.selected)
                    .map(|row| row.id);
                return row.and_then(|row| behavior_action(state, row, -1));
            }
            KeyCode::Char('+') | KeyCode::Char('=') => {
                let row = state
                    .behavior_rows()
                    .get(state.settings.list.selected)
                    .map(|row| row.id);
                return row.and_then(|row| behavior_action(state, row, 1));
            }
            KeyCode::BackTab | KeyCode::Left | KeyCode::Char('h') => {
                state.settings.section = SettingsSection::Display;
                state.normalize_display_selection(None);
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
        SettingsSection::Integrations => {
            let manager = &state.settings.integration_manager;
            if manager.history_detail_open {
                match key.code {
                    KeyCode::Up | KeyCode::Char('k') => {
                        state.settings.integration_manager.history_scroll = state
                            .settings
                            .integration_manager
                            .history_scroll
                            .saturating_sub(1);
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        state.settings.integration_manager.history_scroll = state
                            .settings
                            .integration_manager
                            .history_scroll
                            .saturating_add(1);
                    }
                    KeyCode::Esc => {
                        state.settings.integration_manager.history_detail_open = false;
                        state.settings.integration_manager.history_scroll = 0;
                    }
                    _ => {}
                }
                return None;
            }
            if manager.pending_uninstall.is_some() {
                return match key.code {
                    KeyCode::Enter => confirm_integration_uninstall(state),
                    KeyCode::Esc => {
                        cancel_integration_uninstall(state);
                        None
                    }
                    _ => None,
                };
            }
            if manager.action_menu_target.is_some() {
                match key.code {
                    KeyCode::Up | KeyCode::Char('k') => {
                        state.settings.integration_manager.action_menu_selected = state
                            .settings
                            .integration_manager
                            .action_menu_selected
                            .saturating_sub(1);
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        let target = state
                            .settings
                            .integration_manager
                            .action_menu_target
                            .expect("checked above");
                        let max = integration_actions_for(state, target)
                            .len()
                            .saturating_sub(1);
                        state.settings.integration_manager.action_menu_selected =
                            (state.settings.integration_manager.action_menu_selected + 1).min(max);
                    }
                    KeyCode::Enter | KeyCode::Char(' ') => {
                        return confirm_integration_action_menu(state);
                    }
                    KeyCode::Esc => cancel_integration_action_menu(state),
                    _ => {}
                }
                return None;
            }
            match key.code {
                KeyCode::Up | KeyCode::Char('k') => state.settings.list.move_prev(),
                KeyCode::Down | KeyCode::Char('j') => state
                    .settings
                    .list
                    .move_next(state.integration_recommendations.len()),
                KeyCode::Enter | KeyCode::Char(' ') => open_integration_action_menu(state),
                KeyCode::Char('d') if !state.integration_install_messages.is_empty() => {
                    state.settings.integration_manager.history_detail_open = true;
                }
                KeyCode::BackTab | KeyCode::Left | KeyCode::Char('h') => {
                    state.settings.section = SettingsSection::Behavior;
                    state.normalize_behavior_selection(None);
                }
                KeyCode::Tab | KeyCode::Right | KeyCode::Char('l') => {
                    state.settings.section = SettingsSection::Experiments;
                    state.settings.list.selected = 0;
                }
                _ => {
                    match super::modal::modal_action_from_key(&key, super::modal::SETTINGS_ACTIONS)
                    {
                        Some(super::modal::ModalAction::Apply) => return apply_settings(state),
                        Some(super::modal::ModalAction::Close) => cancel_settings(state),
                        _ => {}
                    }
                }
            }
        }
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
        SettingsSection::Behavior => 0,
        SettingsSection::Experiments => 0,
        SettingsSection::Integrations => 0,
    };
    if section == SettingsSection::Display {
        state.normalize_display_selection(None);
    }
    if section == SettingsSection::Behavior {
        state.normalize_behavior_selection(None);
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

    /// The section content viewport, narrowed to leave room for the shared
    /// scrollbar when the current section's rows overflow it. Render and
    /// every mouse/keyboard handler resolve geometry through this same pure
    /// projection so hit-testing can never diverge from what is drawn.
    pub(crate) fn settings_content_rect(&self) -> Rect {
        crate::ui::compute_settings_view(self, self.screen_rect()).content
    }

    fn settings_target_selected_index(&self, section: SettingsSection) -> usize {
        match section {
            SettingsSection::Theme => current_theme_index(&self.theme_name),
            SettingsSection::Sound => usize::from(!self.sound_enabled()),
            SettingsSection::Toast => toast_delivery_index(self.toast_delivery()),
            SettingsSection::Display
            | SettingsSection::Behavior
            | SettingsSection::Experiments
            | SettingsSection::Integrations => 0,
        }
    }

    fn select_settings_section(&mut self, section: SettingsSection) {
        self.settings.section = section;
        self.settings
            .list
            .select(self.settings_target_selected_index(section));
        if section == SettingsSection::Display {
            self.normalize_display_selection(None);
        }
        if section == SettingsSection::Behavior {
            self.normalize_behavior_selection(None);
        }
    }

    /// Move the current section's selection by one row (keyboard-equivalent
    /// step), used by mouse wheel scrolling so wheel and keyboard resolve
    /// through the same clamped range.
    fn move_settings_selection(&mut self, delta: i32) {
        let count = crate::ui::settings_row_count(self);
        if count == 0 {
            return;
        }
        let current = self.settings.list.selected.min(count - 1) as i32;
        let next = (current + delta).clamp(0, count as i32 - 1) as usize;
        self.settings.list.select(next);
        if self.settings.section == SettingsSection::Theme {
            preview_selected_theme(self);
        }
    }

    /// Resolve the action for a click/drag that landed on a rendered content
    /// row of `view`, if any. Returns `None` both when nothing was hit and
    /// when the hit row does not produce an action (e.g. a non-selectable
    /// Display status line) -- callers distinguish the two via `view.rows`.
    fn settings_row_action_at(
        &mut self,
        view: &crate::ui::SettingsView,
        col: u16,
        row: u16,
    ) -> Option<SettingsAction> {
        let hit = view
            .rows
            .iter()
            .find(|candidate| {
                row == candidate.rect.y
                    && col >= candidate.rect.x
                    && col < candidate.rect.x + candidate.rect.width
            })
            .cloned()?;
        if !hit.selectable {
            return None;
        }
        let idx = crate::ui::settings_row_absolute_index(self, &hit.id)?;
        let display_row = matches!(hit.id, crate::ui::SettingsRowId::Display(_))
            .then(|| self.display_rows().get(idx).cloned())
            .flatten();
        let behavior_row = matches!(hit.id, crate::ui::SettingsRowId::Behavior(_))
            .then(|| self.behavior_rows().get(idx).cloned())
            .flatten();
        self.settings.list.select(idx);
        match self.settings.section {
            SettingsSection::Theme => {
                preview_selected_theme(self);
                None
            }
            SettingsSection::Sound => Some(SettingsAction::SaveSound(idx == 0)),
            SettingsSection::Toast => Some(SettingsAction::SaveToastDelivery(
                toast_delivery_for_index(idx),
            )),
            SettingsSection::Display => {
                let display_row = display_row?;
                let size_delta = if display_row.id == DisplayRowId::DockSize {
                    crate::ui::display_size_delta_at(
                        self.settings_content_rect(),
                        col,
                        row,
                        self.display_scroll_start(
                            self.settings_content_rect().height.saturating_sub(3) as usize,
                        ),
                        idx,
                        self.dock_size,
                    )?
                } else {
                    1
                };
                display_action(self, &display_row.id, size_delta)
            }
            SettingsSection::Behavior => {
                let behavior_row = behavior_row?;
                let size_delta = if behavior_row.id == BehaviorRowId::ScrollLines {
                    crate::ui::behavior_scroll_lines_delta_at(
                        self.settings_content_rect(),
                        col,
                        row,
                        self.behavior_scroll_start(
                            self.settings_content_rect().height.saturating_sub(3) as usize,
                        ),
                        idx,
                        self.mouse_scroll_lines,
                    )?
                } else {
                    1
                };
                behavior_action(self, behavior_row.id, size_delta)
            }
            SettingsSection::Experiments => experiment_toggle_action(self, idx),
            SettingsSection::Integrations => {
                // A click selects and opens the target's action choices in
                // one step, matching Enter's behavior.
                open_integration_action_menu(self);
                None
            }
        }
    }

    /// Mouse handling for whichever Integrations sub-dialog (action menu,
    /// uninstall confirmation, or history detail) is covering the row list.
    /// `Some` means the event was consumed by the dialog — the caller must
    /// not fall through to row/button hit-testing underneath it. Only press
    /// events select a choice; a click anywhere closes the history detail.
    fn handle_integration_dialog_mouse(
        &mut self,
        mouse: MouseEvent,
    ) -> Option<Option<SettingsAction>> {
        if self.settings.section != SettingsSection::Integrations {
            return None;
        }
        let manager = &self.settings.integration_manager;
        if manager.history_detail_open {
            if matches!(mouse.kind, MouseEventKind::Down(MouseButton::Left)) {
                self.settings.integration_manager.history_detail_open = false;
                self.settings.integration_manager.history_scroll = 0;
            }
            return Some(None);
        }
        if manager.pending_uninstall.is_some() {
            if matches!(mouse.kind, MouseEventKind::Down(MouseButton::Left)) {
                let content = self.settings_content_rect();
                let rects = crate::ui::integration_dialog_choice_rects(content, 2);
                let hit = rects.iter().position(|rect| {
                    mouse.row == rect.y
                        && mouse.column >= rect.x
                        && mouse.column < rect.x + rect.width
                });
                return Some(match hit {
                    Some(0) => confirm_integration_uninstall(self),
                    Some(_) => {
                        cancel_integration_uninstall(self);
                        None
                    }
                    None => None,
                });
            }
            return Some(None);
        }
        if manager.action_menu_target.is_some() {
            if matches!(mouse.kind, MouseEventKind::Down(MouseButton::Left)) {
                let target = self
                    .settings
                    .integration_manager
                    .action_menu_target
                    .expect("checked above");
                let count = integration_actions_for(self, target).len();
                let content = self.settings_content_rect();
                let rects = crate::ui::integration_dialog_choice_rects(content, count);
                let hit = rects.iter().position(|rect| {
                    mouse.row == rect.y
                        && mouse.column >= rect.x
                        && mouse.column < rect.x + rect.width
                });
                return Some(match hit {
                    Some(idx) => {
                        self.settings.integration_manager.action_menu_selected = idx;
                        confirm_integration_action_menu(self)
                    }
                    None => None,
                });
            }
            return Some(None);
        }
        None
    }

    pub(super) fn handle_settings_mouse(&mut self, mouse: MouseEvent) -> Option<SettingsAction> {
        if let Some(result) = self.handle_integration_dialog_mouse(mouse) {
            return result;
        }
        match mouse.kind {
            MouseEventKind::ScrollUp => {
                self.move_settings_selection(-1);
                None
            }
            MouseEventKind::ScrollDown => {
                self.move_settings_selection(1);
                None
            }
            MouseEventKind::Down(MouseButton::Left) | MouseEventKind::Drag(MouseButton::Left) => {
                let is_press = matches!(mouse.kind, MouseEventKind::Down(MouseButton::Left));
                let view = crate::ui::compute_settings_view(self, self.screen_rect());

                if is_press {
                    if let Some(delta) = view.nav.chevron_at(mouse.column, mouse.row) {
                        let sections = SettingsSection::ALL;
                        let current = sections
                            .iter()
                            .position(|section| *section == self.settings.section)
                            .unwrap_or(0);
                        let next = (current as i32 + i32::from(delta))
                            .clamp(0, sections.len() as i32 - 1)
                            as usize;
                        self.select_settings_section(sections[next]);
                        return None;
                    }
                    if let Some(section) = view.nav.section_at(mouse.column, mouse.row) {
                        self.select_settings_section(section);
                        return None;
                    }
                }

                let hit_row = view.rows.iter().any(|candidate| {
                    mouse.row == candidate.rect.y
                        && mouse.column >= candidate.rect.x
                        && mouse.column < candidate.rect.x + candidate.rect.width
                });
                if hit_row {
                    return self.settings_row_action_at(&view, mouse.column, mouse.row);
                }

                if let Some(track) = view.scrollbar {
                    if mouse.column == track.x
                        && mouse.row >= track.y
                        && mouse.row < track.y + track.height
                    {
                        let count = crate::ui::settings_row_count(self);
                        let target = crate::ui::top_anchored_offset_from_row(
                            count,
                            view.rows.len(),
                            track,
                            mouse.row,
                        );
                        self.settings
                            .list
                            .select(target.min(count.saturating_sub(1)));
                        if self.settings.section == SettingsSection::Theme {
                            preview_selected_theme(self);
                        }
                        return None;
                    }
                }

                if !is_press {
                    // A drag that lands outside every row/scrollbar target (e.g.
                    // between rows) neither changes selection nor closes the modal.
                    return None;
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
    use crate::ui::SettingsNav;

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

    // -----------------------------------------------------------------
    // Everyday preferences: Display/Behavior keyboard+mouse parity (4.1/4.2)
    // -----------------------------------------------------------------

    #[test]
    fn display_pane_borders_and_agent_sort_toggle_via_keyboard() {
        let mut state = state_with_workspaces(&["test"]);
        state.pane_borders = true;
        state.agent_panel_sort = crate::app::state::AgentPanelSort::Spaces;
        open_settings_at(&mut state, SettingsSection::Display);

        let borders_idx = state
            .display_rows()
            .iter()
            .position(|row| row.id == DisplayRowId::PaneBorders)
            .expect("pane borders row");
        state.settings.list.selected = borders_idx;
        assert_eq!(
            update_settings_state(
                &mut state,
                KeyEvent::new(KeyCode::Enter, KeyModifiers::empty())
            ),
            Some(SettingsAction::SavePaneBorders(false))
        );

        let sort_idx = state
            .display_rows()
            .iter()
            .position(|row| row.id == DisplayRowId::AgentSort)
            .expect("agent sort row");
        state.settings.list.selected = sort_idx;
        assert_eq!(
            update_settings_state(
                &mut state,
                KeyEvent::new(KeyCode::Enter, KeyModifiers::empty())
            ),
            Some(SettingsAction::SaveAgentPanelSort(
                crate::app::state::AgentPanelSort::Priority
            ))
        );
    }

    #[test]
    fn behavior_row_toggle_and_scroll_lines_stepper_via_keyboard() {
        let mut state = state_with_workspaces(&["test"]);
        state.confirm_close = true;
        state.mouse_scroll_lines = crate::app::state::MIN_MOUSE_SCROLL_LINES;
        open_settings_at(&mut state, SettingsSection::Behavior);

        assert_eq!(state.settings.section, SettingsSection::Behavior);
        let confirm_idx = state
            .behavior_rows()
            .iter()
            .position(|row| row.id == BehaviorRowId::ConfirmClose)
            .expect("confirm close row");
        state.settings.list.selected = confirm_idx;
        assert_eq!(
            update_settings_state(
                &mut state,
                KeyEvent::new(KeyCode::Enter, KeyModifiers::empty())
            ),
            Some(SettingsAction::SaveConfirmClose(false))
        );

        let scroll_idx = state
            .behavior_rows()
            .iter()
            .position(|row| row.id == BehaviorRowId::ScrollLines)
            .expect("scroll lines row");
        state.settings.list.selected = scroll_idx;
        assert_eq!(
            update_settings_state(
                &mut state,
                KeyEvent::new(KeyCode::Char('-'), KeyModifiers::empty())
            ),
            None,
            "the floor must not decrement below MIN_MOUSE_SCROLL_LINES"
        );
        assert_eq!(
            update_settings_state(
                &mut state,
                KeyEvent::new(KeyCode::Char('+'), KeyModifiers::empty())
            ),
            Some(SettingsAction::SaveMouseScrollLines(
                crate::app::state::MIN_MOUSE_SCROLL_LINES + 1
            ))
        );
    }

    #[test]
    fn behavior_rows_stay_reachable_and_mouse_matches_keyboard_at_40x20_64x20_80x24() {
        let mut app = app_for_mouse_test();
        open_settings_at(&mut app.state, SettingsSection::Behavior);
        // The second-to-last row (CopyOnSelect), not the numeric ScrollLines
        // stepper: that row's click hit-test targets its `[-]`/`[+]` zones
        // rather than the whole row, which is covered separately above.
        let last = app.state.behavior_rows().len() - 2;
        app.state.settings.list.selected = last;
        let last_id = app.state.behavior_rows()[last].id;

        for (width, height) in [(40u16, 20u16), (64, 20), (80, 24)] {
            app.state.view.terminal_area = ratatui::layout::Rect::new(0, 0, width, height);
            app.state.view.sidebar_rect = ratatui::layout::Rect::default();
            let view = crate::ui::compute_settings_view(&app.state, app.state.screen_rect());
            let rendered_row = view
                .rows
                .iter()
                .find(|row| row.id == crate::ui::SettingsRowId::Behavior(last_id))
                .unwrap_or_else(|| panic!("last behavior row must render at {width}x{height}"));
            let action = app.state.handle_settings_mouse(mouse(
                MouseEventKind::Down(MouseButton::Left),
                rendered_row.rect.x,
                rendered_row.rect.y,
            ));
            let expected = behavior_action(&app.state, last_id, 1);
            assert_eq!(
                action, expected,
                "mouse click on the last behavior row must resolve the same action as keyboard at {width}x{height}"
            );
        }
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

        // Behavior is a declared addition between Display and Integrations
        // (design.md § apply baseline); the rest of the cycle is unchanged.
        update_settings_state(
            &mut state,
            KeyEvent::new(KeyCode::Tab, KeyModifiers::empty()),
        );
        assert_eq!(state.settings.section, SettingsSection::Behavior);

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
        assert_eq!(state.settings.section, SettingsSection::Behavior);

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
        let mut app = app_for_mouse_test();
        let state = &mut app.state;
        state.integration_recommendations = vec![integration_recommendation(
            crate::integration::IntegrationStatusKind::Outdated,
            true,
        )];
        open_settings(state);
        state.settings.section = SettingsSection::Integrations;

        let view = crate::ui::compute_settings_view(state, state.screen_rect());
        // The badge plus the added Behavior section can push the header into
        // overflow at this width; either layout must still make Integrations
        // reachable (see `settings_sections_fall_back_to_overflow_viewport_
        // when_labels_do_not_fit` in ui/settings.rs for the same contract).
        let integrations_rect = match &view.nav {
            SettingsNav::Full(items) => items
                .iter()
                .find(|item| item.section == SettingsSection::Integrations)
                .map(|item| item.rect),
            SettingsNav::Overflow { items, .. } => items
                .iter()
                .find(|item| item.section == SettingsSection::Integrations)
                .map(|item| item.rect),
            SettingsNav::Compact { rect } => Some(*rect),
        }
        .expect("integrations tab should be reachable");

        assert_eq!(
            view.nav.section_at(
                integrations_rect.x + integrations_rect.width - 1,
                integrations_rect.y
            ),
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
            supported: true,
            available,
            path: std::path::PathBuf::from("/tmp/shepherd-test-integration"),
            state,
            installed_version: None,
            expected_version: 1,
        }
    }

    // -----------------------------------------------------------------
    // Integration action menu / uninstall confirmation / history
    // (tasks 3.2, 3.3)
    // -----------------------------------------------------------------

    fn open_integrations_with(
        state: crate::integration::IntegrationStatusKind,
        available: bool,
    ) -> AppState {
        let mut app_state = state_with_workspaces(&["test"]);
        app_state.integration_recommendations = vec![integration_recommendation(state, available)];
        open_settings_at(&mut app_state, SettingsSection::Integrations);
        app_state
    }

    #[test]
    fn integration_enter_on_installable_target_opens_single_choice_menu() {
        let mut state = open_integrations_with(
            crate::integration::IntegrationStatusKind::NotInstalled,
            true,
        );

        let action = update_settings_state(
            &mut state,
            KeyEvent::new(KeyCode::Enter, KeyModifiers::empty()),
        );
        assert_eq!(action, None, "opening the menu is not itself a save action");
        assert_eq!(
            state.settings.integration_manager.action_menu_target,
            Some(crate::api::schema::IntegrationTarget::Claude)
        );

        let action = update_settings_state(
            &mut state,
            KeyEvent::new(KeyCode::Enter, KeyModifiers::empty()),
        );
        assert_eq!(
            action,
            Some(SettingsAction::InstallIntegrationTarget(
                crate::api::schema::IntegrationTarget::Claude
            ))
        );
        assert_eq!(state.settings.integration_manager.action_menu_target, None);
    }

    #[test]
    fn integration_outdated_target_menu_offers_update_then_uninstall() {
        let mut state =
            open_integrations_with(crate::integration::IntegrationStatusKind::Outdated, true);

        update_settings_state(
            &mut state,
            KeyEvent::new(KeyCode::Enter, KeyModifiers::empty()),
        );
        assert_eq!(state.settings.integration_manager.action_menu_selected, 0);

        // Choice 0 (Update) confirms immediately as a save action.
        let action = update_settings_state(
            &mut state,
            KeyEvent::new(KeyCode::Enter, KeyModifiers::empty()),
        );
        assert_eq!(
            action,
            Some(SettingsAction::UpdateIntegrationTarget(
                crate::api::schema::IntegrationTarget::Claude
            ))
        );

        // Reopen and choose Uninstall (choice 1): opens confirmation instead
        // of mutating immediately.
        update_settings_state(
            &mut state,
            KeyEvent::new(KeyCode::Enter, KeyModifiers::empty()),
        );
        update_settings_state(
            &mut state,
            KeyEvent::new(KeyCode::Down, KeyModifiers::empty()),
        );
        let action = update_settings_state(
            &mut state,
            KeyEvent::new(KeyCode::Enter, KeyModifiers::empty()),
        );
        assert_eq!(action, None);
        assert_eq!(
            state.settings.integration_manager.pending_uninstall,
            Some(crate::api::schema::IntegrationTarget::Claude)
        );
    }

    #[test]
    fn integration_uninstall_confirm_cancel_is_mutation_free() {
        let mut state =
            open_integrations_with(crate::integration::IntegrationStatusKind::Current, true);
        state.settings.integration_manager.pending_uninstall =
            Some(crate::api::schema::IntegrationTarget::Claude);

        let action = update_settings_state(
            &mut state,
            KeyEvent::new(KeyCode::Esc, KeyModifiers::empty()),
        );

        assert_eq!(action, None);
        assert_eq!(state.settings.integration_manager.pending_uninstall, None);
        // Cancel never touches recommendations/history.
        assert!(state.settings.integration_manager.history.is_empty());
    }

    #[test]
    fn integration_uninstall_confirm_enter_returns_uninstall_action() {
        let mut state =
            open_integrations_with(crate::integration::IntegrationStatusKind::Current, true);
        state.settings.integration_manager.pending_uninstall =
            Some(crate::api::schema::IntegrationTarget::Claude);

        let action = update_settings_state(
            &mut state,
            KeyEvent::new(KeyCode::Enter, KeyModifiers::empty()),
        );

        assert_eq!(
            action,
            Some(SettingsAction::UninstallIntegrationTarget(
                crate::api::schema::IntegrationTarget::Claude
            ))
        );
        assert_eq!(state.settings.integration_manager.pending_uninstall, None);
    }

    #[test]
    fn integration_operation_in_flight_blocks_opening_action_menu() {
        let mut state = open_integrations_with(
            crate::integration::IntegrationStatusKind::NotInstalled,
            true,
        );
        state.settings.integration_manager.operation_in_flight = true;

        update_settings_state(
            &mut state,
            KeyEvent::new(KeyCode::Enter, KeyModifiers::empty()),
        );

        assert_eq!(
            state.settings.integration_manager.action_menu_target, None,
            "a single-flight operation must block opening a second one"
        );
    }

    #[test]
    fn integration_unsupported_row_enter_does_not_open_menu() {
        let mut state = state_with_workspaces(&["test"]);
        state.integration_recommendations = vec![integration_recommendation_full(false, false)];
        open_settings_at(&mut state, SettingsSection::Integrations);

        update_settings_state(
            &mut state,
            KeyEvent::new(KeyCode::Enter, KeyModifiers::empty()),
        );

        assert_eq!(state.settings.integration_manager.action_menu_target, None);
    }

    fn integration_recommendation_full(
        supported: bool,
        available: bool,
    ) -> crate::integration::IntegrationRecommendation {
        crate::integration::IntegrationRecommendation {
            target: crate::api::schema::IntegrationTarget::Claude,
            label: "claude",
            command: "claude",
            supported,
            available,
            path: std::path::PathBuf::from("/tmp/shepherd-test-integration"),
            state: crate::integration::IntegrationStatusKind::NotInstalled,
            installed_version: None,
            expected_version: 1,
        }
    }

    #[test]
    fn integration_history_detail_toggle_and_scroll() {
        let mut state = open_integrations_with(
            crate::integration::IntegrationStatusKind::NotInstalled,
            true,
        );
        state
            .integration_install_messages
            .extend(["installed claude".to_string(), "updated codex".to_string()]);

        update_settings_state(
            &mut state,
            KeyEvent::new(KeyCode::Char('d'), KeyModifiers::empty()),
        );
        assert!(state.settings.integration_manager.history_detail_open);

        update_settings_state(
            &mut state,
            KeyEvent::new(KeyCode::Down, KeyModifiers::empty()),
        );
        assert_eq!(state.settings.integration_manager.history_scroll, 1);

        update_settings_state(
            &mut state,
            KeyEvent::new(KeyCode::Esc, KeyModifiers::empty()),
        );
        assert!(!state.settings.integration_manager.history_detail_open);
        assert_eq!(state.settings.integration_manager.history_scroll, 0);
    }

    #[test]
    fn integration_dialog_mouse_click_confirms_uninstall() {
        let mut app = app_for_mouse_test();
        app.state.integration_recommendations = vec![integration_recommendation(
            crate::integration::IntegrationStatusKind::Current,
            true,
        )];
        open_settings_at(&mut app.state, SettingsSection::Integrations);
        app.state.settings.integration_manager.pending_uninstall =
            Some(crate::api::schema::IntegrationTarget::Claude);

        let content = app.state.settings_content_rect();
        let rects = crate::ui::integration_dialog_choice_rects(content, 2);
        let action = app.state.handle_settings_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            rects[0].x,
            rects[0].y,
        ));

        assert_eq!(
            action,
            Some(SettingsAction::UninstallIntegrationTarget(
                crate::api::schema::IntegrationTarget::Claude
            ))
        );
    }

    #[test]
    fn integration_dialog_mouse_click_confirms_action_menu_choice() {
        let mut app = app_for_mouse_test();
        app.state.integration_recommendations = vec![integration_recommendation(
            crate::integration::IntegrationStatusKind::NotInstalled,
            true,
        )];
        open_settings_at(&mut app.state, SettingsSection::Integrations);
        app.state.settings.integration_manager.action_menu_target =
            Some(crate::api::schema::IntegrationTarget::Claude);

        let content = app.state.settings_content_rect();
        let rects = crate::ui::integration_dialog_choice_rects(content, 1);
        let action = app.state.handle_settings_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            rects[0].x,
            rects[0].y,
        ));

        assert_eq!(
            action,
            Some(SettingsAction::InstallIntegrationTarget(
                crate::api::schema::IntegrationTarget::Claude
            ))
        );
        assert_eq!(
            app.state.settings.integration_manager.action_menu_target,
            None
        );
    }

    #[test]
    fn install_integration_target_records_history_and_refreshes_recommendations() {
        let (_api_tx, api_rx) = tokio::sync::mpsc::unbounded_channel();
        let mut app = App::new(
            &crate::config::Config::default(),
            true,
            None,
            api_rx,
            crate::api::EventHub::default(),
        );
        let target = crate::api::schema::IntegrationTarget::Claude;

        app.uninstall_integration_target(target);

        assert!(!app.state.settings.integration_manager.operation_in_flight);
        assert!(!app.state.settings.integration_manager.history.is_empty());
        assert!(!app.state.integration_install_messages.is_empty());
        let record = app
            .state
            .settings
            .integration_manager
            .history
            .last()
            .expect("a record must be pushed");
        assert_eq!(record.target, target);
    }
}
