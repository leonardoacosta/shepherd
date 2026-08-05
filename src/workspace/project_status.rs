//! Per-checkout project work-queue status: how many change proposals and tracked issues
//! a space's checkout currently carries.
//!
//! Parsing lives here and stays pure so it can be tested against captured provider output.
//! Process execution lives in `crate::app::project_status_refresh`.

/// Which providers a refresh must run. Resolved from the configured space rows, so an
/// operator who configures neither token pays no process cost. Mirrors
/// `GitStatusRefreshDemand`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ProjectStatusRefreshDemand {
    pub proposals: bool,
    pub beads: bool,
}

impl ProjectStatusRefreshDemand {
    pub fn is_empty(&self) -> bool {
        !self.proposals && !self.beads
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProposalCounts {
    pub open: usize,
    pub in_progress: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BeadCounts {
    pub open: usize,
    pub ready: usize,
    pub blocked: usize,
}

/// Each half is independently optional: a missing issue database must not blank a valid
/// proposal count, and vice versa.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ProjectStatusSnapshot {
    pub proposals: Option<ProposalCounts>,
    pub beads: Option<BeadCounts>,
}

/// One resolved refresh result, addressed back to the workspace that asked for it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceProjectStatus {
    pub workspace_id: String,
    pub checkout_key: std::path::PathBuf,
    pub snapshot: ProjectStatusSnapshot,
}

/// `openspec list --json` -> `{"changes":[{"name","completedTasks","totalTasks",...}]}`.
///
/// `open` counts every unarchived change, since archiving is what retires a proposal.
/// `in_progress` counts changes with some but not all tasks done — derived from the task
/// counts rather than the payload's own `status`, which reports `in-progress` for a change
/// with zero completed tasks.
pub fn parse_proposal_counts(stdout: &str) -> Option<ProposalCounts> {
    let value: serde_json::Value = serde_json::from_str(stdout).ok()?;
    let changes = value.get("changes")?.as_array()?;
    let mut in_progress = 0usize;
    for change in changes {
        let completed = change.get("completedTasks")?.as_u64()?;
        let total = change.get("totalTasks")?.as_u64()?;
        if completed > 0 && completed < total {
            in_progress += 1;
        }
    }
    Some(ProposalCounts {
        open: changes.len(),
        in_progress,
    })
}

/// `bd list --json` -> a flat array of issues carrying `status`.
///
/// `open` counts everything not closed or deferred — the live queue. `blocked` is its own
/// status. `ready` is not derivable from this payload and comes from `bd ready --json`.
pub fn parse_bead_open_and_blocked(stdout: &str) -> Option<(usize, usize)> {
    let issues: serde_json::Value = serde_json::from_str(stdout).ok()?;
    let issues = issues.as_array()?;
    let mut open = 0usize;
    let mut blocked = 0usize;
    for issue in issues {
        match issue.get("status")?.as_str()? {
            "open" | "in_progress" => open += 1,
            "blocked" => {
                open += 1;
                blocked += 1;
            }
            _ => {}
        }
    }
    Some((open, blocked))
}

/// `bd ready --json` -> a flat array of the currently actionable issues.
pub fn parse_bead_ready(stdout: &str) -> Option<usize> {
    let issues: serde_json::Value = serde_json::from_str(stdout).ok()?;
    Some(issues.as_array()?.len())
}

/// Compact rendered forms, sized like the existing `git_status` token.
pub fn render_proposal_counts(counts: ProposalCounts) -> String {
    format!("op: {}o {}ip", counts.open, counts.in_progress)
}

pub fn render_bead_counts(counts: BeadCounts) -> String {
    format!("bd: {}o {}r {}b", counts.open, counts.ready, counts.blocked)
}

#[cfg(test)]
mod tests {
    use super::*;

    const OPENSPEC_FIXTURE: &str = r#"{
  "changes": [
    { "name": "a", "completedTasks": 0, "totalTasks": 24, "status": "in-progress" },
    { "name": "b", "completedTasks": 3, "totalTasks": 5, "status": "in-progress" },
    { "name": "c", "completedTasks": 5, "totalTasks": 5, "status": "in-progress" }
  ]
}"#;

    const BD_LIST_FIXTURE: &str = r#"[
  { "id": "x-1", "status": "open" },
  { "id": "x-2", "status": "in_progress" },
  { "id": "x-3", "status": "blocked" },
  { "id": "x-4", "status": "deferred" },
  { "id": "x-5", "status": "closed" }
]"#;

    const BD_READY_FIXTURE: &str = r#"[{ "id": "x-1" }, { "id": "x-2" }]"#;

    #[test]
    fn proposal_counts_derive_in_progress_from_task_counts() {
        let counts = parse_proposal_counts(OPENSPEC_FIXTURE).expect("parses");
        assert_eq!(counts.open, 3, "every unarchived change is open work");
        assert_eq!(
            counts.in_progress, 1,
            "only the partially-complete change counts as in progress"
        );
    }

    #[test]
    fn bead_counts_treat_blocked_as_open_and_ignore_deferred_and_closed() {
        let (open, blocked) = parse_bead_open_and_blocked(BD_LIST_FIXTURE).expect("parses");
        assert_eq!(open, 3, "open + in_progress + blocked");
        assert_eq!(blocked, 1);
        assert_eq!(parse_bead_ready(BD_READY_FIXTURE).expect("parses"), 2);
    }

    #[test]
    fn empty_payloads_parse_to_zero_rather_than_absent() {
        let counts = parse_proposal_counts(r#"{"changes":[]}"#).expect("parses");
        assert_eq!(counts.open, 0);
        assert_eq!(counts.in_progress, 0);
        let (open, blocked) = parse_bead_open_and_blocked("[]").expect("parses");
        assert_eq!((open, blocked), (0, 0));
    }

    #[test]
    fn malformed_and_unexpected_shapes_resolve_to_none() {
        for body in [
            "",
            "not json",
            "{}",
            r#"{"changes":{}}"#,
            r#"{"changes":[{"name":"a"}]}"#,
        ] {
            assert!(
                parse_proposal_counts(body).is_none(),
                "proposal parse must reject {body:?}"
            );
        }
        for body in ["", "not json", "{}", r#"[{"id":"x"}]"#] {
            assert!(
                parse_bead_open_and_blocked(body).is_none(),
                "bead parse must reject {body:?}"
            );
        }
        assert!(parse_bead_ready("{}").is_none());
    }

    #[test]
    fn rendered_forms_are_compact() {
        assert_eq!(
            render_proposal_counts(ProposalCounts {
                open: 2,
                in_progress: 1
            }),
            "op: 2o 1ip"
        );
        assert_eq!(
            render_bead_counts(BeadCounts {
                open: 3,
                ready: 1,
                blocked: 0
            }),
            "bd: 3o 1r 0b"
        );
    }

    #[test]
    fn demand_is_empty_until_a_token_asks_for_it() {
        assert!(ProjectStatusRefreshDemand::default().is_empty());
        let proposals_only = ProjectStatusRefreshDemand {
            proposals: true,
            beads: false,
        };
        assert!(!proposals_only.is_empty());
        let absent = ProjectStatusSnapshot::default();
        assert!(absent.proposals.is_none());
        assert!(absent.beads.is_none());
    }
}
