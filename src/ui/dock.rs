use ratatui::{
    layout::Rect,
    style::Style,
    widgets::{Block, Borders, Clear},
    Frame,
};

use crate::{app::AppState, terminal::TerminalRuntimeRegistry};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct LiveDock {
    pub ws_idx: usize,
    pub tab_idx: usize,
    pub pane_id: crate::layout::PaneId,
    pub terminal_id: crate::terminal::TerminalId,
}

pub(crate) fn resolve_live_dock(
    app: &AppState,
    runtimes: &TerminalRuntimeRegistry,
) -> Option<LiveDock> {
    if !app.dock_enabled {
        return None;
    }
    let ws_idx = app.active?;
    let tab_idx = app.dock_backing_tab_idx(ws_idx)?;
    let pane_id = app.workspaces.get(ws_idx)?.tabs.get(tab_idx)?.root_pane;
    let terminal_id = app.workspaces[ws_idx]
        .tabs
        .get(tab_idx)?
        .panes
        .get(&pane_id)?
        .attached_terminal_id
        .clone();
    app.runtime_for_pane_in_workspace(runtimes, ws_idx, pane_id)?;
    Some(LiveDock {
        ws_idx,
        tab_idx,
        pane_id,
        terminal_id,
    })
}

pub(crate) fn dock_inner_rect(area: Rect) -> Rect {
    Rect::new(
        area.x.saturating_add(1),
        area.y.saturating_add(1),
        area.width.saturating_sub(2),
        area.height.saturating_sub(2),
    )
}

pub(super) fn resize_dock(
    app: &AppState,
    runtimes: &TerminalRuntimeRegistry,
    area: Rect,
    cell_size: crate::kitty_graphics::HostCellSize,
) {
    let Some(dock) = resolve_live_dock(app, runtimes) else {
        return;
    };
    let inner = dock_inner_rect(area);
    if let Some(runtime) = app.runtime_for_pane_in_workspace(runtimes, dock.ws_idx, dock.pane_id) {
        runtime.resize(
            inner.height,
            inner.width,
            cell_size.width_px,
            cell_size.height_px,
        );
    }
}

pub(super) fn render_dock(
    app: &AppState,
    runtimes: &TerminalRuntimeRegistry,
    frame: &mut Frame,
    area: Rect,
) {
    if area.is_empty() {
        return;
    }
    let Some(dock) = resolve_live_dock(app, runtimes) else {
        return;
    };
    let Some(runtime) = app.runtime_for_pane_in_workspace(runtimes, dock.ws_idx, dock.pane_id)
    else {
        return;
    };
    let inner = dock_inner_rect(area);
    frame.render_widget(Clear, area);
    let border_color = if app
        .view
        .dock_pane_info
        .as_ref()
        .is_some_and(|info| info.is_focused)
    {
        app.palette.accent
    } else {
        app.palette.overlay0
    };
    frame.render_widget(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border_color))
            .title(" dock "),
        area,
    );
    runtime.render(frame, inner, true);
}
