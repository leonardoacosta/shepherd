//! Demand-driven session and provider status refresh.
//!
//! Structurally this is `project_status_refresh.rs` with a different adapter: demand is
//! resolved from the configured rows, work is deduplicated per checkout, and the adapter runs
//! off the render path. Every failure mode degrades to an absent value — see
//! `crate::workspace::session_status` for the parser.
//!
//! Shepherd pulls. Nothing here waits for an outside process to report a value in, which is
//! what lets an absent value mean "no answer" rather than "nobody spoke".

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Instant;

use super::provider::run_provider;
use super::{App, SESSION_STATUS_REFRESH_INTERVAL};
use crate::events::AppEvent;
use crate::workspace::{
    merge_persisted_sessions, parse_context_floor_status, parse_llmtrim_status,
    parse_persisted_session_record, ContextFloorStatus, LlmTrimStatus, SessionStatusRefreshDemand,
    SessionStatusSnapshot, SessionsStatus, WorkspaceSessionStatus,
};

#[derive(Clone, Debug, PartialEq, Eq)]
struct SessionStatusRefreshItem {
    workspace_id: String,
    checkout_key: PathBuf,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct SessionStatusRefreshJob {
    checkout_key: PathBuf,
    workspace_ids: Vec<String>,
}

impl App {
    pub(crate) fn start_session_status_refresh_if_due(&mut self, now: Instant) {
        let Some(deadline) = self.session_status_refresh_deadline() else {
            return;
        };
        if now < deadline {
            return;
        }

        let items = self.session_status_refresh_items();
        if items.is_empty() {
            self.last_session_status_refresh = now;
            return;
        }

        let demand = self.session_status_demand();
        if demand.is_empty() {
            self.last_session_status_refresh = now;
            return;
        }

        self.session_status_refresh_in_flight = true;
        self.last_session_status_refresh = now;
        let event_tx = self.event_tx.clone();
        std::thread::spawn(move || {
            let results = refresh_session_statuses(items, demand);
            let _ = event_tx.blocking_send(AppEvent::SessionStatusRefreshed { results });
        });
    }

    pub(crate) fn session_status_refresh_deadline(&self) -> Option<Instant> {
        (!self.session_status_refresh_in_flight
            && !self.state.workspaces.is_empty()
            && !self.session_status_demand().is_empty())
        .then_some(self.last_session_status_refresh + SESSION_STATUS_REFRESH_INTERVAL)
    }

    /// Only the tokens an operator actually configured cause an adapter run. Reads every
    /// surface that consumes the Agent vocabulary — the topbar, the right panel, and the
    /// per-agent sidebar rows — because any one of them renders the token.
    fn session_status_demand(&self) -> SessionStatusRefreshDemand {
        let mut demand = SessionStatusRefreshDemand::default();
        let agent_rows = self
            .state
            .sidebar_agents
            .rows
            .iter()
            .chain(self.state.topbar_rows.iter())
            .chain(self.state.right_panel_rows.iter());
        for token in agent_rows.flatten() {
            if matches!(token.parts().0, crate::config::AgentSidebarToken::Spend) {
                demand.llmtrim = true;
            }
        }
        demand
    }

    fn session_status_refresh_items(&self) -> Vec<SessionStatusRefreshItem> {
        self.state
            .workspaces
            .iter()
            .filter_map(|ws| {
                Some(SessionStatusRefreshItem {
                    workspace_id: ws.id.clone(),
                    checkout_key: ws.session_status_key()?,
                })
            })
            .collect()
    }

    pub(crate) fn apply_session_status_results(&mut self, results: Vec<WorkspaceSessionStatus>) {
        self.session_status_refresh_in_flight = false;
        for result in results {
            let Some(ws) = self
                .state
                .workspaces
                .iter_mut()
                .find(|ws| ws.id == result.workspace_id)
            else {
                continue;
            };
            ws.apply_session_status(result.checkout_key, result.snapshot);
        }
    }
}

/// Several spaces resolving to the same checkout share one adapter run.
fn deduplicate_session_status_items(
    items: Vec<SessionStatusRefreshItem>,
) -> Vec<SessionStatusRefreshJob> {
    let mut indexes = HashMap::<PathBuf, usize>::new();
    let mut jobs = Vec::<SessionStatusRefreshJob>::new();

    for item in items {
        if let Some(&index) = indexes.get(&item.checkout_key) {
            jobs[index].workspace_ids.push(item.workspace_id);
            continue;
        }
        indexes.insert(item.checkout_key.clone(), jobs.len());
        jobs.push(SessionStatusRefreshJob {
            checkout_key: item.checkout_key,
            workspace_ids: vec![item.workspace_id],
        });
    }

    jobs
}

fn refresh_session_statuses(
    items: Vec<SessionStatusRefreshItem>,
    demand: SessionStatusRefreshDemand,
) -> Vec<WorkspaceSessionStatus> {
    let mut results = Vec::new();
    for job in deduplicate_session_status_items(items) {
        let snapshot = session_status_for_checkout(&job.checkout_key, demand);
        results.extend(
            job.workspace_ids
                .into_iter()
                .map(|workspace_id| WorkspaceSessionStatus {
                    workspace_id,
                    checkout_key: job.checkout_key.clone(),
                    snapshot: snapshot.clone(),
                }),
        );
    }
    results
}

/// One invocation per source, each gated on its own demand disjunct — a missing `llmtrim`
/// binary elides only `llmtrim`, an unreadable session store elides only `sessions`, and so on.
fn session_status_for_checkout(
    checkout: &Path,
    demand: SessionStatusRefreshDemand,
) -> SessionStatusSnapshot {
    session_status_for_checkout_with_sessions_root(
        checkout,
        demand,
        default_persisted_session_store_root(),
    )
}

/// Split out from `session_status_for_checkout` so tests can point the `sessions` source at a
/// fixture directory instead of the real, environment-resolved store root.
fn session_status_for_checkout_with_sessions_root(
    checkout: &Path,
    demand: SessionStatusRefreshDemand,
    sessions_root: Option<PathBuf>,
) -> SessionStatusSnapshot {
    SessionStatusSnapshot {
        llmtrim: demand
            .llmtrim
            .then(|| llmtrim_status_for_checkout(checkout))
            .flatten(),
        context_floor: demand
            .context_floor
            .then(|| context_floor_status_for_checkout(checkout))
            .flatten(),
        sessions: demand
            .sessions
            .then(|| sessions_status_for_store(sessions_root))
            .flatten(),
    }
}

fn llmtrim_status_for_checkout(checkout: &Path) -> Option<LlmTrimStatus> {
    parse_llmtrim_status(&run_provider(checkout, "llmtrim", &["status", "--json"])?)
}

fn context_floor_status_for_checkout(checkout: &Path) -> Option<ContextFloorStatus> {
    parse_context_floor_status(&run_provider(checkout, "context-floor", &["--json"])?)
}

/// `SHEPHERD_STATE_SNAPSHOT_DIR` mirrors the companion's own override so a developer pointing
/// the companion at a scratch snapshot store also redirects this read — the two halves share
/// one store, so they must agree on where it lives. Falls back to the companion's own default
/// resolution: `$XDG_STATE_HOME/shepherd-state/snapshots/v1`, else
/// `$HOME/.local/state/shepherd-state/snapshots/v1`.
fn default_persisted_session_store_root() -> Option<PathBuf> {
    if let Ok(root) = std::env::var("SHEPHERD_STATE_SNAPSHOT_DIR") {
        if !root.is_empty() {
            return Some(PathBuf::from(root));
        }
    }
    if let Ok(root) = std::env::var("XDG_STATE_HOME") {
        if !root.is_empty() {
            return Some(PathBuf::from(root).join("shepherd-state/snapshots/v1"));
        }
    }
    dirs_home().map(|home| home.join(".local/state/shepherd-state/snapshots/v1"))
}

#[cfg(unix)]
fn dirs_home() -> Option<PathBuf> {
    std::env::var_os("HOME").map(PathBuf::from)
}

#[cfg(windows)]
fn dirs_home() -> Option<PathBuf> {
    std::env::var_os("USERPROFILE").map(PathBuf::from)
}

/// Walks every session file under the store root, skipping the `.locks` directory the
/// companion's writer uses for its own flock stripes — mirrors `persistedSessionStoreProducer`.
/// One unreadable or malformed entry is isolated rather than failing the whole source, and an
/// absent or empty root is a valid zero-session store rather than a failure: `read_dir` on a
/// missing directory is simply skipped by `walk_session_store`, same as the companion's
/// `WalkDir`, which never treats a missing root as an error for this producer.
fn sessions_status_for_store(root: Option<PathBuf>) -> Option<SessionsStatus> {
    let root = root?;
    let mut records = Vec::new();
    walk_session_store(&root, &mut records);
    Some(merge_persisted_sessions(&records))
}

fn walk_session_store(dir: &Path, records: &mut Vec<crate::workspace::PersistedSessionRecord>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        if file_type.is_dir() {
            if entry.file_name() == ".locks" {
                continue;
            }
            walk_session_store(&path, records);
            continue;
        }
        if path.extension().and_then(|ext| ext.to_str()) != Some("json") {
            continue;
        }
        let Ok(contents) = std::fs::read_to_string(&path) else {
            continue;
        };
        if let Some(record) = parse_persisted_session_record(&contents) {
            records.push(record);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::AgentSidebarToken;

    fn item(workspace_id: &str, checkout: &str) -> SessionStatusRefreshItem {
        SessionStatusRefreshItem {
            workspace_id: workspace_id.to_string(),
            checkout_key: PathBuf::from(checkout),
        }
    }

    fn app_with(config: crate::config::Config) -> super::super::App {
        super::super::App::new(
            &config,
            true,
            None,
            tokio::sync::mpsc::unbounded_channel().1,
            crate::api::EventHub::default(),
        )
    }

    fn test_app(rows: Vec<Vec<AgentSidebarToken>>) -> super::super::App {
        let mut config = crate::config::Config::default();
        config.ui.topbar.rows = rows;
        app_with(config)
    }

    #[test]
    fn spaces_sharing_a_checkout_collapse_to_one_job() {
        let jobs = deduplicate_session_status_items(vec![
            item("w1", "/repo"),
            item("w2", "/repo"),
            item("w3", "/repo"),
        ]);

        assert_eq!(jobs.len(), 1, "one adapter run serves all three spaces");
        assert_eq!(jobs[0].workspace_ids, vec!["w1", "w2", "w3"]);
    }

    #[test]
    fn every_space_in_a_shared_checkout_receives_the_same_snapshot() {
        let results = refresh_session_statuses(
            vec![
                item("w1", "/nonexistent-checkout"),
                item("w2", "/nonexistent-checkout"),
            ],
            SessionStatusRefreshDemand::default(),
        );

        assert_eq!(results.len(), 2);
        assert_eq!(results[0].snapshot, results[1].snapshot);
    }

    #[test]
    fn absent_demand_runs_no_adapter_and_yields_an_empty_snapshot() {
        let snapshot = session_status_for_checkout(
            Path::new("/nonexistent-checkout"),
            SessionStatusRefreshDemand::default(),
        );
        assert!(snapshot.llmtrim.is_none());
        assert!(snapshot.context_floor.is_none());
        assert!(snapshot.sessions.is_none());
    }

    #[test]
    fn no_configured_consumer_starts_no_refresh_and_runs_no_adapter() {
        let mut app = test_app(vec![vec![AgentSidebarToken::Workspace]]);
        app.state
            .workspaces
            .push(crate::workspace::Workspace::test_new("test"));
        let now = Instant::now();
        app.last_session_status_refresh = now - SESSION_STATUS_REFRESH_INTERVAL;

        app.start_session_status_refresh_if_due(now);

        assert!(app.session_status_refresh_deadline().is_none());
        assert!(!app.session_status_refresh_in_flight);
        assert!(app.event_rx.try_recv().is_err());
    }

    #[test]
    fn each_token_enables_only_its_own_adapter() {
        let cases = [
            (
                AgentSidebarToken::Workspace,
                SessionStatusRefreshDemand::default(),
            ),
            (
                AgentSidebarToken::Spend,
                SessionStatusRefreshDemand {
                    llmtrim: true,
                    ..Default::default()
                },
            ),
        ];

        for (token, expected) in cases {
            let app = test_app(vec![vec![token.clone()]]);
            assert_eq!(
                app.session_status_demand(),
                expected,
                "demand must follow the configured token {token:?}"
            );
        }
    }

    /// The Agent vocabulary renders on three surfaces. A token configured on any one of them
    /// is a real consumer, so demand must not read the topbar alone.
    #[test]
    fn demand_reads_every_surface_that_renders_the_agent_vocabulary() {
        let mut sidebar = crate::config::Config::default();
        sidebar.ui.sidebar.agents.rows = vec![vec![AgentSidebarToken::Spend]];
        assert_eq!(
            app_with(sidebar).session_status_demand(),
            SessionStatusRefreshDemand {
                llmtrim: true,
                ..Default::default()
            },
            "a token configured on the agent sidebar is a consumer"
        );

        let mut panel = crate::config::Config::default();
        panel.ui.right_panel.rows = vec![vec![AgentSidebarToken::Spend]];
        assert_eq!(
            app_with(panel).session_status_demand(),
            SessionStatusRefreshDemand {
                llmtrim: true,
                ..Default::default()
            },
            "a token configured on the right panel is a consumer"
        );
    }

    #[test]
    fn deadline_is_suppressed_while_a_refresh_is_in_flight() {
        let mut app = test_app(vec![vec![AgentSidebarToken::Spend]]);
        app.state
            .workspaces
            .push(crate::workspace::Workspace::test_new("test"));
        assert!(app.session_status_refresh_deadline().is_some());

        app.session_status_refresh_in_flight = true;
        assert!(app.session_status_refresh_deadline().is_none());
    }

    #[test]
    fn session_status_deadline_is_independent_of_the_git_deadline() {
        let mut app = test_app(vec![vec![AgentSidebarToken::Spend]]);
        app.state
            .workspaces
            .push(crate::workspace::Workspace::test_new("test"));
        let now = Instant::now();
        app.last_session_status_refresh = now;

        let session = app
            .session_status_refresh_deadline()
            .expect("configured consumer keeps the timer live");
        assert_eq!(session, now + SESSION_STATUS_REFRESH_INTERVAL);
        assert!(
            SESSION_STATUS_REFRESH_INTERVAL > super::super::GIT_REMOTE_STATUS_REFRESH_INTERVAL,
            "session status must poll slower than git status"
        );
    }

    /// The store hangs off the workspace, so identity churn — closed panes, reordered tabs,
    /// public numbers that diverge from raw ids — must not detach it or violate an invariant.
    #[test]
    fn the_store_survives_adversarial_identity_state() {
        let mut app = test_app(vec![vec![AgentSidebarToken::Spend]]);
        app.state
            .workspaces
            .push(crate::workspace::Workspace::test_adversarial_identity_state());
        app.state.active = Some(0);
        let id = app.state.workspaces[0].id.clone();

        app.state.workspaces[0].assert_invariants_for_test();
        assert_eq!(
            app.state.workspaces[0].session_status(),
            crate::workspace::SessionStatusSnapshot::default(),
            "a fresh workspace starts with an absent value, not a stale one"
        );

        app.apply_session_status_results(vec![WorkspaceSessionStatus {
            workspace_id: id,
            checkout_key: PathBuf::from("/repo"),
            snapshot: SessionStatusSnapshot {
                llmtrim: Some(crate::workspace::LlmTrimStatus {
                    spend: Some(crate::workspace::SpendStatus { cents: 4200 }),
                    ..Default::default()
                }),
                ..Default::default()
            },
        }]);

        // Workspace-level, deliberately: the store is a `Workspace` field, and the AppState
        // invariant additionally requires live terminals for every attached pane — wiring
        // unrelated to what this change can break.
        app.state.workspaces[0].assert_invariants_for_test();
        assert_eq!(
            app.state.workspaces[0]
                .session_status()
                .llmtrim
                .and_then(|l| l.spend),
            Some(crate::workspace::SpendStatus { cents: 4200 }),
            "the store round-trips through adversarial identity state"
        );
    }

    /// The store is what render reads. A refresh in flight must leave the previously cached
    /// value in place rather than blanking it while the adapter runs.
    #[test]
    fn an_in_flight_refresh_leaves_the_cached_value_readable() {
        let mut app = test_app(vec![vec![AgentSidebarToken::Spend]]);
        app.state
            .workspaces
            .push(crate::workspace::Workspace::test_new("test"));
        let id = app.state.workspaces[0].id.clone();
        app.apply_session_status_results(vec![WorkspaceSessionStatus {
            workspace_id: id,
            checkout_key: PathBuf::from("/repo"),
            snapshot: SessionStatusSnapshot {
                llmtrim: Some(crate::workspace::LlmTrimStatus {
                    spend: Some(crate::workspace::SpendStatus { cents: 4200 }),
                    ..Default::default()
                }),
                ..Default::default()
            },
        }]);

        app.session_status_refresh_in_flight = true;

        assert_eq!(
            app.state.workspaces[0]
                .session_status()
                .llmtrim
                .and_then(|l| l.spend),
            Some(crate::workspace::SpendStatus { cents: 4200 }),
            "render keeps reading the cached value while a refresh runs"
        );
    }

    /// One-off scratch directory under the OS temp root, removed on drop. Not `scratchpad/` —
    /// that convention is for this session's own working files, not test fixtures a `cargo
    /// nextest` worker creates and tears down per-run.
    struct TempSessionStore {
        root: PathBuf,
    }

    impl TempSessionStore {
        fn new(name: &str) -> Self {
            let root = std::env::temp_dir().join(format!(
                "shepherd-session-status-test-{name}-{}-{}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_nanos())
                    .unwrap_or_default()
            ));
            std::fs::create_dir_all(&root).expect("create scratch session store dir");
            Self { root }
        }

        fn write(&self, relative: &str, contents: &str) {
            let path = self.root.join(relative);
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).expect("create scratch session store subdir");
            }
            std::fs::write(path, contents).expect("write scratch session file");
        }
    }

    impl Drop for TempSessionStore {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.root);
        }
    }

    fn session_record_json(session_id: &str, model: &str, observed_at: &str) -> String {
        format!(
            r#"{{"version":1,"harness":"codex","session_id":"{session_id}","model":"{model}","current_context_tokens":1000,"context_window_tokens":200000,"observed_at":"{observed_at}"}}"#
        )
    }

    // --- independence (task 2.4) ---

    /// `llmtrim` is not installed in the test environment, so its adapter fails via the "binary
    /// absent" class while `sessions` succeeds by reading a fixture directory directly — proving
    /// one failing adapter and one succeeding adapter resolve independently within one snapshot.
    #[test]
    fn one_failing_adapter_and_one_succeeding_adapter_resolve_independently() {
        // See the PATH-clearing comment on `llmtrim_adapter_elides_when_the_binary_is_absent` —
        // `llmtrim` may be genuinely installed in this dev environment.
        std::env::remove_var("PATH");

        let store = TempSessionStore::new("independence");
        store.write(
            "codex/session-a.json",
            &session_record_json("session-a", "gpt-5.6-sol", "2026-08-02T19:00:00Z"),
        );

        let snapshot = session_status_for_checkout_with_sessions_root(
            Path::new("/nonexistent-checkout"),
            SessionStatusRefreshDemand {
                llmtrim: true,
                sessions: true,
                ..Default::default()
            },
            Some(store.root.clone()),
        );

        assert!(
            snapshot.llmtrim.is_none(),
            "llmtrim has no binary in the test environment and must elide, not panic or fall back"
        );
        assert_eq!(
            snapshot.sessions.and_then(|s| s.model),
            Some("gpt-5.6-sol".to_string()),
            "sessions must still resolve from its own fixture, unaffected by llmtrim's failure"
        );
    }

    // --- llmtrim / context-floor: spawn failure classes ---

    #[test]
    fn llmtrim_adapter_elides_when_the_binary_is_absent() {
        // `llmtrim` and `context-floor` are real cc-tooling binaries and may be installed on a
        // developer's own PATH (as they are in this repo's dev environment), so "binary absent"
        // cannot be exercised by relying on the ambient environment. `cargo nextest` runs each
        // test in its own process, so clearing PATH here cannot affect any other test.
        std::env::remove_var("PATH");
        assert!(llmtrim_status_for_checkout(Path::new(".")).is_none());
    }

    #[test]
    fn context_floor_adapter_elides_when_the_binary_is_absent() {
        std::env::remove_var("PATH");
        assert!(context_floor_status_for_checkout(Path::new(".")).is_none());
    }

    // --- sessions: read failure classes (task 2.5) ---

    #[test]
    fn sessions_adapter_yields_a_zero_count_snapshot_when_the_root_is_absent() {
        let missing = Path::new("/nonexistent-shepherd-state-session-store-root");
        let status = sessions_status_for_store(Some(missing.to_path_buf()))
            .expect("an absent root is a valid empty store, not a failure");
        assert_eq!(status, crate::workspace::SessionsStatus::default());
    }

    #[test]
    fn sessions_adapter_elides_entirely_when_no_root_can_be_resolved() {
        assert!(sessions_status_for_store(None).is_none());
    }

    #[test]
    fn a_malformed_session_file_is_isolated_from_the_rest_of_the_store() {
        let store = TempSessionStore::new("malformed");
        store.write("codex/broken.json", "not json");
        store.write(
            "codex/valid.json",
            &session_record_json("session-a", "gpt-5.6-sol", "2026-08-02T19:00:00Z"),
        );

        let status = sessions_status_for_store(Some(store.root.clone())).expect("root exists");
        assert_eq!(
            status.session_count, 1,
            "the malformed entry is not counted"
        );
        assert_eq!(status.model.as_deref(), Some("gpt-5.6-sol"));
    }

    #[test]
    fn a_partially_written_session_file_is_isolated_from_the_rest_of_the_store() {
        // Missing session_id / observed_at: the shape a writer's crash mid-write can leave
        // behind, distinct from `broken.json`'s not-JSON-at-all case above.
        let store = TempSessionStore::new("partial");
        store.write(
            "codex/partial.json",
            r#"{"version":1,"model":"gpt-5.6-sol"}"#,
        );
        store.write(
            "codex/valid.json",
            &session_record_json("session-a", "claude-opus-5", "2026-08-02T19:00:00Z"),
        );

        let status = sessions_status_for_store(Some(store.root.clone())).expect("root exists");
        assert_eq!(status.session_count, 1);
        assert_eq!(status.model.as_deref(), Some("claude-opus-5"));
    }

    #[test]
    fn the_locks_directory_is_never_walked() {
        let store = TempSessionStore::new("locks");
        store.write(
            "codex/valid.json",
            &session_record_json("session-a", "gpt-5.6-sol", "2026-08-02T19:00:00Z"),
        );
        // A file directly under `.locks` that would parse as a session if it were ever visited.
        store.write(
            ".locks/session-a.json",
            &session_record_json("session-b", "should-not-be-seen", "2026-08-02T20:00:00Z"),
        );

        let status = sessions_status_for_store(Some(store.root.clone())).expect("root exists");
        assert_eq!(
            status.session_count, 1,
            ".locks entries must not be counted"
        );
        assert_eq!(status.model.as_deref(), Some("gpt-5.6-sol"));
    }

    #[test]
    fn non_json_entries_under_the_store_are_ignored() {
        let store = TempSessionStore::new("nonjson");
        store.write("codex/README.md", "not a session");
        store.write(
            "codex/valid.json",
            &session_record_json("session-a", "gpt-5.6-sol", "2026-08-02T19:00:00Z"),
        );

        let status = sessions_status_for_store(Some(store.root.clone())).expect("root exists");
        assert_eq!(status.session_count, 1);
    }

    #[cfg(unix)]
    #[test]
    fn an_unreadable_session_file_is_isolated_from_the_rest_of_the_store() {
        use std::os::unix::fs::PermissionsExt;

        let store = TempSessionStore::new("unreadable");
        store.write(
            "codex/valid.json",
            &session_record_json("session-a", "gpt-5.6-sol", "2026-08-02T19:00:00Z"),
        );
        store.write(
            "codex/locked.json",
            &session_record_json("session-b", "should-not-be-seen", "2026-08-02T20:00:00Z"),
        );
        let locked_path = store.root.join("codex/locked.json");
        std::fs::set_permissions(&locked_path, std::fs::Permissions::from_mode(0o000))
            .expect("chmod scratch fixture");

        let status = sessions_status_for_store(Some(store.root.clone())).expect("root exists");

        // Restore permissions before the scratch directory is removed on drop.
        std::fs::set_permissions(&locked_path, std::fs::Permissions::from_mode(0o644))
            .expect("restore scratch fixture permissions");

        assert_eq!(
            status.session_count, 1,
            "the unreadable entry is not counted"
        );
        assert_eq!(status.model.as_deref(), Some("gpt-5.6-sol"));
    }
}
