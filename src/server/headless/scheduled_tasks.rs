use std::time::Instant;

use super::HeadlessServer;

pub(super) fn handle_scheduled_tasks_headless(
    server: &mut HeadlessServer,
    now: Instant,
    geometry_dirty: bool,
) -> bool {
    let mut changed = false;

    // No resize polling needed — server has no terminal.
    // Client resize messages drive size changes instead.

    if server
        .app
        .config_diagnostic_deadline
        .is_some_and(|deadline| now >= deadline)
    {
        server.app.config_diagnostic_deadline = None;
        server.app.state.config_diagnostic = None;
        changed = true;
    }

    if server
        .app
        .toast_deadline
        .is_some_and(|deadline| now >= deadline)
    {
        server.app.toast_deadline = None;
        server.app.state.toast = None;
        changed = true;
    }

    if server
        .app
        .state
        .next_pending_agent_notification_deadline()
        .is_some_and(|deadline| now >= deadline)
    {
        let previous_toast = server.app.state.toast.clone();
        let mut deliveries = server.app.state.drain_due_agent_notifications(now);
        if !deliveries.is_empty() {
            server
                .app
                .refresh_agent_notification_delivery_contexts(&mut deliveries);
            server.app.sync_toast_deadline(previous_toast);
            for delivery in &deliveries {
                server.forward_agent_notification_delivery(delivery);
            }
            changed = true;
        }
    }

    if server
        .app
        .copy_feedback_deadline
        .is_some_and(|deadline| now >= deadline)
    {
        server.app.copy_feedback_deadline = None;
        server.app.state.copy_feedback = None;
        changed = true;
    }

    if server
        .app
        .selection_autoscroll_deadline
        .is_some_and(|deadline| now >= deadline)
    {
        server.app.tick_selection_autoscroll(now);
        changed = true;
    }

    changed |= server.app.clear_due_selection_highlight(now);

    if server.has_app_client() {
        server.app.start_git_status_refresh_if_due(now);
        server.app.start_project_status_refresh_if_due(now);
        server.app.start_session_status_refresh_if_due(now);
        server.app.start_terminal_transcript_refresh_if_due(now);
    }

    if server
        .app
        .next_auto_update_check
        .is_some_and(|deadline| now >= deadline)
    {
        server.app.run_auto_update_check();
    }

    if server
        .app
        .next_agent_manifest_update_check
        .is_some_and(|deadline| now >= deadline)
    {
        server.app.run_agent_manifest_update_check();
    }

    if server
        .app
        .session_save_deadline
        .is_some_and(|deadline| now >= deadline)
    {
        server.sync_persisted_client_view_projection();
        server.app.start_background_session_save();
    }

    if let Some(deadline) = server
        .app
        .agent_metadata_deadline
        .filter(|deadline| now >= *deadline)
    {
        server.app.expire_metadata_at(deadline, now);
        changed = true;
    }

    if geometry_dirty || server.foreground_client_id.is_none() {
        server.app.pending_agent_resume_deadline = None;
    } else {
        server.app.sync_pending_agent_resume_deadline(now);
        changed |= server
            .app
            .start_pending_agent_resumes(server.app.pending_agent_resume_due(now));
    }
    changed
}
