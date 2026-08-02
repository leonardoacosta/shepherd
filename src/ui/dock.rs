use ratatui::{
    layout::Rect,
    style::Style,
    widgets::{Block, Borders, Clear},
    Frame,
};

use crate::{app::AppState, terminal::TerminalRuntimeRegistry};

fn dock_terminal_id(app: &AppState) -> Option<&crate::terminal::TerminalId> {
    let ws_idx = app.active?;
    let workspace = app.workspaces.get(ws_idx)?;
    let dock = app.dock_panes.get(&workspace.id)?;
    let pane = app
        .workspaces
        .get(ws_idx)?
        .tabs
        .get(dock.tab_idx)?
        .panes
        .get(&dock.pane_id)?;
    Some(&pane.attached_terminal_id)
}

pub(super) fn resize_dock(
    app: &AppState,
    runtimes: &TerminalRuntimeRegistry,
    area: Rect,
    cell_size: crate::kitty_graphics::HostCellSize,
) {
    let Some(terminal_id) = dock_terminal_id(app) else {
        return;
    };
    let inner = Rect::new(
        area.x.saturating_add(1),
        area.y.saturating_add(1),
        area.width.saturating_sub(2),
        area.height.saturating_sub(2),
    );
    if let Some(runtime) = runtimes.get(terminal_id) {
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
    let Some(terminal_id) = dock_terminal_id(app) else {
        return;
    };
    let Some(runtime) = runtimes.get(terminal_id) else {
        return;
    };
    let inner = Rect::new(
        area.x.saturating_add(1),
        area.y.saturating_add(1),
        area.width.saturating_sub(2),
        area.height.saturating_sub(2),
    );
    frame.render_widget(Clear, area);
    frame.render_widget(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(app.palette.accent))
            .title(" dock "),
        area,
    );
    runtime.render(frame, inner, true);
}
