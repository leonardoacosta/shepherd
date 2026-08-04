use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Paragraph, Wrap},
    Frame,
};

use super::keybind_help::keybind_label;
use super::widgets::{
    action_button_width, modal_stack_areas, panel_contrast_fg, render_action_button,
    render_modal_shell,
};
use crate::app::AppState;

/// Semantic prefix/help/settings shortcut lines, wrapped to `width`. Built from the
/// effective `AppState` keybindings so onboarding never drifts from the keybind help
/// surface (see `keybind_label`).
fn onboarding_shortcut_paragraph(app: &AppState) -> Paragraph<'static> {
    let key_style = Style::default()
        .fg(app.palette.accent)
        .add_modifier(Modifier::BOLD);
    let prose_style = Style::default().fg(app.palette.overlay1);

    let prefix_label = crate::config::format_key_combo((app.prefix_code, app.prefix_mods));
    let help_label = keybind_label(&app.keybinds.help);
    let settings_label = keybind_label(&app.keybinds.settings);

    let shortcut_line = |key: String, prose: &'static str| {
        Line::from(vec![
            Span::styled("  ", prose_style),
            Span::styled(key, key_style),
            Span::styled(prose, prose_style),
        ])
    };

    Paragraph::new(vec![
        shortcut_line(prefix_label, " enters prefix mode"),
        shortcut_line(help_label, " shows keybinds"),
        shortcut_line(settings_label, " opens settings"),
    ])
    .wrap(Wrap { trim: false })
}

pub(super) fn render_onboarding_overlay(app: &AppState, frame: &mut Frame, area: Rect) {
    super::dim_background(frame, area);
    render_onboarding_welcome(app, frame, area);
}

pub(crate) fn onboarding_welcome_continue_rect(area: Rect) -> Rect {
    Rect::new(
        area.x,
        area.y,
        action_button_width(Some("↵"), "continue"),
        1,
    )
}

fn render_onboarding_welcome(app: &AppState, frame: &mut Frame, area: Rect) {
    let Some(inner) = render_modal_shell(frame, area, 64, 16, &app.palette) else {
        return;
    };
    if inner.height < 11 {
        return;
    }

    let stack = modal_stack_areas(inner, 2, 0, 1, 1);
    let header_rows =
        Layout::vertical([Constraint::Length(1), Constraint::Length(1)]).areas::<2>(stack.header);

    let shortcuts = onboarding_shortcut_paragraph(app);
    let shortcut_rows = shortcuts.line_count(stack.content.width.max(1)) as u16;
    let content_rows = Layout::vertical([
        Constraint::Length(3),
        Constraint::Length(1),
        Constraint::Length(shortcut_rows),
        Constraint::Min(0),
    ])
    .areas::<4>(stack.content);

    frame.render_widget(
        Paragraph::new("  shepherd").style(
            Style::default()
                .fg(app.palette.text)
                .add_modifier(Modifier::BOLD),
        ),
        header_rows[0],
    );
    frame.render_widget(
        Paragraph::new("  terminal workspace manager for coding agents")
            .style(Style::default().fg(app.palette.overlay0)),
        header_rows[1],
    );

    frame.render_widget(
        Paragraph::new(
            "  this is a mouse-first terminal.\n  click the sidebar to switch workspaces, drag pane\n  borders to resize, right-click for context menus.",
        )
        .style(Style::default().fg(app.palette.overlay1)),
        content_rows[0],
    );

    frame.render_widget(shortcuts, content_rows[2]);

    frame.render_widget(
        Paragraph::new("  next: install optional agent integrations for more reliable state")
            .style(Style::default().fg(app.palette.overlay1)),
        content_rows[3],
    );

    let continue_rect = onboarding_welcome_continue_rect(stack.actions.unwrap_or_default());
    render_action_button(
        frame,
        continue_rect,
        Some("↵"),
        "continue",
        Style::default()
            .fg(panel_contrast_fg(&app.palette))
            .bg(app.palette.accent)
            .add_modifier(Modifier::BOLD),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::state::AppState;
    use crate::config::ActionKeybinds;
    use crossterm::event::{KeyCode, KeyModifiers};
    use ratatui::{backend::TestBackend, buffer::Buffer, Terminal};

    /// Joins the buffer into one string per row so wrapped text from the same
    /// paragraph can be matched token-by-token without falsely stitching together
    /// unrelated cells that happen to sit on adjacent rows.
    fn rendered_lines(buffer: &Buffer) -> String {
        (0..buffer.area.height)
            .map(|y| {
                (0..buffer.area.width)
                    .map(|x| buffer[(x, y)].symbol())
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn render(app: &AppState, width: u16, height: u16) -> String {
        let mut terminal = Terminal::new(TestBackend::new(width, height))
            .expect("test terminal should initialize");
        terminal
            .draw(|frame| render_onboarding_welcome(app, frame, Rect::new(0, 0, width, height)))
            .expect("onboarding welcome should render");
        rendered_lines(terminal.backend().buffer())
    }

    #[test]
    fn repeated_renders_are_deterministic_and_do_not_mutate_state() {
        let app = AppState::test_new();

        let first = render(&app, 64, 16);
        let second = render(&app, 64, 16);

        assert_eq!(
            first, second,
            "rendering onboarding twice from the same &AppState must be pure"
        );
    }

    #[test]
    fn default_shortcuts_show_effective_prefix_help_and_settings_bindings() {
        let app = AppState::test_new();
        let rendered = render(&app, 64, 16);

        assert!(rendered.contains("ctrl+b"));
        assert!(rendered.contains("enters prefix mode"));
        assert!(rendered.contains("prefix+?"));
        assert!(rendered.contains("shows keybinds"));
        assert!(rendered.contains("prefix+s"));
        assert!(rendered.contains("opens settings"));
    }

    #[test]
    fn custom_shortcuts_reflect_configured_bindings_not_defaults() {
        let mut app = AppState::test_new();
        app.prefix_code = KeyCode::Char('a');
        app.prefix_mods = KeyModifiers::CONTROL;
        app.keybinds.help = ActionKeybinds::prefix("h");
        app.keybinds.settings = ActionKeybinds::direct("f2");

        let rendered = render(&app, 64, 16);

        assert!(rendered.contains("ctrl+a"));
        assert!(rendered.contains("prefix+h"));
        assert!(rendered.contains("f2"));
        assert!(!rendered.contains("ctrl+b"));
        assert!(!rendered.contains("prefix+?"));
        assert!(!rendered.contains("prefix+s"));
    }

    #[test]
    fn unset_actions_render_unset_label_for_help_and_settings() {
        let mut app = AppState::test_new();
        app.keybinds.help = ActionKeybinds::default();
        app.keybinds.settings = ActionKeybinds::default();

        let rendered = render(&app, 64, 16);

        assert!(rendered.contains("unset shows keybinds"));
        assert!(rendered.contains("unset opens settings"));
    }

    #[test]
    fn multiple_bindings_render_every_configured_alternative() {
        let mut app = AppState::test_new();
        let mut help = ActionKeybinds::prefix("?");
        help.bindings.extend(ActionKeybinds::prefix("h").bindings);
        app.keybinds.help = help;

        let rendered = render(&app, 64, 16);

        assert!(rendered.contains("prefix+? / prefix+h"));
    }

    #[test]
    fn narrow_forty_column_render_keeps_every_shortcut_label_intact() {
        let mut app = AppState::test_new();
        app.keybinds.help = ActionKeybinds::prefix("h");
        app.keybinds.settings = ActionKeybinds::direct("f2");

        let rendered = render(&app, 40, 16);

        assert!(rendered.contains("prefix+h"));
        assert!(rendered.contains("f2"));
    }

    #[test]
    fn long_customized_label_wraps_across_rows_without_dropping_tokens() {
        let mut app = AppState::test_new();
        let mut settings = ActionKeybinds::prefix("ctrl+shift+f12");
        settings
            .bindings
            .extend(ActionKeybinds::prefix("ctrl+alt+shift+f11").bindings);
        app.keybinds.settings = settings;

        let rendered = render(&app, 40, 24);

        for token in ["prefix+ctrl+shift+f12", "/", "prefix+ctrl+alt+shift+f11"] {
            assert!(
                rendered.contains(token),
                "expected rendered onboarding to contain {token:?}, got:\n{rendered}"
            );
        }
    }
}
