//! Demand-driven per-checkout project status refresh.
//!
//! Structurally this is `git_refresh.rs` with a different probe and a slower clock: demand is
//! resolved from the configured space rows, work is deduplicated per checkout, and the probes
//! run off the render path. Every failure mode degrades to an absent value for that provider
//! alone — see `crate::workspace::project_status` for the parsers.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use super::{App, PROJECT_STATUS_REFRESH_INTERVAL};
use crate::events::AppEvent;
use crate::workspace::parse_bead_open_and_blocked;
use crate::workspace::parse_bead_ready;
use crate::workspace::parse_proposal_counts;
use crate::workspace::BeadCounts;
use crate::workspace::ProjectStatusRefreshDemand;
use crate::workspace::ProjectStatusSnapshot;
use crate::workspace::WorkspaceProjectStatus;

/// A provider that has not answered by now is not worth a frame's staleness. Kills the child
/// rather than letting a hung tool pin a thread for the process lifetime.
const PROVIDER_TIMEOUT: Duration = Duration::from_secs(5);
const PROVIDER_POLL_INTERVAL: Duration = Duration::from_millis(25);

#[derive(Clone, Debug, PartialEq, Eq)]
struct ProjectStatusRefreshItem {
    workspace_id: String,
    checkout_key: PathBuf,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ProjectStatusRefreshJob {
    checkout_key: PathBuf,
    workspace_ids: Vec<String>,
}

impl App {
    pub(crate) fn start_project_status_refresh_if_due(&mut self, now: Instant) {
        let Some(deadline) = self.project_status_refresh_deadline() else {
            return;
        };
        if now < deadline {
            return;
        }

        let items = self.project_status_refresh_items();
        if items.is_empty() {
            self.last_project_status_refresh = now;
            return;
        }

        let demand = self.project_status_demand();
        if demand.is_empty() {
            self.last_project_status_refresh = now;
            return;
        }

        self.project_status_refresh_in_flight = true;
        self.last_project_status_refresh = now;
        let event_tx = self.event_tx.clone();
        std::thread::spawn(move || {
            let results = refresh_project_statuses(items, demand);
            let _ = event_tx.blocking_send(AppEvent::ProjectStatusRefreshed { results });
        });
    }

    pub(crate) fn project_status_refresh_deadline(&self) -> Option<Instant> {
        (!self.project_status_refresh_in_flight
            && !self.state.workspaces.is_empty()
            && !self.project_status_demand().is_empty())
        .then_some(self.last_project_status_refresh + PROJECT_STATUS_REFRESH_INTERVAL)
    }

    /// Only the tokens an operator actually configured cause a probe. Mirrors
    /// `git_refresh_demand`, and reads the same single source.
    fn project_status_demand(&self) -> ProjectStatusRefreshDemand {
        let mut demand = ProjectStatusRefreshDemand::default();
        for token in self.state.sidebar_spaces.rows.iter().flatten() {
            match token.parts().0 {
                crate::config::SpaceSidebarToken::Proposals => demand.proposals = true,
                crate::config::SpaceSidebarToken::Beads => demand.beads = true,
                _ => {}
            }
        }
        demand
    }

    fn project_status_refresh_items(&self) -> Vec<ProjectStatusRefreshItem> {
        self.state
            .workspaces
            .iter()
            .filter_map(|ws| {
                Some(ProjectStatusRefreshItem {
                    workspace_id: ws.id.clone(),
                    checkout_key: ws.project_status_key()?,
                })
            })
            .collect()
    }

    pub(crate) fn apply_project_status_results(&mut self, results: Vec<WorkspaceProjectStatus>) {
        self.project_status_refresh_in_flight = false;
        for result in results {
            let Some(ws) = self
                .state
                .workspaces
                .iter_mut()
                .find(|ws| ws.id == result.workspace_id)
            else {
                continue;
            };
            ws.apply_project_status(result.checkout_key, result.snapshot);
        }
    }
}

/// Several spaces resolving to the same checkout share one probe.
fn deduplicate_project_status_items(
    items: Vec<ProjectStatusRefreshItem>,
) -> Vec<ProjectStatusRefreshJob> {
    let mut indexes = HashMap::<PathBuf, usize>::new();
    let mut jobs = Vec::<ProjectStatusRefreshJob>::new();

    for item in items {
        if let Some(&index) = indexes.get(&item.checkout_key) {
            jobs[index].workspace_ids.push(item.workspace_id);
            continue;
        }
        indexes.insert(item.checkout_key.clone(), jobs.len());
        jobs.push(ProjectStatusRefreshJob {
            checkout_key: item.checkout_key,
            workspace_ids: vec![item.workspace_id],
        });
    }

    jobs
}

fn refresh_project_statuses(
    items: Vec<ProjectStatusRefreshItem>,
    demand: ProjectStatusRefreshDemand,
) -> Vec<WorkspaceProjectStatus> {
    let mut results = Vec::new();
    for job in deduplicate_project_status_items(items) {
        let snapshot = project_status_for_checkout(&job.checkout_key, demand);
        results.extend(
            job.workspace_ids
                .into_iter()
                .map(|workspace_id| WorkspaceProjectStatus {
                    workspace_id,
                    checkout_key: job.checkout_key.clone(),
                    snapshot,
                }),
        );
    }
    results
}

fn project_status_for_checkout(
    checkout: &Path,
    demand: ProjectStatusRefreshDemand,
) -> ProjectStatusSnapshot {
    ProjectStatusSnapshot {
        proposals: demand
            .proposals
            .then(|| proposal_counts_for_checkout(checkout))
            .flatten(),
        beads: demand
            .beads
            .then(|| bead_counts_for_checkout(checkout))
            .flatten(),
    }
}

fn proposal_counts_for_checkout(checkout: &Path) -> Option<crate::workspace::ProposalCounts> {
    parse_proposal_counts(&run_provider(checkout, "openspec", &["list", "--json"])?)
}

/// Two probes: `bd list` carries open/blocked, `bd ready` carries the actionable count. If
/// either fails the whole token elides, so a half-populated triple is never rendered.
fn bead_counts_for_checkout(checkout: &Path) -> Option<BeadCounts> {
    let listed = run_provider(checkout, "bd", &["list", "--json"])?;
    let (open, blocked) = parse_bead_open_and_blocked(&listed)?;
    let ready = parse_bead_ready(&run_provider(checkout, "bd", &["ready", "--json"])?)?;
    Some(BeadCounts {
        open,
        ready,
        blocked,
    })
}

/// Every failure — binary absent, spawn error, non-zero exit, timeout — returns `None`.
/// Never panics and never surfaces an error to the user; an absent value is the signal.
fn run_provider(cwd: &Path, program: &str, args: &[&str]) -> Option<String> {
    let mut child = std::process::Command::new(program)
        .args(args)
        .current_dir(cwd)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
        .ok()?;

    // stdout must be drained concurrently with the wait. A provider whose output exceeds the
    // pipe buffer blocks on write until someone reads, so polling `try_wait` alone deadlocks
    // until the timeout — observed with `bd list --json` over a few hundred issues.
    let stdout = child.stdout.take()?;
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let mut buffer = Vec::new();
        let mut stdout = stdout;
        let read = std::io::Read::read_to_end(&mut stdout, &mut buffer);
        let _ = tx.send(read.map(|_| buffer));
    });

    let deadline = Instant::now() + PROVIDER_TIMEOUT;
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                if !status.success() {
                    return None;
                }
                break;
            }
            Ok(None) => {
                if Instant::now() >= deadline {
                    let _ = child.kill();
                    let _ = child.wait();
                    tracing::debug!(program, "project status provider timed out");
                    return None;
                }
                std::thread::sleep(PROVIDER_POLL_INTERVAL);
            }
            Err(_) => return None,
        }
    }

    let remaining = deadline.saturating_duration_since(Instant::now());
    let stdout = rx.recv_timeout(remaining).ok()?.ok()?;
    String::from_utf8(stdout).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(workspace_id: &str, checkout: &str) -> ProjectStatusRefreshItem {
        ProjectStatusRefreshItem {
            workspace_id: workspace_id.to_string(),
            checkout_key: PathBuf::from(checkout),
        }
    }

    #[test]
    fn spaces_sharing_a_checkout_collapse_to_one_job() {
        let jobs = deduplicate_project_status_items(vec![
            item("w1", "/repo"),
            item("w2", "/repo"),
            item("w3", "/repo"),
        ]);

        assert_eq!(jobs.len(), 1, "one probe serves all three spaces");
        assert_eq!(jobs[0].workspace_ids, vec!["w1", "w2", "w3"]);
    }

    #[test]
    fn linked_worktrees_of_one_repo_do_not_collapse() {
        let jobs = deduplicate_project_status_items(vec![
            item("w1", "/repo"),
            item("w2", "/repo/.worktrees/feature"),
        ]);

        assert_eq!(
            jobs.len(),
            2,
            "separate checkouts hold separate proposals and issue databases"
        );
    }

    #[test]
    fn every_space_in_a_shared_checkout_receives_the_same_snapshot() {
        let results = refresh_project_statuses(
            vec![
                item("w1", "/nonexistent-checkout"),
                item("w2", "/nonexistent-checkout"),
            ],
            ProjectStatusRefreshDemand::default(),
        );

        assert_eq!(results.len(), 2);
        assert_eq!(results[0].snapshot, results[1].snapshot);
    }

    #[test]
    fn absent_demand_runs_no_provider_and_yields_an_empty_snapshot() {
        let snapshot = project_status_for_checkout(
            Path::new("/nonexistent-checkout"),
            ProjectStatusRefreshDemand::default(),
        );
        assert!(snapshot.proposals.is_none());
        assert!(snapshot.beads.is_none());
    }

    #[test]
    fn a_missing_binary_resolves_to_none_rather_than_panicking() {
        let resolved = run_provider(
            Path::new("."),
            "shepherd-provider-that-does-not-exist",
            &["--json"],
        );
        assert!(resolved.is_none());
    }

    #[test]
    fn a_nonzero_exit_resolves_to_none() {
        assert!(run_provider(Path::new("."), "false", &[]).is_none());
    }

    #[test]
    fn a_zero_exit_returns_stdout() {
        let out = run_provider(Path::new("."), "echo", &["hello"]);
        assert_eq!(out.as_deref().map(str::trim), Some("hello"));
    }

    /// Regression: stdout must be drained while waiting. Polling `try_wait` against an
    /// undrained pipe deadlocks once the child's output exceeds the pipe buffer, which
    /// silently turned every real issue database into a timeout.
    #[test]
    fn output_larger_than_the_pipe_buffer_is_returned_whole() {
        let payload_bytes = 512 * 1024;
        let out = run_provider(
            Path::new("."),
            "sh",
            &[
                "-c",
                &format!("head -c {payload_bytes} /dev/zero | tr '\\0' 'x'"),
            ],
        )
        .expect("a large payload must not deadlock the wait loop");

        assert_eq!(out.len(), payload_bytes, "output must not be truncated");
    }

    fn test_app(rows: Vec<Vec<crate::config::SpaceSidebarToken>>) -> super::super::App {
        let mut config = crate::config::Config::default();
        config.ui.sidebar.spaces.rows = rows;
        super::super::App::new(
            &config,
            true,
            None,
            tokio::sync::mpsc::unbounded_channel().1,
            crate::api::EventHub::default(),
        )
    }

    #[test]
    fn no_configured_consumer_starts_no_refresh_and_spawns_no_provider() {
        let mut app = test_app(vec![vec![crate::config::SpaceSidebarToken::Workspace]]);
        app.state
            .workspaces
            .push(crate::workspace::Workspace::test_new("test"));
        let now = Instant::now();
        app.last_project_status_refresh = now - PROJECT_STATUS_REFRESH_INTERVAL;

        app.start_project_status_refresh_if_due(now);

        assert!(app.project_status_refresh_deadline().is_none());
        assert!(!app.project_status_refresh_in_flight);
        assert!(app.event_rx.try_recv().is_err());
    }

    #[test]
    fn each_token_enables_only_its_own_provider() {
        use crate::config::SpaceSidebarToken;
        let cases = [
            (
                SpaceSidebarToken::Workspace,
                ProjectStatusRefreshDemand::default(),
            ),
            (
                SpaceSidebarToken::Proposals,
                ProjectStatusRefreshDemand {
                    proposals: true,
                    beads: false,
                },
            ),
            (
                SpaceSidebarToken::Beads,
                ProjectStatusRefreshDemand {
                    proposals: false,
                    beads: true,
                },
            ),
        ];

        for (token, expected) in cases {
            let app = test_app(vec![vec![token.clone()]]);
            assert_eq!(
                app.project_status_demand(),
                expected,
                "demand must follow the configured token {token:?}"
            );
        }

        let both = test_app(vec![
            vec![SpaceSidebarToken::Proposals],
            vec![SpaceSidebarToken::Beads],
        ]);
        assert_eq!(
            both.project_status_demand(),
            ProjectStatusRefreshDemand {
                proposals: true,
                beads: true,
            }
        );
    }

    #[test]
    fn deadline_is_suppressed_while_a_refresh_is_in_flight() {
        let mut app = test_app(vec![vec![crate::config::SpaceSidebarToken::Proposals]]);
        app.state
            .workspaces
            .push(crate::workspace::Workspace::test_new("test"));
        assert!(app.project_status_refresh_deadline().is_some());

        app.project_status_refresh_in_flight = true;
        assert!(app.project_status_refresh_deadline().is_none());
    }

    #[test]
    fn project_status_deadline_is_independent_of_the_git_deadline() {
        let mut app = test_app(vec![vec![crate::config::SpaceSidebarToken::Proposals]]);
        app.state
            .workspaces
            .push(crate::workspace::Workspace::test_new("test"));
        let now = Instant::now();
        app.last_project_status_refresh = now;

        let project = app
            .project_status_refresh_deadline()
            .expect("configured consumer keeps the timer live");
        assert_eq!(project, now + PROJECT_STATUS_REFRESH_INTERVAL);
        assert!(
            PROJECT_STATUS_REFRESH_INTERVAL > super::super::GIT_REMOTE_STATUS_REFRESH_INTERVAL,
            "project status must poll slower than git status"
        );
    }
}
