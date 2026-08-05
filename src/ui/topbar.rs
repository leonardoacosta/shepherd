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
        session_status: workspace.session_status(),
    };
    for (name, value) in workspace.metadata_tokens.values() {
        entry.tokens.insert(name.clone(), value.clone());
    }
    Some(entry)
}

/// Both chrome panels — the topbar and the right panel — resolve the same
/// `AgentSidebarToken` rows against the active main pane through the sidebar's own
/// resolver. One vocabulary, one resolver; a second copy is what this module exists to
/// prevent (`anchor-chrome-to-sidebar` design.md, Decision 2).
fn resolved_chrome_rows(
    app: &AppState,
    enabled: bool,
    rows: &[Vec<crate::config::AgentSidebarToken>],
) -> Vec<Vec<ResolvedToken>> {
    if !enabled {
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
    sidebar::tokens::agent_rows_from(rows, &entry, state_text)
}

pub(super) fn resolved_rows(app: &AppState) -> Vec<Vec<ResolvedToken>> {
    resolved_chrome_rows(app, app.topbar_enabled, &app.topbar_rows)
}

pub(super) fn resolved_right_panel_rows(app: &AppState) -> Vec<Vec<ResolvedToken>> {
    resolved_chrome_rows(app, app.right_panel_enabled, &app.right_panel_rows)
}

pub(super) fn render_right_panel(app: &AppState, frame: &mut Frame, area: Rect) {
    render_chrome_rows(app, frame, area, resolved_right_panel_rows(app));
}

pub(super) fn render_topbar(app: &AppState, frame: &mut Frame, area: Rect) {
    render_chrome_rows(app, frame, area, resolved_rows(app));
}

fn render_chrome_rows(
    app: &AppState,
    frame: &mut Frame,
    area: Rect,
    rows: Vec<Vec<ResolvedToken>>,
) {
    if area.is_empty() {
        return;
    }
    let Some(entry) = active_main_entry(app) else {
        return;
    };
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
    fn right_panel_resolves_tokens_elides_absent_values_and_omits_empty_rows() {
        let mut app = AppState::test_new();
        app.workspaces = vec![Workspace::test_new("right-panel-ws")];
        app.active = Some(0);
        app.ensure_test_terminals();
        let pane_id = app.workspaces[0].tabs[0].root_pane;
        let terminal_id = app.workspaces[0].tabs[0].panes[&pane_id]
            .attached_terminal_id
            .clone();
        app.terminals
            .get_mut(&terminal_id)
            .expect("terminal")
            .detected_agent = Some(crate::detect::Agent::Codex);
        app.right_panel_enabled = true;

        // A row whose every token elides is omitted; the populated row survives.
        app.right_panel_rows = vec![
            vec![AgentSidebarToken::Workspace, AgentSidebarToken::Agent],
            vec![AgentSidebarToken::Custom("missing".into())],
        ];
        let rows = resolved_right_panel_rows(&app);
        assert_eq!(rows.len(), 1, "fully unresolved row must be omitted");
        assert_eq!(rows[0].len(), 2, "both populated tokens resolve");

        // An absent token elides individually, taking its separator with it.
        app.right_panel_rows = vec![vec![
            AgentSidebarToken::Workspace,
            AgentSidebarToken::Custom("missing".into()),
        ]];
        let rows = resolved_right_panel_rows(&app);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].len(), 1, "absent token elides inside the row");

        let mut terminal = Terminal::new(TestBackend::new(20, 4)).expect("test terminal");
        terminal
            .draw(|frame| render_right_panel(&app, frame, Rect::new(0, 0, 20, 4)))
            .expect("right panel render");
        let first_row = (0..20)
            .map(|x| terminal.backend().buffer()[(x, 0)].symbol())
            .collect::<String>();
        assert!(first_row.contains("right-panel-ws"), "{first_row:?}");
        assert!(
            !first_row.contains('·'),
            "no separator survives an elided token: {first_row:?}"
        );
    }

    #[test]
    fn right_panel_disabled_resolves_no_rows() {
        let mut app = AppState::test_new();
        app.workspaces = vec![Workspace::test_new("right-panel-ws")];
        app.active = Some(0);
        app.ensure_test_terminals();
        app.right_panel_enabled = false;
        app.right_panel_rows = vec![vec![AgentSidebarToken::Workspace]];

        assert!(resolved_right_panel_rows(&app).is_empty());
    }

    #[test]
    fn topbar_has_no_rows_without_active_workspace() {
        let mut app = AppState::test_new();
        app.topbar_enabled = true;
        app.topbar_rows = vec![vec![AgentSidebarToken::Workspace]];
        assert!(resolved_rows(&app).is_empty());
    }
}

/// Rendered-evidence characterization for `research-presentation-layout-presets`.
/// Renders named topbar candidate row layouts through production token
/// resolution (`sidebar::tokens::agent_rows_from`, the same call `resolved_rows`
/// makes) and production cell rendering (`sidebar::resolved_token_spans`) at the
/// widths declared in the change's `evidence/layouts.md`. Not a runtime behavior
/// change — read-only drift characterization the terminal user gate consults.
#[cfg(test)]
mod presentation_layout_candidate {
    use super::sidebar::AgentPanelEntry;
    use super::{sidebar, state_dot};
    use crate::config::AgentSidebarToken;
    use crate::detect::AgentState;
    use ratatui::style::Style;

    fn entry(
        primary_label: &str,
        tab: Option<&str>,
        pane: Option<&str>,
        agent_label: Option<&str>,
        agent: Option<crate::detect::Agent>,
        state: AgentState,
        seen: bool,
    ) -> AgentPanelEntry {
        AgentPanelEntry {
            ws_idx: 0,
            tab_idx: 0,
            pane_id: crate::layout::PaneId::from_raw(1),
            primary_label: primary_label.into(),
            primary_tab_label: tab.map(str::to_string),
            pane_label: pane.map(str::to_string),
            terminal_title: None,
            terminal_title_stripped: None,
            agent_label: agent_label.map(str::to_string),
            agent_kind_label: agent_label.map(str::to_string),
            agent,
            state,
            seen,
            last_agent_state_change_seq: None,
            state_labels: std::collections::HashMap::new(),
            tokens: std::collections::HashMap::new(),
            session_status: Default::default(),
        }
    }

    struct Case {
        name: &'static str,
        entry: AgentPanelEntry,
        state_text: &'static str,
    }

    fn cases() -> Vec<Case> {
        vec![
            Case {
                name: "representative",
                entry: entry(
                    "shepherd",
                    Some("feature-auth"),
                    Some("review pane"),
                    Some("claude"),
                    Some(crate::detect::Agent::Claude),
                    AgentState::Working,
                    true,
                ),
                state_text: "working",
            },
            Case {
                name: "long",
                entry: entry(
                    "backend-services-payment-orchestration-monorepo",
                    Some("refactor-billing-reconciliation-pipeline"),
                    Some("review pane for the billing reconciliation output"),
                    Some("claude-opus-5-sonnet-reasoning"),
                    Some(crate::detect::Agent::Claude),
                    AgentState::Blocked,
                    true,
                ),
                state_text: "blocked",
            },
            Case {
                name: "missing",
                entry: entry(
                    "solo-workspace",
                    None,
                    None,
                    None,
                    None,
                    AgentState::Unknown,
                    true,
                ),
                state_text: "idle",
            },
        ]
    }

    fn candidates() -> Vec<(&'static str, Vec<Vec<AgentSidebarToken>>)> {
        vec![
            (
                "baseline-workspace-only",
                vec![vec![AgentSidebarToken::Workspace]],
            ),
            (
                "workspace-tab",
                vec![vec![AgentSidebarToken::Workspace, AgentSidebarToken::Tab]],
            ),
            (
                "workspace-agent-two-row",
                vec![
                    vec![AgentSidebarToken::Workspace],
                    vec![AgentSidebarToken::Agent],
                ],
            ),
            (
                "full-context",
                vec![
                    vec![
                        AgentSidebarToken::Workspace,
                        AgentSidebarToken::Tab,
                        AgentSidebarToken::Pane,
                    ],
                    vec![AgentSidebarToken::Agent],
                ],
            ),
            (
                "status-first",
                vec![vec![
                    AgentSidebarToken::StateIcon,
                    AgentSidebarToken::StateText,
                    AgentSidebarToken::Workspace,
                ]],
            ),
        ]
    }

    #[test]
    fn topbar_candidates_render_within_declared_widths() {
        let palette = crate::app::AppState::test_new().palette;
        let widths = [40usize, 80];

        for (candidate_name, rows_config) in candidates() {
            for case in cases() {
                let resolved =
                    sidebar::tokens::agent_rows_from(&rows_config, &case.entry, case.state_text);
                let icon = state_dot(case.entry.state, case.entry.seen, &palette);
                for width in widths {
                    let lines: Vec<String> = resolved
                        .iter()
                        .map(|row| {
                            sidebar::resolved_token_spans(
                                row,
                                icon,
                                Style::default(),
                                Style::default(),
                                Style::default(),
                                Style::default(),
                                &palette,
                                width,
                            )
                            .iter()
                            .map(|span| span.content.as_ref())
                            .collect::<String>()
                        })
                        .collect();
                    for line in &lines {
                        assert!(
                            crate::ui::text::display_width(line) <= width,
                            "{candidate_name}/{}/{width}: {line:?} exceeds declared width",
                            case.name
                        );
                    }
                    println!(
                        "TOPBAR|{candidate_name}|{}|{width}|{}",
                        case.name,
                        lines.join(" \u{23ce} ")
                    );
                }
            }
        }
    }

    #[test]
    fn topbar_missing_tab_and_pane_elide_from_full_context_row() {
        let (_, rows_config) = candidates()
            .into_iter()
            .find(|(name, _)| *name == "full-context")
            .expect("full-context candidate defined above");
        let representative = cases().remove(0);
        let missing = cases().remove(2);

        let representative_rows = sidebar::tokens::agent_rows_from(
            &rows_config,
            &representative.entry,
            representative.state_text,
        );
        let missing_rows =
            sidebar::tokens::agent_rows_from(&rows_config, &missing.entry, missing.state_text);

        assert_eq!(
            representative_rows[0].len(),
            3,
            "workspace, tab, and pane all resolve when populated"
        );
        assert_eq!(
            missing_rows.len(),
            1,
            "the workspace/tab/pane row survives on workspace alone; the agent-only \
             second row elides entirely without an agent"
        );
        assert_eq!(
            missing_rows[0].len(),
            1,
            "tab and pane elide individually inside the row when absent"
        );
    }
}
