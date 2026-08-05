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
use crate::workspace::parse_spend_status;
use crate::workspace::SessionStatusRefreshDemand;
use crate::workspace::SessionStatusSnapshot;
use crate::workspace::WorkspaceSessionStatus;

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
                demand.spend = true;
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
                    snapshot,
                }),
        );
    }
    results
}

fn session_status_for_checkout(
    checkout: &Path,
    demand: SessionStatusRefreshDemand,
) -> SessionStatusSnapshot {
    SessionStatusSnapshot {
        spend: demand
            .spend
            .then(|| spend_status_for_checkout(checkout))
            .flatten(),
    }
}

fn spend_status_for_checkout(checkout: &Path) -> Option<crate::workspace::SpendStatus> {
    parse_spend_status(&run_provider(
        checkout,
        "shepherd-state",
        &["chrome", "--once", "--json"],
    )?)
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
        assert!(snapshot.spend.is_none());
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
                SessionStatusRefreshDemand { spend: true },
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
            SessionStatusRefreshDemand { spend: true },
            "a token configured on the agent sidebar is a consumer"
        );

        let mut panel = crate::config::Config::default();
        panel.ui.right_panel.rows = vec![vec![AgentSidebarToken::Spend]];
        assert_eq!(
            app_with(panel).session_status_demand(),
            SessionStatusRefreshDemand { spend: true },
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
                spend: Some(crate::workspace::SpendStatus { cents: 4200 }),
            },
        }]);

        // Workspace-level, deliberately: the store is a `Workspace` field, and the AppState
        // invariant additionally requires live terminals for every attached pane — wiring
        // unrelated to what this change can break.
        app.state.workspaces[0].assert_invariants_for_test();
        assert_eq!(
            app.state.workspaces[0].session_status().spend,
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
                spend: Some(crate::workspace::SpendStatus { cents: 4200 }),
            },
        }]);

        app.session_status_refresh_in_flight = true;

        assert_eq!(
            app.state.workspaces[0].session_status().spend,
            Some(crate::workspace::SpendStatus { cents: 4200 }),
            "render keeps reading the cached value while a refresh runs"
        );
    }
}
