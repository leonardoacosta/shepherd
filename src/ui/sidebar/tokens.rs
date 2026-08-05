use super::AgentPanelEntry;
use crate::config::{
    AgentSidebarToken, AgentsSidebarConfig, SidebarTokenStyle, SpaceSidebarToken,
    SpacesSidebarConfig,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::ui) struct ResolvedToken {
    pub kind: ResolvedTokenKind,
    pub style: SidebarTokenStyle,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::ui) enum ResolvedTokenKind {
    StateIcon,
    StateText(String),
    Workspace(String),
    Tab(String),
    Pane(String),
    Agent(String),
    TerminalTitle(String),
    Branch(String),
    GitStatus { ahead: usize, behind: usize },
    Custom(String),
}

impl ResolvedToken {
    fn new(kind: ResolvedTokenKind, style: SidebarTokenStyle) -> Self {
        Self { kind, style }
    }

    #[cfg(test)]
    pub(super) fn unstyled(kind: ResolvedTokenKind) -> Self {
        Self::new(kind, SidebarTokenStyle::default())
    }
}

pub(super) fn agent_rows(
    config: &AgentsSidebarConfig,
    entry: &AgentPanelEntry,
    state_text: &str,
) -> Vec<Vec<ResolvedToken>> {
    agent_rows_from(config.rows_for_agent(entry.agent), entry, state_text)
}

pub(in crate::ui) fn agent_rows_from(
    rows: &[Vec<AgentSidebarToken>],
    entry: &AgentPanelEntry,
    state_text: &str,
) -> Vec<Vec<ResolvedToken>> {
    rows.iter()
        .filter_map(|row| {
            let resolved = row
                .iter()
                .filter_map(|configured| {
                    let (token, style) = configured.parts();
                    let kind = match token {
                        AgentSidebarToken::StateIcon => Some(ResolvedTokenKind::StateIcon),
                        AgentSidebarToken::StateText => {
                            Some(ResolvedTokenKind::StateText(state_text.to_string()))
                        }
                        AgentSidebarToken::Workspace => {
                            Some(ResolvedTokenKind::Workspace(entry.primary_label.clone()))
                        }
                        AgentSidebarToken::Tab => {
                            entry.primary_tab_label.clone().map(ResolvedTokenKind::Tab)
                        }
                        AgentSidebarToken::Pane => {
                            entry.pane_label.clone().map(ResolvedTokenKind::Pane)
                        }
                        AgentSidebarToken::Agent => {
                            entry.agent_label.clone().map(ResolvedTokenKind::Agent)
                        }
                        AgentSidebarToken::TerminalTitle => entry
                            .terminal_title
                            .clone()
                            .map(ResolvedTokenKind::TerminalTitle),
                        AgentSidebarToken::TerminalTitleStripped => entry
                            .terminal_title_stripped
                            .clone()
                            .map(ResolvedTokenKind::TerminalTitle),
                        // Rendered as Custom so the value inherits the existing styling,
                        // separator, and elision rules instead of gaining its own path.
                        AgentSidebarToken::Spend => entry
                            .session_status
                            .spend
                            .map(crate::workspace::render_spend_status)
                            .map(ResolvedTokenKind::Custom),
                        AgentSidebarToken::Custom(name) => entry
                            .tokens
                            .get(name)
                            .cloned()
                            .map(ResolvedTokenKind::Custom),
                        AgentSidebarToken::Styled { .. } => None,
                    }?;
                    Some(ResolvedToken::new(kind, style))
                })
                .collect::<Vec<_>>();
            (!resolved.is_empty()).then_some(resolved)
        })
        .collect()
}

pub(super) struct SpaceTokenContext<'a> {
    pub workspace: &'a str,
    pub branch: Option<&'a str>,
    pub state_text: &'a str,
    pub ahead_behind: Option<(usize, usize)>,
    pub tokens: &'a std::collections::HashMap<String, String>,
    pub suppress_git_details: bool,
    /// Per-checkout work-queue counts. Each half is independently absent, so one
    /// unavailable provider elides only its own token.
    pub project_status: crate::workspace::ProjectStatusSnapshot,
}

pub(super) fn space_rows(
    config: &SpacesSidebarConfig,
    context: SpaceTokenContext<'_>,
) -> Vec<Vec<ResolvedToken>> {
    config
        .rows
        .iter()
        .filter_map(|row| {
            let resolved = row
                .iter()
                .filter_map(|configured| {
                    let (token, style) = configured.parts();
                    let kind = match token {
                        SpaceSidebarToken::StateIcon => Some(ResolvedTokenKind::StateIcon),
                        SpaceSidebarToken::StateText => {
                            Some(ResolvedTokenKind::StateText(context.state_text.to_string()))
                        }
                        SpaceSidebarToken::Workspace => {
                            Some(ResolvedTokenKind::Workspace(context.workspace.to_string()))
                        }
                        SpaceSidebarToken::Branch if !context.suppress_git_details => context
                            .branch
                            .map(|branch| ResolvedTokenKind::Branch(branch.to_string())),
                        SpaceSidebarToken::Branch => None,
                        SpaceSidebarToken::GitStatus if !context.suppress_git_details => context
                            .ahead_behind
                            .filter(|(ahead, behind)| *ahead > 0 || *behind > 0)
                            .map(|(ahead, behind)| ResolvedTokenKind::GitStatus { ahead, behind }),
                        SpaceSidebarToken::GitStatus => None,
                        // Rendered as Custom so the counts inherit the existing styling,
                        // separator, and elision rules instead of gaining their own path.
                        SpaceSidebarToken::Proposals => context
                            .project_status
                            .proposals
                            .map(crate::workspace::render_proposal_counts)
                            .map(ResolvedTokenKind::Custom),
                        SpaceSidebarToken::Beads => context
                            .project_status
                            .beads
                            .map(crate::workspace::render_bead_counts)
                            .map(ResolvedTokenKind::Custom),
                        SpaceSidebarToken::Custom(name) => context
                            .tokens
                            .get(name)
                            .cloned()
                            .map(ResolvedTokenKind::Custom),
                        SpaceSidebarToken::Styled { .. } => None,
                    }?;
                    Some(ResolvedToken::new(kind, style))
                })
                .collect::<Vec<_>>();
            (!resolved.is_empty()).then_some(resolved)
        })
        .collect()
}

pub(super) fn separator(previous: &ResolvedToken, current: &ResolvedToken) -> &'static str {
    if matches!(previous.kind, ResolvedTokenKind::StateIcon)
        || matches!(current.kind, ResolvedTokenKind::GitStatus { .. })
    {
        " "
    } else {
        " · "
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{AgentSidebarToken, SpaceSidebarToken};
    use crate::detect::AgentState;

    fn entry() -> AgentPanelEntry {
        AgentPanelEntry {
            ws_idx: 0,
            tab_idx: 0,
            pane_id: crate::layout::PaneId::from_raw(1),
            primary_label: "repo".into(),
            primary_tab_label: None,
            pane_label: None,
            terminal_title: None,
            terminal_title_stripped: None,
            agent_label: Some("pi".into()),
            agent_kind_label: Some("pi".into()),
            agent: Some(crate::detect::Agent::Pi),
            state: AgentState::Working,
            seen: true,
            last_agent_state_change_seq: None,
            state_labels: std::collections::HashMap::new(),
            tokens: std::collections::HashMap::new(),
            session_status: Default::default(),
        }
    }

    #[test]
    fn spend_resolves_from_the_store_and_elides_when_absent() {
        let config = AgentsSidebarConfig {
            rows: vec![vec![AgentSidebarToken::Workspace, AgentSidebarToken::Spend]],
            ..Default::default()
        };

        let mut resolved = entry();
        resolved.session_status = crate::workspace::SessionStatusSnapshot {
            spend: Some(crate::workspace::SpendStatus { cents: 1_021_070 }),
        };
        assert_eq!(
            agent_rows(&config, &resolved, "working"),
            vec![vec![
                ResolvedToken::unstyled(ResolvedTokenKind::Workspace("repo".into())),
                ResolvedToken::unstyled(ResolvedTokenKind::Custom("$10210.70".into())),
            ]],
            "a resolved value renders beside its row-mates"
        );

        // Absent is the degraded state for every adapter failure class.
        let absent = entry();
        assert_eq!(
            agent_rows(&config, &absent, "working"),
            vec![vec![ResolvedToken::unstyled(ResolvedTokenKind::Workspace(
                "repo".into()
            ))]],
            "an absent value elides its own token and leaves the row otherwise intact"
        );
    }

    #[test]
    fn a_row_of_only_absent_spend_is_omitted_entirely() {
        let config = AgentsSidebarConfig {
            rows: vec![vec![AgentSidebarToken::Spend]],
            ..Default::default()
        };
        assert!(
            agent_rows(&config, &entry(), "working").is_empty(),
            "a fully unresolved row takes no space"
        );
    }

    #[test]
    fn missing_custom_tokens_elide_rows_and_separators() {
        let entry = entry();
        let config = AgentsSidebarConfig {
            rows: vec![
                vec![
                    AgentSidebarToken::StateIcon,
                    AgentSidebarToken::Custom("missing".into()),
                ],
                vec![AgentSidebarToken::Custom("missing".into())],
                vec![AgentSidebarToken::Agent],
            ],
            ..Default::default()
        };

        let rows = agent_rows(&config, &entry, "working");

        assert_eq!(rows.len(), 2);
        assert_eq!(
            rows[0],
            vec![ResolvedToken::unstyled(ResolvedTokenKind::StateIcon)]
        );
        assert_eq!(
            rows[1],
            vec![ResolvedToken::unstyled(ResolvedTokenKind::Agent(
                "pi".into()
            ))]
        );
    }

    #[test]
    fn state_text_and_arbitrary_values_are_independent_tokens() {
        let mut entry = entry();
        entry
            .tokens
            .insert("summary".into(), "reviewing auth".into());
        let config = AgentsSidebarConfig {
            rows: vec![vec![
                AgentSidebarToken::StateText,
                AgentSidebarToken::Custom("summary".into()),
            ]],
            ..Default::default()
        };

        assert_eq!(
            agent_rows(&config, &entry, "deep in the mines"),
            vec![vec![
                ResolvedToken::unstyled(ResolvedTokenKind::StateText("deep in the mines".into())),
                ResolvedToken::unstyled(ResolvedTokenKind::Custom("reviewing auth".into())),
            ]]
        );
    }

    #[test]
    fn terminal_title_builtins_are_distinct_from_custom_tokens() {
        let mut entry = entry();
        entry.terminal_title = Some("⠋ raw title".into());
        entry.terminal_title_stripped = Some("raw title".into());
        entry
            .tokens
            .insert("terminal_title".into(), "custom title".into());
        let config = AgentsSidebarConfig {
            rows: vec![vec![
                AgentSidebarToken::TerminalTitle,
                AgentSidebarToken::TerminalTitleStripped,
                AgentSidebarToken::Custom("terminal_title".into()),
            ]],
            ..Default::default()
        };

        assert_eq!(
            agent_rows(&config, &entry, "working"),
            vec![vec![
                ResolvedToken::unstyled(ResolvedTokenKind::TerminalTitle("⠋ raw title".into())),
                ResolvedToken::unstyled(ResolvedTokenKind::TerminalTitle("raw title".into())),
                ResolvedToken::unstyled(ResolvedTokenKind::Custom("custom title".into())),
            ]]
        );
    }

    #[test]
    fn known_agent_override_replaces_default_rows() {
        let mut config = AgentsSidebarConfig {
            rows: vec![vec![AgentSidebarToken::Workspace]],
            ..Default::default()
        };
        config
            .rows_by_agent
            .insert("pi".into(), vec![vec![AgentSidebarToken::Agent]]);
        let mut pi = entry();
        pi.agent_label = Some("renamed pi".into());

        assert_eq!(
            agent_rows(&config, &pi, "working"),
            vec![vec![ResolvedToken::unstyled(ResolvedTokenKind::Agent(
                "renamed pi".into()
            ))]]
        );

        pi.agent = None;
        assert_eq!(
            agent_rows(&config, &pi, "working"),
            vec![vec![ResolvedToken::unstyled(ResolvedTokenKind::Workspace(
                "repo".into()
            ))]]
        );
    }

    #[test]
    fn grouped_children_suppress_all_builtin_git_details() {
        let config = SpacesSidebarConfig::default();

        assert_eq!(
            space_rows(
                &config,
                SpaceTokenContext {
                    workspace: "feature",
                    branch: Some("worktree/feature"),
                    state_text: "idle",
                    ahead_behind: Some((2, 1)),
                    tokens: &std::collections::HashMap::new(),
                    suppress_git_details: true,
                    project_status: Default::default(),
                },
            ),
            vec![vec![
                ResolvedToken::unstyled(ResolvedTokenKind::StateIcon),
                ResolvedToken::unstyled(ResolvedTokenKind::Workspace("feature".into())),
            ]]
        );
    }

    #[test]
    fn workspace_custom_token_can_replace_git_specific_details() {
        let tokens = std::collections::HashMap::from([("jj_status".into(), "2 changes".into())]);
        let config = SpacesSidebarConfig {
            rows: vec![vec![SpaceSidebarToken::Custom("jj_status".into())]],
            ..Default::default()
        };

        assert_eq!(
            space_rows(
                &config,
                SpaceTokenContext {
                    workspace: "repo",
                    branch: None,
                    state_text: "idle",
                    ahead_behind: None,
                    tokens: &tokens,
                    suppress_git_details: false,
                    project_status: Default::default(),
                },
            ),
            vec![vec![ResolvedToken::unstyled(ResolvedTokenKind::Custom(
                "2 changes".into()
            ))]]
        );
    }

    fn project_status(
        proposals: Option<(usize, usize)>,
        beads: Option<(usize, usize, usize)>,
    ) -> crate::workspace::ProjectStatusSnapshot {
        crate::workspace::ProjectStatusSnapshot {
            proposals: proposals
                .map(|(open, in_progress)| crate::workspace::ProposalCounts { open, in_progress }),
            beads: beads.map(|(open, ready, blocked)| crate::workspace::BeadCounts {
                open,
                ready,
                blocked,
            }),
        }
    }

    fn project_status_rows(
        snapshot: crate::workspace::ProjectStatusSnapshot,
        rows: Vec<Vec<SpaceSidebarToken>>,
    ) -> Vec<Vec<ResolvedToken>> {
        space_rows(
            &SpacesSidebarConfig {
                rows,
                ..Default::default()
            },
            SpaceTokenContext {
                workspace: "repo",
                branch: Some("dev"),
                state_text: "idle",
                ahead_behind: Some((2, 0)),
                tokens: &std::collections::HashMap::new(),
                suppress_git_details: false,
                project_status: snapshot,
            },
        )
    }

    #[test]
    fn both_project_tokens_resolve_when_both_halves_are_present() {
        let resolved = project_status_rows(
            project_status(Some((2, 1)), Some((3, 1, 0))),
            vec![vec![SpaceSidebarToken::Proposals, SpaceSidebarToken::Beads]],
        );

        assert_eq!(
            resolved,
            vec![vec![
                ResolvedToken::unstyled(ResolvedTokenKind::Custom("op: 2o 1ip".into())),
                ResolvedToken::unstyled(ResolvedTokenKind::Custom("bd: 3o 1r 0b".into())),
            ]]
        );
    }

    #[test]
    fn an_absent_half_elides_only_its_own_token() {
        let resolved = project_status_rows(
            project_status(Some((2, 1)), None),
            vec![vec![
                SpaceSidebarToken::Proposals,
                SpaceSidebarToken::Beads,
                SpaceSidebarToken::GitStatus,
            ]],
        );

        assert_eq!(
            resolved,
            vec![vec![
                ResolvedToken::unstyled(ResolvedTokenKind::Custom("op: 2o 1ip".into())),
                ResolvedToken::unstyled(ResolvedTokenKind::GitStatus {
                    ahead: 2,
                    behind: 0
                }),
            ]],
            "the missing beads token drops out; git tokens are untouched"
        );
    }

    #[test]
    fn a_row_of_only_absent_project_tokens_is_omitted() {
        let resolved = project_status_rows(
            project_status(None, None),
            vec![
                vec![SpaceSidebarToken::Workspace],
                vec![SpaceSidebarToken::Proposals, SpaceSidebarToken::Beads],
            ],
        );

        assert_eq!(
            resolved,
            vec![vec![ResolvedToken::unstyled(ResolvedTokenKind::Workspace(
                "repo".into()
            ))]],
            "a fully unresolved row takes no space"
        );
    }

    #[test]
    fn project_tokens_carry_configured_inline_styles() {
        let style = SidebarTokenStyle {
            bold: Some(true),
            ..Default::default()
        };
        let resolved = project_status_rows(
            project_status(Some((1, 0)), None),
            vec![vec![SpaceSidebarToken::Styled {
                token: Box::new(SpaceSidebarToken::Proposals),
                style,
            }]],
        );

        assert_eq!(resolved.len(), 1);
        assert_eq!(resolved[0][0].style.bold, Some(true));
    }
}

/// Rendered-evidence characterization for `research-presentation-layout-presets`.
/// Renders named candidate row layouts through production token resolution
/// (`agent_rows_from` / `space_rows`) and production cell rendering
/// (`sidebar::resolved_token_spans`) at the widths declared in the change's
/// `evidence/layouts.md`. Not a runtime behavior change — read-only drift
/// characterization the terminal user gate consults.
#[cfg(test)]
mod presentation_layout_candidate {
    use super::*;
    use crate::config::AgentSidebarToken;
    use crate::detect::AgentState;
    use ratatui::style::Style;

    fn agent_entry(
        primary_label: &str,
        tab: Option<&str>,
        pane: Option<&str>,
        agent_label: Option<&str>,
        terminal_title: Option<&str>,
        terminal_title_stripped: Option<&str>,
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
            terminal_title: terminal_title.map(str::to_string),
            terminal_title_stripped: terminal_title_stripped.map(str::to_string),
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

    /// Renders each resolved row through the production span renderer and
    /// flattens spans to plain text, mirroring how `render_agent_detail` /
    /// `render_topbar` compose a row's visible content.
    fn render_rows(
        resolved: &[Vec<ResolvedToken>],
        state_icon: (&str, Style),
        width: usize,
        palette: &crate::app::state::Palette,
    ) -> Vec<String> {
        resolved
            .iter()
            .map(|row| {
                super::super::resolved_token_spans(
                    row,
                    state_icon,
                    Style::default(),
                    Style::default(),
                    Style::default(),
                    Style::default(),
                    palette,
                    width,
                )
                .iter()
                .map(|span| span.content.as_ref())
                .collect::<String>()
            })
            .collect()
    }

    struct AgentCase {
        name: &'static str,
        entry: AgentPanelEntry,
        state_text: &'static str,
    }

    fn agent_cases() -> Vec<AgentCase> {
        vec![
            AgentCase {
                name: "representative",
                entry: agent_entry(
                    "shepherd",
                    Some("feature-auth"),
                    Some("review pane"),
                    Some("claude"),
                    Some("\u{280b} building"),
                    Some("building"),
                    Some(crate::detect::Agent::Claude),
                    AgentState::Working,
                    true,
                ),
                state_text: "working",
            },
            AgentCase {
                name: "long",
                entry: agent_entry(
                    "backend-services-payment-orchestration-monorepo",
                    Some("refactor-billing-reconciliation-pipeline"),
                    Some("review pane for the billing reconciliation output"),
                    Some("claude-opus-5-sonnet-reasoning"),
                    Some("\u{280b} 修复用户认证模块并迁移到统一登录服务超长标题"),
                    Some("修复用户认证模块并迁移到统一登录服务超长标题"),
                    Some(crate::detect::Agent::Claude),
                    AgentState::Blocked,
                    true,
                ),
                state_text: "blocked",
            },
            AgentCase {
                name: "missing",
                entry: agent_entry(
                    "solo-workspace",
                    None,
                    None,
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

    fn agent_candidates() -> Vec<(&'static str, Vec<Vec<AgentSidebarToken>>)> {
        vec![
            (
                "baseline-compact",
                vec![
                    vec![
                        AgentSidebarToken::StateIcon,
                        AgentSidebarToken::Workspace,
                        AgentSidebarToken::Tab,
                    ],
                    vec![AgentSidebarToken::Agent],
                ],
            ),
            (
                "single-line",
                vec![vec![
                    AgentSidebarToken::StateIcon,
                    AgentSidebarToken::Workspace,
                    AgentSidebarToken::Agent,
                ]],
            ),
            (
                "status-first",
                vec![
                    vec![AgentSidebarToken::StateIcon, AgentSidebarToken::StateText],
                    vec![AgentSidebarToken::Workspace, AgentSidebarToken::Tab],
                    vec![AgentSidebarToken::Agent],
                ],
            ),
            (
                "terminal-title",
                vec![
                    vec![
                        AgentSidebarToken::StateIcon,
                        AgentSidebarToken::TerminalTitleStripped,
                    ],
                    vec![AgentSidebarToken::Agent],
                ],
            ),
            (
                "full-context",
                vec![
                    vec![
                        AgentSidebarToken::StateIcon,
                        AgentSidebarToken::Workspace,
                        AgentSidebarToken::Tab,
                    ],
                    vec![AgentSidebarToken::Pane, AgentSidebarToken::Agent],
                ],
            ),
        ]
    }

    #[test]
    fn agent_candidates_render_within_declared_sidebar_widths() {
        let palette = crate::app::AppState::test_new().palette;
        let widths = [18usize, 24, 36];

        for (candidate_name, rows_config) in agent_candidates() {
            for case in agent_cases() {
                let resolved = agent_rows_from(&rows_config, &case.entry, case.state_text);
                let state_icon =
                    crate::ui::status::state_dot(case.entry.state, case.entry.seen, &palette);
                for width in widths {
                    let lines = render_rows(&resolved, state_icon, width, &palette);
                    for line in &lines {
                        assert!(
                            crate::ui::text::display_width(line) <= width,
                            "{candidate_name}/{}/{width}: {line:?} exceeds declared width",
                            case.name
                        );
                    }
                    println!(
                        "AGENT|{candidate_name}|{}|{width}|{}",
                        case.name,
                        lines.join(" \u{23ce} ")
                    );
                }
            }
        }
    }

    #[test]
    fn agent_missing_fields_elide_rows_and_reduce_row_count_versus_representative() {
        let palette = crate::app::AppState::test_new().palette;
        let (_, rows_config) = agent_candidates()
            .into_iter()
            .find(|(name, _)| *name == "full-context")
            .expect("full-context candidate defined above");
        let representative = agent_cases().remove(0);
        let missing = agent_cases().remove(2);

        let representative_rows = agent_rows_from(
            &rows_config,
            &representative.entry,
            representative.state_text,
        );
        let missing_rows = agent_rows_from(&rows_config, &missing.entry, missing.state_text);

        assert_eq!(
            representative_rows.len(),
            2,
            "pane+agent row survives when populated"
        );
        assert_eq!(
            missing_rows.len(),
            1,
            "pane+agent row elides entirely when both tokens are absent, not just blanked"
        );
        let state_icon =
            crate::ui::status::state_dot(missing.entry.state, missing.entry.seen, &palette);
        let lines = render_rows(&missing_rows, state_icon, 24, &palette);
        assert!(
            lines
                .iter()
                .all(|line| !line.contains("Ν/A") && !line.contains("None")),
            "elided tokens never render sentinel text: {lines:?}"
        );
    }

    #[test]
    fn agent_long_values_truncate_with_ellipsis_at_narrow_widths() {
        let palette = crate::app::AppState::test_new().palette;
        let (_, rows_config) = agent_candidates()
            .into_iter()
            .find(|(name, _)| *name == "baseline-compact")
            .expect("baseline-compact candidate defined above");
        let long = agent_cases().remove(1);
        let resolved = agent_rows_from(&rows_config, &long.entry, long.state_text);
        let state_icon = crate::ui::status::state_dot(long.entry.state, long.entry.seen, &palette);

        let lines = render_rows(&resolved, state_icon, 18, &palette);
        assert!(
            lines.iter().any(|line| line.contains('\u{2026}')),
            "long workspace/tab values truncate with an ellipsis at 18 cols: {lines:?}"
        );
        for line in &lines {
            assert!(crate::ui::text::display_width(line) <= 18);
        }
    }

    struct SpaceCase {
        name: &'static str,
        workspace: &'static str,
        branch: Option<&'static str>,
        ahead_behind: Option<(usize, usize)>,
        state_text: &'static str,
    }

    fn space_cases() -> Vec<SpaceCase> {
        vec![
            SpaceCase {
                name: "representative",
                workspace: "shepherd",
                branch: Some("main"),
                ahead_behind: Some((1, 0)),
                state_text: "idle",
            },
            SpaceCase {
                name: "long",
                workspace: "backend-services-payment-orchestration-monorepo",
                branch: Some("refactor/billing-reconciliation-pipeline-cleanup"),
                ahead_behind: Some((12, 34)),
                state_text: "working",
            },
            SpaceCase {
                name: "missing",
                workspace: "solo-workspace",
                branch: None,
                ahead_behind: None,
                state_text: "idle",
            },
        ]
    }

    fn space_candidates() -> Vec<(&'static str, Vec<Vec<SpaceSidebarToken>>)> {
        vec![
            (
                "baseline",
                vec![
                    vec![SpaceSidebarToken::StateIcon, SpaceSidebarToken::Workspace],
                    vec![SpaceSidebarToken::Branch, SpaceSidebarToken::GitStatus],
                ],
            ),
            (
                "single-line",
                vec![vec![
                    SpaceSidebarToken::StateIcon,
                    SpaceSidebarToken::Workspace,
                    SpaceSidebarToken::Branch,
                ]],
            ),
            (
                "status-first",
                vec![
                    vec![SpaceSidebarToken::StateIcon, SpaceSidebarToken::StateText],
                    vec![SpaceSidebarToken::Workspace],
                    vec![SpaceSidebarToken::Branch, SpaceSidebarToken::GitStatus],
                ],
            ),
            (
                "branch-only",
                vec![
                    vec![SpaceSidebarToken::StateIcon, SpaceSidebarToken::Workspace],
                    vec![SpaceSidebarToken::Branch],
                ],
            ),
        ]
    }

    #[test]
    fn space_candidates_render_within_declared_sidebar_widths() {
        let palette = crate::app::AppState::test_new().palette;
        let widths = [18usize, 24, 36];
        let empty_tokens = std::collections::HashMap::new();
        let state_icon = crate::ui::status::state_dot(AgentState::Idle, true, &palette);

        for (candidate_name, rows_config) in space_candidates() {
            let config = SpacesSidebarConfig {
                rows: rows_config,
                row_gap: 0,
            };
            for case in space_cases() {
                let resolved = space_rows(
                    &config,
                    SpaceTokenContext {
                        workspace: case.workspace,
                        branch: case.branch,
                        state_text: case.state_text,
                        ahead_behind: case.ahead_behind,
                        tokens: &empty_tokens,
                        suppress_git_details: false,
                        project_status: Default::default(),
                    },
                );
                for width in widths {
                    let lines = render_rows(&resolved, state_icon, width, &palette);
                    for line in &lines {
                        assert!(
                            crate::ui::text::display_width(line) <= width,
                            "{candidate_name}/{}/{width}: {line:?} exceeds declared width",
                            case.name
                        );
                    }
                    println!(
                        "SPACE|{candidate_name}|{}|{width}|{}",
                        case.name,
                        lines.join(" \u{23ce} ")
                    );
                }
            }
        }
    }

    #[test]
    fn space_zero_ahead_behind_elides_git_status_but_missing_branch_elides_row() {
        let palette = crate::app::AppState::test_new().palette;
        let empty_tokens = std::collections::HashMap::new();
        let config = SpacesSidebarConfig {
            rows: vec![
                vec![SpaceSidebarToken::StateIcon, SpaceSidebarToken::Workspace],
                vec![SpaceSidebarToken::Branch, SpaceSidebarToken::GitStatus],
            ],
            row_gap: 0,
        };

        let zero_ahead_behind = space_rows(
            &config,
            SpaceTokenContext {
                workspace: "repo",
                branch: Some("main"),
                state_text: "idle",
                ahead_behind: Some((0, 0)),
                tokens: &empty_tokens,
                suppress_git_details: false,
                project_status: Default::default(),
            },
        );
        assert_eq!(
            zero_ahead_behind.len(),
            2,
            "branch alone keeps the second row"
        );
        assert_eq!(
            zero_ahead_behind[1].len(),
            1,
            "zero ahead/behind elides only git_status"
        );

        let missing_branch = space_rows(
            &config,
            SpaceTokenContext {
                workspace: "repo",
                branch: None,
                state_text: "idle",
                ahead_behind: Some((2, 1)),
                tokens: &empty_tokens,
                suppress_git_details: false,
                project_status: Default::default(),
            },
        );
        assert_eq!(
            missing_branch.len(),
            2,
            "git_status alone still keeps the second row even without a branch"
        );
        assert_eq!(missing_branch[1].len(), 1);

        let _ = palette;
    }
}
