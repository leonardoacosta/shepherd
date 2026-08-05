//! Server-owned advanced config editor: `server.config.edit`.
//!
//! Mirrors the scrollback-editor overlay pattern (`app/input/navigate.rs`)
//! but never treats the config file as disposable: the pane's `temp_files`
//! list stays empty, and pre/post byte comparison drives the reload outcome
//! reported through `server.config_edit_finished` instead of unconditional
//! `rm -f` cleanup.

use super::App;

/// Bookkeeping for the single Shepherd-owned config editor pane, kept alive
/// independent of the initiating client's connection until the editor exits.
#[derive(Debug)]
pub(crate) struct ConfigEditorOperation {
    pane_id: crate::layout::PaneId,
    config_path: std::path::PathBuf,
    pre_existed: bool,
    pre_bytes: Option<Vec<u8>>,
}

/// Result of a `server.config.edit` open request: identity of the (possibly
/// pre-existing) editor pane plus whether this was a duplicate request.
#[derive(Debug)]
pub(crate) struct ConfigEditorOpenOutcome {
    pub(crate) ws_idx: usize,
    pub(crate) pane_id: crate::layout::PaneId,
    pub(crate) already_open: bool,
}

/// Bounded diagnostics kept in `AppState` for Settings to render locally,
/// independent of the `server.config_edit_finished` event remote clients see.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ConfigEditorLastResult {
    pub(crate) outcome: crate::api::schema::ConfigEditOutcome,
    pub(crate) diagnostics: Vec<String>,
}

/// Diagnostics shown to any one consumer are capped so a pathological config
/// (thousands of unknown-key warnings) can't flood the completion event or
/// the bounded Settings row.
const MAX_CONFIG_EDIT_DIAGNOSTICS: usize = 8;

fn bounded_diagnostics(mut diagnostics: Vec<String>) -> Vec<String> {
    diagnostics.truncate(MAX_CONFIG_EDIT_DIAGNOSTICS);
    diagnostics
}

impl App {
    /// Opens (or returns the existing) Shepherd-owned config editor pane.
    /// The server always resolves its own path — nothing here accepts or
    /// interpolates a client-supplied path.
    pub(crate) fn open_config_editor(&mut self) -> std::io::Result<ConfigEditorOpenOutcome> {
        if let Some(existing) = &self.active_config_editor {
            if let Some((ws_idx, _)) = self.find_pane(existing.pane_id) {
                return Ok(ConfigEditorOpenOutcome {
                    ws_idx,
                    pane_id: existing.pane_id,
                    already_open: true,
                });
            }
            // The pane died but our PaneDied completion handler has not run
            // yet (e.g. a test driving the event queue directly) — the stale
            // entry must not block reopening.
            self.active_config_editor = None;
        }

        let path = crate::config::config_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let pre_bytes = match std::fs::read(&path) {
            Ok(bytes) => Some(bytes),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => None,
            Err(err) => return Err(err),
        };
        let pre_existed = pre_bytes.is_some();

        let argv = crate::platform::config_editor_argv(&path)?;
        // Empty temp_files: unlike the scrollback editor overlay, exit must
        // never delete the real config file.
        let (ws_idx, mut new_pane) =
            self.spawn_overlay_argv_command(&argv, None, Vec::new(), Vec::new())?;
        // Clearing launch_argv keeps this pane out of session snapshots'
        // replay path: a saved session must never relaunch the editor
        // pointed at the config file on the next restore.
        new_pane.terminal.launch_argv = None;
        let pane_id = new_pane.pane_id;
        let terminal_id = new_pane.terminal.id.clone();
        self.terminal_runtimes
            .insert(terminal_id.clone(), new_pane.runtime);
        self.state.remove_alias_shadowed_by_new_pane(pane_id);
        self.state.terminals.insert(terminal_id, new_pane.terminal);

        self.active_config_editor = Some(ConfigEditorOperation {
            pane_id,
            config_path: path,
            pre_existed,
            pre_bytes,
        });

        Ok(ConfigEditorOpenOutcome {
            ws_idx,
            pane_id,
            already_open: false,
        })
    }

    /// Takes the active config-editor operation if `pane_id` is the pane that
    /// just died, so ordinary pane exits never pay for this check.
    pub(super) fn take_config_editor_completion(
        &mut self,
        pane_id: crate::layout::PaneId,
    ) -> Option<ConfigEditorOperation> {
        if self.active_config_editor.as_ref()?.pane_id != pane_id {
            return None;
        }
        self.active_config_editor.take()
    }

    /// Validates final bytes against the pre-edit snapshot, reloads through
    /// the existing config pipeline when appropriate, and publishes the
    /// completion event. Called once the editor pane has exited but before
    /// its `AppEvent::PaneDied` is folded into `AppState` — pane identity
    /// (`public_pane_id`/`public_workspace_id`) must resolve while the pane
    /// still exists in the workspace.
    pub(super) fn finish_config_editor(&mut self, operation: ConfigEditorOperation) {
        let Some((ws_idx, pane_state)) = self.find_pane(operation.pane_id) else {
            return;
        };
        let Some(pane_id) = self.public_pane_id(ws_idx, operation.pane_id) else {
            return;
        };
        let workspace_id = self.public_workspace_id(ws_idx);
        let terminal_id = pane_state.attached_terminal_id.clone();
        let editor_exit_success = self
            .terminal_runtimes
            .get(&terminal_id)
            .and_then(|runtime| runtime.last_exit_success());

        let (outcome, diagnostics) =
            self.validate_config_editor_exit(&operation, editor_exit_success);

        self.state.config_editor_last_result = Some(ConfigEditorLastResult {
            outcome,
            diagnostics: diagnostics.clone(),
        });

        self.emit_event(crate::api::schema::EventEnvelope {
            event: crate::api::schema::EventKind::ServerConfigEditFinished,
            data: crate::api::schema::EventData::ServerConfigEditFinished {
                pane_id,
                workspace_id,
                outcome,
                diagnostics,
            },
        });
    }

    fn validate_config_editor_exit(
        &mut self,
        operation: &ConfigEditorOperation,
        editor_exit_success: Option<bool>,
    ) -> (crate::api::schema::ConfigEditOutcome, Vec<String>) {
        use crate::api::schema::ConfigEditOutcome;

        let path = &operation.config_path;
        // Unchanged bytes (present or, per the branch below, still absent) fold
        // together whether or not the editor itself failed: a nonzero exit with
        // no file change is `editor_failed`, distinct from an unreadable
        // *changed* file, which is always `invalid` regardless of exit code.
        let unchanged_outcome = || {
            if editor_exit_success == Some(false) {
                (
                    ConfigEditOutcome::EditorFailed,
                    vec!["editor process exited unsuccessfully".to_string()],
                )
            } else {
                (ConfigEditOutcome::Unchanged, Vec::new())
            }
        };

        match std::fs::read(path) {
            Ok(final_bytes) => {
                if operation.pre_bytes.as_ref() == Some(&final_bytes) {
                    return unchanged_outcome();
                }
                let report = self.apply_config_from_disk(false);
                let mut diagnostics = vec![format!("{}", path.display())];
                diagnostics.extend(report.diagnostics);
                match report.status {
                    crate::config::ConfigReloadStatus::Failed => {
                        (ConfigEditOutcome::Invalid, bounded_diagnostics(diagnostics))
                    }
                    crate::config::ConfigReloadStatus::Applied
                    | crate::config::ConfigReloadStatus::Partial => {
                        if editor_exit_success == Some(false) {
                            diagnostics.push(
                                "editor process exited unsuccessfully; changed bytes were valid and were reloaded"
                                    .to_string(),
                            );
                        }
                        (
                            ConfigEditOutcome::Reloaded,
                            bounded_diagnostics(diagnostics),
                        )
                    }
                }
            }
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
                if operation.pre_existed {
                    // Deliberate removal always reloads defaults, independent
                    // of the editor's own exit code.
                    let report = self.apply_config_from_disk(false);
                    let mut diagnostics = vec![format!(
                        "{} was removed; reloaded default configuration",
                        path.display()
                    )];
                    diagnostics.extend(report.diagnostics);
                    (
                        ConfigEditOutcome::Reloaded,
                        bounded_diagnostics(diagnostics),
                    )
                } else {
                    unchanged_outcome()
                }
            }
            Err(err) => {
                // A changed-but-unreadable file is always `invalid` — this is
                // independent of the editor's own exit code, unlike the
                // unchanged-file case above.
                let diagnostics = bounded_diagnostics(vec![format!(
                    "{}: failed to read changed config: {err}",
                    path.display()
                )]);
                (ConfigEditOutcome::Invalid, diagnostics)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::schema::ConfigEditOutcome;
    use crate::config::CONFIG_PATH_ENV_VAR;

    struct ConfigPathGuard {
        previous: Option<std::ffi::OsString>,
        path: std::path::PathBuf,
    }

    impl ConfigPathGuard {
        fn new(name: &str) -> Self {
            let dir = unique_temp_path(name);
            let path = dir.join("config.toml");
            let previous = std::env::var_os(CONFIG_PATH_ENV_VAR);
            std::env::set_var(CONFIG_PATH_ENV_VAR, &path);
            Self { previous, path }
        }
    }

    impl Drop for ConfigPathGuard {
        fn drop(&mut self) {
            match self.previous.take() {
                Some(value) => std::env::set_var(CONFIG_PATH_ENV_VAR, value),
                None => std::env::remove_var(CONFIG_PATH_ENV_VAR),
            }
            if let Some(dir) = self.path.parent() {
                let _ = std::fs::remove_dir_all(dir);
            }
        }
    }

    fn unique_temp_path(name: &str) -> std::path::PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        std::env::temp_dir().join(format!(
            "shepherd-config-editor-{name}-{}-{nanos}",
            std::process::id()
        ))
    }

    fn test_app_with_pane() -> (App, crate::layout::PaneId) {
        let (_api_tx, api_rx) = tokio::sync::mpsc::unbounded_channel();
        let mut app = App::new(
            &crate::config::Config::default(),
            true,
            None,
            api_rx,
            crate::api::EventHub::default(),
        );
        let workspace = crate::workspace::Workspace::test_new("config-editor-test");
        let pane_id = workspace.tabs[0].root_pane;
        app.state.workspaces = vec![workspace];
        app.state.active = Some(0);
        app.state.selected = 0;
        app.state.ensure_test_terminals();
        (app, pane_id)
    }

    fn operation_for(
        pane_id: crate::layout::PaneId,
        path: std::path::PathBuf,
        pre_bytes: Option<Vec<u8>>,
    ) -> ConfigEditorOperation {
        ConfigEditorOperation {
            pane_id,
            pre_existed: pre_bytes.is_some(),
            config_path: path,
            pre_bytes,
        }
    }

    #[test]
    fn open_config_editor_reports_surface_unavailable_without_active_workspace() {
        let (_api_tx, api_rx) = tokio::sync::mpsc::unbounded_channel();
        let mut app = App::new(
            &crate::config::Config::default(),
            true,
            None,
            api_rx,
            crate::api::EventHub::default(),
        );
        app.state.active = None;

        let err = app.open_config_editor().unwrap_err();

        assert!(app.active_config_editor.is_none());
        assert!(!err.to_string().is_empty());
    }

    #[test]
    fn finish_config_editor_reports_unchanged_when_bytes_are_identical() {
        let _guard = ConfigPathGuard::new("unchanged");
        let (mut app, pane_id) = test_app_with_pane();
        let path = crate::config::config_path();
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, "onboarding = false\n").unwrap();
        let operation = operation_for(pane_id, path, Some(b"onboarding = false\n".to_vec()));

        app.finish_config_editor(operation);

        let result = app.state.config_editor_last_result.clone().unwrap();
        assert_eq!(result.outcome, ConfigEditOutcome::Unchanged);
        assert!(result.diagnostics.is_empty());
    }

    #[test]
    fn finish_config_editor_reports_unchanged_when_config_never_existed() {
        let _guard = ConfigPathGuard::new("never-existed");
        let (mut app, pane_id) = test_app_with_pane();
        let path = crate::config::config_path();
        let operation = operation_for(pane_id, path, None);

        app.finish_config_editor(operation);

        let result = app.state.config_editor_last_result.clone().unwrap();
        assert_eq!(result.outcome, ConfigEditOutcome::Unchanged);
        assert!(result.diagnostics.is_empty());
    }

    #[test]
    fn finish_config_editor_reloads_valid_changed_config() {
        let _guard = ConfigPathGuard::new("valid-reload");
        let (mut app, pane_id) = test_app_with_pane();
        let path = crate::config::config_path();
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, "onboarding = false\n").unwrap();
        let operation = operation_for(pane_id, path, Some(Vec::new()));

        app.finish_config_editor(operation);

        let result = app.state.config_editor_last_result.clone().unwrap();
        assert_eq!(result.outcome, ConfigEditOutcome::Reloaded);
    }

    #[test]
    fn finish_config_editor_keeps_invalid_bytes_and_last_valid_runtime() {
        let _guard = ConfigPathGuard::new("invalid");
        let (mut app, pane_id) = test_app_with_pane();
        let path = crate::config::config_path();
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, "not valid toml [[[").unwrap();
        let operation = operation_for(pane_id, path.clone(), Some(Vec::new()));

        app.finish_config_editor(operation);

        let result = app.state.config_editor_last_result.clone().unwrap();
        assert_eq!(result.outcome, ConfigEditOutcome::Invalid);
        assert!(!result.diagnostics.is_empty());
        // The invalid bytes must remain untouched on disk.
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            "not valid toml [[["
        );
    }

    #[test]
    fn finish_config_editor_reloads_defaults_when_config_is_removed() {
        let _guard = ConfigPathGuard::new("removed");
        let (mut app, pane_id) = test_app_with_pane();
        let path = crate::config::config_path();
        // File existed before the edit but is absent now — no create_dir_all,
        // no write: the removal itself is what we're validating.
        let operation = operation_for(pane_id, path, Some(b"onboarding = false\n".to_vec()));

        app.finish_config_editor(operation);

        let result = app.state.config_editor_last_result.clone().unwrap();
        assert_eq!(result.outcome, ConfigEditOutcome::Reloaded);
        assert!(result
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.contains("removed")));
    }

    #[test]
    fn finish_config_editor_reports_invalid_when_changed_bytes_are_unreadable() {
        let _guard = ConfigPathGuard::new("unreadable");
        let (mut app, pane_id) = test_app_with_pane();
        let path = crate::config::config_path();
        // A directory at the config path is not NotFound but also not
        // readable as file bytes — simulates an unreadable changed file
        // without relying on platform-specific permission games.
        std::fs::create_dir_all(&path).unwrap();
        let operation = operation_for(pane_id, path, Some(b"onboarding = false\n".to_vec()));

        app.finish_config_editor(operation);

        let result = app.state.config_editor_last_result.clone().unwrap();
        assert_eq!(result.outcome, ConfigEditOutcome::Invalid);
        assert!(!result.diagnostics.is_empty());
    }

    #[test]
    fn finish_config_editor_emits_completion_event_exactly_once() {
        let _guard = ConfigPathGuard::new("completion-event");
        let (mut app, pane_id) = test_app_with_pane();
        let event_hub = app.event_hub.clone();
        let path = crate::config::config_path();
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, "onboarding = false\n").unwrap();
        let operation = operation_for(pane_id, path, Some(Vec::new()));

        app.finish_config_editor(operation);

        let events = event_hub.events_after(0);
        let matches = events
            .iter()
            .filter(|(_, event)| {
                event.event == crate::api::schema::EventKind::ServerConfigEditFinished
            })
            .count();
        assert_eq!(
            matches, 1,
            "exactly one completion event per pane per outcome"
        );
        let (_, event) = events
            .iter()
            .find(|(_, event)| {
                event.event == crate::api::schema::EventKind::ServerConfigEditFinished
            })
            .unwrap();
        match &event.data {
            crate::api::schema::EventData::ServerConfigEditFinished {
                outcome,
                pane_id: event_pane_id,
                ..
            } => {
                assert_eq!(*outcome, ConfigEditOutcome::Reloaded);
                assert!(!event_pane_id.is_empty());
            }
            other => panic!("unexpected event data: {other:?}"),
        }
    }

    #[cfg(unix)]
    fn write_editor_script(dir: &std::path::Path, body: &str) -> std::path::PathBuf {
        use std::os::unix::fs::PermissionsExt;

        std::fs::create_dir_all(dir).unwrap();
        let script_path = dir.join("editor.sh");
        std::fs::write(&script_path, format!("#!/bin/sh\n{body}\n")).unwrap();
        std::fs::set_permissions(&script_path, std::fs::Permissions::from_mode(0o755)).unwrap();
        script_path
    }

    #[cfg(unix)]
    struct EditorEnvGuard(Option<std::ffi::OsString>);

    #[cfg(unix)]
    impl EditorEnvGuard {
        fn set(script_path: &std::path::Path) -> Self {
            let previous = std::env::var_os("EDITOR");
            std::env::set_var("EDITOR", script_path);
            std::env::remove_var("VISUAL");
            Self(previous)
        }
    }

    #[cfg(unix)]
    impl Drop for EditorEnvGuard {
        fn drop(&mut self) {
            match self.0.take() {
                Some(value) => std::env::set_var("EDITOR", value),
                None => std::env::remove_var("EDITOR"),
            }
        }
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn open_config_editor_returns_existing_pane_on_duplicate_request() {
        let _guard = ConfigPathGuard::new("duplicate");
        let (mut app, _pane_id) = test_app_with_pane();
        let script = write_editor_script(&unique_temp_path("duplicate-editor"), "sleep 30");
        let _editor_guard = EditorEnvGuard::set(&script);

        let first = app.open_config_editor().unwrap();
        assert!(!first.already_open);

        let second = app.open_config_editor().unwrap();
        assert!(second.already_open);
        assert_eq!(second.pane_id, first.pane_id);
        assert_eq!(app.overlay_panes.len(), 1);

        let runtimes: Vec<_> = app.terminal_runtimes.drain().collect();
        for (_terminal_id, runtime) in runtimes {
            runtime.shutdown();
        }
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn open_config_editor_never_registers_config_path_for_temp_file_cleanup() {
        let _guard = ConfigPathGuard::new("no-temp-cleanup");
        let (mut app, _pane_id) = test_app_with_pane();
        let script = write_editor_script(&unique_temp_path("no-cleanup-editor"), "sleep 30");
        let _editor_guard = EditorEnvGuard::set(&script);

        let outcome = app.open_config_editor().unwrap();

        let overlay = app.overlay_panes.get(&outcome.pane_id).unwrap();
        assert!(overlay.temp_files.is_empty());

        let runtimes: Vec<_> = app.terminal_runtimes.drain().collect();
        for (_terminal_id, runtime) in runtimes {
            runtime.shutdown();
        }
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn open_config_editor_clears_launch_argv_so_restore_never_replays_the_editor() {
        let _guard = ConfigPathGuard::new("no-restore-replay");
        let (mut app, _pane_id) = test_app_with_pane();
        let script = write_editor_script(&unique_temp_path("no-replay-editor"), "sleep 30");
        let _editor_guard = EditorEnvGuard::set(&script);

        let outcome = app.open_config_editor().unwrap();
        let (_, pane_state) = app.find_pane(outcome.pane_id).unwrap();
        let terminal_id = pane_state.attached_terminal_id.clone();
        let terminal = app.state.terminals.get(&terminal_id).unwrap();
        assert!(terminal.launch_argv.is_none());

        let runtimes: Vec<_> = app.terminal_runtimes.drain().collect();
        for (_terminal_id, runtime) in runtimes {
            runtime.shutdown();
        }
    }

    // The remaining two scenarios (unchanged-but-nonzero-exit, and
    // changed-valid-but-nonzero-exit) exercise `validate_config_editor_exit`
    // directly with a synthetic `editor_exit_success`, the same way the
    // outcome tests above exercise it with `None`. A real end-to-end test
    // that waits for an actually-spawned editor process to exit and its
    // `PaneDied` to arrive is deliberately not attempted here: the scrollback
    // editor's own real-process test (`app/input/navigate.rs`) only waits for
    // its output file to appear on disk for exactly this reason — waiting on
    // real child-process reaping from a test is unreliable across sandboxes.

    #[test]
    fn validate_config_editor_exit_marks_editor_failed_when_unchanged_and_nonzero() {
        let _guard = ConfigPathGuard::new("validate-unchanged-nonzero");
        let (mut app, pane_id) = test_app_with_pane();
        let path = crate::config::config_path();
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, "onboarding = false\n").unwrap();
        let operation = operation_for(pane_id, path, Some(b"onboarding = false\n".to_vec()));

        let (outcome, diagnostics) = app.validate_config_editor_exit(&operation, Some(false));

        assert_eq!(outcome, ConfigEditOutcome::EditorFailed);
        assert!(!diagnostics.is_empty());
    }

    #[test]
    fn validate_config_editor_exit_reloads_valid_bytes_despite_nonzero_exit() {
        let _guard = ConfigPathGuard::new("validate-changed-nonzero");
        let (mut app, pane_id) = test_app_with_pane();
        let path = crate::config::config_path();
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, "onboarding = false\n").unwrap();
        let operation = operation_for(pane_id, path, Some(Vec::new()));

        let (outcome, diagnostics) = app.validate_config_editor_exit(&operation, Some(false));

        assert_eq!(outcome, ConfigEditOutcome::Reloaded);
        assert!(diagnostics
            .iter()
            .any(|diagnostic| diagnostic.contains("exited unsuccessfully")));
    }

    /// The two `validate_config_editor_exit` cases above pass a synthetic status,
    /// which proves the outcome logic but not that a real editor's exit code ever
    /// reaches it. This drives a real child to a nonzero exit and asserts the
    /// whole path — process exit -> `PaneRuntime::last_exit_success` ->
    /// `finish_config_editor` -> outcome.
    ///
    /// Deliberately polls the runtime accessor rather than awaiting `PaneDied`:
    /// the watcher stores the status from a `spawn_blocking` thread, but delivers
    /// `PaneDied` via `Handle::block_on`, which a current-thread test runtime
    /// cannot drive while the test itself is blocked. The accessor is what
    /// `finish_config_editor` reads, so it is the contract worth pinning.
    #[cfg(unix)]
    #[tokio::test]
    async fn real_editor_nonzero_exit_reaches_finish_through_the_runtime() {
        let _guard = ConfigPathGuard::new("real-nonzero-exit");
        let path = crate::config::config_path();
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, "onboarding = false\n").unwrap();
        let pre_bytes = std::fs::read(&path).unwrap();

        let (mut app, _pane_id) = test_app_with_pane();
        // Write valid TOML, then fail: the "nonzero exit with valid bytes" case.
        let script = write_editor_script(
            &unique_temp_path("real-nonzero-editor"),
            "printf 'onboarding = true\\n' > \"$1\"\nexit 3",
        );
        let _editor_guard = EditorEnvGuard::set(&script);

        let opened = app.open_config_editor().unwrap();
        assert!(!opened.already_open);

        // Only the editor pane owns a real child, so any observed exit is its own.
        let mut observed = None;
        for _ in 0..200 {
            observed = app
                .terminal_runtimes
                .values()
                .find_map(|runtime| runtime.last_exit_success());
            if observed.is_some() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(25));
        }
        assert_eq!(
            observed,
            Some(false),
            "real editor exit status should reach the runtime accessor"
        );

        let operation = app
            .take_config_editor_completion(opened.pane_id)
            .expect("editor operation should still be active");
        assert_eq!(operation.pre_bytes.as_deref(), Some(pre_bytes.as_slice()));

        app.finish_config_editor(operation);

        let result = app
            .state
            .config_editor_last_result
            .as_ref()
            .expect("completion should record a result");
        assert_eq!(result.outcome, ConfigEditOutcome::Reloaded);
        assert!(result
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.contains("exited unsuccessfully")));

        let runtimes: Vec<_> = app.terminal_runtimes.drain().collect();
        for (_terminal_id, runtime) in runtimes {
            runtime.shutdown();
        }
    }
}
