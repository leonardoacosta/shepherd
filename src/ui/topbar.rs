use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::Line,
    widgets::Paragraph,
    Frame,
};

use super::{
    sidebar::{self, tokens::ResolvedToken, AgentPanelEntry},
    status::{state_dot, state_label, state_label_color},
};
use crate::app::AppState;

fn active_main_entry(app: &AppState) -> Option<AgentPanelEntry> {
    let ws_idx = app.active?;
    let workspace = app.workspaces.get(ws_idx)?;
    let tab_idx = workspace.active_tab;
    let tab = workspace.tabs.get(tab_idx)?;
    let pane_id = tab.layout.focused();
    let pane = tab.panes.get(&pane_id)?;
    let terminal = app.terminals.get(&pane.attached_terminal_id);
    let presentation = terminal.map(crate::terminal::TerminalState::effective_presentation);
    let agent = terminal.and_then(crate::terminal::TerminalState::effective_known_agent);
    let agent_kind_label = terminal
        .and_then(crate::terminal::TerminalState::effective_agent_label)
        .map(str::to_string);
    let agent_label = terminal
        .and_then(crate::terminal::TerminalState::effective_display_agent)
        .or_else(|| agent_kind_label.clone());
    let show_tab = app.visible_tab_indices(ws_idx).len() > 1 || !tab.is_auto_named();
    let mut entry = AgentPanelEntry {
        ws_idx,
        tab_idx,
        pane_id,
        primary_label: workspace.display_name_from_terminals(&app.terminals),
        primary_tab_label: show_tab
            .then(|| workspace.tab_display_name(tab_idx))
            .flatten(),
        pane_label: terminal
            .and_then(crate::terminal::TerminalState::effective_title)
            .or_else(|| terminal.and_then(|terminal| terminal.manual_label.clone())),
        terminal_title: terminal.and_then(|terminal| terminal.terminal_title.clone()),
        terminal_title_stripped: terminal
            .and_then(crate::terminal::TerminalState::terminal_title_stripped),
        agent_label,
        agent_kind_label,
        agent,
        state: terminal.map_or(crate::detect::AgentState::Unknown, |terminal| {
            terminal.state
        }),
        seen: pane.seen,
        last_agent_state_change_seq: terminal
            .and_then(|terminal| terminal.last_agent_state_change_seq),
        state_labels: presentation
            .as_ref()
            .map(|presentation| presentation.state_labels.clone())
            .unwrap_or_default(),
        tokens: terminal
            .map(|terminal| terminal.metadata_tokens.values())
            .unwrap_or_default(),
    };
    for (name, value) in workspace.metadata_tokens.values() {
        entry.tokens.insert(name.clone(), value.clone());
    }
    Some(entry)
}

pub(super) fn resolved_rows(app: &AppState) -> Vec<Vec<ResolvedToken>> {
    if !app.topbar_enabled {
        return Vec::new();
    }
    let Some(entry) = active_main_entry(app) else {
        return Vec::new();
    };
    let state_text = entry
        .state_labels
        .get(sidebar::agent_panel_status_key(entry.state, entry.seen))
        .map(String::as_str)
        .unwrap_or_else(|| state_label(entry.state, entry.seen));
    sidebar::tokens::agent_rows_from(&app.topbar_rows, &entry, state_text)
}

pub(super) fn render_topbar(app: &AppState, frame: &mut Frame, area: Rect) {
    if area.is_empty() {
        return;
    }
    let Some(entry) = active_main_entry(app) else {
        return;
    };
    let rows = resolved_rows(app);
    let label_color = state_label_color(entry.state, entry.seen, &app.palette);
    for (row_idx, row) in rows.iter().take(area.height as usize).enumerate() {
        let spans = sidebar::resolved_token_spans(
            row,
            state_dot(entry.state, entry.seen, &app.palette),
            Style::default().fg(label_color),
            Style::default()
                .fg(app.palette.text)
                .add_modifier(Modifier::BOLD),
            Style::default().fg(app.palette.subtext0),
            Style::default().fg(app.palette.overlay1),
            &app.palette,
            area.width as usize,
        );
        let row_area = Rect::new(area.x, area.y + row_idx as u16, area.width, 1);
        frame.render_widget(
            Paragraph::new(Line::from(spans)).style(Style::default().bg(app.palette.panel_bg)),
            row_area,
        );
    }
    if rows.len() < area.height as usize {
        for row in area.y + rows.len() as u16..area.y + area.height {
            frame.render_widget(
                Paragraph::new("").style(Style::default().bg(app.palette.panel_bg)),
                Rect::new(area.x, row, area.width, 1),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{AgentSidebarToken, TopbarConfig};
    use crate::workspace::Workspace;
    use ratatui::{backend::TestBackend, Terminal};

    #[test]
    fn topbar_resolved_rows_drive_multiline_rendering() {
        let mut app = AppState::test_new();
        app.workspaces = vec![Workspace::test_new("topbar-workspace")];
        app.active = Some(0);
        app.topbar_enabled = true;
        app.ensure_test_terminals();
        app.topbar_rows = vec![
            vec![AgentSidebarToken::Workspace],
            vec![AgentSidebarToken::Workspace],
        ];

        let mut terminal = Terminal::new(TestBackend::new(40, 2)).expect("test terminal");
        terminal
            .draw(|frame| render_topbar(&app, frame, Rect::new(0, 0, 40, 2)))
            .expect("topbar render");
        let buffer = terminal.backend().buffer();
        let second_row = (0..40).map(|x| buffer[(x, 1)].symbol()).collect::<String>();

        assert!(second_row.contains("topbar-workspace"), "{second_row:?}");
    }

    #[test]
    fn topbar_resolves_main_pane_builtins_workspace_custom_precedence_and_styles() {
        let mut app = AppState::test_new();
        let mut workspace = Workspace::test_new("topbar-workspace");
        workspace.metadata_tokens.patch(
            std::collections::HashMap::from([(
                "owner".to_string(),
                Some("workspace-owner".to_string()),
            )]),
            None,
            std::time::Instant::now(),
        );
        app.workspaces = vec![workspace];
        app.active = Some(0);
        app.ensure_test_terminals();
        let pane_id = app.workspaces[0].tabs[0].root_pane;
        let terminal_id = app.workspaces[0].tabs[0].panes[&pane_id]
            .attached_terminal_id
            .clone();
        let terminal = app.terminals.get_mut(&terminal_id).expect("terminal");
        terminal.detected_agent = Some(crate::detect::Agent::Codex);
        terminal.set_manual_label("review pane".into());
        terminal.terminal_title = Some("raw title".into());
        terminal.metadata_tokens.patch(
            std::collections::HashMap::from([
                ("owner".to_string(), Some("pane-owner".to_string())),
                ("pane_only".to_string(), Some("pane-value".to_string())),
            ]),
            None,
            std::time::Instant::now(),
        );
        let config: TopbarConfig = toml::from_str(
            r##"
enabled = true
rows = [
  [{ token = "workspace", fg = "#ff0000", bold = true }, "$owner"],
  ["pane", "agent", "terminal_title", "$pane_only"],
  ["$missing"],
]
"##,
        )
        .expect("topbar config");
        app.topbar_enabled = config.enabled;
        app.topbar_rows = config.rows;

        let rows = resolved_rows(&app);

        assert_eq!(rows.len(), 2, "empty configured rows must elide");
        assert!(rows[0][0].style.bold == Some(true));
        assert!(rows[0][0].style.fg.is_some());
        assert!(matches!(
            &rows[0][1].kind,
            super::sidebar::tokens::ResolvedTokenKind::Custom(value)
                if value == "workspace-owner"
        ));
        assert!(matches!(
            &rows[1][0].kind,
            super::sidebar::tokens::ResolvedTokenKind::Pane(value) if value == "review pane"
        ));
        assert!(matches!(
            &rows[1][1].kind,
            super::sidebar::tokens::ResolvedTokenKind::Agent(value) if value == "codex"
        ));
        assert!(matches!(
            &rows[1][2].kind,
            super::sidebar::tokens::ResolvedTokenKind::TerminalTitle(value)
                if value == "raw title"
        ));
        assert!(matches!(
            &rows[1][3].kind,
            super::sidebar::tokens::ResolvedTokenKind::Custom(value) if value == "pane-value"
        ));

        let mut terminal = Terminal::new(TestBackend::new(80, 2)).expect("test terminal");
        terminal
            .draw(|frame| render_topbar(&app, frame, Rect::new(0, 0, 80, 2)))
            .expect("topbar render");
        let first_row = (0..80)
            .map(|x| terminal.backend().buffer()[(x, 0)].symbol())
            .collect::<String>();
        assert!(
            first_row.contains("topbar-workspace · workspace-owner"),
            "{first_row:?}"
        );
        let styled = terminal.backend().buffer()[(0, 0)].style();
        assert!(styled.add_modifier.contains(Modifier::BOLD));
    }

    #[test]
    fn topbar_has_no_rows_without_active_workspace() {
        let mut app = AppState::test_new();
        app.topbar_enabled = true;
        app.topbar_rows = vec![vec![AgentSidebarToken::Workspace]];
        assert!(resolved_rows(&app).is_empty());
    }
}
