use ratatui::layout::Rect;
use tracing::{debug, warn};

use crate::protocol::{self, FrameData, ServerMessage, MAX_FRAME_SIZE, MAX_GRAPHICS_FRAME_SIZE};
use crate::server::clients::{render_targets, ClientConnectionMode, DeferredRender};

use super::{
    apply_terminal_dirty_patch, dirty_patch_intersects_hyperlinks, rect_fits_frame, HeadlessServer,
};

pub(super) fn render_retained_pty_update_and_stream(server: &mut HeadlessServer) -> bool {
    crate::render_prof::event("retained.attempt");
    let retained_started = crate::render_prof::timer();
    macro_rules! retained_fallback {
        ($reason:literal) => {{
            crate::render_prof::event(concat!("retained_fallback.", $reason));
            crate::render_prof::duration_since("retained.total", retained_started);
            return false;
        }};
    }
    macro_rules! retained_success {
        ($reason:literal) => {{
            crate::render_prof::event("retained.success");
            crate::render_prof::event(concat!("retained_success.", $reason));
            crate::render_prof::duration_since("retained.total", retained_started);
            return true;
        }};
    }

    if !server.retained_pty_update_allowed_by_app_state() {
        retained_fallback!("unsafe_app_state");
    }

    let render_targets = render_targets(&server.clients, server.foreground_client_id);
    let [(client_id, (cols, rows), cell_size, _is_foreground, mode)] = render_targets.as_slice()
    else {
        retained_fallback!("multiple_or_no_target");
    };
    if !matches!(mode, ClientConnectionMode::App) {
        retained_fallback!("not_app_client");
    }
    let Some(client) = server.clients.get(client_id) else {
        retained_fallback!("client_missing");
    };
    if client.deferred_render() != DeferredRender::None {
        retained_fallback!("render_pending");
    }
    if server.app.state.kitty_graphics_enabled && !client.graphics_cache.is_empty() {
        retained_fallback!("graphics_cache_active");
    }
    if client.graphics_surface_reset_pending {
        retained_fallback!("graphics_surface_reset");
    }
    if server.app.state.kitty_graphics_enabled
        && cell_size.is_known()
        && crate::kitty_graphics::has_visible_pane_graphics(
            &server.app.state,
            &server.app.terminal_runtimes,
            server.app.state.view.tab_surface(),
            *cell_size,
        )
    {
        retained_fallback!("visible_kitty_graphics");
    }
    let Some(mut frame) = client.render_state.last_frame().cloned() else {
        retained_fallback!("no_last_frame");
    };
    if frame.width != *cols || frame.height != *rows {
        retained_fallback!("frame_size_mismatch");
    }
    frame.graphics.clear();

    let Some(ws_idx) = server.app.state.active else {
        retained_fallback!("no_active_workspace");
    };
    let pane_infos = server.app.state.view.pane_infos.clone();
    if pane_infos.is_empty() {
        retained_fallback!("no_pane_info");
    }

    let mut touched = false;
    for info in pane_infos {
        if !rect_fits_frame(info.inner_rect, &frame) {
            retained_fallback!("pane_rect_outside_frame");
        }
        let Some(runtime) = server.app.state.runtime_for_pane_in_workspace(
            &server.app.terminal_runtimes,
            ws_idx,
            info.id,
        ) else {
            retained_fallback!("missing_runtime");
        };
        match runtime.collect_dirty_patch(info.inner_rect.width, info.inner_rect.height) {
            crate::pane::TerminalDirtyPatchOutcome::Clean => {
                crate::render_prof::event("retained.pane_clean");
            }
            crate::pane::TerminalDirtyPatchOutcome::Fallback => {
                retained_fallback!("dirty_patch_fallback");
            }
            crate::pane::TerminalDirtyPatchOutcome::Patch(patch) => {
                crate::render_prof::event("retained.pane_patch");
                crate::render_prof::counter("retained.patch_rows", patch.rows.len() as u64);
                if dirty_patch_intersects_hyperlinks(&frame, info.inner_rect, &patch) {
                    retained_fallback!("hyperlink_intersection");
                }
                if !apply_terminal_dirty_patch(&mut frame, info.inner_rect, patch) {
                    retained_fallback!("patch_apply_failed");
                }
                touched = true;
            }
        }
    }

    let previous_cursor = frame.cursor.clone();
    frame.cursor = crate::server::render_stream::focused_terminal_cursor(
        &server.app.state,
        &server.app.terminal_runtimes,
    );
    let cursor_changed = frame.cursor != previous_cursor;

    if !touched && !cursor_changed {
        retained_success!("clean_no_cursor_change");
    }

    let mut broken_clients = Vec::new();
    let sent = server.send_retained_frame_to_client(*client_id, frame, &mut broken_clients);
    for broken_client in broken_clients {
        server.remove_client_and_resize_if_needed(broken_client);
    }
    if sent {
        retained_success!("sent");
    }
    retained_fallback!("send_failed");
}

pub(super) fn render_and_stream(server: &mut HeadlessServer) {
    let full_started = crate::render_prof::timer();
    let render_targets = render_targets(&server.clients, server.foreground_client_id);

    if render_targets.is_empty() {
        let (cols, rows) = server.effective_size;
        let area = Rect::new(0, 0, cols, rows);
        let resize_panes = server.app.state.view.pane_infos.is_empty();
        let render_started = crate::render_prof::timer();
        let _ = crate::server::render_stream::render_virtual_with_runtime_registry(
            &mut server.app.state,
            &server.app.terminal_runtimes,
            area,
            resize_panes,
            crate::kitty_graphics::HostCellSize::default(),
        );
        crate::render_prof::duration_since("full_render.render_virtual", render_started);
        server.app.full_redraw_pending = false;
        crate::render_prof::duration_since("full_render.total", full_started);
        debug!(
            cols,
            rows, resize_panes, "rendered virtual frame with no attached clients"
        );
        return;
    }

    let mut broken_clients: Vec<u64> = Vec::new();
    let mut deferred_frame = false;
    for (client_id, (cols, rows), cell_size, is_foreground, mode) in render_targets {
        let area = Rect::new(0, 0, cols, rows);
        let is_app_client = matches!(mode, ClientConnectionMode::App);
        let mut frame = match mode {
            ClientConnectionMode::App => {
                let client_view_projection = server.project_client_view(client_id);
                server.apply_client_view_projection(client_view_projection);
                let render_started = crate::render_prof::timer();
                let render_cell_size =
                    if server.app.state.kitty_graphics_enabled && cell_size.is_known() {
                        cell_size
                    } else {
                        crate::kitty_graphics::HostCellSize::default()
                    };
                let (buffer, cursor) =
                    crate::server::render_stream::render_virtual_with_runtime_registry(
                        &mut server.app.state,
                        &server.app.terminal_runtimes,
                        area,
                        is_foreground,
                        render_cell_size,
                    );
                crate::render_prof::duration_since("full_render.render_virtual", render_started);
                let hyperlinks_started = crate::render_prof::timer();
                let hyperlinks = crate::server::render_stream::visible_hyperlinks(
                    &server.app.state,
                    &server.app.terminal_runtimes,
                );
                crate::render_prof::duration_since(
                    "full_render.visible_hyperlinks",
                    hyperlinks_started,
                );
                let frame_started = crate::render_prof::timer();
                let frame =
                    FrameData::from_ratatui_buffer_with_hyperlinks(&buffer, cursor, &hyperlinks);
                crate::render_prof::duration_since("full_render.frame_build", frame_started);
                server.refresh_app_client_view_snapshot(client_id);
                frame
            }
            ClientConnectionMode::TerminalAttach { terminal_id }
            | ClientConnectionMode::TerminalObserve { terminal_id } => {
                let Some(runtime) = server.runtime_for_terminal_id_string(&terminal_id) else {
                    server.send_to_client(
                        client_id,
                        ServerMessage::ServerShutdown {
                            reason: Some(format!(
                                "terminal attach ended: terminal {terminal_id} not found"
                            )),
                        },
                    );
                    broken_clients.push(client_id);
                    continue;
                };
                let render_started = crate::render_prof::timer();
                let (buffer, cursor) =
                    crate::server::render_stream::render_terminal_virtual(runtime, area);
                crate::render_prof::duration_since(
                    "full_render.render_terminal_virtual",
                    render_started,
                );
                let hyperlinks_started = crate::render_prof::timer();
                let hyperlinks = runtime.visible_hyperlinks(area);
                crate::render_prof::duration_since(
                    "full_render.visible_hyperlinks",
                    hyperlinks_started,
                );
                let frame_started = crate::render_prof::timer();
                let frame =
                    FrameData::from_ratatui_buffer_with_hyperlinks(&buffer, cursor, &hyperlinks);
                crate::render_prof::duration_since("full_render.frame_build", frame_started);
                frame
            }
        };

        let Some(client) = server.clients.get_mut(&client_id) else {
            continue;
        };
        let mut next_graphics_cache = client.graphics_cache.clone();
        let graphics_surface_reset_pending = client.graphics_surface_reset_pending;
        if is_app_client && server.app.state.kitty_graphics_enabled && cell_size.is_known() {
            if graphics_surface_reset_pending {
                frame.graphics = next_graphics_cache.clear_bytes();
            }
            let graphics_started = crate::render_prof::timer();
            frame
                .graphics
                .extend(crate::kitty_graphics::encode_local_pane_graphics(
                    &server.app.state,
                    &server.app.terminal_runtimes,
                    server.app.state.view.tab_surface(),
                    cell_size,
                    &mut next_graphics_cache,
                ));
            crate::render_prof::duration_since("full_render.graphics_encode", graphics_started);
        } else {
            frame.graphics = next_graphics_cache.clear_bytes();
        }

        let Some(writer) = client.writer.as_ref().cloned() else {
            crate::render_prof::event("full_render.writer_missing");
            continue;
        };

        let mut commit_graphics_cache = true;
        if frame.graphics.len() > MAX_GRAPHICS_FRAME_SIZE {
            warn!(
                client_id,
                graphics_bytes = frame.graphics.len(),
                max = MAX_GRAPHICS_FRAME_SIZE,
                "dropping oversized graphics payload for client frame"
            );
            frame.graphics.clear();
            commit_graphics_cache = false;
        }

        let max_frame_size = if frame.graphics.is_empty() {
            MAX_FRAME_SIZE
        } else {
            MAX_GRAPHICS_FRAME_SIZE
        };
        let has_graphics = !frame.graphics.is_empty();
        let prepare_started = crate::render_prof::timer();
        let Some(mut prepared) = client.render_state.prepare_frame(frame) else {
            client.clear_deferred_render();
            crate::render_prof::event("full_render.skip_identical");
            crate::render_prof::duration_since("full_render.prepare_frame", prepare_started);
            continue;
        };
        crate::render_prof::duration_since("full_render.prepare_frame", prepare_started);

        let serialize_started = crate::render_prof::timer();
        let serialized = match HeadlessServer::frame_server_message_with_max(
            prepared.message(),
            max_frame_size,
        ) {
            Ok(framed) => {
                crate::render_prof::duration_since("full_render.serialize", serialize_started);
                framed
            }
            Err(protocol::FramingError::Oversized { claimed, max }) if has_graphics => {
                warn!(
                    client_id,
                    claimed, max, "dropping graphics from oversized frame for client"
                );
                let Some(mut text_only_frame) = prepared.into_frame() else {
                    crate::render_prof::event("full_render.serialize_error");
                    crate::render_prof::duration_since("full_render.serialize", serialize_started);
                    continue;
                };
                text_only_frame.graphics.clear();
                let Some(text_only_prepared) = client.render_state.prepare_frame(text_only_frame)
                else {
                    client.clear_deferred_render();
                    crate::render_prof::event("full_render.skip_identical_text_only");
                    crate::render_prof::duration_since("full_render.serialize", serialize_started);
                    continue;
                };
                let framed = match HeadlessServer::frame_server_message(
                    text_only_prepared.message(),
                ) {
                    Ok(framed) => framed,
                    Err(err) => {
                        warn!(client_id, err = %err, "failed to serialize text-only frame for client");
                        broken_clients.push(client_id);
                        crate::render_prof::event("full_render.serialize_error");
                        crate::render_prof::duration_since(
                            "full_render.serialize",
                            serialize_started,
                        );
                        continue;
                    }
                };
                prepared = text_only_prepared;
                commit_graphics_cache = false;
                crate::render_prof::duration_since("full_render.serialize", serialize_started);
                framed
            }
            Err(protocol::FramingError::Oversized { claimed, max }) => {
                warn!(
                    client_id,
                    claimed, max, "skipping oversized frame for client"
                );
                crate::render_prof::event("full_render.serialize_oversized");
                crate::render_prof::duration_since("full_render.serialize", serialize_started);
                continue;
            }
            Err(err) => {
                warn!(client_id, err = %err, "failed to serialize frame for client");
                broken_clients.push(client_id);
                crate::render_prof::event("full_render.serialize_error");
                crate::render_prof::duration_since("full_render.serialize", serialize_started);
                continue;
            }
        };
        crate::render_prof::counter("full_render.bytes", serialized.len() as u64);

        let send_started = crate::render_prof::timer();
        match writer.render.try_send(serialized) {
            Ok(()) => {
                client.clear_deferred_render();
                if commit_graphics_cache {
                    client.graphics_cache = next_graphics_cache;
                    client.graphics_surface_reset_pending = false;
                }
                client.render_state.commit_sent_frame(prepared);
                crate::render_prof::event("full_render.sent");
                crate::render_prof::duration_since("full_render.try_send", send_started);
            }
            Err(std::sync::mpsc::TrySendError::Full(_)) => {
                client.defer_full_render();
                deferred_frame = true;
                crate::render_prof::event("full_render.queue_full");
                crate::render_prof::duration_since("full_render.try_send", send_started);
                debug!(client_id, "render queue full, deferring latest frame");
                continue;
            }
            Err(std::sync::mpsc::TrySendError::Disconnected(_)) => {
                debug!(client_id, "client writer channel closed, marking as broken");
                broken_clients.push(client_id);
                crate::render_prof::event("full_render.writer_disconnected");
                crate::render_prof::duration_since("full_render.try_send", send_started);
                continue;
            }
        }
    }

    let foreground_client_view = server.project_foreground_client_view();
    server.apply_client_view_projection(foreground_client_view);
    server.refresh_app_client_view_snapshots();

    if !broken_clients.is_empty() {
        for client_id in broken_clients {
            server.remove_client_and_resize_if_needed(client_id);
        }
    }

    let (cols, rows) = server.effective_size;
    if !deferred_frame {
        server.app.full_redraw_pending = false;
    }
    crate::render_prof::duration_since("full_render.total", full_started);
    debug!(cols, rows, foreground_client_id = ?server.foreground_client_id, "rendered virtual frame(s)");
}
