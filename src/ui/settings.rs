use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{List, ListItem, ListState, Paragraph},
    Frame,
};

use super::widgets::{
    action_button_row_rects, centered_popup_rect, modal_stack_areas, panel_contrast_fg,
    render_action_button, render_modal_choice_list, render_panel_shell, ActionButtonSpec,
};
use crate::{
    app::{
        state::{ExperimentSetting, Palette},
        AppState,
    },
    config::ToastDelivery,
};

pub(crate) const SETTINGS_POPUP_WIDTH: u16 = 76;
pub(crate) const SETTINGS_POPUP_BASE_HEIGHT: u16 = 22;

// ---------------------------------------------------------------------------
// Pure Settings view model: one geometry projection shared by render() and
// keyboard/mouse input, so hit-testing can never diverge from what is drawn.
// `compute_settings_view` takes only `&AppState` and the available `Rect`; it
// never mutates state. Selection is stored by stable row identity elsewhere
// (`SettingsState::list.selected` indexes into the same ordered spec list
// this module derives rows from), and content scroll offset is always
// re-derived from that identity plus the current viewport rather than
// persisted, so a resize can never leave a stale offset behind.
// ---------------------------------------------------------------------------

use crate::app::state::SettingsSection;

/// One rendered/clickable section tab.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SettingsSectionItem {
    pub section: SettingsSection,
    pub rect: Rect,
}

/// How the section header adapts to available width.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SettingsNav {
    /// Every section label fits; each item covers only its own label.
    Full(Vec<SettingsSectionItem>),
    /// Not everything fits: a window of consecutive sections including the
    /// selected one, with previous/next chevrons when more exist off-screen.
    Overflow {
        prev: Option<Rect>,
        items: Vec<SettingsSectionItem>,
        next: Option<Rect>,
    },
    /// Not even one label plus chevrons fits: show the selected section only.
    Compact { rect: Rect },
}

impl SettingsNav {
    pub(crate) fn section_at(&self, col: u16, row: u16) -> Option<SettingsSection> {
        let hit =
            |rect: Rect| row == rect.y && col >= rect.x && col < rect.x.saturating_add(rect.width);
        match self {
            SettingsNav::Full(items) => items
                .iter()
                .find(|item| hit(item.rect))
                .map(|item| item.section),
            SettingsNav::Overflow { items, .. } => items
                .iter()
                .find(|item| hit(item.rect))
                .map(|item| item.section),
            SettingsNav::Compact { .. } => None,
        }
    }

    /// The previous/next chevron hit at this position, if any.
    pub(crate) fn chevron_at(&self, col: u16, row: u16) -> Option<i8> {
        let hit =
            |rect: Rect| row == rect.y && col >= rect.x && col < rect.x.saturating_add(rect.width);
        match self {
            SettingsNav::Overflow { prev, next, .. } => {
                if prev.is_some_and(&hit) {
                    Some(-1)
                } else if next.is_some_and(hit) {
                    Some(1)
                } else {
                    None
                }
            }
            _ => None,
        }
    }
}

fn settings_section_label_width(app: &AppState, section: SettingsSection) -> u16 {
    let badge_width = if app.settings_section_has_badge(section) {
        2
    } else {
        0
    };
    section.label().len() as u16 + 2 + badge_width
}

/// Compute section navigation for a header row of the given rect. When every
/// label fits, each item covers only its own label (`Full`). Otherwise a
/// window of consecutive sections including the selected one is shown with
/// previous/next chevrons (`Overflow`); if even the selected label plus both
/// chevrons cannot fit, only the selected section renders (`Compact`).
pub(crate) fn compute_settings_nav(app: &AppState, header: Rect) -> SettingsNav {
    let sections = SettingsSection::ALL;
    let widths: Vec<u16> = sections
        .iter()
        .map(|section| settings_section_label_width(app, *section))
        .collect();
    let divider = 1u16;
    let total: u16 = widths.iter().sum::<u16>()
        + divider.saturating_mul(sections.len().saturating_sub(1) as u16);

    if total <= header.width {
        let mut x = header.x;
        let items: Vec<SettingsSectionItem> = sections
            .iter()
            .zip(widths.iter())
            .map(|(section, width)| {
                let rect = Rect::new(x, header.y, *width, 1);
                x += width + divider;
                SettingsSectionItem {
                    section: *section,
                    rect,
                }
            })
            .collect();
        return SettingsNav::Full(items);
    }

    let selected_idx = sections
        .iter()
        .position(|section| *section == app.settings.section)
        .unwrap_or(0);
    let chevron_w = 2u16;
    let reserved = chevron_w.saturating_mul(2);
    if header.width <= reserved || widths[selected_idx] > header.width - reserved {
        return SettingsNav::Compact {
            rect: Rect::new(header.x, header.y, header.width, 1),
        };
    }

    let avail = header.width - reserved;
    let mut start = selected_idx;
    let mut end = selected_idx;
    let mut used = widths[selected_idx];
    loop {
        let can_left = start > 0 && used + divider + widths[start - 1] <= avail;
        let can_right = end + 1 < sections.len() && used + divider + widths[end + 1] <= avail;
        if can_left {
            start -= 1;
            used += divider + widths[start];
        } else if can_right {
            end += 1;
            used += divider + widths[end];
        } else {
            break;
        }
    }

    let has_prev = start > 0;
    let has_next = end + 1 < sections.len();
    let mut x = header.x + if has_prev { chevron_w } else { 0 };
    let mut items = Vec::new();
    for idx in start..=end {
        let width = widths[idx];
        items.push(SettingsSectionItem {
            section: sections[idx],
            rect: Rect::new(x, header.y, width, 1),
        });
        x += width + divider;
    }
    let prev = has_prev.then(|| Rect::new(header.x, header.y, chevron_w, 1));
    let next =
        has_next.then(|| Rect::new(header.x + header.width - chevron_w, header.y, chevron_w, 1));
    SettingsNav::Overflow { prev, items, next }
}

/// Stable identity for a Settings content row. Static-length sections
/// (Theme/Sound/Toast/Experiments) are keyed by position because their row
/// count never changes at runtime; Display already has typed dynamic
/// identity and is threaded through unchanged.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SettingsRowId {
    Index(usize),
    Display(crate::app::state::DisplayRowId),
    Behavior(crate::app::state::BehaviorRowId),
    Integration(crate::api::schema::IntegrationTarget),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SettingsRow {
    pub id: SettingsRowId,
    pub rect: Rect,
    pub selectable: bool,
}

fn settings_row_specs(app: &AppState) -> Vec<(SettingsRowId, bool)> {
    match app.settings.section {
        SettingsSection::Theme => (0..crate::app::state::THEME_NAMES.len())
            .map(|idx| (SettingsRowId::Index(idx), true))
            .collect(),
        SettingsSection::Sound => (0..2)
            .map(|idx| (SettingsRowId::Index(idx), true))
            .collect(),
        SettingsSection::Toast => (0..4)
            .map(|idx| (SettingsRowId::Index(idx), true))
            .collect(),
        SettingsSection::Display => app
            .display_rows()
            .into_iter()
            .map(|row| (SettingsRowId::Display(row.id), row.selectable))
            .collect(),
        SettingsSection::Behavior => app
            .behavior_rows()
            .into_iter()
            .map(|row| (SettingsRowId::Behavior(row.id), true))
            .collect(),
        SettingsSection::Experiments => (0..ExperimentSetting::ALL.len())
            .map(|idx| (SettingsRowId::Index(idx), true))
            .collect(),
        // The row list always reflects targets; any open action-menu/confirm/
        // history sub-dialog renders as an overlay on top of this same
        // content rect instead of replacing these row specs (see
        // `render_settings_integrations`), so scrolling geometry never shifts
        // out from under a dialog that happens to be open.
        SettingsSection::Integrations => app
            .integration_recommendations
            .iter()
            .map(|item| (SettingsRowId::Integration(item.target), true))
            .collect(),
    }
}

/// Total row count for the current section, independent of viewport size.
/// Used by wheel/scrollbar mouse handling to clamp the target selection.
pub(crate) fn settings_row_count(app: &AppState) -> usize {
    settings_row_specs(app).len()
}

fn settings_section_list_offset(section: SettingsSection) -> u16 {
    match section {
        SettingsSection::Theme => 0,
        // title(1) + description(2) + spacer(1), matching
        // `render_settings_integrations`'s header layout.
        SettingsSection::Integrations => 4,
        _ => 3,
    }
}

fn settings_row_height(section: SettingsSection) -> u16 {
    match section {
        SettingsSection::Toast => 2,
        _ => 1,
    }
}

/// Scroll offset (index of the first visible row) that keeps `selected`
/// inside a viewport of `visible_rows`, scrolling the minimum amount.
/// Shared by every Settings section instead of each keeping its own copy.
pub(crate) fn settings_scroll_start(selected: usize, visible_rows: usize) -> usize {
    if visible_rows == 0 {
        return 0;
    }
    selected.saturating_add(1).saturating_sub(visible_rows)
}

/// Visible, rect-bearing rows for the current section's content viewport.
/// The same list drives both rendering and mouse hit-testing.
pub(crate) fn settings_content_rows(app: &AppState, content: Rect) -> Vec<SettingsRow> {
    let specs = settings_row_specs(app);
    if specs.is_empty() {
        return Vec::new();
    }
    let offset = settings_section_list_offset(app.settings.section);
    let row_h = settings_row_height(app.settings.section).max(1);
    if content.height <= offset {
        return Vec::new();
    }
    let list_area_height = content.height - offset;
    let visible_rows = ((list_area_height / row_h) as usize).max(1);
    let selected = app
        .settings
        .list
        .selected
        .min(specs.len().saturating_sub(1));
    let start = settings_scroll_start(selected, visible_rows);

    specs
        .into_iter()
        .enumerate()
        .skip(start)
        .take(visible_rows)
        .map(|(idx, (id, selectable))| {
            let visible_idx = (idx - start) as u16;
            let y = content.y + offset + visible_idx * row_h;
            SettingsRow {
                id,
                rect: Rect::new(content.x, y, content.width, row_h),
                selectable,
            }
        })
        .collect()
}

/// Whether the current section's rows exceed the content viewport and thus
/// need a scrollbar track (reserved from the rightmost content column).
pub(crate) fn settings_content_scrollable(app: &AppState, content: Rect) -> bool {
    let specs_len = settings_row_specs(app).len();
    let offset = settings_section_list_offset(app.settings.section);
    let row_h = settings_row_height(app.settings.section).max(1);
    if content.height <= offset {
        return false;
    }
    let visible_rows = (((content.height - offset) / row_h) as usize).max(1);
    specs_len > visible_rows
}

/// Full pure projection consumed by both `render_settings_overlay` and every
/// keyboard/mouse handler in `app/input/settings.rs`. `content` is already
/// narrowed to leave room for `scrollbar` when one is present, so section
/// renderers and row hit-testing never need to reason about the scrollbar
/// column themselves.
pub(crate) struct SettingsView {
    pub popup: Option<Rect>,
    pub nav: SettingsNav,
    pub content: Rect,
    pub rows: Vec<SettingsRow>,
    pub scrollbar: Option<Rect>,
}

pub(crate) fn compute_settings_view(app: &AppState, area: Rect) -> SettingsView {
    let empty = || SettingsView {
        popup: None,
        nav: SettingsNav::Compact {
            rect: Rect::default(),
        },
        content: Rect::default(),
        rows: Vec::new(),
        scrollbar: None,
    };
    let Some(popup) = centered_popup_rect(area, SETTINGS_POPUP_WIDTH, settings_popup_height(app))
    else {
        return empty();
    };
    let inner = Rect::new(
        popup.x + 1,
        popup.y + 1,
        popup.width.saturating_sub(2),
        popup.height.saturating_sub(2),
    );
    if inner.height < 4 || inner.width < 10 {
        return SettingsView {
            popup: Some(popup),
            ..empty()
        };
    }

    let stack = modal_stack_areas(inner, 3, 2, 0, 1);
    let header_rows = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
    ])
    .areas::<3>(stack.header);
    let nav = compute_settings_nav(app, header_rows[1]);

    let content = stack.content;
    let scrollable = settings_content_scrollable(app, content);
    let (rows_area, scrollbar) = if scrollable && content.width > 1 {
        let offset = settings_section_list_offset(app.settings.section);
        let track = Rect::new(
            content.x + content.width - 1,
            content.y + offset,
            1,
            content.height.saturating_sub(offset),
        );
        (
            Rect::new(content.x, content.y, content.width - 1, content.height),
            Some(track),
        )
    } else {
        (content, None)
    };
    let rows = settings_content_rows(app, rows_area);

    SettingsView {
        popup: Some(popup),
        nav,
        content: rows_area,
        rows,
        scrollbar,
    }
}

fn render_settings_nav_item(
    app: &AppState,
    frame: &mut Frame,
    p: &Palette,
    item: SettingsSectionItem,
) {
    let selected = item.section == app.settings.section;
    let mut spans = Vec::new();
    if app.settings_section_has_badge(item.section) {
        spans.push(Span::styled(
            "● ",
            Style::default().fg(p.accent).add_modifier(Modifier::BOLD),
        ));
    }
    spans.push(Span::raw(format!(" {} ", item.section.label())));
    let style = if selected {
        Style::default()
            .fg(panel_contrast_fg(p))
            .bg(p.accent)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(p.overlay1)
    };
    frame.render_widget(Paragraph::new(Line::from(spans)).style(style), item.rect);
}

fn render_settings_nav(app: &AppState, frame: &mut Frame, nav: &SettingsNav, p: &Palette) {
    match nav {
        SettingsNav::Full(items) => {
            for item in items {
                render_settings_nav_item(app, frame, p, *item);
            }
        }
        SettingsNav::Overflow { prev, items, next } => {
            if let Some(rect) = prev {
                frame.render_widget(
                    Paragraph::new(Span::styled("‹", Style::default().fg(p.overlay1))),
                    *rect,
                );
            }
            for item in items {
                render_settings_nav_item(app, frame, p, *item);
            }
            if let Some(rect) = next {
                frame.render_widget(
                    Paragraph::new(Span::styled("›", Style::default().fg(p.overlay1))),
                    *rect,
                );
            }
        }
        SettingsNav::Compact { rect } => {
            let index = SettingsSection::ALL
                .iter()
                .position(|section| *section == app.settings.section)
                .unwrap_or(0);
            let text = format!(
                " {} ({}/{}) ",
                app.settings.section.label(),
                index + 1,
                SettingsSection::ALL.len()
            );
            frame.render_widget(
                Paragraph::new(Span::styled(
                    text,
                    Style::default()
                        .fg(panel_contrast_fg(p))
                        .bg(p.accent)
                        .add_modifier(Modifier::BOLD),
                )),
                *rect,
            );
        }
    }
}

/// Row index (into `display_rows()` for Display, or the direct spec index
/// for every other section) that the first visible content row represents.
/// Drives the shared scrollbar thumb position.
fn settings_first_visible_index(app: &AppState, rows: &[SettingsRow]) -> usize {
    match rows.first().map(|row| &row.id) {
        Some(SettingsRowId::Index(idx)) => *idx,
        Some(SettingsRowId::Display(id)) => app
            .display_rows()
            .iter()
            .position(|row| &row.id == id)
            .unwrap_or(0),
        Some(SettingsRowId::Behavior(id)) => app
            .behavior_rows()
            .iter()
            .position(|row| &row.id == id)
            .unwrap_or(0),
        Some(SettingsRowId::Integration(target)) => app
            .integration_recommendations
            .iter()
            .position(|item| &item.target == target)
            .unwrap_or(0),
        None => 0,
    }
}

/// `settings.list.selected`-compatible absolute index for a row identity.
/// `Index(idx)` already is that index; `Display` rows resolve through the
/// current `display_rows()` list since Display's positions can shift.
pub(crate) fn settings_row_absolute_index(app: &AppState, id: &SettingsRowId) -> Option<usize> {
    match id {
        SettingsRowId::Index(idx) => Some(*idx),
        SettingsRowId::Display(display_id) => app
            .display_rows()
            .iter()
            .position(|row| &row.id == display_id),
        SettingsRowId::Behavior(behavior_id) => app
            .behavior_rows()
            .iter()
            .position(|row| &row.id == behavior_id),
        SettingsRowId::Integration(target) => app
            .integration_recommendations
            .iter()
            .position(|item| &item.target == target),
    }
}

fn render_settings_scrollbar(app: &AppState, frame: &mut Frame, view: &SettingsView, track: Rect) {
    let specs_len = settings_row_specs(app).len();
    let visible = view.rows.len();
    let first_visible = settings_first_visible_index(app, &view.rows);
    let metrics = super::scrollbar::top_anchored_scroll_metrics(specs_len, visible, first_visible);
    let p = &app.palette;
    super::scrollbar::render_scrollbar(frame, metrics, track, p.surface0, p.overlay1, "▐");
}

pub(crate) fn settings_popup_height(app: &AppState) -> u16 {
    if app.settings.section != crate::app::state::SettingsSection::Integrations {
        return SETTINGS_POPUP_BASE_HEIGHT;
    }
    let footer_rows = integrations_footer_height(app, SETTINGS_POPUP_WIDTH - 2);
    // borders 2 + header 3 + stack gaps 2 + modal footer 2 + section title 1
    // + description 2 + spacers 2. The target list itself now scrolls within
    // whatever height remains (task 3.1/3.2) instead of growing the popup
    // per target, so only the bounded footer summary affects this height.
    (14 + footer_rows).max(SETTINGS_POPUP_BASE_HEIGHT)
}

pub(super) fn render_settings_overlay(app: &AppState, frame: &mut Frame, area: Rect) {
    use crate::app::state::SettingsSection;

    let p = &app.palette;
    let view = compute_settings_view(app, area);
    let Some(popup) = view.popup else {
        return;
    };

    super::dim_background(frame, area);

    let Some(inner) = render_panel_shell(frame, popup, p.accent, p.panel_bg) else {
        return;
    };
    if inner.height < 4 || inner.width < 10 {
        return;
    }

    let stack = modal_stack_areas(inner, 3, 2, 0, 1);
    let header_rows = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
    ])
    .areas::<3>(stack.header);

    frame.render_widget(
        Paragraph::new(Line::from(vec![Span::styled(
            " settings",
            Style::default().fg(p.text).add_modifier(Modifier::BOLD),
        )])),
        header_rows[0],
    );

    render_settings_nav(app, frame, &view.nav, p);

    let sep = "─".repeat(inner.width as usize);
    frame.render_widget(
        Paragraph::new(Span::styled(&sep, Style::default().fg(p.surface0))),
        header_rows[2],
    );

    let content_area = view.content;

    if let Some(track) = view.scrollbar {
        render_settings_scrollbar(app, frame, &view, track);
    }

    match app.settings.section {
        SettingsSection::Theme => {
            render_settings_theme(app, frame, content_area);
        }
        SettingsSection::Sound => {
            render_settings_toggle(
                frame,
                content_area,
                p,
                "sound alerts",
                "play sounds when agents change state in background",
                app.sound_enabled(),
                app.settings.list.selected,
            );
        }
        SettingsSection::Toast => {
            render_modal_choice_list(
                frame,
                content_area,
                "notification popups",
                "choose where background popup notifications should appear",
                &[
                    ("off", ToastDelivery::Off),
                    ("inside shepherd", ToastDelivery::Shepherd),
                    ("via terminal", ToastDelivery::Terminal),
                    ("via system", ToastDelivery::System),
                ],
                app.toast_delivery(),
                app.settings.list.selected,
                p,
                2,
            );
        }
        SettingsSection::Display => render_settings_display(app, frame, content_area),
        SettingsSection::Behavior => render_settings_behavior(app, frame, content_area),
        SettingsSection::Experiments => {
            render_settings_experiments(app, frame, content_area);
        }
        SettingsSection::Integrations => {
            render_settings_integrations(app, frame, content_area, &view.rows);
        }
    }

    if let Some(footer_area) = stack.footer {
        let footer_rows = Layout::vertical([Constraint::Length(1), Constraint::Length(1)])
            .areas::<2>(footer_area);
        let primary_label = settings_primary_button_label(app.settings.section);
        let show_primary = settings_show_primary_action(app);
        let (apply_rect, close_rect) =
            settings_button_rects(inner, app.settings.section, show_primary);
        if let Some(apply_rect) = apply_rect {
            render_action_button(
                frame,
                apply_rect,
                Some("↵"),
                primary_label,
                Style::default()
                    .fg(panel_contrast_fg(p))
                    .bg(p.accent)
                    .add_modifier(Modifier::BOLD),
            );
        }
        render_action_button(
            frame,
            close_rect,
            Some("esc"),
            "close",
            Style::default()
                .fg(p.text)
                .bg(p.surface0)
                .add_modifier(Modifier::BOLD),
        );

        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled(" ↑↓", Style::default().fg(p.overlay0)),
                Span::styled(" select  ", Style::default().fg(p.overlay1)),
                Span::styled("tab", Style::default().fg(p.overlay0)),
                Span::styled(" section", Style::default().fg(p.overlay1)),
            ])),
            footer_rows[0],
        );
    }
}

pub(crate) fn settings_primary_button_label(
    section: crate::app::state::SettingsSection,
) -> &'static str {
    match section {
        crate::app::state::SettingsSection::Integrations => "install",
        _ => "apply",
    }
}

pub(crate) fn settings_show_primary_action(app: &AppState) -> bool {
    match app.settings.section {
        crate::app::state::SettingsSection::Integrations => app
            .integration_recommendations
            .iter()
            .any(crate::integration::IntegrationRecommendation::needs_install),
        _ => true,
    }
}

pub(crate) fn settings_button_rects(
    inner: Rect,
    section: crate::app::state::SettingsSection,
    show_primary: bool,
) -> (Option<Rect>, Rect) {
    if !show_primary {
        let rects = action_button_row_rects(
            inner,
            &[ActionButtonSpec {
                hint: Some("esc"),
                label: "close",
            }],
            2,
            inner.height.saturating_sub(1),
        );
        return (None, rects[0]);
    }

    let rects = action_button_row_rects(
        inner,
        &[
            ActionButtonSpec {
                hint: Some("↵"),
                label: settings_primary_button_label(section),
            },
            ActionButtonSpec {
                hint: Some("esc"),
                label: "close",
            },
        ],
        2,
        inner.height.saturating_sub(1),
    );
    (Some(rects[0]), rects[1])
}

fn integrations_footer_paragraph(app: &AppState) -> Paragraph<'static> {
    let p = &app.palette;
    let mut footer_lines = Vec::new();
    if !app.integration_install_messages.is_empty() {
        for message in &app.integration_install_messages {
            footer_lines.push(Line::from(Span::styled(
                format!(" {message}"),
                Style::default().fg(p.overlay1),
            )));
        }
    } else {
        let found_any = app.integration_recommendations.iter().any(|item| {
            item.available || item.state != crate::integration::IntegrationStatusKind::NotInstalled
        });
        let hint = if app
            .integration_recommendations
            .iter()
            .any(crate::integration::IntegrationRecommendation::needs_install)
        {
            " press install to add available or outdated integrations"
        } else if found_any {
            " all detected integrations are installed"
        } else {
            " no supported agent CLIs found on PATH"
        };
        footer_lines.push(Line::from(Span::styled(
            hint.to_string(),
            Style::default().fg(p.overlay1),
        )));
    }
    Paragraph::new(footer_lines).wrap(ratatui::widgets::Wrap { trim: false })
}

fn integrations_footer_height(app: &AppState, width: u16) -> u16 {
    (integrations_footer_paragraph(app).line_count(width) as u16).min(6)
}

/// Dialog choice rects for the Integrations action-menu/uninstall-confirm
/// sub-views, matching `render_modal_choice_list`'s internal layout exactly
/// (`Length(2)` description + `Length(1)` spacer before the list starts) so
/// mouse hit-testing in `app/input/settings.rs` never drifts from render.
pub(crate) fn integration_dialog_choice_rects(content: Rect, count: usize) -> Vec<Rect> {
    let list_area = Rect::new(
        content.x,
        content.y.saturating_add(3),
        content.width,
        content.height.saturating_sub(3),
    );
    super::widgets::modal_choice_rows(list_area, count, 1)
}

fn render_settings_integrations(
    app: &AppState,
    frame: &mut Frame,
    area: Rect,
    rows: &[SettingsRow],
) {
    let manager = &app.settings.integration_manager;
    if manager.history_detail_open {
        render_integration_history_detail(app, frame, area);
        return;
    }
    if let Some(target) = manager.pending_uninstall {
        render_integration_uninstall_confirm(app, frame, area, target);
        return;
    }
    if let Some(target) = manager.action_menu_target {
        render_integration_action_menu(app, frame, area, target);
        return;
    }
    render_integration_target_list(app, frame, area, rows);
}

fn render_integration_target_list(
    app: &AppState,
    frame: &mut Frame,
    area: Rect,
    rows: &[SettingsRow],
) {
    let p = &app.palette;

    let footer = integrations_footer_paragraph(app);
    let footer_height = integrations_footer_height(app, area.width);

    let layout = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(2),
        Constraint::Length(1),
        Constraint::Min(0),
        Constraint::Length(1),
        Constraint::Length(footer_height),
    ])
    .areas::<6>(area);

    frame.render_widget(
        Paragraph::new("agent integrations")
            .style(Style::default().fg(p.text).add_modifier(Modifier::BOLD)),
        layout[0],
    );
    frame.render_widget(
        Paragraph::new(
            "let agents report state directly instead of relying only on process detection",
        )
        .style(Style::default().fg(p.overlay1))
        .wrap(ratatui::widgets::Wrap { trim: false }),
        layout[1],
    );

    if rows.is_empty() && app.integration_recommendations.is_empty() {
        frame.render_widget(
            Paragraph::new(Span::styled(
                " no integration targets available",
                Style::default().fg(p.overlay1),
            )),
            layout[3],
        );
    }
    for row in rows {
        let SettingsRowId::Integration(target) = &row.id else {
            continue;
        };
        let Some(item) = app
            .integration_recommendations
            .iter()
            .find(|item| &item.target == target)
        else {
            continue;
        };
        let selected = app
            .integration_recommendations
            .get(app.settings.list.selected)
            .is_some_and(|selected| &selected.target == target);
        let marker = match item.state {
            crate::integration::IntegrationStatusKind::Current => "✓",
            crate::integration::IntegrationStatusKind::Outdated => "↻",
            crate::integration::IntegrationStatusKind::NotInstalled if item.available => "+",
            crate::integration::IntegrationStatusKind::NotInstalled => "–",
        };
        let marker_style = match item.state {
            crate::integration::IntegrationStatusKind::Current => Style::default().fg(p.green),
            crate::integration::IntegrationStatusKind::Outdated => Style::default().fg(p.yellow),
            crate::integration::IntegrationStatusKind::NotInstalled if item.available => {
                Style::default().fg(p.accent)
            }
            crate::integration::IntegrationStatusKind::NotInstalled => {
                Style::default().fg(p.overlay0)
            }
        };
        let label_style = if selected {
            Style::default()
                .bg(p.surface0)
                .fg(p.text)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(p.subtext0)
        };
        let status = item
            .unavailable_reason()
            .unwrap_or_else(|| item.status_label());
        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled(format!(" {marker} "), marker_style),
                Span::styled(format!("{:<9}", item.label), label_style),
                Span::styled(status, Style::default().fg(p.overlay1)),
            ])),
            row.rect,
        );
    }

    frame.render_widget(footer, layout[5]);
}

fn render_integration_action_menu(
    app: &AppState,
    frame: &mut Frame,
    area: Rect,
    target: crate::api::schema::IntegrationTarget,
) {
    let p = &app.palette;
    let Some(item) = app
        .integration_recommendations
        .iter()
        .find(|item| item.target == target)
    else {
        return;
    };
    let actions = item.available_actions();
    // Highlight the recommended action with a checkmark: Update when
    // outdated (safer than a silent Uninstall default), Install/Uninstall
    // otherwise since those are the only choice.
    let recommended = actions
        .iter()
        .find(|action| **action == crate::integration::IntegrationAction::Update)
        .or_else(|| actions.first())
        .copied();
    let options: Vec<(&str, crate::integration::IntegrationAction)> = actions
        .iter()
        .map(|action| (action.label(), *action))
        .collect();
    let selected = app.settings.integration_manager.action_menu_selected;
    super::widgets::render_modal_choice_list(
        frame,
        area,
        item.label,
        &format!("{} · choose an action", item.label),
        &options,
        recommended.unwrap_or(crate::integration::IntegrationAction::Install),
        selected,
        p,
        1,
    );
}

fn render_integration_uninstall_confirm(
    app: &AppState,
    frame: &mut Frame,
    area: Rect,
    target: crate::api::schema::IntegrationTarget,
) {
    let p = &app.palette;
    let Some(item) = app
        .integration_recommendations
        .iter()
        .find(|item| item.target == target)
    else {
        return;
    };
    let description = format!(
        "remove the installed {} integration at {}",
        item.label,
        item.path.display()
    );
    let options = [("confirm uninstall", true), ("cancel", false)];
    super::widgets::render_modal_choice_list(
        frame,
        area,
        "uninstall",
        &description,
        &options,
        false,
        0,
        p,
        1,
    );
}

fn render_integration_history_detail(app: &AppState, frame: &mut Frame, area: Rect) {
    let p = &app.palette;
    let [header_area, list_area] =
        Layout::vertical([Constraint::Length(2), Constraint::Min(1)]).areas::<2>(area);
    super::widgets::render_modal_description(
        frame,
        header_area,
        "operation history · esc to close",
        Style::default().fg(p.overlay1),
    );

    let messages = &app.integration_install_messages;
    let visible = list_area.height as usize;
    let max_scroll = messages.len().saturating_sub(visible);
    let scroll = app
        .settings
        .integration_manager
        .history_scroll
        .min(max_scroll);
    let lines: Vec<Line> = messages
        .iter()
        .skip(scroll)
        .take(visible)
        .map(|message| {
            Line::from(Span::styled(
                format!(" {message}"),
                Style::default().fg(p.subtext0),
            ))
        })
        .collect();
    frame.render_widget(Paragraph::new(lines), list_area);
}

fn render_settings_theme(app: &AppState, frame: &mut Frame, area: Rect) {
    use crate::app::state::THEME_NAMES;

    let p = &app.palette;
    let items: Vec<ListItem> = THEME_NAMES
        .iter()
        .map(|name| {
            let is_current = name.to_lowercase().replace([' ', '_'], "-")
                == app.theme_name.to_lowercase().replace([' ', '_'], "-");
            let marker = if is_current { " ✓" } else { "" };
            ListItem::new(Line::from(vec![
                Span::styled(*name, Style::default().fg(p.subtext0)),
                Span::styled(marker, Style::default().fg(p.green)),
            ]))
        })
        .collect();

    let list = List::new(items)
        .highlight_style(
            Style::default()
                .bg(p.surface0)
                .fg(p.text)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol(" ▸ ")
        .style(Style::default().fg(p.subtext0));

    let mut state = ListState::default().with_selected(Some(app.settings.list.selected));
    frame.render_stateful_widget(list, area, &mut state);
}

fn render_settings_toggle(
    frame: &mut Frame,
    area: Rect,
    p: &Palette,
    title: &str,
    description: &str,
    current_value: bool,
    selected_idx: usize,
) {
    render_modal_choice_list(
        frame,
        area,
        title,
        description,
        &[("on", true), ("off", false)],
        current_value,
        selected_idx,
        p,
        1,
    );
}

fn render_settings_display(app: &AppState, frame: &mut Frame, area: Rect) {
    let p = &app.palette;
    let [desc_area, _, list_area] = Layout::vertical([
        Constraint::Length(2),
        Constraint::Length(1),
        Constraint::Min(1),
    ])
    .areas::<3>(area);
    super::widgets::render_modal_description(
        frame,
        desc_area,
        "desktop chrome · enter toggles, -/+ adjusts dock size",
        Style::default().fg(p.overlay1),
    );

    let rows = app.display_rows();
    let start = app.display_scroll_start(list_area.height as usize);
    for (visible_idx, row) in rows
        .iter()
        .skip(start)
        .take(list_area.height as usize)
        .enumerate()
    {
        let idx = start + visible_idx;
        let selected = row.selectable && app.settings.list.selected == idx;
        let style = if selected {
            Style::default()
                .bg(p.surface0)
                .fg(p.text)
                .add_modifier(Modifier::BOLD)
        } else if row.selectable {
            Style::default().fg(p.subtext0)
        } else if row.id == crate::app::state::DisplayRowId::Diagnostic {
            Style::default().fg(p.yellow)
        } else {
            Style::default().fg(p.overlay1)
        };
        let marker = if selected { " ▸ " } else { "   " };
        let rect = Rect::new(
            list_area.x,
            list_area.y + visible_idx as u16,
            list_area.width,
            1,
        );
        frame.render_widget(
            Paragraph::new(format!("{marker}{}", row.label)).style(style),
            rect,
        );
    }
}

/// Column/row of a `[-] value [+]` numeric stepper's minus/plus hit zones,
/// shared by every `label: [-] value [+]` row (`dock size`, `mouse scroll
/// lines`) so the geometry math lives in exactly one place.
fn numeric_stepper_delta_at(
    content_area: Rect,
    column: u16,
    row: u16,
    scroll_start: usize,
    row_idx: usize,
    label_prefix_len: u16,
    value_len: u16,
) -> Option<i8> {
    let visible_idx = row_idx.checked_sub(scroll_start)?;
    let expected_row = content_area.y.saturating_add(3 + visible_idx as u16);
    if row != expected_row {
        return None;
    }
    // These rows use a three-cell selection marker before their label.
    let label_x = content_area.x.saturating_add(3);
    let minus = Rect::new(label_x.saturating_add(label_prefix_len), row, 3, 1);
    let plus_x = minus
        .x
        .saturating_add(3)
        .saturating_add(1)
        .saturating_add(value_len)
        .saturating_add(1);
    let plus = Rect::new(plus_x, row, 3, 1);
    if column >= minus.x && column < minus.x.saturating_add(minus.width) {
        Some(-1)
    } else if column >= plus.x && column < plus.x.saturating_add(plus.width) {
        Some(1)
    } else {
        None
    }
}

fn render_settings_behavior(app: &AppState, frame: &mut Frame, area: Rect) {
    let p = &app.palette;
    let [desc_area, _, list_area] = Layout::vertical([
        Constraint::Length(2),
        Constraint::Length(1),
        Constraint::Min(1),
    ])
    .areas::<3>(area);
    super::widgets::render_modal_description(
        frame,
        desc_area,
        "interaction preferences · enter toggles, -/+ adjusts scroll speed",
        Style::default().fg(p.overlay1),
    );

    let rows = app.behavior_rows();
    let start = app.behavior_scroll_start(list_area.height as usize);
    for (visible_idx, row) in rows
        .iter()
        .skip(start)
        .take(list_area.height as usize)
        .enumerate()
    {
        let idx = start + visible_idx;
        let selected = app.settings.list.selected == idx;
        let style = if selected {
            Style::default()
                .bg(p.surface0)
                .fg(p.text)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(p.subtext0)
        };
        let marker = if selected { " ▸ " } else { "   " };
        let rect = Rect::new(
            list_area.x,
            list_area.y + visible_idx as u16,
            list_area.width,
            1,
        );
        frame.render_widget(
            Paragraph::new(format!("{marker}{}", row.label)).style(style),
            rect,
        );
    }
}

pub(crate) fn display_size_delta_at(
    content_area: Rect,
    column: u16,
    row: u16,
    scroll_start: usize,
    row_idx: usize,
    dock_size: u16,
) -> Option<i8> {
    numeric_stepper_delta_at(
        content_area,
        column,
        row,
        scroll_start,
        row_idx,
        "dock size: ".len() as u16,
        dock_size.to_string().len() as u16,
    )
}

pub(crate) fn behavior_scroll_lines_delta_at(
    content_area: Rect,
    column: u16,
    row: u16,
    scroll_start: usize,
    row_idx: usize,
    scroll_lines: usize,
) -> Option<i8> {
    numeric_stepper_delta_at(
        content_area,
        column,
        row,
        scroll_start,
        row_idx,
        "mouse scroll lines: ".len() as u16,
        scroll_lines.to_string().len() as u16,
    )
}

fn render_settings_experiments(app: &AppState, frame: &mut Frame, area: Rect) {
    let p = &app.palette;
    let [desc_area, _, list_area] = Layout::vertical([
        Constraint::Length(2),
        Constraint::Length(1),
        Constraint::Min(1),
    ])
    .areas::<3>(area);

    super::widgets::render_modal_description(
        frame,
        desc_area,
        "optional features that are off by default",
        Style::default().fg(p.overlay1),
    );

    for (idx, setting) in ExperimentSetting::ALL.iter().copied().enumerate() {
        let marker = if setting.enabled(app) { "[✓]" } else { "[ ]" };
        let style = if app.settings.list.selected == idx {
            Style::default()
                .bg(p.surface0)
                .fg(p.text)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(p.subtext0)
        };
        let row = Rect::new(list_area.x, list_area.y + idx as u16, list_area.width, 1);
        frame.render_widget(
            Paragraph::new(format!(" {} {marker}", setting.label())).style(style),
            row,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::{state::SettingsSection, Mode};
    use ratatui::{backend::TestBackend, Terminal};

    #[test]
    fn experiments_pane_history_uses_settings_checkmark_marker() {
        let mut app = AppState::test_new();
        app.pane_history_persistence = true;
        app.settings.section = SettingsSection::Experiments;
        app.settings.list.selected = 0;
        app.mode = Mode::Settings;

        let mut terminal =
            Terminal::new(TestBackend::new(80, 24)).expect("test terminal should initialize");
        terminal
            .draw(|frame| render_settings_overlay(&app, frame, Rect::new(0, 0, 80, 24)))
            .expect("settings overlay should render");

        let rendered = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();

        assert!(rendered.contains("pane screen history [✓]"));
        assert!(!rendered.contains("[x]"));
    }

    #[test]
    fn experiments_pane_history_keeps_empty_checkbox_marker_when_disabled() {
        let mut app = AppState::test_new();
        app.pane_history_persistence = false;
        app.settings.section = SettingsSection::Experiments;
        app.settings.list.selected = 0;
        app.mode = Mode::Settings;

        let mut terminal =
            Terminal::new(TestBackend::new(80, 24)).expect("test terminal should initialize");
        terminal
            .draw(|frame| render_settings_overlay(&app, frame, Rect::new(0, 0, 80, 24)))
            .expect("settings overlay should render");

        let rendered = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();

        assert!(rendered.contains("pane screen history [ ]"));
    }

    #[test]
    fn experiments_renders_switch_ascii_input_source_row() {
        let mut app = AppState::test_new();
        app.switch_ascii_input_source_in_prefix = true;
        app.settings.section = SettingsSection::Experiments;
        app.settings.list.selected = 1;
        app.mode = Mode::Settings;

        let mut terminal =
            Terminal::new(TestBackend::new(80, 24)).expect("test terminal should initialize");
        terminal
            .draw(|frame| render_settings_overlay(&app, frame, Rect::new(0, 0, 80, 24)))
            .expect("settings overlay should render");

        let rendered = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();

        assert!(rendered.contains("switch to ascii input source in prefix (macOS/Windows) [✓]"));
    }

    // -----------------------------------------------------------------
    // Responsive Settings geometry (task 2.1 characterization/failing cases)
    // -----------------------------------------------------------------

    #[test]
    fn settings_sections_render_in_declared_order_when_width_permits() {
        let mut state = AppState::test_new();
        state.settings.section = SettingsSection::Theme;

        let header = Rect::new(0, 0, 80, 1);
        let nav = compute_settings_nav(&state, header);
        let SettingsNav::Full(items) = nav else {
            panic!("expected full nav at ample width");
        };
        let sections: Vec<SettingsSection> = items.iter().map(|item| item.section).collect();
        assert_eq!(sections, SettingsSection::ALL.to_vec());
        for pair in items.windows(2) {
            assert!(
                pair[0].rect.x + pair[0].rect.width <= pair[1].rect.x,
                "section tabs must not overlap: {pair:?}"
            );
        }
    }

    #[test]
    fn settings_sections_fall_back_to_overflow_viewport_when_labels_do_not_fit() {
        let mut state = AppState::test_new();
        state.settings.section = SettingsSection::Experiments;

        // Narrower than the combined width of every section label.
        let header = Rect::new(0, 0, 30, 1);
        let nav = compute_settings_nav(&state, header);
        assert!(
            !matches!(nav, SettingsNav::Full(_)),
            "labels wider than the header must not render as Full, got {nav:?}"
        );
        let selected_visible = match &nav {
            SettingsNav::Overflow { items, .. } => items
                .iter()
                .any(|item| item.section == SettingsSection::Experiments),
            SettingsNav::Compact { .. } => true,
            SettingsNav::Full(_) => false,
        };
        assert!(
            selected_visible,
            "selected section must remain reachable in overflow/compact nav"
        );
    }

    #[test]
    fn settings_selected_row_stays_visible_after_resize_between_40x20_64x20_80x24() {
        let mut state = AppState::test_new();
        state.settings.section = SettingsSection::Theme;
        state.settings.list.selected = crate::app::state::THEME_NAMES.len() - 1;

        for (width, height) in [(40u16, 20u16), (64, 20), (80, 24)] {
            let view = compute_settings_view(&state, Rect::new(0, 0, width, height));
            let selected_visible = view
                .rows
                .iter()
                .any(|row| row.id == SettingsRowId::Index(state.settings.list.selected));
            assert!(
                selected_visible,
                "selected theme row must stay visible at {width}x{height}, got rows {:?}",
                view.rows
            );
        }
    }

    #[test]
    fn settings_mouse_hit_targets_resolve_same_row_as_keyboard_at_each_supported_size() {
        let mut state = AppState::test_new();
        state.settings.section = SettingsSection::Integrations;

        for (width, height) in [(40u16, 20u16), (64, 20), (80, 24)] {
            let area = Rect::new(0, 0, width, height);
            let view = compute_settings_view(&state, area);
            let Some(popup) = view.popup else {
                continue;
            };
            let selected_rect = match &view.nav {
                SettingsNav::Full(items) => items
                    .iter()
                    .find(|item| item.section == SettingsSection::Integrations)
                    .map(|item| item.rect),
                SettingsNav::Overflow { items, .. } => items
                    .iter()
                    .find(|item| item.section == SettingsSection::Integrations)
                    .map(|item| item.rect),
                SettingsNav::Compact { rect } => Some(*rect),
            };
            let rect = selected_rect.unwrap_or_else(|| {
                panic!("selected section must be reachable at {width}x{height}")
            });
            assert!(
                rect.x + rect.width <= popup.x + popup.width,
                "selected section tab must stay within the popup at {width}x{height}, got {rect:?} in popup {popup:?}"
            );
            assert_eq!(
                view.nav.section_at(rect.x, rect.y),
                Some(SettingsSection::Integrations),
                "mouse hit-test at the rendered rect must resolve the selected section at {width}x{height}"
            );
        }
    }

    #[test]
    fn settings_scrollbar_thumb_reflects_visible_range_for_long_sections() {
        let mut state = AppState::test_new();
        state.settings.section = SettingsSection::Theme;
        state.settings.list.selected = crate::app::state::THEME_NAMES.len() - 1;

        let view = compute_settings_view(&state, Rect::new(0, 0, 40, 20));
        let track = view
            .scrollbar
            .expect("18 themes must exceed a 40x20 content viewport");
        let first_visible = match view.rows.first().map(|row| &row.id) {
            Some(SettingsRowId::Index(idx)) => *idx,
            other => panic!("expected indexed theme rows, got {other:?}"),
        };
        let metrics = crate::ui::scrollbar::top_anchored_scroll_metrics(
            crate::app::state::THEME_NAMES.len(),
            view.rows.len(),
            first_visible,
        );
        let thumb = crate::ui::scrollbar::scrollbar_thumb(metrics, track)
            .expect("scrollable content must have a thumb");
        assert!(
            thumb.top + thumb.len >= track.y + track.height,
            "selecting the last theme must scroll the thumb to the bottom of the track, got {thumb:?} in track {track:?}"
        );
    }

    #[test]
    fn settings_interaction_preserves_adversarial_identity_invariants() {
        let mut state = AppState::test_with_adversarial_identity_state();
        state.settings.section = SettingsSection::Display;
        state.mode = Mode::Settings;

        for (width, height) in [(40u16, 20u16), (64, 20), (80, 24)] {
            let _ = compute_settings_view(&state, Rect::new(0, 0, width, height));
        }

        state.assert_invariants_for_test();
    }

    fn fake_integration_recommendation(
        target: crate::api::schema::IntegrationTarget,
        supported: bool,
        available: bool,
        state: crate::integration::IntegrationStatusKind,
    ) -> crate::integration::IntegrationRecommendation {
        crate::integration::IntegrationRecommendation {
            target,
            label: crate::integration::integration_target_label(target),
            command: "test",
            supported,
            available,
            path: std::path::PathBuf::from("/tmp/shepherd-test-integration"),
            state,
            installed_version: None,
            expected_version: 1,
        }
    }

    // -----------------------------------------------------------------
    // Integration manager geometry (task 3.1)
    // -----------------------------------------------------------------

    // -----------------------------------------------------------------
    // Everyday preferences: Display/Behavior row reachability (task 4.1)
    // -----------------------------------------------------------------

    #[test]
    fn settings_new_display_rows_stay_reachable_at_40x20_64x20_80x24() {
        let mut state = AppState::test_new();
        state.settings.section = SettingsSection::Display;
        let rows = state.display_rows();
        let agent_sort_idx = rows
            .iter()
            .position(|row| row.id == crate::app::state::DisplayRowId::AgentSort)
            .expect("agent sort row must exist in Display");
        state.settings.list.selected = agent_sort_idx;

        for (width, height) in [(40u16, 20u16), (64, 20), (80, 24)] {
            let view = compute_settings_view(&state, Rect::new(0, 0, width, height));
            assert!(
                view.rows.iter().any(|row| row.id
                    == SettingsRowId::Display(crate::app::state::DisplayRowId::AgentSort)),
                "agent sort row must stay visible at {width}x{height}, got {:?}",
                view.rows
            );
        }
    }

    #[test]
    fn settings_behavior_rows_stay_reachable_at_40x20_64x20_80x24() {
        let mut state = AppState::test_new();
        state.settings.section = SettingsSection::Behavior;
        let last = state.behavior_rows().len() - 1;
        state.settings.list.selected = last;
        let last_id = state.behavior_rows()[last].id;

        for (width, height) in [(40u16, 20u16), (64, 20), (80, 24)] {
            let view = compute_settings_view(&state, Rect::new(0, 0, width, height));
            assert!(
                view.rows
                    .iter()
                    .any(|row| row.id == SettingsRowId::Behavior(last_id)),
                "last behavior row must stay visible at {width}x{height}, got {:?}",
                view.rows
            );
        }
    }

    #[test]
    fn settings_integration_targets_all_reachable_at_40x20() {
        let mut state = AppState::test_new();
        state.settings.section = SettingsSection::Integrations;
        state.integration_recommendations = crate::api::schema::IntegrationTarget::ALL
            .iter()
            .map(|target| {
                fake_integration_recommendation(
                    *target,
                    true,
                    true,
                    crate::integration::IntegrationStatusKind::NotInstalled,
                )
            })
            .collect();
        let last = state.integration_recommendations.len() - 1;
        state.settings.list.selected = last;

        let view = compute_settings_view(&state, Rect::new(0, 0, 40, 20));
        let target = state.integration_recommendations[last].target;
        assert!(
            view.rows
                .iter()
                .any(|row| row.id == SettingsRowId::Integration(target)),
            "the selected (last) target must stay visible at 40x20, got rows {:?}",
            view.rows
        );
    }

    #[test]
    fn settings_integration_unsupported_target_stays_visible_with_reason() {
        let mut state = AppState::test_new();
        state.settings.section = SettingsSection::Integrations;
        state.integration_recommendations = vec![fake_integration_recommendation(
            crate::api::schema::IntegrationTarget::Claude,
            false,
            false,
            crate::integration::IntegrationStatusKind::NotInstalled,
        )];

        let mut terminal =
            Terminal::new(TestBackend::new(80, 24)).expect("test terminal should initialize");
        state.mode = Mode::Settings;
        terminal
            .draw(|frame| render_settings_overlay(&state, frame, Rect::new(0, 0, 80, 24)))
            .expect("settings overlay should render");

        let rendered = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();

        assert!(
            rendered.contains("not supported on this platform"),
            "unsupported target must stay visible with an explanation, got: {rendered}"
        );
    }
}
