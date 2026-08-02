use ratatui::{
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use crate::{app::AppState, config::AgentSidebarToken};

pub(super) fn render_topbar(app: &AppState, frame: &mut Frame, area: Rect) {
    if area.is_empty() {
        return;
    }
    let Some(workspace) = app.active.and_then(|index| app.workspaces.get(index)) else {
        return;
    };
    let metadata = workspace.metadata_tokens.values();
    let workspace_label = workspace.display_name_from_terminals(&app.terminals);
    let mut spans = Vec::new();
    for row in &app.topbar_rows {
        for token in row {
            let value = match token.parts().0 {
                AgentSidebarToken::Workspace => Some(workspace_label.as_str()),
                AgentSidebarToken::Custom(name) => metadata.get(name).map(String::as_str),
                _ => None,
            };
            if let Some(value) = value.filter(|value| !value.is_empty()) {
                if !spans.is_empty() {
                    spans.push(Span::raw("  "));
                }
                spans.push(Span::styled(
                    value.to_string(),
                    Style::default().fg(app.palette.text),
                ));
            }
        }
    }
    frame.render_widget(
        Paragraph::new(Line::from(spans)).style(Style::default().bg(app.palette.panel_bg)),
        area,
    );
}
