pub(super) mod tokens;

use std::borrow::Cow;

use ratatui::{
    layout::{Alignment, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use self::tokens::{ResolvedToken, ResolvedTokenKind, SpaceTokenContext};
use super::scrollbar::{render_scrollbar, should_show_scrollbar};
use super::status::{state_dot, state_label, state_label_color};
use super::text::{display_width, display_width_u16, truncate_end};
use crate::app::state::{AgentPanelSort, Palette};
use crate::app::{AppState, Mode};
use crate::detect::AgentState;
use crate::terminal::TerminalRuntimeRegistry;

const WORKSPACE_SECTION_HEADER_ROWS: u16 = 2;

#[derive(Clone)]
pub(crate) struct AgentPanelEntry {
    pub ws_idx: usize,
    pub tab_idx: usize,
    pub pane_id: crate::layout::PaneId,
    pub primary_label: String,
    pub primary_tab_label: Option<String>,
    pub pane_label: Option<String>,
    pub terminal_title: Option<String>,
    pub terminal_title_stripped: Option<String>,
    pub agent_label: Option<String>,
    pub agent_kind_label: Option<String>,
    pub agent: Option<crate::detect::Agent>,
    pub state: AgentState,
    pub seen: bool,
    pub last_agent_state_change_seq: Option<u64>,
    pub state_labels: std::collections::HashMap<String, String>,
    pub tokens: std::collections::HashMap<String, String>,
}

/// `sort: grouped`/`sort: priority`, so the header identifies itself as a
/// sort control instead of a bare value. Shares `AgentPanelSort::label()`
/// with the Settings Display row (`AgentSort`) so both surfaces can never
/// drift on wording.
fn agent_panel_sort_label(sort: AgentPanelSort) -> String {
    format!("sort: {}", sort.label())
}

pub(crate) fn agent_panel_toggle_rect(area: Rect, sort: AgentPanelSort) -> Rect {
    agent_panel_header_label_rect(area, &agent_panel_sort_label(sort))
}

/// Right half of the merged list's single header row; the left half carries
/// the `spaces` caption.
fn agent_panel_header_label_rect(area: Rect, label: &str) -> Rect {
    if area.width == 0 || area.height == 0 {
        return Rect::default();
    }

    let width = display_width_u16(label).min(area.width);
    Rect::new(area.x + area.width.saturating_sub(width), area.y, width, 1)
}

fn active_agent_view_label(app: &AppState) -> Option<&str> {
    app.agent_view_override
        .as_ref()
        .map(|view| view.label.as_deref().unwrap_or("filtered"))
}

pub(crate) fn agent_panel_entries(app: &AppState) -> Vec<AgentPanelEntry> {
    agent_panel_entries_with_runtimes(app, None)
}

pub(crate) fn all_agent_panel_entries(app: &AppState) -> Vec<AgentPanelEntry> {
    collect_agent_panel_entries_with_runtimes(app, None)
}

pub(crate) fn agent_panel_entries_from(
    app: &AppState,
    terminal_runtimes: &TerminalRuntimeRegistry,
) -> Vec<AgentPanelEntry> {
    agent_panel_entries_with_runtimes(app, Some(terminal_runtimes))
}

fn agent_panel_entries_with_runtimes(
    app: &AppState,
    terminal_runtimes: Option<&TerminalRuntimeRegistry>,
) -> Vec<AgentPanelEntry> {
    let mut entries = collect_agent_panel_entries_with_runtimes(app, terminal_runtimes);
    crate::app::agent_view::apply_agent_view(app, &mut entries);
    entries
}

fn collect_agent_panel_entries_with_runtimes(
    app: &AppState,
    terminal_runtimes: Option<&TerminalRuntimeRegistry>,
) -> Vec<AgentPanelEntry> {
    let empty_runtimes;
    let terminal_runtimes = match terminal_runtimes {
        Some(terminal_runtimes) => terminal_runtimes,
        None => {
            empty_runtimes = TerminalRuntimeRegistry::new();
            &empty_runtimes
        }
    };

    app.workspaces
        .iter()
        .enumerate()
        .flat_map(|(ws_idx, ws)| {
            let dock_tab_idx = app.dock_backing_tab_idx(ws_idx);
            let multi_tab = app.visible_tab_indices(ws_idx).len() > 1;
            let workspace_label = ws.display_name_from(&app.terminals, terminal_runtimes);
            ws.pane_details(&app.terminals)
                .into_iter()
                .filter(move |detail| Some(detail.tab_idx) != dock_tab_idx)
                .map(move |detail| {
                    let show_tab = multi_tab
                        || ws
                            .tabs
                            .get(detail.tab_idx)
                            .is_some_and(|tab| !tab.is_auto_named());
                    AgentPanelEntry {
                        ws_idx,
                        tab_idx: detail.tab_idx,
                        pane_id: detail.pane_id,
                        primary_label: workspace_label.clone(),
                        primary_tab_label: show_tab.then_some(detail.tab_label),
                        pane_label: detail.pane_label,
                        terminal_title: detail.terminal_title,
                        terminal_title_stripped: detail.terminal_title_stripped,
                        agent_label: Some(detail.agent_label),
                        agent_kind_label: detail.agent_kind_label,
                        agent: detail.agent,
                        state: detail.state,
                        seen: detail.seen,
                        last_agent_state_change_seq: detail.last_agent_state_change_seq,
                        state_labels: detail.state_labels,
                        tokens: detail.tokens,
                    }
                })
        })
        .collect()
}

pub(super) fn agent_panel_status_key(state: AgentState, seen: bool) -> &'static str {
    match (state, seen) {
        (AgentState::Idle, false) => "done",
        (AgentState::Idle, true) => "idle",
        (AgentState::Working, _) => "working",
        (AgentState::Blocked, _) => "blocked",
        (AgentState::Unknown, _) => "unknown",
    }
}

fn workspace_row_height(app: &AppState, ws: &crate::workspace::Workspace, indented: bool) -> u16 {
    let (state, seen) = ws.aggregate_state(&app.terminals);
    let label = if indented {
        grouped_child_display_label(
            &ws.display_name_from_terminals(&app.terminals),
            ws.branch().as_deref(),
            ws.custom_name.is_some(),
        )
    } else {
        ws.display_name_from_terminals(&app.terminals)
    };
    let token_values = ws.metadata_tokens.values();
    tokens::space_rows(
        &app.sidebar_spaces,
        SpaceTokenContext {
            workspace: &label,
            branch: ws.branch().as_deref(),
            state_text: state_label(state, seen),
            ahead_behind: ws.git_ahead_behind(),
            tokens: &token_values,
            suppress_git_details: indented,
            project_status: ws.project_status(),
        },
    )
    .len()
    .max(1)
    .min(u16::MAX as usize) as u16
}

fn workspace_row_height_in_body(
    app: &AppState,
    workspace: &crate::workspace::Workspace,
    indented: bool,
    body_height: u16,
) -> u16 {
    workspace_row_height(app, workspace, indented).min(body_height)
}

fn workspace_attention_priority(state: AgentState, seen: bool) -> u8 {
    match (state, seen) {
        (AgentState::Blocked, _) => 4,
        (AgentState::Idle, false) => 3,
        (AgentState::Working, _) => 2,
        (AgentState::Idle, true) => 1,
        (AgentState::Unknown, _) => 0,
    }
}

fn space_aggregate_state(app: &AppState, key: &str) -> (AgentState, bool) {
    app.workspaces
        .iter()
        .filter(|ws| ws.worktree_space().is_some_and(|space| space.key == key))
        .map(|ws| ws.aggregate_state(&app.terminals))
        .max_by_key(|(state, seen)| workspace_attention_priority(*state, *seen))
        .unwrap_or((AgentState::Unknown, true))
}

pub(crate) fn workspace_parent_group_state(
    app: &AppState,
    ws_idx: usize,
) -> Option<(String, bool)> {
    let space = app.workspaces.get(ws_idx)?.worktree_space()?;
    if space.is_linked_worktree {
        return None;
    }
    let member_count = app
        .workspaces
        .iter()
        .filter(|ws| {
            ws.worktree_space()
                .is_some_and(|member| member.key == space.key)
        })
        .count();
    (member_count >= 2).then(|| {
        (
            space.key.clone(),
            app.collapsed_space_keys.contains(&space.key),
        )
    })
}

pub(crate) fn grouped_child_display_label(
    label: &str,
    branch: Option<&str>,
    has_custom_name: bool,
) -> String {
    if has_custom_name {
        return label.to_string();
    }
    let Some(branch) = branch else {
        return label.to_string();
    };
    branch
        .strip_prefix("worktree/")
        .unwrap_or(branch)
        .to_string()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum WorkspaceListEntry {
    Workspace { ws_idx: usize, indented: bool },
}

pub(crate) fn next_entry_is_indented_workspace(entries: &[WorkspaceListEntry], idx: usize) -> bool {
    matches!(
        entries.get(idx.saturating_add(1)),
        Some(WorkspaceListEntry::Workspace { indented: true, .. })
    )
}

pub(crate) fn workspace_list_entries(app: &AppState) -> Vec<WorkspaceListEntry> {
    workspace_list_entries_inner(app, false)
}

/// Like [`workspace_list_entries`] but always expands worktree groups, ignoring
/// `collapsed_space_keys`. The mobile switcher has no collapse affordance and
/// always shows the full worktree tree.
pub(crate) fn workspace_list_entries_expanded(app: &AppState) -> Vec<WorkspaceListEntry> {
    workspace_list_entries_inner(app, true)
}

fn workspace_list_entries_inner(app: &AppState, force_expanded: bool) -> Vec<WorkspaceListEntry> {
    let mut members_by_key = std::collections::HashMap::<String, Vec<usize>>::new();
    for (ws_idx, ws) in app.workspaces.iter().enumerate() {
        if let Some(space) = ws.worktree_space() {
            members_by_key
                .entry(space.key.clone())
                .or_default()
                .push(ws_idx);
        }
    }
    let grouped_keys = members_by_key
        .iter()
        .filter(|(_, members)| {
            members.len() >= 2
                && members.iter().any(|idx| {
                    app.workspaces
                        .get(*idx)
                        .and_then(|ws| ws.worktree_space())
                        .is_some_and(|space| !space.is_linked_worktree)
                })
        })
        .map(|(key, _)| key.clone())
        .collect::<std::collections::HashSet<_>>();

    let visible_group_idx = if matches!(app.mode, Mode::Navigate) {
        Some(app.selected)
    } else {
        app.active
    };
    let active_group = visible_group_idx.and_then(|idx| {
        app.workspaces
            .get(idx)
            .and_then(|ws| ws.worktree_space())
            .map(|space| space.key.clone())
    });

    let mut emitted_groups = std::collections::HashSet::<String>::new();
    let mut entries = Vec::new();
    for (ws_idx, ws) in app.workspaces.iter().enumerate() {
        let Some(space) = ws
            .worktree_space()
            .filter(|space| grouped_keys.contains(&space.key))
        else {
            entries.push(WorkspaceListEntry::Workspace {
                ws_idx,
                indented: false,
            });
            continue;
        };

        if !emitted_groups.insert(space.key.clone()) {
            continue;
        }

        let Some(members) = members_by_key.get(&space.key) else {
            continue;
        };
        let Some(parent_idx) = members.iter().copied().find(|idx| {
            app.workspaces
                .get(*idx)
                .and_then(|member| member.worktree_space())
                .is_some_and(|member_space| !member_space.is_linked_worktree)
        }) else {
            entries.push(WorkspaceListEntry::Workspace {
                ws_idx,
                indented: false,
            });
            continue;
        };
        let collapsed = !force_expanded && app.collapsed_space_keys.contains(&space.key);
        entries.push(WorkspaceListEntry::Workspace {
            ws_idx: parent_idx,
            indented: false,
        });

        if collapsed {
            if let Some(active_idx) = visible_group_idx
                .filter(|idx| *idx != parent_idx)
                .filter(|_| active_group.as_deref() == Some(space.key.as_str()))
            {
                entries.push(WorkspaceListEntry::Workspace {
                    ws_idx: active_idx,
                    indented: true,
                });
            }
        } else {
            for member_idx in members {
                if *member_idx == parent_idx {
                    continue;
                }
                entries.push(WorkspaceListEntry::Workspace {
                    ws_idx: *member_idx,
                    indented: true,
                });
            }
        }
    }
    entries
}

/// One rendered row of the merged sidebar list.
///
/// Deliberately separate from [`WorkspaceListEntry`]: that enum is iterated by
/// the mobile switcher and by workspace reorder, neither of which has an agent
/// concept, so widening it would change surfaces this list does not own.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SidebarRow {
    Space {
        ws_idx: usize,
        indented: bool,
    },
    /// Position in the agent entry slice this row list was built from.
    Agent {
        entry_idx: usize,
    },
    NoAgents {
        ws_idx: usize,
    },
}

/// Space rows each followed by their own agents, in space order. No tab rows at
/// any depth — an agent row is the switch target, and its tab is implied by the
/// entry it points at.
pub(crate) fn sidebar_rows(app: &AppState, entries: &[AgentPanelEntry]) -> Vec<SidebarRow> {
    let mut rows = Vec::new();
    for WorkspaceListEntry::Workspace { ws_idx, indented } in workspace_list_entries(app) {
        rows.push(SidebarRow::Space { ws_idx, indented });

        // A collapsed worktree parent hides its whole subtree, agents included,
        // so an absent child row reads as collapse and never as "nothing here".
        if !indented
            && workspace_parent_group_state(app, ws_idx).is_some_and(|(_, collapsed)| collapsed)
        {
            continue;
        }

        let before = rows.len();
        rows.extend(
            entries
                .iter()
                .enumerate()
                .filter(|(_, agent)| agent.ws_idx == ws_idx)
                .map(|(entry_idx, _)| SidebarRow::Agent { entry_idx }),
        );
        if rows.len() == before {
            rows.push(SidebarRow::NoAgents { ws_idx });
        }
    }
    rows
}

fn sidebar_row_height_in_body(
    app: &AppState,
    row: &SidebarRow,
    entries: &[AgentPanelEntry],
    body_height: u16,
) -> u16 {
    match row {
        SidebarRow::Space { ws_idx, indented } => app
            .workspaces
            .get(*ws_idx)
            .map(|ws| workspace_row_height_in_body(app, ws, *indented, body_height))
            .unwrap_or(0),
        SidebarRow::Agent { entry_idx } => entries
            .get(*entry_idx)
            .map(|entry| agent_entry_height_in_body(app, entry, body_height))
            .unwrap_or(0),
        SidebarRow::NoAgents { .. } => 1u16.min(body_height),
    }
}

fn sidebar_row_ws_idx(row: &SidebarRow, entries: &[AgentPanelEntry]) -> Option<usize> {
    match row {
        SidebarRow::Space { ws_idx, .. } | SidebarRow::NoAgents { ws_idx } => Some(*ws_idx),
        SidebarRow::Agent { entry_idx } => entries.get(*entry_idx).map(|entry| entry.ws_idx),
    }
}

fn sidebar_row_gap(
    app: &AppState,
    rows: &[SidebarRow],
    entries: &[AgentPanelEntry],
    idx: usize,
) -> u16 {
    let Some(next) = rows.get(idx.saturating_add(1)) else {
        return 0;
    };
    let current = &rows[idx];

    // Within one space, only agent-to-agent carries a gap; everything else in
    // the block packs against the space row that owns it.
    if sidebar_row_ws_idx(current, entries) == sidebar_row_ws_idx(next, entries) {
        return match (current, next) {
            (SidebarRow::Agent { .. }, SidebarRow::Agent { .. }) => app.sidebar_agents.row_gap,
            _ => 0,
        };
    }

    // Worktree siblings stay compact against each other, as they did when the
    // space list stood alone.
    let leaving_indented_space = rows[..=idx].iter().rev().find_map(|row| match row {
        SidebarRow::Space { indented, .. } => Some(*indented),
        _ => None,
    }) == Some(true);
    if leaving_indented_space && matches!(next, SidebarRow::Space { indented: true, .. }) {
        return 0;
    }

    app.sidebar_spaces.row_gap
}

pub(crate) fn workspace_list_rect(area: Rect) -> Rect {
    // The rightmost column is the sidebar's vertical separator, not list body.
    let content = Rect::new(area.x, area.y, area.width.saturating_sub(1), area.height);
    if content.width == 0 || content.height == 0 {
        return Rect::default();
    }
    content
}

pub(crate) fn workspace_list_body_rect(area: Rect, has_scrollbar: bool) -> Rect {
    if area.width == 0 || area.height <= WORKSPACE_SECTION_HEADER_ROWS {
        return Rect::default();
    }

    let body_y = area.y.saturating_add(WORKSPACE_SECTION_HEADER_ROWS);
    let footer_y = area.y + area.height.saturating_sub(1);
    let body_height = footer_y.saturating_sub(body_y);
    let body_width = area.width.saturating_sub(u16::from(has_scrollbar));
    Rect::new(area.x, body_y, body_width, body_height)
}

fn sidebar_list_visible_count(
    app: &AppState,
    rows: &[SidebarRow],
    entries: &[AgentPanelEntry],
    area: Rect,
    scroll: usize,
) -> usize {
    let body = workspace_list_body_rect(area, false);
    if body.width == 0 || body.height == 0 {
        return 0;
    }

    let mut used_rows = 0u16;
    let mut visible = 0usize;
    for (row_idx, row) in rows.iter().enumerate().skip(scroll) {
        let row_height = sidebar_row_height_in_body(app, row, entries, body.height);
        if used_rows.saturating_add(row_height) > body.height {
            break;
        }
        used_rows = used_rows.saturating_add(row_height);
        visible += 1;
        used_rows = used_rows
            .saturating_add(sidebar_row_gap(app, rows, entries, row_idx))
            .min(body.height);
    }
    visible
}

fn sidebar_list_bottom_start(
    app: &AppState,
    rows: &[SidebarRow],
    entries: &[AgentPanelEntry],
    area: Rect,
) -> usize {
    let body = workspace_list_body_rect(area, false);
    if rows.is_empty() || body.width == 0 || body.height == 0 {
        return 0;
    }

    let mut used_rows = 0u16;
    let mut start = rows.len();
    for (row_idx, row) in rows.iter().enumerate().rev() {
        let needed = sidebar_row_height_in_body(app, row, entries, body.height)
            .saturating_add(sidebar_row_gap(app, rows, entries, row_idx));
        if used_rows.saturating_add(needed) > body.height {
            break;
        }
        used_rows = used_rows.saturating_add(needed);
        start = row_idx;
    }
    start.min(rows.len().saturating_sub(1))
}

pub(crate) fn workspace_list_scroll_metrics_from_entries(
    app: &AppState,
    entries: &[AgentPanelEntry],
    area: Rect,
) -> crate::pane::ScrollMetrics {
    let rows = sidebar_rows(app, entries);
    let max_scroll = sidebar_list_bottom_start(app, &rows, entries, area);
    let scroll = app.workspace_scroll.min(max_scroll);
    let viewport_rows = sidebar_list_visible_count(app, &rows, entries, area, scroll);

    crate::pane::ScrollMetrics {
        offset_from_bottom: max_scroll.saturating_sub(scroll),
        max_offset_from_bottom: max_scroll,
        viewport_rows,
    }
}

pub(crate) fn workspace_list_scroll_metrics(
    app: &AppState,
    area: Rect,
) -> crate::pane::ScrollMetrics {
    let entries = frame_agent_panel_entries(app, None);
    workspace_list_scroll_metrics_from_entries(app, &entries, area)
}

pub(crate) fn normalized_workspace_scroll(app: &AppState, area: Rect, requested: usize) -> usize {
    let entries = frame_agent_panel_entries(app, None);
    normalized_workspace_scroll_from_entries(app, &entries, area, requested)
}

pub(crate) fn normalized_workspace_scroll_from_entries(
    app: &AppState,
    entries: &[AgentPanelEntry],
    area: Rect,
    requested: usize,
) -> usize {
    let list_area = workspace_list_rect(area);
    let body = workspace_list_body_rect(list_area, false);
    if body.height == 0 {
        return requested;
    }

    let rows = sidebar_rows(app, entries);
    if rows.is_empty() {
        0
    } else {
        requested.min(sidebar_list_bottom_start(app, &rows, entries, list_area))
    }
}

pub(crate) fn workspace_list_scrollbar_rect(app: &AppState, area: Rect) -> Option<Rect> {
    let metrics = workspace_list_scroll_metrics(app, area);
    let body = workspace_list_body_rect(area, true);
    (should_show_scrollbar(metrics) && body.width > 0 && body.height > 0).then_some(Rect::new(
        area.x + area.width.saturating_sub(1),
        body.y,
        1,
        body.height,
    ))
}

fn resolved_agent_rows(app: &AppState, entry: &AgentPanelEntry) -> Vec<Vec<ResolvedToken>> {
    let label = entry
        .state_labels
        .get(agent_panel_status_key(entry.state, entry.seen))
        .map(String::as_str)
        .unwrap_or_else(|| state_label(entry.state, entry.seen));
    tokens::agent_rows(&app.sidebar_agents, entry, label)
}

fn agent_entry_height_in_body(app: &AppState, entry: &AgentPanelEntry, body_height: u16) -> u16 {
    (resolved_agent_rows(app, entry)
        .len()
        .max(1)
        .min(u16::MAX as usize) as u16)
        .min(body_height)
}

/// Scroll offset that brings the merged row for `entry_idx` into view. The
/// caller addresses agents by their entry index; the row index they occupy in
/// the merged list is this function's business.
pub(crate) fn sidebar_scroll_for_agent_entry(
    app: &AppState,
    area: Rect,
    current_scroll: usize,
    entry_idx: usize,
) -> usize {
    let entries = frame_agent_panel_entries(app, None);
    let rows = sidebar_rows(app, &entries);
    let Some(target) = rows
        .iter()
        .position(|row| matches!(row, SidebarRow::Agent { entry_idx: idx } if *idx == entry_idx))
    else {
        return current_scroll;
    };

    let max_scroll = sidebar_list_bottom_start(app, &rows, &entries, area);
    if target < current_scroll {
        return target.min(max_scroll);
    }
    let mut scroll = current_scroll.min(max_scroll);
    while scroll < target {
        let visible = sidebar_list_visible_count(app, &rows, &entries, area, scroll);
        if visible > 0 && target < scroll.saturating_add(visible) {
            break;
        }
        scroll += 1;
    }
    scroll.min(max_scroll)
}

fn view_has_computed_frame(app: &AppState) -> bool {
    app.view.sidebar_rect != Rect::default()
        || app.view.terminal_area != Rect::default()
        || app.view.mobile_header_rect != Rect::default()
}

fn frame_agent_panel_entries<'a>(
    app: &'a AppState,
    terminal_runtimes: Option<&TerminalRuntimeRegistry>,
) -> Cow<'a, [AgentPanelEntry]> {
    if view_has_computed_frame(app) {
        return Cow::Borrowed(&app.view.agent_panel_entries);
    }

    match terminal_runtimes {
        Some(terminal_runtimes) => Cow::Owned(agent_panel_entries_from(app, terminal_runtimes)),
        None => Cow::Owned(agent_panel_entries(app)),
    }
}

/// Space and agent geometry from a single walk of the merged rows, so the two
/// cannot drift apart.
pub(crate) fn compute_workspace_list_areas(
    app: &AppState,
    entries: &[AgentPanelEntry],
    area: Rect,
) -> (
    Vec<crate::app::state::WorkspaceCardArea>,
    Vec<crate::app::state::SidebarAgentRowArea>,
) {
    let list_area = workspace_list_rect(area);
    if list_area == Rect::default() {
        return (Vec::new(), Vec::new());
    }

    let metrics = workspace_list_scroll_metrics_from_entries(app, entries, list_area);
    let body = workspace_list_body_rect(list_area, should_show_scrollbar(metrics));
    if body.width == 0 || body.height == 0 {
        return (Vec::new(), Vec::new());
    }

    let rows = sidebar_rows(app, entries);
    // A stale scroll past the end must still show a list, not a blank body.
    let scroll = app.workspace_scroll.min(metrics.max_offset_from_bottom);
    let mut row_y = body.y;
    let body_bottom = body.y + body.height;
    let mut cards = Vec::new();
    let mut agent_rows = Vec::new();

    for (row_idx, row) in rows.iter().enumerate().skip(scroll) {
        let row_height = sidebar_row_height_in_body(app, row, entries, body.height);
        if row_y.saturating_add(row_height) > body_bottom {
            break;
        }
        let rect = Rect::new(body.x, row_y, body.width, row_height);
        match row {
            SidebarRow::Space { ws_idx, indented } => {
                cards.push(crate::app::state::WorkspaceCardArea {
                    ws_idx: *ws_idx,
                    rect,
                    indented: *indented,
                });
            }
            SidebarRow::Agent { entry_idx } => {
                agent_rows.push(crate::app::state::SidebarAgentRowArea {
                    rect,
                    entry_idx: Some(*entry_idx),
                });
            }
            SidebarRow::NoAgents { .. } => {
                agent_rows.push(crate::app::state::SidebarAgentRowArea {
                    rect,
                    entry_idx: None,
                });
            }
        }
        row_y = row_y
            .saturating_add(row_height)
            .saturating_add(sidebar_row_gap(app, &rows, entries, row_idx))
            .min(body_bottom);
    }

    (cards, agent_rows)
}

pub(crate) fn compute_workspace_card_areas(
    app: &AppState,
    area: Rect,
) -> Vec<crate::app::state::WorkspaceCardArea> {
    let entries = frame_agent_panel_entries(app, None);
    compute_workspace_list_areas(app, &entries, area).0
}

pub(crate) fn workspace_group_chevron_rect(card: &crate::app::state::WorkspaceCardArea) -> Rect {
    if card.rect.width == 0 || card.rect.height == 0 {
        return Rect::default();
    }

    Rect::new(
        card.rect.x + card.rect.width.saturating_sub(1),
        card.rect.y,
        1,
        1,
    )
}

/// Collapsed sidebar body: the merged list rendered one row per entry, minus the
/// trailing separator column.
pub(crate) fn collapsed_sidebar_content_rect(area: Rect) -> Rect {
    let content = Rect::new(area.x, area.y, area.width.saturating_sub(1), area.height);
    if content.width == 0 || content.height == 0 {
        return Rect::default();
    }
    content
}

/// Collapsed sidebar: the same flat space-then-agent ordering as the expanded
/// list, compressed to one glyph row per entry.
pub(super) fn render_sidebar_collapsed(app: &AppState, frame: &mut Frame, area: Rect) {
    if area.width == 0 || area.height == 0 {
        return;
    }

    let is_navigating = matches!(app.mode, Mode::Navigate);

    let p = &app.palette;
    let sep_style = if is_navigating {
        Style::default().fg(p.accent)
    } else {
        Style::default().fg(p.surface_dim)
    };
    let sep_x = area.x + area.width.saturating_sub(1);
    let buf = frame.buffer_mut();
    for y in area.y..area.y + area.height {
        buf[(sep_x, y)].set_symbol("│");
        buf[(sep_x, y)].set_style(sep_style);
    }

    let content = collapsed_sidebar_content_rect(area);
    if content == Rect::default() {
        render_sidebar_toggle(app, frame, area, true, p);
        return;
    }

    let entries = frame_agent_panel_entries(app, None);
    let content_bottom = content.y + content.height;
    let mut space_position = 0usize;
    let mut agent_position = 0usize;
    for (row_idx, row) in sidebar_rows(app, &entries).iter().enumerate() {
        let y = content.y + row_idx as u16;
        if y >= content_bottom {
            break;
        }
        let row_rect = Rect::new(content.x, y, content.width, 1);

        match row {
            SidebarRow::Space { ws_idx, .. } => {
                space_position += 1;
                let Some(ws) = app.workspaces.get(*ws_idx) else {
                    continue;
                };
                let (agg_state, agg_seen) = ws.aggregate_state(&app.terminals);
                let (icon, icon_style) = state_dot(agg_state, agg_seen, p);
                let is_selected = *ws_idx == app.selected && is_navigating;
                let is_active = Some(*ws_idx) == app.active;
                let row_style = if is_selected {
                    Style::default().bg(p.surface0)
                } else if is_active {
                    Style::default().bg(p.surface_dim)
                } else {
                    Style::default()
                };
                let num_style = if is_selected {
                    Style::default().fg(p.overlay1).bg(p.surface0)
                } else if is_active {
                    Style::default().fg(p.text).bg(p.surface_dim)
                } else {
                    Style::default().fg(p.overlay0)
                };

                if is_selected || is_active {
                    let buf = frame.buffer_mut();
                    for x in row_rect.x..row_rect.x + row_rect.width {
                        buf[(x, y)].set_style(row_style);
                    }
                }

                frame.render_widget(
                    Paragraph::new(Line::from(vec![
                        Span::styled(format!("{space_position:<2}"), num_style),
                        Span::styled(icon, icon_style),
                    ])),
                    row_rect,
                );
            }
            SidebarRow::Agent { entry_idx } => {
                let Some(detail) = entries.get(*entry_idx) else {
                    continue;
                };
                agent_position += 1;
                let (icon, icon_style) = state_dot(detail.state, detail.seen, p);
                frame.render_widget(
                    Paragraph::new(Line::from(vec![
                        Span::styled(
                            format!("{agent_position:<2}"),
                            Style::default().fg(p.overlay0).add_modifier(Modifier::DIM),
                        ),
                        Span::styled(icon, icon_style),
                    ])),
                    row_rect,
                );
            }
            SidebarRow::NoAgents { .. } => {
                frame.render_widget(
                    Paragraph::new(Span::styled(
                        "  -",
                        Style::default().fg(p.overlay0).add_modifier(Modifier::DIM),
                    )),
                    row_rect,
                );
            }
        }
    }

    render_sidebar_toggle(app, frame, area, true, p);
}

pub(crate) fn workspace_drop_slots(
    app: &AppState,
    cards: &[crate::app::state::WorkspaceCardArea],
    area: Rect,
) -> Vec<(crate::app::state::WorkspaceDropTarget, u16)> {
    if area.height == 0 || cards.is_empty() {
        return Vec::new();
    }
    let list_bottom = area.y + area.height.saturating_sub(1);
    let entries = workspace_list_entries(app);
    let entry_position = |ws_idx| {
        entries.iter().position(|entry| {
            matches!(
                entry,
                WorkspaceListEntry::Workspace {
                    ws_idx: entry_ws_idx,
                    ..
                } if *entry_ws_idx == ws_idx
            )
        })
    };
    let block_root_at = |entry_idx: usize| {
        entries[..=entry_idx]
            .iter()
            .rev()
            .find_map(|entry| match entry {
                WorkspaceListEntry::Workspace {
                    ws_idx,
                    indented: false,
                } => Some(*ws_idx),
                WorkspaceListEntry::Workspace { .. } => None,
            })
    };

    let mut slots = Vec::new();
    let mut previous_root = None;
    for card in cards {
        let Some(entry_idx) = entry_position(card.ws_idx) else {
            continue;
        };
        let Some(root_idx) = block_root_at(entry_idx) else {
            continue;
        };
        if previous_root == Some(root_idx) {
            continue;
        }
        previous_root = Some(root_idx);
        if let Some(row) = card.rect.y.checked_sub(1).filter(|row| *row < list_bottom) {
            slots.push((
                crate::app::state::WorkspaceDropTarget::Before(root_idx),
                row,
            ));
        }
    }

    let Some(last) = cards.last() else {
        return slots;
    };
    let Some(last_entry_idx) = entry_position(last.ws_idx) else {
        return slots;
    };
    let next_entry = entries.get(last_entry_idx.saturating_add(1));
    if matches!(
        next_entry,
        Some(WorkspaceListEntry::Workspace { indented: true, .. })
    ) {
        return slots;
    }
    let target = match next_entry {
        Some(WorkspaceListEntry::Workspace { ws_idx, .. }) => {
            crate::app::state::WorkspaceDropTarget::Before(*ws_idx)
        }
        None => crate::app::state::WorkspaceDropTarget::End,
    };
    let row = last.rect.y.saturating_add(last.rect.height);
    if row < list_bottom
        && slots
            .last()
            .is_none_or(|(last_target, _)| *last_target != target)
    {
        slots.push((target, row));
    }
    slots
}

pub(crate) fn workspace_drop_indicator_row(
    app: &AppState,
    cards: &[crate::app::state::WorkspaceCardArea],
    area: Rect,
    target: crate::app::state::WorkspaceDropTarget,
) -> Option<u16> {
    workspace_drop_slots(app, cards, area)
        .into_iter()
        .find_map(|(candidate, row)| (candidate == target).then_some(row))
}

pub(super) fn render_sidebar(
    app: &AppState,
    terminal_runtimes: &TerminalRuntimeRegistry,
    frame: &mut Frame,
    area: Rect,
) {
    let p = &app.palette;
    let is_navigating = matches!(app.mode, Mode::Navigate);
    let sep_style = if is_navigating {
        Style::default().fg(p.accent)
    } else {
        Style::default().fg(p.surface_dim)
    };

    let sep_x = area.x + area.width.saturating_sub(1);
    let buf = frame.buffer_mut();
    for y in area.y..area.y + area.height {
        buf[(sep_x, y)].set_symbol("│");
        buf[(sep_x, y)].set_style(sep_style);
    }

    render_workspace_list(app, terminal_runtimes, frame, area, is_navigating);
    render_sidebar_toggle(app, frame, area, false, p);
}

pub(super) fn resolved_token_spans(
    resolved: &[ResolvedToken],
    state_icon: (&str, Style),
    state_text_style: Style,
    workspace_style: Style,
    secondary_style: Style,
    custom_style: Style,
    p: &Palette,
    max_width: usize,
) -> Vec<Span<'static>> {
    let fixed_widths = resolved
        .iter()
        .map(|token| match &token.kind {
            ResolvedTokenKind::StateIcon => display_width(state_icon.0),
            ResolvedTokenKind::GitStatus { ahead, behind } => {
                usize::from(*ahead > 0) * display_width(&format!("↑{ahead}"))
                    + usize::from(*behind > 0) * display_width(&format!("↓{behind}"))
                    + usize::from(*ahead > 0 && *behind > 0)
            }
            _ => 0,
        })
        .collect::<Vec<_>>();
    let flexible_widths = resolved
        .iter()
        .map(|token| match &token.kind {
            ResolvedTokenKind::StateText(text)
            | ResolvedTokenKind::Workspace(text)
            | ResolvedTokenKind::Tab(text)
            | ResolvedTokenKind::Pane(text)
            | ResolvedTokenKind::Agent(text)
            | ResolvedTokenKind::TerminalTitle(text)
            | ResolvedTokenKind::Branch(text)
            | ResolvedTokenKind::Custom(text) => display_width(text),
            _ => 0,
        })
        .collect::<Vec<_>>();
    let minimum_width = |active: &[bool]| {
        let indices = active
            .iter()
            .enumerate()
            .filter_map(|(index, active)| active.then_some(index))
            .collect::<Vec<_>>();
        let content = indices
            .iter()
            .map(|index| fixed_widths[*index] + usize::from(flexible_widths[*index] > 0))
            .sum::<usize>();
        let separators = indices
            .windows(2)
            .map(|pair| display_width(tokens::separator(&resolved[pair[0]], &resolved[pair[1]])))
            .sum::<usize>();
        content + separators
    };
    let mut active = resolved.iter().map(|_| true).collect::<Vec<_>>();
    if minimum_width(&active) > max_width {
        for (index, width) in flexible_widths.iter().enumerate() {
            if *width > 0 {
                active[index] = false;
            }
        }
        for index in (0..resolved.len()).rev() {
            if flexible_widths[index] == 0 {
                continue;
            }
            active[index] = true;
            if minimum_width(&active) > max_width {
                active[index] = false;
            }
        }
    }
    let visible_indices = active
        .iter()
        .enumerate()
        .filter_map(|(index, active)| active.then_some(index))
        .collect::<Vec<_>>();
    let separator_width = visible_indices
        .windows(2)
        .map(|pair| display_width(tokens::separator(&resolved[pair[0]], &resolved[pair[1]])))
        .sum::<usize>();
    let fixed_width = visible_indices
        .iter()
        .map(|index| fixed_widths[*index])
        .sum::<usize>();
    let mut budgets = flexible_widths
        .iter()
        .enumerate()
        .map(|(index, width)| usize::from(active[index] && *width > 0))
        .collect::<Vec<_>>();
    let minimum = budgets.iter().sum::<usize>();
    let mut remaining = max_width
        .saturating_sub(separator_width + fixed_width)
        .saturating_sub(minimum);
    while remaining > 0 {
        let mut grew = false;
        for (budget, width) in budgets.iter_mut().zip(&flexible_widths) {
            if *budget > 0 && *budget < *width {
                *budget += 1;
                remaining -= 1;
                grew = true;
                if remaining == 0 {
                    break;
                }
            }
        }
        if !grew {
            break;
        }
    }
    let mut spans = Vec::new();
    for (position, index) in visible_indices.iter().copied().enumerate() {
        let token = &resolved[index];
        if position > 0 {
            let previous = &resolved[visible_indices[position - 1]];
            spans.push(Span::styled(
                tokens::separator(previous, token),
                Style::default().fg(p.overlay0).add_modifier(Modifier::DIM),
            ));
        }
        match &token.kind {
            ResolvedTokenKind::StateIcon => {
                spans.push(Span::styled(
                    state_icon.0.to_string(),
                    apply_token_style(state_icon.1, token.style),
                ));
            }
            ResolvedTokenKind::StateText(text) => {
                spans.push(Span::styled(
                    truncate_end(text, budgets[index]),
                    apply_token_style(state_text_style, token.style),
                ));
            }
            ResolvedTokenKind::Workspace(text) => {
                spans.push(Span::styled(
                    truncate_end(text, budgets[index]),
                    apply_token_style(workspace_style, token.style),
                ));
            }
            ResolvedTokenKind::Tab(text)
            | ResolvedTokenKind::Pane(text)
            | ResolvedTokenKind::Agent(text)
            | ResolvedTokenKind::Branch(text) => {
                spans.push(Span::styled(
                    truncate_end(text, budgets[index]),
                    apply_token_style(secondary_style, token.style),
                ));
            }
            ResolvedTokenKind::GitStatus { ahead, behind } => {
                if *ahead > 0 {
                    spans.push(Span::styled(
                        format!("↑{ahead}"),
                        apply_token_style(Style::default().fg(p.green), token.style),
                    ));
                }
                if *ahead > 0 && *behind > 0 {
                    spans.push(Span::styled(
                        " ",
                        apply_token_style(Style::default(), token.style),
                    ));
                }
                if *behind > 0 {
                    spans.push(Span::styled(
                        format!("↓{behind}"),
                        apply_token_style(Style::default().fg(p.red), token.style),
                    ));
                }
            }
            ResolvedTokenKind::TerminalTitle(text) | ResolvedTokenKind::Custom(text) => {
                spans.push(Span::styled(
                    truncate_end(text, budgets[index]),
                    apply_token_style(custom_style, token.style),
                ));
            }
        }
    }
    spans
}

fn apply_token_style(mut style: Style, patch: crate::config::SidebarTokenStyle) -> Style {
    if let Some(fg) = patch.fg {
        style = style.fg(fg.ratatui());
    }
    if let Some(bold) = patch.bold {
        style = if bold {
            style.add_modifier(Modifier::BOLD)
        } else {
            style.remove_modifier(Modifier::BOLD)
        };
    }
    if let Some(dim) = patch.dim {
        style = if dim {
            style.add_modifier(Modifier::DIM)
        } else {
            style.remove_modifier(Modifier::DIM)
        };
    }
    style
}

/// The merged list: every space row followed by its own agent rows. Geometry
/// comes from the computed frame, so this only draws.
fn render_workspace_list(
    app: &AppState,
    terminal_runtimes: &TerminalRuntimeRegistry,
    frame: &mut Frame,
    sidebar_area: Rect,
    is_navigating: bool,
) {
    let area = workspace_list_rect(sidebar_area);
    if area == Rect::default() {
        return;
    }

    let p = &app.palette;
    let agent_entries = frame_agent_panel_entries(app, Some(terminal_runtimes));
    let (cards, agent_row_areas) = if view_has_computed_frame(app) {
        (
            Cow::Borrowed(&app.view.workspace_card_areas[..]),
            Cow::Borrowed(&app.view.agent_row_areas[..]),
        )
    } else {
        let (cards, agent_rows) = compute_workspace_list_areas(app, &agent_entries, sidebar_area);
        (Cow::Owned(cards), Cow::Owned(agent_rows))
    };

    let dragged_ws_idx = match app.drag.as_ref().map(|drag| &drag.target) {
        Some(crate::app::state::DragTarget::WorkspaceReorder { source_ws_idx, .. }) => {
            Some(*source_ws_idx)
        }
        _ => None,
    };
    let insertion_row = match app.drag.as_ref().map(|drag| &drag.target) {
        Some(crate::app::state::DragTarget::WorkspaceReorder {
            drop_target: Some(drop_target),
            ..
        }) => workspace_drop_indicator_row(app, &cards, area, *drop_target),
        _ => None,
    };

    let list_bottom = area.y + area.height.saturating_sub(1);
    if area.height > 0 {
        frame.render_widget(
            Paragraph::new(Line::from(vec![Span::styled(
                " spaces",
                Style::default().fg(p.overlay0).add_modifier(Modifier::BOLD),
            )])),
            Rect::new(area.x, area.y, area.width, 1),
        );
        render_agent_sort_control(app, frame, area);
    }

    let metrics = workspace_list_scroll_metrics_from_entries(app, &agent_entries, area);
    let scrollbar_rect = workspace_list_scrollbar_rect(app, area);
    let entries = workspace_list_entries(app);

    for card in cards.iter() {
        let i = card.ws_idx;
        let ws = &app.workspaces[i];
        let row_y = card.rect.y;
        let row_height = card.rect.height;
        let selected = i == app.selected && is_navigating;
        let is_active = Some(i) == app.active;
        let is_dragged = dragged_ws_idx == Some(i);
        let highlighted = selected || is_active || is_dragged;
        let (agg_state, agg_seen) = ws.aggregate_state(&app.terminals);

        if highlighted {
            let bg = if selected {
                p.surface0
            } else if is_dragged {
                p.surface1
            } else {
                p.surface_dim
            };
            let buf = frame.buffer_mut();
            for y in row_y..row_y + row_height {
                if y >= list_bottom {
                    break;
                }
                for x in card.rect.x..card.rect.x + card.rect.width {
                    buf[(x, y)].set_style(Style::default().bg(bg));
                }
            }
        }

        let name_style = if selected || is_active || is_dragged {
            Style::default().fg(p.text).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(p.subtext0)
        };

        let label = ws.display_name_from(&app.terminals, terminal_runtimes);
        let display_label = if card.indented {
            grouped_child_display_label(&label, ws.branch().as_deref(), ws.custom_name.is_some())
        } else {
            label
        };
        let parent_group = (!card.indented)
            .then(|| workspace_parent_group_state(app, i))
            .flatten();
        let is_last_child = card.indented
            && entries
                .iter()
                .position(|entry| {
                    matches!(
                        entry,
                        WorkspaceListEntry::Workspace { ws_idx, .. } if *ws_idx == i
                    )
                })
                .is_none_or(|entry_idx| !next_entry_is_indented_workspace(&entries, entry_idx));
        let (display_state, display_seen) = parent_group
            .as_ref()
            .filter(|(_, collapsed)| *collapsed)
            .map(|(key, _)| space_aggregate_state(app, key))
            .unwrap_or((agg_state, agg_seen));
        let state_icon = state_dot(display_state, display_seen, p);
        let state_text_style = Style::default()
            .fg(state_label_color(display_state, display_seen, p))
            .add_modifier(Modifier::DIM);
        let branch_style = Style::default().fg(if selected || is_active {
            p.mauve
        } else {
            p.overlay0
        });
        let token_values = ws.metadata_tokens.values();
        let rows = tokens::space_rows(
            &app.sidebar_spaces,
            SpaceTokenContext {
                workspace: &display_label,
                branch: ws.branch().as_deref(),
                state_text: state_label(display_state, display_seen),
                ahead_behind: ws.git_ahead_behind(),
                tokens: &token_values,
                suppress_git_details: card.indented,
                project_status: ws.project_status(),
            },
        );

        for (row_index, resolved) in rows.iter().enumerate() {
            if row_index as u16 >= row_height || row_y + row_index as u16 >= list_bottom {
                break;
            }
            let mut spans = Vec::new();
            let prefix_width = if card.indented {
                spans.push(Span::raw("   "));
                if row_index == 0 {
                    spans.push(Span::styled(
                        if is_last_child { "└─ " } else { "├─ " },
                        Style::default().fg(p.overlay0),
                    ));
                    6
                } else if is_last_child {
                    spans.push(Span::raw("     "));
                    8
                } else {
                    spans.push(Span::styled("│", Style::default().fg(p.overlay0)));
                    spans.push(Span::raw("    "));
                    8
                }
            } else if row_index == 0 {
                spans.push(Span::raw(" "));
                1
            } else {
                spans.push(Span::raw("   "));
                3
            };
            let trailing_width = if row_index == 0 && parent_group.is_some() {
                2
            } else {
                0
            };
            spans.extend(resolved_token_spans(
                resolved,
                state_icon,
                state_text_style,
                name_style,
                branch_style,
                branch_style,
                p,
                card.rect
                    .width
                    .saturating_sub(prefix_width + trailing_width) as usize,
            ));
            frame.render_widget(
                Paragraph::new(Line::from(spans)),
                Rect::new(card.rect.x, row_y + row_index as u16, card.rect.width, 1),
            );
        }

        if let Some((_, collapsed)) = parent_group {
            frame.render_widget(
                Paragraph::new(Span::styled(
                    if collapsed { "▸" } else { "▾" },
                    Style::default().fg(p.accent),
                )),
                workspace_group_chevron_rect(card),
            );
        }
    }

    for agent_row in agent_row_areas.iter() {
        let Some(entry_idx) = agent_row.entry_idx else {
            render_no_agents_row(app, frame, agent_row.rect);
            continue;
        };
        let Some(detail) = agent_entries.get(entry_idx) else {
            continue;
        };
        render_agent_row(app, frame, agent_row.rect, detail);
    }

    if let Some(y) = insertion_row.filter(|y| *y < list_bottom) {
        let indicator_right = scrollbar_rect
            .map(|rect| rect.x)
            .unwrap_or(area.x + area.width);
        let buf = frame.buffer_mut();
        for x in area.x..indicator_right {
            buf[(x, y)].set_symbol("─");
            buf[(x, y)].set_style(Style::default().fg(p.accent));
        }
    }

    if let Some(track) = scrollbar_rect {
        render_scrollbar(frame, metrics, track, p.surface_dim, p.overlay0, "▕");
    }

    if app.mouse_capture && list_bottom > area.y {
        let new_rect = app.sidebar_new_button_rect();
        frame.render_widget(
            Paragraph::new(Span::styled(" new", Style::default().fg(p.overlay0))),
            new_rect,
        );

        let menu_rect = app.global_launcher_rect();
        let menu_line = if app.global_menu_attention_badge_visible() {
            Line::from(vec![
                Span::styled(
                    "● ",
                    Style::default().fg(p.accent).add_modifier(Modifier::BOLD),
                ),
                Span::styled("menu", Style::default().fg(p.overlay0)),
            ])
        } else {
            Line::from(vec![Span::styled("menu", Style::default().fg(p.overlay0))])
        };
        frame.render_widget(
            Paragraph::new(menu_line).alignment(Alignment::Right),
            menu_rect,
        );
    }
}

/// The sort toggle doubles as the active-agent-view indicator, right-aligned on
/// the merged list's single header row.
fn render_agent_sort_control(app: &AppState, frame: &mut Frame, area: Rect) {
    let p = &app.palette;
    let control_label = active_agent_view_label(app)
        .map(str::to_string)
        .unwrap_or_else(|| agent_panel_sort_label(app.agent_panel_sort));
    let toggle_rect = agent_panel_header_label_rect(area, &control_label);
    if toggle_rect == Rect::default() {
        return;
    }
    let color = if app.agent_view_override.is_some() {
        p.accent
    } else {
        p.overlay0
    };
    frame.render_widget(
        Paragraph::new(Span::styled(
            control_label,
            Style::default().fg(color).add_modifier(Modifier::BOLD),
        ))
        .alignment(Alignment::Right),
        toggle_rect,
    );
}

fn render_agent_row(app: &AppState, frame: &mut Frame, rect: Rect, detail: &AgentPanelEntry) {
    let p = &app.palette;
    let label_color = state_label_color(detail.state, detail.seen, p);
    let rows = resolved_agent_rows(app, detail);

    let is_active = app.is_active_pane(detail.ws_idx, detail.tab_idx, detail.pane_id);
    let row_style = if is_active {
        Style::default().bg(p.surface_dim)
    } else {
        Style::default()
    };
    let name_style = if is_active {
        Style::default().fg(p.text).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(p.subtext0).add_modifier(Modifier::BOLD)
    };
    let status_style = if is_active {
        Style::default().fg(label_color)
    } else {
        Style::default().fg(label_color).add_modifier(Modifier::DIM)
    };
    let agent_style = Style::default().fg(p.overlay0).add_modifier(Modifier::DIM);
    let state_icon = state_dot(detail.state, detail.seen, p);

    for (row_index, resolved) in rows.iter().take(rect.height as usize).enumerate() {
        // Agents sit one level under their space row, so the first line indents
        // past the space row's own single-column gutter.
        let indent = if row_index == 0 { "   " } else { "     " };
        let mut spans = vec![Span::raw(indent)];
        spans.extend(resolved_token_spans(
            resolved,
            state_icon,
            status_style,
            name_style,
            agent_style,
            agent_style,
            p,
            rect.width.saturating_sub(indent.len() as u16) as usize,
        ));
        frame.render_widget(
            Paragraph::new(Line::from(spans)).style(row_style),
            Rect::new(rect.x, rect.y + row_index as u16, rect.width, 1),
        );
    }
}

/// "Nothing running here" has to be visible; an absent row is indistinguishable
/// from a collapsed space.
fn render_no_agents_row(app: &AppState, frame: &mut Frame, rect: Rect) {
    let p = &app.palette;
    let label = if app.agent_view_override.is_some() {
        "   no matching agents"
    } else {
        "   no agents"
    };
    frame.render_widget(
        Paragraph::new(label).style(Style::default().fg(p.overlay0).add_modifier(Modifier::DIM)),
        Rect::new(rect.x, rect.y, rect.width, 1),
    );
}

pub(crate) fn collapsed_sidebar_toggle_rect(area: Rect) -> Rect {
    let bottom_y = area.y + area.height.saturating_sub(1);
    let content_w = area.width.saturating_sub(1);
    if content_w == 0 || area.height == 0 {
        return Rect::default();
    }
    let x = area.x + content_w / 2;
    Rect::new(x, bottom_y, 1, 1)
}

pub(crate) fn expanded_sidebar_toggle_rect(area: Rect) -> Rect {
    if area.width <= 1 || area.height == 0 {
        return Rect::default();
    }
    Rect::new(
        area.x + area.width.saturating_sub(2),
        area.y + area.height.saturating_sub(1),
        1,
        1,
    )
}

fn render_sidebar_toggle(
    app: &AppState,
    frame: &mut Frame,
    area: Rect,
    collapsed: bool,
    p: &Palette,
) {
    let toggle_area = if collapsed {
        collapsed_sidebar_toggle_rect(area)
    } else {
        expanded_sidebar_toggle_rect(area)
    };
    if toggle_area == Rect::default() {
        return;
    }
    let icon = if collapsed { "»" } else { "«" };
    let icon_style = if collapsed && app.global_menu_attention_badge_visible() {
        Style::default().fg(p.accent).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(p.overlay0)
    };
    frame.render_widget(Paragraph::new(Span::styled(icon, icon_style)), toggle_area);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{detect::Agent, workspace::Workspace};
    use ratatui::{backend::TestBackend, Terminal};

    fn row_text(buffer: &ratatui::buffer::Buffer, row: u16, width: u16) -> String {
        (0..width)
            .map(|x| buffer[(x, row)].symbol())
            .collect::<String>()
            .trim_end()
            .to_string()
    }

    /// Body rect of the merged list — the first row is the first space row,
    /// with that space's agents directly beneath it.
    fn merged_body(app: &crate::app::state::AppState, area: Rect) -> Rect {
        let list_area = workspace_list_rect(area);
        let metrics = workspace_list_scroll_metrics(app, list_area);
        workspace_list_body_rect(list_area, should_show_scrollbar(metrics))
    }

    /// Agent (and empty-state) row rects of the merged list, in visible order.
    fn agent_row_rects(
        app: &crate::app::state::AppState,
        area: Rect,
    ) -> Vec<crate::app::state::SidebarAgentRowArea> {
        compute_workspace_list_areas(app, &agent_panel_entries(app), area).1
    }

    fn find_symbol_x(buffer: &ratatui::buffer::Buffer, row: u16, width: u16, symbol: &str) -> u16 {
        (0..width)
            .find(|x| buffer[(*x, row)].symbol() == symbol)
            .unwrap_or_else(|| {
                panic!(
                    "missing symbol {symbol:?} in row {}",
                    row_text(buffer, row, width)
                )
            })
    }

    #[test]
    fn default_agent_rows_remove_redundant_state_text() {
        let mut app = crate::app::state::AppState::test_new();
        let workspace = Workspace::test_new("one");
        let pane_id = workspace.tabs[0].root_pane;
        app.workspaces = vec![workspace];
        app.ensure_test_terminals();
        app.active = Some(0);
        let terminal_id = app.workspaces[0].tabs[0].panes[&pane_id]
            .attached_terminal_id
            .clone();
        let terminal_state = app.terminals.get_mut(&terminal_id).unwrap();
        terminal_state.detected_agent = Some(Agent::Pi);
        terminal_state.state = AgentState::Working;

        let area = Rect::new(0, 0, 26, 20);
        let mut terminal = Terminal::new(TestBackend::new(26, 20)).unwrap();
        terminal
            .draw(|frame| render_sidebar(&app, &TerminalRuntimeRegistry::new(), frame, area))
            .unwrap();
        let buffer = terminal.backend().buffer();
        let body = merged_body(&app, area);
        let agent_y = agent_row_rects(&app, area)[0].rect.y;
        assert!(agent_y > body.y, "the agent row sits under its space row");

        let first = row_text(buffer, agent_y, 25);
        let second = row_text(buffer, agent_y + 1, 25);
        assert!(first.contains("one"));
        assert_eq!(second, "     pi");
        assert!(!first.contains("working"));
        assert!(!second.contains("working"));

        let workspace_x = find_symbol_x(buffer, agent_y, body.width, "o");
        let workspace_style = buffer[(workspace_x, agent_y)].style();
        assert_eq!(workspace_style.fg, Some(app.palette.text));
        assert!(workspace_style.add_modifier.contains(Modifier::BOLD));
        assert!(!workspace_style.add_modifier.contains(Modifier::DIM));
        assert_eq!(workspace_style.bg, Some(app.palette.surface_dim));

        let agent_x = find_symbol_x(buffer, agent_y + 1, body.width, "p");
        let agent_style = buffer[(agent_x, agent_y + 1)].style();
        assert_eq!(agent_style.fg, Some(app.palette.overlay0));
        assert!(agent_style.add_modifier.contains(Modifier::DIM));
        assert!(!agent_style.add_modifier.contains(Modifier::BOLD));
        assert_eq!(agent_style.bg, Some(app.palette.surface_dim));
    }

    #[test]
    fn occurrence_false_removes_default_workspace_bold_and_agent_dim() {
        let config: crate::config::Config = toml::from_str(
            r##"
[ui.sidebar.agents]
rows = [[{ token = "workspace", bold = false }, { token = "agent", dim = false }]]
"##,
        )
        .unwrap();
        let mut app = crate::app::state::AppState::test_new();
        app.sidebar_agents = config.ui.sidebar.agents;
        let workspace = Workspace::test_new("one");
        let pane_id = workspace.tabs[0].root_pane;
        app.workspaces = vec![workspace];
        app.ensure_test_terminals();
        app.active = Some(0);
        let terminal_id = app.workspaces[0].tabs[0].panes[&pane_id]
            .attached_terminal_id
            .clone();
        app.terminals.get_mut(&terminal_id).unwrap().detected_agent = Some(Agent::Pi);

        let area = Rect::new(0, 0, 26, 20);
        let mut terminal = Terminal::new(TestBackend::new(26, 20)).unwrap();
        terminal
            .draw(|frame| render_sidebar(&app, &TerminalRuntimeRegistry::new(), frame, area))
            .unwrap();
        let body = merged_body(&app, area);
        let agent_y = agent_row_rects(&app, area)[0].rect.y;
        let buffer = terminal.backend().buffer();
        let workspace = buffer[(find_symbol_x(buffer, agent_y, body.width, "o"), agent_y)].style();
        let agent = buffer[(find_symbol_x(buffer, agent_y, body.width, "p"), agent_y)].style();

        assert_eq!(workspace.fg, Some(app.palette.text));
        assert!(!workspace.add_modifier.contains(Modifier::BOLD));
        assert_eq!(agent.fg, Some(app.palette.overlay0));
        assert!(!agent.add_modifier.contains(Modifier::DIM));
    }

    #[test]
    fn default_space_workspace_style_tracks_active_state() {
        let mut app = crate::app::state::AppState::test_new();
        app.workspaces = vec![Workspace::test_new("one"), Workspace::test_new("two")];
        app.active = Some(0);
        app.mode = Mode::Terminal;
        let area = Rect::new(0, 0, 26, 20);
        app.view.workspace_card_areas = compute_workspace_card_areas(&app, area);
        let first_row = app.view.workspace_card_areas[0].rect.y;
        let second_row = app.view.workspace_card_areas[1].rect.y;
        let mut terminal = Terminal::new(TestBackend::new(26, 20)).unwrap();
        terminal
            .draw(|frame| render_sidebar(&app, &TerminalRuntimeRegistry::new(), frame, area))
            .unwrap();
        let buffer = terminal.backend().buffer();

        let active = buffer[(find_symbol_x(buffer, first_row, 25, "o"), first_row)].style();
        assert_eq!(active.fg, Some(app.palette.text));
        assert!(active.add_modifier.contains(Modifier::BOLD));
        assert!(!active.add_modifier.contains(Modifier::DIM));
        assert_eq!(active.bg, Some(app.palette.surface_dim));

        let inactive = buffer[(find_symbol_x(buffer, second_row, 25, "t"), second_row)].style();
        assert_eq!(inactive.fg, Some(app.palette.subtext0));
        assert!(!inactive
            .add_modifier
            .intersects(Modifier::BOLD | Modifier::DIM));
        assert_eq!(inactive.bg, Some(ratatui::style::Color::Reset));
    }

    #[test]
    fn space_occurrence_style_applies_without_styling_separator() {
        let config: crate::config::Config = toml::from_str(
            r##"
[ui.sidebar.spaces]
rows = [[{ token = "$hype", fg = "#abcdef", bold = true, dim = false }, "workspace"]]
"##,
        )
        .unwrap();
        let mut app = crate::app::state::AppState::test_new();
        app.sidebar_spaces = config.ui.sidebar.spaces;
        app.workspaces = vec![Workspace::test_new("one")];
        app.active = Some(0);
        app.mode = Mode::Terminal;
        app.workspaces[0].metadata_tokens.patch(
            std::collections::HashMap::from([("hype".into(), Some("HI".into()))]),
            None,
            std::time::Instant::now(),
        );

        let area = Rect::new(0, 0, 26, 20);
        app.view.workspace_card_areas = compute_workspace_card_areas(&app, area);
        let row = app.view.workspace_card_areas[0].rect.y;
        let mut terminal = Terminal::new(TestBackend::new(26, 20)).unwrap();
        terminal
            .draw(|frame| render_sidebar(&app, &TerminalRuntimeRegistry::new(), frame, area))
            .unwrap();
        let buffer = terminal.backend().buffer();
        let h = buffer[(find_symbol_x(buffer, row, 25, "H"), row)].style();
        let i = buffer[(find_symbol_x(buffer, row, 25, "I"), row)].style();
        let separator = buffer[(find_symbol_x(buffer, row, 25, "·"), row)].style();

        for style in [h, i] {
            assert_eq!(style.fg, Some(ratatui::style::Color::Rgb(0xab, 0xcd, 0xef)));
            assert!(style.add_modifier.contains(Modifier::BOLD));
            assert!(!style.add_modifier.contains(Modifier::DIM));
            assert_eq!(style.bg, Some(app.palette.surface_dim));
        }
        assert_eq!(separator.fg, Some(app.palette.overlay0));
        assert!(separator.add_modifier.contains(Modifier::DIM));
        assert!(!separator.add_modifier.contains(Modifier::BOLD));
        assert_eq!(separator.bg, Some(app.palette.surface_dim));
    }

    #[test]
    fn occurrence_foreground_flattens_composite_git_status_colors() {
        let config: crate::config::Config = toml::from_str(
            r##"[ui.sidebar.spaces]
rows = [[{ token = "git_status", fg = "#123456" }]]
"##,
        )
        .unwrap();
        let spans = resolved_token_spans(
            &[ResolvedToken {
                kind: ResolvedTokenKind::GitStatus {
                    ahead: 2,
                    behind: 1,
                },
                style: config.ui.sidebar.spaces.rows[0][0].parts().1,
            }],
            ("", Style::default()),
            Style::default(),
            Style::default(),
            Style::default(),
            Style::default(),
            &crate::app::state::AppState::test_new().palette,
            20,
        );

        assert_eq!(
            spans
                .iter()
                .map(|span| span.content.as_ref())
                .collect::<String>(),
            "↑2 ↓1"
        );
        assert!(spans
            .iter()
            .all(|span| { span.style.fg == Some(ratatui::style::Color::Rgb(0x12, 0x34, 0x56)) }));
    }

    #[test]
    fn default_agent_row_gap_packs_rendering_and_scroll_geometry() {
        let mut app = crate::app::state::AppState::test_new();
        app.workspaces = vec![Workspace::test_new("one"), Workspace::test_new("two")];
        app.ensure_test_terminals();
        for (workspace, agent) in app.workspaces.iter().zip([Agent::Pi, Agent::Claude]) {
            let pane_id = workspace.tabs[0].root_pane;
            let terminal_id = workspace.tabs[0].panes[&pane_id]
                .attached_terminal_id
                .clone();
            app.terminals.get_mut(&terminal_id).unwrap().detected_agent = Some(agent);
        }
        app.sidebar_agents.rows = vec![vec![crate::config::AgentSidebarToken::Agent]];
        assert_eq!(app.sidebar_agents.row_gap, 0);

        let area = Rect::new(0, 0, 20, 10);
        let metrics = workspace_list_scroll_metrics(&app, workspace_list_rect(area));
        let body = merged_body(&app, area);
        let mut terminal = Terminal::new(TestBackend::new(20, 10)).unwrap();
        terminal
            .draw(|frame| render_sidebar(&app, &TerminalRuntimeRegistry::new(), frame, area))
            .unwrap();
        let buffer = terminal.backend().buffer();

        // space, agent, space, agent -- four rows, no gap between an agent and
        // the space it belongs to.
        let agent_rows = agent_row_rects(&app, area);
        assert_eq!(metrics.viewport_rows, 4);
        assert_eq!(metrics.max_offset_from_bottom, 0);
        assert!(row_text(buffer, body.y, body.width).contains("one"));
        assert_eq!(row_text(buffer, agent_rows[0].rect.y, body.width), "   pi");
        assert_eq!(
            row_text(buffer, agent_rows[1].rect.y, body.width),
            "   claude"
        );
        // Each agent row starts where its space row ends -- no gap between them.
        let (cards, _) = compute_workspace_list_areas(&app, &agent_panel_entries(&app), area);
        assert_eq!(agent_rows[0].rect.y, cards[0].rect.y + cards[0].rect.height);
        assert_eq!(agent_rows[1].rect.y, cards[1].rect.y + cards[1].rect.height);
    }

    #[test]
    fn computed_view_agent_cache_drives_metrics_and_render() {
        let mut app = crate::app::state::AppState::test_new();
        app.workspaces = vec![Workspace::test_new("one"), Workspace::test_new("two")];
        app.ensure_test_terminals();
        for (workspace, agent) in app.workspaces.iter().zip([Agent::Pi, Agent::Claude]) {
            let pane_id = workspace.tabs[0].root_pane;
            let terminal_id = workspace.tabs[0].panes[&pane_id]
                .attached_terminal_id
                .clone();
            app.terminals.get_mut(&terminal_id).unwrap().detected_agent = Some(agent);
        }
        app.sidebar_agents.rows = vec![vec![crate::config::AgentSidebarToken::Agent]];

        let area = Rect::new(0, 0, 20, 10);
        let mut cached = agent_panel_entries(&app);
        cached.truncate(1);
        let (cards, agent_rows) = compute_workspace_list_areas(&app, &cached, area);
        app.view.sidebar_rect = area;
        app.view.agent_panel_entries = cached;
        app.view.workspace_card_areas = cards;
        app.view.agent_row_areas = agent_rows;

        let body = merged_body(&app, area);
        let mut terminal = Terminal::new(TestBackend::new(20, 10)).unwrap();
        terminal
            .draw(|frame| render_sidebar(&app, &TerminalRuntimeRegistry::new(), frame, area))
            .unwrap();
        let buffer = terminal.backend().buffer();

        // The cached frame knows one agent, so the second space renders its
        // empty state rather than the agent the live derivation would find.
        let rows = app.view.agent_row_areas.clone();
        assert_eq!(rows.len(), 2);
        assert_eq!(row_text(buffer, rows[0].rect.y, body.width), "   pi");
        assert_eq!(rows[1].entry_idx, None);
        assert_eq!(row_text(buffer, rows[1].rect.y, body.width), "   no agents");
    }

    #[test]
    fn narrow_agent_rows_preserve_later_tab_tokens() {
        let mut app = crate::app::state::AppState::test_new();
        let mut workspace = Workspace::test_new("very-long-workspace-name");
        let tab_idx = workspace.test_add_tab(Some("logs"));
        let pane_id = workspace.tabs[tab_idx].root_pane;
        app.workspaces = vec![workspace];
        app.ensure_test_terminals();
        let terminal_id = app.workspaces[0].tabs[tab_idx].panes[&pane_id]
            .attached_terminal_id
            .clone();
        app.terminals.get_mut(&terminal_id).unwrap().detected_agent = Some(Agent::Pi);

        let area = Rect::new(0, 0, 18, 20);
        let mut terminal = Terminal::new(TestBackend::new(18, 20)).unwrap();
        terminal
            .draw(|frame| render_sidebar(&app, &TerminalRuntimeRegistry::new(), frame, area))
            .unwrap();
        let buffer = terminal.backend().buffer();
        let agent_y = agent_row_rects(&app, area)[0].rect.y;
        let first = row_text(buffer, agent_y, 17);

        assert!(first.contains("logs"), "rendered row: {first:?}");
        assert!(first.contains('·'), "rendered row: {first:?}");
    }

    #[test]
    fn stripped_terminal_title_renders_with_unicode_width_truncation() {
        let mut app = crate::app::state::AppState::test_new();
        let workspace = Workspace::test_new("one");
        let pane_id = workspace.tabs[0].root_pane;
        app.workspaces = vec![workspace];
        app.ensure_test_terminals();
        let terminal_id = app.workspaces[0].tabs[0].panes[&pane_id]
            .attached_terminal_id
            .clone();
        let terminal = app.terminals.get_mut(&terminal_id).unwrap();
        terminal.detected_agent = Some(Agent::Claude);
        terminal.set_terminal_title(Some("⠋ 修复🙂标题很长".into()));
        app.sidebar_agents.rows = vec![vec![
            crate::config::AgentSidebarToken::TerminalTitleStripped,
        ]];

        let area = Rect::new(0, 0, 10, 12);
        let mut renderer = Terminal::new(TestBackend::new(10, 12)).unwrap();
        renderer
            .draw(|frame| render_sidebar(&app, &TerminalRuntimeRegistry::new(), frame, area))
            .unwrap();
        let agent_y = agent_row_rects(&app, area)[0].rect.y;
        let rendered = row_text(renderer.backend().buffer(), agent_y, 9);

        assert!(!rendered.contains('⠋'));
        assert!(rendered.contains('修') && rendered.contains('复'));

        let spans = resolved_token_spans(
            &[ResolvedToken::unstyled(ResolvedTokenKind::TerminalTitle(
                "修复🙂标题很长".into(),
            ))],
            ("", Style::default()),
            Style::default(),
            Style::default(),
            Style::default(),
            Style::default(),
            &app.palette,
            8,
        );
        let text = spans
            .iter()
            .map(|span| span.content.as_ref())
            .collect::<String>();
        assert!(display_width(&text) <= 8, "resolved title: {text:?}");
    }

    #[test]
    fn variable_agent_heights_pack_the_bottom_and_reveal_targets() {
        let mut app = crate::app::state::AppState::test_new();
        app.workspaces = vec![
            Workspace::test_new("one"),
            Workspace::test_new("two"),
            Workspace::test_new("three"),
        ];
        app.ensure_test_terminals();
        for workspace in &app.workspaces {
            let pane_id = workspace.tabs[0].root_pane;
            let terminal_id = workspace.tabs[0].panes[&pane_id]
                .attached_terminal_id
                .clone();
            app.terminals.get_mut(&terminal_id).unwrap().detected_agent = Some(Agent::Pi);
        }
        let first_pane = app.workspaces[0].tabs[0].root_pane;
        let first_terminal = app.workspaces[0].tabs[0].panes[&first_pane]
            .attached_terminal_id
            .clone();
        app.terminals
            .get_mut(&first_terminal)
            .unwrap()
            .metadata_tokens
            .patch(
                std::collections::HashMap::from([
                    ("a".into(), Some("a".into())),
                    ("b".into(), Some("b".into())),
                ]),
                None,
                std::time::Instant::now(),
            );
        app.sidebar_agents.rows = vec![
            vec![crate::config::AgentSidebarToken::Agent],
            vec![crate::config::AgentSidebarToken::Custom("a".into())],
            vec![crate::config::AgentSidebarToken::Custom("b".into())],
        ];
        let area = Rect::new(0, 0, 20, 9);
        let list_area = workspace_list_rect(area);

        let metrics = workspace_list_scroll_metrics(&app, list_area);
        assert!(metrics.max_offset_from_bottom > 0);
        // Scrolling to the last agent parks it inside the viewport.
        let scroll = sidebar_scroll_for_agent_entry(&app, list_area, 0, 2);
        assert!(scroll > 0);
        assert!(scroll <= metrics.max_offset_from_bottom);
    }

    #[test]
    fn oversized_space_layout_is_clipped_to_the_section_body() {
        let mut app = crate::app::state::AppState::test_new();
        app.workspaces = vec![Workspace::test_new("one"), Workspace::test_new("two")];
        app.sidebar_spaces.rows = vec![vec![crate::config::SpaceSidebarToken::Workspace]; 6];
        let area = Rect::new(0, 0, 20, 10);
        let workspace_area = workspace_list_rect(area);
        let body = workspace_list_body_rect(workspace_area, false);

        let metrics = workspace_list_scroll_metrics(&app, workspace_area);
        let (cards, _) = compute_workspace_list_areas(&app, &agent_panel_entries(&app), area);

        // The clipped space row plus its empty-state row fill the body.
        assert_eq!(metrics.viewport_rows, 2);
        assert_eq!(cards.len(), 1);
        assert_eq!(cards[0].ws_idx, 0);
        assert_eq!(cards[0].rect.height, body.height.saturating_sub(1));
    }

    #[test]
    fn oversized_agent_override_is_clipped_to_the_panel_body() {
        let mut app = crate::app::state::AppState::test_new();
        let workspace = Workspace::test_new("one");
        let pane_id = workspace.tabs[0].root_pane;
        app.workspaces = vec![workspace];
        app.ensure_test_terminals();
        let terminal_id = app.workspaces[0].tabs[0].panes[&pane_id]
            .attached_terminal_id
            .clone();
        app.terminals.get_mut(&terminal_id).unwrap().detected_agent = Some(Agent::Claude);
        app.sidebar_agents.rows_by_agent.insert(
            "claude".into(),
            vec![vec![crate::config::AgentSidebarToken::Agent]; 6],
        );
        let sidebar = Rect::new(0, 0, 20, 12);
        let body = merged_body(&app, sidebar);

        // The oversized agent row is clipped to the merged body rather than
        // overrunning it.
        let (_, agent_rows) =
            compute_workspace_list_areas(&app, &agent_panel_entries(&app), sidebar);
        assert_eq!(agent_rows.len(), 1);
        assert!(agent_rows[0].rect.height <= body.height);
        assert!(agent_rows[0].rect.y + agent_rows[0].rect.height <= body.y + body.height);
    }

    #[test]
    fn render_sidebar_toggle_draws_expanded_collapse_icon() {
        let app = crate::app::state::AppState::test_new();
        let area = Rect::new(0, 0, 26, 20);
        let mut terminal =
            Terminal::new(TestBackend::new(26, 20)).expect("test terminal should initialize");

        terminal
            .draw(|frame| render_sidebar_toggle(&app, frame, area, false, &app.palette))
            .expect("sidebar toggle should render");

        let toggle = expanded_sidebar_toggle_rect(area);
        assert_eq!(
            terminal.backend().buffer()[(toggle.x, toggle.y)].symbol(),
            "«"
        );
    }

    #[test]
    fn expanded_sidebar_toggle_sits_inside_sidebar_content() {
        let area = Rect::new(0, 0, 26, 20);
        let toggle = expanded_sidebar_toggle_rect(area);

        assert_eq!(toggle.x, area.x + area.width - 2);
        assert_eq!(toggle.y, area.y + area.height - 1);
    }

    #[test]
    fn agent_panel_tab_label_visibility_tracks_tab_identity() {
        let mut app = crate::app::state::AppState::test_new();
        let single_auto = Workspace::test_new("auto");
        let mut single_custom = Workspace::test_new("custom");
        single_custom.tabs[0].set_custom_name("focus".into());
        let mut multi = Workspace::test_new("multi");
        multi.test_add_tab(Some("logs"));

        app.workspaces = vec![single_auto, single_custom, multi];
        app.ensure_test_terminals();
        for (ws_idx, tab_idx, agent) in [
            (0, 0, Agent::Pi),
            (1, 0, Agent::Claude),
            (2, 0, Agent::Codex),
            (2, 1, Agent::Pi),
        ] {
            let pane_id = app.workspaces[ws_idx].tabs[tab_idx].root_pane;
            let terminal_id = app.workspaces[ws_idx].tabs[tab_idx].panes[&pane_id]
                .attached_terminal_id
                .clone();
            app.terminals.get_mut(&terminal_id).unwrap().detected_agent = Some(agent);
        }

        let entries = agent_panel_entries(&app);
        let labels: Vec<_> = entries
            .iter()
            .map(|entry| {
                (
                    entry.primary_label.as_str(),
                    entry.primary_tab_label.as_deref(),
                )
            })
            .collect();

        assert_eq!(
            labels,
            [
                ("auto", None),
                ("custom", Some("focus")),
                ("multi", Some("1")),
                ("multi", Some("logs")),
            ]
        );
    }

    #[test]
    fn priority_agent_panel_sort_uses_attention_then_space_order() {
        let mut app = crate::app::state::AppState::test_new();
        app.workspaces = vec![
            Workspace::test_new("one"),
            Workspace::test_new("two"),
            Workspace::test_new("three"),
            Workspace::test_new("four"),
        ];
        app.ensure_test_terminals();
        app.active = Some(0);
        app.selected = 0;
        app.agent_panel_sort = crate::app::state::AgentPanelSort::Priority;

        let set_state = |app: &mut crate::app::state::AppState, ws_idx: usize, state| {
            let pane = app.workspaces[ws_idx].tabs[0].root_pane;
            let terminal_id = app.workspaces[ws_idx].tabs[0].panes[&pane]
                .attached_terminal_id
                .clone();
            let terminal = app.terminals.get_mut(&terminal_id).unwrap();
            terminal.detected_agent = Some(Agent::Claude);
            terminal.state = state;
        };
        set_state(&mut app, 0, AgentState::Working);
        set_state(&mut app, 1, AgentState::Idle);
        set_state(&mut app, 2, AgentState::Working);
        set_state(&mut app, 3, AgentState::Blocked);

        let done_pane = app.workspaces[1].tabs[0].root_pane;
        app.workspaces[1].tabs[0]
            .panes
            .get_mut(&done_pane)
            .unwrap()
            .seen = false;

        let labels: Vec<String> = agent_panel_entries(&app)
            .into_iter()
            .map(|entry| entry.primary_label)
            .collect();

        assert_eq!(labels, ["four", "two", "one", "three"]);
    }

    #[test]
    fn collapsed_sidebar_numbers_grouped_agents_by_list_position() {
        let mut app = crate::app::state::AppState::test_new();
        app.workspaces = vec![Workspace::test_new("one"), Workspace::test_new("two")];
        app.ensure_test_terminals();

        for ws_idx in 0..app.workspaces.len() {
            let pane = app.workspaces[ws_idx].tabs[0].root_pane;
            let terminal_id = app.workspaces[ws_idx].tabs[0].panes[&pane]
                .attached_terminal_id
                .clone();
            app.terminals.get_mut(&terminal_id).unwrap().detected_agent = Some(Agent::Claude);
        }

        let area = Rect::new(0, 0, 4, 12);
        let content = collapsed_sidebar_content_rect(area);
        let mut terminal = Terminal::new(TestBackend::new(area.width, area.height))
            .expect("test terminal should initialize");

        terminal
            .draw(|frame| render_sidebar_collapsed(&app, frame, area))
            .expect("collapsed sidebar should render");

        // space, agent, space, agent -- agent numbers run by visible position
        // across the whole merged list, never restarting per space.
        let buffer = terminal.backend().buffer();
        assert_eq!(buffer[(content.x, content.y)].symbol(), "1");
        assert_eq!(buffer[(content.x, content.y + 1)].symbol(), "1");
        assert_eq!(buffer[(content.x, content.y + 2)].symbol(), "2");
        assert_eq!(buffer[(content.x, content.y + 3)].symbol(), "2");
    }

    #[test]
    fn collapsed_sidebar_keeps_status_visible_for_two_digit_positions() {
        let mut app = crate::app::state::AppState::test_new();
        app.workspaces = (1..=10)
            .map(|idx| Workspace::test_new(&format!("workspace-{idx}")))
            .collect();
        app.ensure_test_terminals();

        for ws_idx in 0..app.workspaces.len() {
            let pane = app.workspaces[ws_idx].tabs[0].root_pane;
            let terminal_id = app.workspaces[ws_idx].tabs[0].panes[&pane]
                .attached_terminal_id
                .clone();
            app.terminals.get_mut(&terminal_id).unwrap().detected_agent = Some(Agent::Claude);
        }

        let area = Rect::new(0, 0, 4, 25);
        let content = collapsed_sidebar_content_rect(area);
        let mut terminal = Terminal::new(TestBackend::new(area.width, area.height))
            .expect("test terminal should initialize");

        terminal
            .draw(|frame| render_sidebar_collapsed(&app, frame, area))
            .expect("collapsed sidebar should render");

        // Tenth agent sits on row 19: ten space rows interleaved with its nine
        // predecessors. Two digits must not push the status dot off the row.
        let tenth_agent_row = content.y + 19;
        let buffer = terminal.backend().buffer();
        assert_eq!(buffer[(content.x, tenth_agent_row)].symbol(), "1");
        assert_eq!(buffer[(content.x + 1, tenth_agent_row)].symbol(), "0");
        assert_eq!(buffer[(content.x + 2, tenth_agent_row)].symbol(), "·");
    }

    #[test]
    fn collapsed_sidebar_numbers_priority_agents_by_list_position() {
        let first = Workspace::test_new("one");
        let first_pane = first.tabs[0].root_pane;
        let mut second = Workspace::test_new("two");
        let second_pane = second.tabs[0].root_pane;
        let urgent_pane = second.test_split(ratatui::layout::Direction::Horizontal);

        let mut app = crate::app::state::AppState::test_new();
        app.workspaces = vec![first, second];
        app.ensure_test_terminals();
        app.agent_panel_sort = crate::app::state::AgentPanelSort::Priority;

        let set_state = |app: &mut crate::app::state::AppState, ws_idx: usize, pane_id, state| {
            let terminal_id = app.workspaces[ws_idx].tabs[0].panes[&pane_id]
                .attached_terminal_id
                .clone();
            let terminal = app.terminals.get_mut(&terminal_id).unwrap();
            terminal.detected_agent = Some(Agent::Claude);
            terminal.state = state;
        };
        set_state(&mut app, 0, first_pane, AgentState::Working);
        set_state(&mut app, 1, second_pane, AgentState::Working);
        set_state(&mut app, 1, urgent_pane, AgentState::Blocked);

        assert_eq!(app.workspaces[1].public_pane_number(urgent_pane), Some(2));
        assert_eq!(agent_panel_entries(&app)[0].pane_id, urgent_pane);

        let area = Rect::new(0, 0, 4, 16);
        let content = collapsed_sidebar_content_rect(area);
        let mut terminal = Terminal::new(TestBackend::new(area.width, area.height))
            .expect("test terminal should initialize");

        terminal
            .draw(|frame| render_sidebar_collapsed(&app, frame, area))
            .expect("collapsed sidebar should render");

        // Rows: space one, its agent, space two, then two's agents in priority
        // order -- the blocked one first within that space.
        let buffer = terminal.backend().buffer();
        assert_eq!(buffer[(content.x, content.y + 1)].symbol(), "1");
        assert_eq!(buffer[(content.x, content.y + 3)].symbol(), "2");
        assert_eq!(buffer[(content.x, content.y + 4)].symbol(), "3");
        assert_eq!(buffer[(content.x + 2, content.y + 3)].symbol(), "●");
        assert_eq!(
            buffer[(content.x + 2, content.y + 3)].style().fg,
            Some(app.palette.red)
        );
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn all_workspaces_agent_panel_entries_use_live_root_runtime_cwd_for_workspace_label() {
        let unique = format!(
            "shepherd-agent-panel-runtime-cwd-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        let root = std::env::temp_dir().join(unique);
        let stale_cwd = root.join("issue-264-nix-support");
        let live_cwd = root.join("shepherd");
        std::fs::create_dir_all(stale_cwd.join(".git")).unwrap();
        std::fs::create_dir_all(live_cwd.join(".git")).unwrap();

        let mut app = crate::app::state::AppState::test_new();
        let mut workspace = Workspace::test_new("stale-name");
        workspace.custom_name = None;
        workspace.identity_cwd = stale_cwd.clone();
        let pane = workspace.tabs[0].root_pane;

        app.workspaces = vec![workspace];
        app.ensure_test_terminals();
        let terminal_id = app.workspaces[0].tabs[0].panes[&pane]
            .attached_terminal_id
            .clone();
        let terminal = app.terminals.get_mut(&terminal_id).unwrap();
        terminal.cwd = stale_cwd;
        terminal.detected_agent = Some(Agent::Pi);
        app.active = Some(0);
        app.selected = 0;

        let (events, _) = tokio::sync::mpsc::channel(4);
        let runtime = crate::terminal::TerminalRuntime::spawn(
            pane,
            24,
            80,
            live_cwd.clone(),
            0,
            crate::terminal_theme::TerminalTheme::default(),
            crate::pane::PaneShellConfig::new("/bin/sh", crate::config::ShellModeConfig::NonLogin),
            &crate::pane::PaneLaunchEnv::default(),
            events,
            std::sync::Arc::new(tokio::sync::Notify::new()),
            std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
        )
        .unwrap();

        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
        while runtime.cwd() != Some(live_cwd.clone()) && std::time::Instant::now() < deadline {
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }

        let mut runtime_registry = TerminalRuntimeRegistry::new();
        runtime_registry.insert(terminal_id, runtime);
        let entries = agent_panel_entries_from(&app, &runtime_registry);
        let primary_label = entries[0].primary_label.clone();

        for (_, runtime) in runtime_registry.drain() {
            runtime.shutdown();
        }
        let _ = std::fs::remove_dir_all(root);

        assert_eq!(primary_label, "shepherd");
    }

    #[test]
    fn all_workspaces_agent_panel_entries_prefer_agent_names_for_agent_identity() {
        let mut app = crate::app::state::AppState::test_new();
        let workspace = Workspace::test_new("bridge");
        let first_pane = workspace.tabs[0].root_pane;

        app.workspaces = vec![workspace];
        app.ensure_test_terminals();
        let first_terminal_id = app.workspaces[0].tabs[0].panes[&first_pane]
            .attached_terminal_id
            .clone();
        app.terminals
            .get_mut(&first_terminal_id)
            .unwrap()
            .detected_agent = Some(Agent::Pi);
        app.terminals
            .get_mut(&first_terminal_id)
            .unwrap()
            .set_agent_name("planner".into());
        app.active = Some(0);
        app.selected = 0;

        let entries = agent_panel_entries(&app);
        assert_eq!(entries[0].primary_label, "bridge");
        assert_eq!(entries[0].agent_label.as_deref(), Some("planner"));
    }

    #[test]
    fn sidebar_list_claims_the_full_height_beside_the_separator() {
        assert_eq!(
            workspace_list_rect(Rect::new(0, 0, 20, 5)),
            Rect::new(0, 0, 19, 5)
        );
    }

    #[test]
    fn grouped_child_label_keeps_custom_workspace_name() {
        assert_eq!(
            grouped_child_display_label("renamed issue", Some("worktree/issue-137"), true),
            "renamed issue"
        );
    }

    #[test]
    fn grouped_child_label_uses_short_branch_for_auto_named_workspace() {
        assert_eq!(
            grouped_child_display_label("shepherd-issue", Some("worktree/issue-137"), false),
            "issue-137"
        );
    }

    #[test]
    fn workspace_list_truncates_cjk_branch_without_panic() {
        let mut app = crate::app::state::AppState::test_new();
        let mut ws = Workspace::test_new("repo");
        ws.cached_git_branch = Some("feature/中文-分支-644".into());
        app.workspaces = vec![ws];
        app.active = Some(0);
        app.selected = 0;
        app.mode = Mode::Terminal;
        app.view.workspace_card_areas = vec![crate::app::state::WorkspaceCardArea {
            ws_idx: 0,
            rect: Rect::new(0, 1, 15, 2),
            indented: false,
        }];

        let mut terminal = Terminal::new(TestBackend::new(15, 6)).expect("test terminal");
        let runtimes = crate::terminal::TerminalRuntimeRegistry::new();

        terminal
            .draw(|frame| {
                render_workspace_list(&app, &runtimes, frame, Rect::new(0, 0, 15, 6), false)
            })
            .expect("workspace list should render");
    }

    fn workspace_with_worktree_space(
        name: &str,
        key: Option<&str>,
        checkout_key: &str,
    ) -> crate::workspace::Workspace {
        let mut ws = crate::workspace::Workspace::test_new(name);
        if let Some(key) = key {
            ws.worktree_space = Some(crate::workspace::WorktreeSpaceMembership {
                key: key.into(),
                label: "shepherd".into(),
                repo_root: std::path::PathBuf::from("/repo/shepherd"),
                checkout_path: std::path::PathBuf::from(checkout_key),
                is_linked_worktree: name != "main",
            });
        }
        ws
    }

    fn workspace_with_git_space(name: &str, key: &str) -> crate::workspace::Workspace {
        let mut ws = crate::workspace::Workspace::test_new(name);
        ws.cached_git_space = Some(crate::workspace::GitSpaceMetadata {
            key: key.into(),
            checkout_key: format!("/repo/{name}"),
            repo_name: "shepherd".into(),
            repo_root: std::path::PathBuf::from(format!("/repo/{name}")),
            is_linked_worktree: false,
        });
        ws
    }

    #[test]
    fn desktop_worktree_tree_aligns_parents_and_marks_children() {
        let mut app = AppState::test_new();
        app.workspaces = vec![
            workspace_with_worktree_space("main", Some("repo-key"), "/repo/shepherd"),
            workspace_with_worktree_space("issue", Some("repo-key"), "/repo/shepherd-issue"),
            workspace_with_worktree_space("review", Some("repo-key"), "/repo/shepherd-review"),
            Workspace::test_new("notes"),
        ];
        app.sidebar_spaces.rows = vec![vec![
            crate::config::SpaceSidebarToken::StateIcon,
            crate::config::SpaceSidebarToken::Workspace,
        ]];
        app.sidebar_spaces.row_gap = 0;
        let area = Rect::new(0, 0, 30, 20);
        app.view.workspace_card_areas = compute_workspace_card_areas(&app, area);

        let mut terminal = Terminal::new(TestBackend::new(area.width, area.height)).unwrap();
        terminal
            .draw(|frame| {
                render_workspace_list(&app, &TerminalRuntimeRegistry::new(), frame, area, false)
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let cards = &app.view.workspace_card_areas;
        let parent_name_x = find_symbol_x(buffer, cards[0].rect.y, cards[0].rect.width, "m");
        let plain_name_x = find_symbol_x(buffer, cards[3].rect.y, cards[3].rect.width, "n");
        assert_eq!(parent_name_x, plain_name_x);
        assert_eq!(buffer[(cards[1].rect.x + 3, cards[1].rect.y)].symbol(), "├");
        assert_eq!(buffer[(cards[2].rect.x + 3, cards[2].rect.y)].symbol(), "└");
        assert_eq!(
            buffer[(cards[0].rect.x + cards[0].rect.width - 1, cards[0].rect.y)].symbol(),
            "▾"
        );
    }

    #[test]
    fn desktop_worktree_connector_uses_full_list_at_viewport_boundary() {
        let mut app = AppState::test_new();
        app.workspaces = vec![
            workspace_with_worktree_space("main", Some("repo-key"), "/repo/shepherd"),
            workspace_with_worktree_space("issue", Some("repo-key"), "/repo/shepherd-issue"),
            workspace_with_worktree_space("review", Some("repo-key"), "/repo/shepherd-review"),
        ];
        app.sidebar_spaces.rows = vec![vec![crate::config::SpaceSidebarToken::Workspace]];
        app.sidebar_spaces.row_gap = 0;
        let area = Rect::new(0, 0, 30, 7);
        app.view.workspace_card_areas = compute_workspace_card_areas(&app, area);
        assert_eq!(app.view.workspace_card_areas.len(), 2);

        let mut terminal = Terminal::new(TestBackend::new(area.width, area.height)).unwrap();
        terminal
            .draw(|frame| {
                render_workspace_list(&app, &TerminalRuntimeRegistry::new(), frame, area, false)
            })
            .unwrap();

        let child = app.view.workspace_card_areas[1];
        assert_eq!(
            terminal.backend().buffer()[(child.rect.x + 3, child.rect.y)].symbol(),
            "├"
        );
    }

    #[test]
    fn parent_workspace_row_stays_clickable_when_grouped() {
        let mut app = AppState::test_new();
        app.workspaces = vec![
            workspace_with_worktree_space("main", Some("repo-key"), "/repo/shepherd"),
            workspace_with_worktree_space("issue", Some("repo-key"), "/repo/shepherd-issue"),
        ];
        app.sidebar_spaces.row_gap = 1;

        let (cards, _agent_rows) =
            compute_workspace_list_areas(&app, &agent_panel_entries(&app), Rect::new(0, 0, 30, 20));

        assert_eq!(cards[0].ws_idx, 0);
        assert!(!cards[0].indented);
        assert_eq!(cards[1].ws_idx, 1);
        assert!(cards[1].indented);
        // The parent's empty-state row sits between the two space rows.
        assert_eq!(
            cards[1].rect.y,
            cards[0].rect.y + cards[0].rect.height + 1 + 1
        );
    }

    #[test]
    fn space_row_gap_preserves_compact_worktree_children() {
        let mut app = AppState::test_new();
        app.workspaces = vec![
            workspace_with_worktree_space("main", Some("repo-key"), "/repo/shepherd"),
            workspace_with_worktree_space("issue", Some("repo-key"), "/repo/shepherd-issue"),
            workspace_with_worktree_space("review", Some("repo-key"), "/repo/shepherd-review"),
            Workspace::test_new("notes"),
        ];
        app.sidebar_spaces.rows = vec![vec![crate::config::SpaceSidebarToken::Workspace]];
        app.sidebar_spaces.row_gap = 2;

        let (spacious, _) =
            compute_workspace_list_areas(&app, &agent_panel_entries(&app), Rect::new(0, 0, 30, 30));
        // Each space carries a one-row empty state, then the configured gap --
        // except between worktree siblings, which stay compact.
        assert_eq!(
            spacious[1].rect.y,
            spacious[0].rect.y + spacious[0].rect.height + 1 + 2
        );
        assert_eq!(
            spacious[2].rect.y,
            spacious[1].rect.y + spacious[1].rect.height + 1
        );
        assert_eq!(
            spacious[3].rect.y,
            spacious[2].rect.y + spacious[2].rect.height + 1 + 2
        );
        let spacious_metrics = workspace_list_scroll_metrics(&app, Rect::new(0, 0, 30, 7));
        assert_eq!(spacious_metrics.viewport_rows, 2);
        assert_eq!(spacious_metrics.max_offset_from_bottom, 6);

        app.sidebar_spaces.row_gap = 0;
        let (packed, _) =
            compute_workspace_list_areas(&app, &agent_panel_entries(&app), Rect::new(0, 0, 30, 30));
        assert!(packed
            .windows(2)
            .all(|pair| pair[1].rect.y == pair[0].rect.y + pair[0].rect.height + 1));
        let packed_metrics = workspace_list_scroll_metrics(&app, Rect::new(0, 0, 30, 7));
        assert_eq!(packed_metrics.viewport_rows, 4);
        assert_eq!(packed_metrics.max_offset_from_bottom, 4);
    }

    #[test]
    fn packed_workspace_drag_indicator_overlays_an_internal_boundary() {
        let mut app = AppState::test_new();
        app.workspaces = vec![
            Workspace::test_new("a"),
            Workspace::test_new("b"),
            Workspace::test_new("c"),
        ];
        app.sidebar_spaces.rows = vec![vec![crate::config::SpaceSidebarToken::Workspace]];
        app.sidebar_spaces.row_gap = 0;
        let area = Rect::new(0, 0, 30, 20);
        app.view.workspace_card_areas = compute_workspace_card_areas(&app, area);
        let list_area = workspace_list_rect(area);
        let indicator_row = workspace_drop_indicator_row(
            &app,
            &app.view.workspace_card_areas,
            list_area,
            crate::app::state::WorkspaceDropTarget::Before(2),
        )
        .unwrap();
        // Packed rows leave no gap, so the indicator overlays the row directly
        // above the space it precedes -- that space's predecessor's last row.
        assert_eq!(
            indicator_row,
            app.view.workspace_card_areas[2].rect.y.saturating_sub(1)
        );
        app.drag = Some(crate::app::state::DragState {
            target: crate::app::state::DragTarget::WorkspaceReorder {
                source_ws_idx: 0,
                drop_target: Some(crate::app::state::WorkspaceDropTarget::Before(2)),
            },
        });

        let mut terminal = Terminal::new(TestBackend::new(area.width, area.height)).unwrap();
        terminal
            .draw(|frame| {
                render_workspace_list(&app, &TerminalRuntimeRegistry::new(), frame, area, false)
            })
            .unwrap();

        assert_eq!(
            terminal.backend().buffer()[(list_area.x, indicator_row)].symbol(),
            "─"
        );
    }

    #[test]
    fn linked_only_worktree_members_do_not_form_parentless_group() {
        let mut app = AppState::test_new();
        app.workspaces = vec![
            workspace_with_worktree_space("issue", Some("repo-key"), "/repo/shepherd-issue"),
            workspace_with_worktree_space("review", Some("repo-key"), "/repo/shepherd-review"),
        ];

        let entries = workspace_list_entries(&app);

        assert_eq!(
            entries,
            vec![
                WorkspaceListEntry::Workspace {
                    ws_idx: 0,
                    indented: false
                },
                WorkspaceListEntry::Workspace {
                    ws_idx: 1,
                    indented: false
                },
            ]
        );
    }

    #[test]
    fn compact_space_group_scroll_clamps_when_all_entries_fit() {
        let mut app = AppState::test_new();
        app.workspaces = vec![
            workspace_with_worktree_space("main", Some("repo-key"), "/repo/shepherd"),
            workspace_with_worktree_space("one", Some("repo-key"), "/repo/shepherd-one"),
            workspace_with_worktree_space("two", Some("repo-key"), "/repo/shepherd-two"),
        ];
        let area = Rect::new(0, 0, 30, 20);
        app.workspace_scroll = normalized_workspace_scroll(&app, area, 2);

        let (cards, _agent_rows) =
            compute_workspace_list_areas(&app, &agent_panel_entries(&app), area);

        assert_eq!(app.workspace_scroll, 0);
        assert_eq!(cards.len(), 3);
        assert_eq!(cards[2].ws_idx, 2);
    }

    #[test]
    fn workspace_scroll_metrics_count_display_entries_not_raw_workspaces() {
        let mut app = AppState::test_new();
        app.workspaces = vec![
            workspace_with_worktree_space("main", Some("repo-key"), "/repo/shepherd"),
            workspace_with_worktree_space("issue", Some("repo-key"), "/repo/shepherd-issue"),
            Workspace::test_new("notes"),
        ];
        for workspace in &mut app.workspaces {
            workspace.cached_git_branch = Some("main".into());
        }
        app.collapsed_space_keys.insert("repo-key".into());
        app.active = None;
        app.mode = Mode::Terminal;

        let ws_area = Rect::new(0, 0, 30, 6);
        let metrics = workspace_list_scroll_metrics(&app, ws_area);

        assert_eq!(metrics.viewport_rows, 1);
        assert_eq!(metrics.max_offset_from_bottom, 1);
        assert_eq!(metrics.offset_from_bottom, 1);
    }

    #[test]
    fn workspace_scroll_offset_applies_to_group_children() {
        let mut app = AppState::test_new();
        app.workspaces = vec![
            workspace_with_worktree_space("main", Some("repo-key"), "/repo/shepherd"),
            workspace_with_worktree_space("issue", Some("repo-key"), "/repo/shepherd-issue"),
            Workspace::test_new("notes"),
        ];
        app.sidebar_spaces.rows = vec![vec![crate::config::SpaceSidebarToken::Workspace]];
        app.sidebar_spaces.row_gap = 0;
        app.collapsed_space_keys.insert("repo-key".into());
        app.active = None;
        app.mode = Mode::Terminal;
        app.workspace_scroll = 1;

        // Collapsed parent, then the ungrouped space and its empty state --
        // three rows into a two-row body, so the offset is legal.
        let (cards, _agent_rows) =
            compute_workspace_list_areas(&app, &agent_panel_entries(&app), Rect::new(0, 0, 30, 5));

        assert_eq!(cards.len(), 1);
        assert_eq!(cards[0].ws_idx, 2);
    }

    #[test]
    fn workspace_list_entries_group_multiple_workspaces_in_same_git_space() {
        let mut app = AppState::test_new();
        app.workspaces = vec![
            workspace_with_worktree_space("main", Some("repo-key"), "/repo/shepherd"),
            workspace_with_worktree_space("issue", Some("repo-key"), "/repo/shepherd-issue"),
        ];

        assert_eq!(
            workspace_list_entries(&app),
            vec![
                WorkspaceListEntry::Workspace {
                    ws_idx: 0,
                    indented: false,
                },
                WorkspaceListEntry::Workspace {
                    ws_idx: 1,
                    indented: true,
                },
            ]
        );
    }

    #[test]
    fn workspace_list_entries_group_non_contiguous_explicit_members() {
        let mut app = AppState::test_new();
        app.workspaces = vec![
            workspace_with_worktree_space("main", Some("repo-key"), "/repo/shepherd"),
            workspace_with_git_space("normal", "other-key"),
            workspace_with_worktree_space("issue", Some("repo-key"), "/repo/shepherd-issue"),
        ];

        assert_eq!(
            workspace_list_entries(&app),
            vec![
                WorkspaceListEntry::Workspace {
                    ws_idx: 0,
                    indented: false,
                },
                WorkspaceListEntry::Workspace {
                    ws_idx: 2,
                    indented: true,
                },
                WorkspaceListEntry::Workspace {
                    ws_idx: 1,
                    indented: false,
                },
            ]
        );
    }

    #[test]
    fn workspace_list_entries_do_not_group_normal_git_workspaces() {
        let mut app = AppState::test_new();
        app.workspaces = vec![
            workspace_with_git_space("one", "repo-key"),
            workspace_with_git_space("two", "repo-key"),
        ];

        assert_eq!(
            workspace_list_entries(&app),
            vec![
                WorkspaceListEntry::Workspace {
                    ws_idx: 0,
                    indented: false,
                },
                WorkspaceListEntry::Workspace {
                    ws_idx: 1,
                    indented: false,
                },
            ]
        );
    }

    #[test]
    fn workspace_list_entries_do_not_auto_attach_normal_git_workspace_to_group() {
        let mut app = AppState::test_new();
        app.workspaces = vec![
            workspace_with_worktree_space("main", Some("repo-key"), "/repo/shepherd"),
            workspace_with_git_space("scratch", "repo-key"),
            workspace_with_worktree_space("issue", Some("repo-key"), "/repo/shepherd-issue"),
        ];

        assert_eq!(
            workspace_list_entries(&app),
            vec![
                WorkspaceListEntry::Workspace {
                    ws_idx: 0,
                    indented: false,
                },
                WorkspaceListEntry::Workspace {
                    ws_idx: 2,
                    indented: true,
                },
                WorkspaceListEntry::Workspace {
                    ws_idx: 1,
                    indented: false,
                },
            ]
        );
    }

    #[test]
    fn workspace_list_entries_leave_single_git_and_non_git_workspaces_flat() {
        let mut app = AppState::test_new();
        app.workspaces = vec![
            workspace_with_git_space("one", "repo-key"),
            workspace_with_worktree_space("notes", None, "/notes"),
        ];

        assert_eq!(
            workspace_list_entries(&app),
            vec![
                WorkspaceListEntry::Workspace {
                    ws_idx: 0,
                    indented: false,
                },
                WorkspaceListEntry::Workspace {
                    ws_idx: 1,
                    indented: false,
                },
            ]
        );
    }

    #[test]
    fn collapsed_group_hides_inactive_children_but_keeps_active_visible() {
        let mut app = AppState::test_new();
        app.workspaces = vec![
            workspace_with_worktree_space("main", Some("repo-key"), "/repo/shepherd"),
            workspace_with_worktree_space("issue", Some("repo-key"), "/repo/shepherd-issue"),
        ];
        app.active = Some(1);
        app.mode = Mode::Terminal;
        app.collapsed_space_keys.insert("repo-key".into());

        assert_eq!(
            workspace_list_entries(&app),
            vec![
                WorkspaceListEntry::Workspace {
                    ws_idx: 0,
                    indented: false,
                },
                WorkspaceListEntry::Workspace {
                    ws_idx: 1,
                    indented: true,
                },
            ]
        );

        app.active = None;
        app.mode = Mode::Terminal;
        assert_eq!(
            workspace_list_entries(&app),
            vec![WorkspaceListEntry::Workspace {
                ws_idx: 0,
                indented: false,
            }]
        );
    }

    #[test]
    fn collapsed_group_keeps_selected_child_visible_in_navigate_mode() {
        let mut app = AppState::test_new();
        app.workspaces = vec![
            workspace_with_worktree_space("main", Some("repo-key"), "/repo/shepherd"),
            workspace_with_worktree_space("issue", Some("repo-key"), "/repo/shepherd-issue"),
        ];
        app.mode = Mode::Navigate;
        app.selected = 1;
        app.active = Some(1);
        app.collapsed_space_keys.insert("repo-key".into());

        assert_eq!(
            workspace_list_entries(&app),
            vec![
                WorkspaceListEntry::Workspace {
                    ws_idx: 0,
                    indented: false,
                },
                WorkspaceListEntry::Workspace {
                    ws_idx: 1,
                    indented: true,
                },
            ]
        );
    }

    // -----------------------------------------------------------------
    // Agent panel sort header clarity (task 4.3)
    // -----------------------------------------------------------------

    #[test]
    fn agent_panel_header_labels_itself_as_a_sort_control() {
        let mut app = AppState::test_new();
        app.workspaces = vec![Workspace::test_new("one")];
        app.active = Some(0);
        let terminal_runtimes = TerminalRuntimeRegistry::default();

        for sort in [AgentPanelSort::Spaces, AgentPanelSort::Priority] {
            app.agent_panel_sort = sort;
            let mut terminal =
                Terminal::new(TestBackend::new(30, 24)).expect("test terminal should initialize");
            terminal
                .draw(|frame| {
                    render_sidebar(&app, &terminal_runtimes, frame, Rect::new(0, 0, 30, 24))
                })
                .expect("sidebar should render");
            let rendered = terminal
                .backend()
                .buffer()
                .content()
                .iter()
                .map(|cell| cell.symbol())
                .collect::<String>();
            let expected = format!("sort: {}", sort.label());
            assert!(
                rendered.contains(&expected),
                "expected header to contain {expected:?}, got: {rendered}"
            );
        }
    }

    // -----------------------------------------------------------------
    // Merged space-and-agent row model
    // -----------------------------------------------------------------

    /// Marks every pane of `ws_idx` as running an agent.
    fn detect_agents_in(app: &mut AppState, ws_idx: usize, agent: Agent) {
        let panes = app.workspaces[ws_idx]
            .tabs
            .iter()
            .enumerate()
            .flat_map(|(tab_idx, tab)| tab.panes.keys().map(move |pane_id| (tab_idx, *pane_id)))
            .collect::<Vec<_>>();
        for (tab_idx, pane_id) in panes {
            let terminal_id = app.workspaces[ws_idx].tabs[tab_idx].panes[&pane_id]
                .attached_terminal_id
                .clone();
            app.terminals.get_mut(&terminal_id).unwrap().detected_agent = Some(agent);
        }
    }

    #[test]
    fn merged_rows_put_each_space_ahead_of_its_own_agents() {
        let mut app = AppState::test_new();
        app.workspaces = vec![Workspace::test_new("one"), Workspace::test_new("two")];
        app.ensure_test_terminals();
        detect_agents_in(&mut app, 0, Agent::Pi);
        detect_agents_in(&mut app, 1, Agent::Claude);

        let entries = agent_panel_entries(&app);
        let rows = sidebar_rows(&app, &entries);

        assert_eq!(
            rows,
            vec![
                SidebarRow::Space {
                    ws_idx: 0,
                    indented: false
                },
                SidebarRow::Agent { entry_idx: 0 },
                SidebarRow::Space {
                    ws_idx: 1,
                    indented: false
                },
                SidebarRow::Agent { entry_idx: 1 },
            ]
        );
        assert_eq!(entries[0].ws_idx, 0);
        assert_eq!(entries[1].ws_idx, 1);
    }

    #[test]
    fn merged_rows_emit_no_tab_row_for_a_multi_tab_space() {
        let mut app = AppState::test_new();
        let mut workspace = Workspace::test_new("one");
        workspace.test_add_tab(Some("logs"));
        workspace.test_add_tab(Some("review"));
        app.workspaces = vec![workspace];
        app.ensure_test_terminals();
        detect_agents_in(&mut app, 0, Agent::Claude);

        let entries = agent_panel_entries(&app);
        let rows = sidebar_rows(&app, &entries);

        // One space row, then one row per agent -- the three tabs contribute no
        // rows of their own.
        assert_eq!(entries.len(), 3);
        assert_eq!(
            rows.iter()
                .filter(|row| matches!(row, SidebarRow::Space { .. }))
                .count(),
            1
        );
        assert!(matches!(rows[0], SidebarRow::Space { ws_idx: 0, .. }));
        assert!(rows[1..]
            .iter()
            .all(|row| matches!(row, SidebarRow::Agent { .. })));
    }

    #[test]
    fn a_space_with_no_detected_agent_emits_one_empty_state_row() {
        let mut app = AppState::test_new();
        app.workspaces = vec![Workspace::test_new("busy"), Workspace::test_new("quiet")];
        app.ensure_test_terminals();
        detect_agents_in(&mut app, 0, Agent::Pi);

        let entries = agent_panel_entries(&app);
        let rows = sidebar_rows(&app, &entries);

        assert_eq!(
            rows,
            vec![
                SidebarRow::Space {
                    ws_idx: 0,
                    indented: false
                },
                SidebarRow::Agent { entry_idx: 0 },
                SidebarRow::Space {
                    ws_idx: 1,
                    indented: false
                },
                SidebarRow::NoAgents { ws_idx: 1 },
            ]
        );

        // The empty-state row is not selectable as an agent.
        let area = Rect::new(0, 0, 26, 20);
        let agent_rows = agent_row_rects(&app, area);
        assert_eq!(agent_rows.len(), 2);
        assert_eq!(agent_rows[1].entry_idx, None);
    }

    #[test]
    fn a_collapsed_space_emits_no_child_rows_at_all() {
        let mut app = AppState::test_new();
        app.workspaces = vec![
            workspace_with_worktree_space("main", Some("repo-key"), "/repo/shepherd"),
            workspace_with_worktree_space("issue", Some("repo-key"), "/repo/shepherd-issue"),
            Workspace::test_new("quiet"),
        ];
        app.ensure_test_terminals();
        detect_agents_in(&mut app, 0, Agent::Claude);
        detect_agents_in(&mut app, 1, Agent::Pi);
        app.collapsed_space_keys.insert("repo-key".into());
        app.active = None;
        app.mode = Mode::Terminal;

        let entries = agent_panel_entries(&app);
        let rows = sidebar_rows(&app, &entries);

        // Collapse hides the worktree children AND every agent under the group,
        // so absence reads as collapse; the agentless space still shows its
        // empty state.
        assert_eq!(
            rows,
            vec![
                SidebarRow::Space {
                    ws_idx: 0,
                    indented: false
                },
                SidebarRow::Space {
                    ws_idx: 2,
                    indented: false
                },
                SidebarRow::NoAgents { ws_idx: 2 },
            ]
        );
    }

    #[test]
    fn merged_rows_keep_worktree_children_indented_under_their_parent() {
        let mut app = AppState::test_new();
        app.workspaces = vec![
            workspace_with_worktree_space("main", Some("repo-key"), "/repo/shepherd"),
            workspace_with_worktree_space("issue", Some("repo-key"), "/repo/shepherd-issue"),
        ];
        app.ensure_test_terminals();
        detect_agents_in(&mut app, 1, Agent::Pi);

        let entries = agent_panel_entries(&app);

        assert_eq!(
            sidebar_rows(&app, &entries),
            vec![
                SidebarRow::Space {
                    ws_idx: 0,
                    indented: false
                },
                SidebarRow::NoAgents { ws_idx: 0 },
                SidebarRow::Space {
                    ws_idx: 1,
                    indented: true
                },
                SidebarRow::Agent { entry_idx: 0 },
            ]
        );
    }

    #[test]
    fn agent_sort_reorders_rows_inside_each_space_from_one_source() {
        let mut app = AppState::test_new();
        let mut workspace = Workspace::test_new("one");
        let calm_pane = workspace.tabs[0].root_pane;
        let urgent_pane = workspace.test_split(ratatui::layout::Direction::Horizontal);
        app.workspaces = vec![workspace, Workspace::test_new("two")];
        app.ensure_test_terminals();
        detect_agents_in(&mut app, 0, Agent::Claude);
        detect_agents_in(&mut app, 1, Agent::Pi);
        let urgent_terminal = app.workspaces[0].tabs[0].panes[&urgent_pane]
            .attached_terminal_id
            .clone();
        app.terminals.get_mut(&urgent_terminal).unwrap().state = AgentState::Blocked;

        let grouped = agent_panel_entries(&app);
        let grouped_rows = sidebar_rows(&app, &grouped);
        let grouped_panes = grouped_rows
            .iter()
            .filter_map(|row| match row {
                SidebarRow::Agent { entry_idx } => Some(grouped[*entry_idx].pane_id),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(grouped_panes[0], calm_pane);

        app.agent_panel_sort = AgentPanelSort::Priority;
        let priority = agent_panel_entries(&app);
        let priority_rows = sidebar_rows(&app, &priority);
        let priority_panes = priority_rows
            .iter()
            .filter_map(|row| match row {
                SidebarRow::Agent { entry_idx } => Some(priority[*entry_idx].pane_id),
                _ => None,
            })
            .collect::<Vec<_>>();

        // The sort reorders agents within their own space; it never moves one
        // out from under the space row that owns it.
        assert_eq!(priority_panes[0], urgent_pane);
        assert!(matches!(
            priority_rows[0],
            SidebarRow::Space { ws_idx: 0, .. }
        ));
        assert!(matches!(
            priority_rows[3],
            SidebarRow::Space { ws_idx: 1, .. }
        ));
    }

    #[test]
    fn collapsed_sidebar_keeps_the_flat_space_then_agent_ordering() {
        let mut app = AppState::test_new();
        app.workspaces = vec![Workspace::test_new("busy"), Workspace::test_new("quiet")];
        app.ensure_test_terminals();
        detect_agents_in(&mut app, 0, Agent::Claude);

        let area = Rect::new(0, 0, 4, 12);
        let content = collapsed_sidebar_content_rect(area);
        let mut terminal = Terminal::new(TestBackend::new(area.width, area.height))
            .expect("test terminal should initialize");
        terminal
            .draw(|frame| render_sidebar_collapsed(&app, frame, area))
            .expect("collapsed sidebar should render");

        let buffer = terminal.backend().buffer();
        // space, its agent, space, its empty state.
        assert_eq!(buffer[(content.x, content.y)].symbol(), "1");
        assert_eq!(buffer[(content.x, content.y + 1)].symbol(), "1");
        assert_eq!(buffer[(content.x, content.y + 2)].symbol(), "2");
        assert_eq!(buffer[(content.x + 2, content.y + 3)].symbol(), "-");
    }
}
