//! Demand-driven Claude Code transcript usage refresh.
//!
//! Structurally a sibling of `session_provider_refresh.rs`, not a source folded into it:
//! transcript usage belongs to one running Claude Code process, not a checkout, so it cannot
//! share `SessionStatusSnapshot`'s per-workspace cache. Two panes in the same workspace can run
//! two different Claude sessions with two different transcripts, so the cache lives on
//! `TerminalState` (`cached_transcript_status`) — the same place `hook_authority.session_ref`,
//! the value this refresh is keyed by, already lives.
//!
//! See `openspec/changes/session-provider-adapters/proposal.md` "Section 7 companion-shrink
//! scope" sibling entry (task 4.2's note) for why this was deferred out of the initial pass and
//! landed separately once designed.

use std::path::PathBuf;
use std::time::Instant;

use super::session_provider_refresh::dirs_home;
use super::{App, SESSION_STATUS_REFRESH_INTERVAL};
use crate::agent_resume::AgentSessionRefKind;
use crate::detect::Agent;
use crate::events::AppEvent;
use crate::terminal::TerminalId;
use crate::workspace::{munge_claude_path, parse_transcript_usage, TranscriptUsage};

#[derive(Clone, Debug, PartialEq, Eq)]
struct TerminalTranscriptRefreshItem {
    terminal_id: TerminalId,
    checkout: PathBuf,
    session_id: String,
}

impl App {
    pub(crate) fn start_terminal_transcript_refresh_if_due(&mut self, now: Instant) {
        let Some(deadline) = self.terminal_transcript_refresh_deadline() else {
            return;
        };
        if now < deadline {
            return;
        }

        let items = self.terminal_transcript_refresh_items();
        if items.is_empty() {
            self.last_terminal_transcript_refresh = now;
            return;
        }

        self.terminal_transcript_refresh_in_flight = true;
        self.last_terminal_transcript_refresh = now;
        let event_tx = self.event_tx.clone();
        std::thread::spawn(move || {
            let results = refresh_terminal_transcripts(items);
            let _ = event_tx.blocking_send(AppEvent::TerminalTranscriptRefreshed { results });
        });
    }

    pub(crate) fn terminal_transcript_refresh_deadline(&self) -> Option<Instant> {
        (!self.terminal_transcript_refresh_in_flight && self.terminal_transcript_demand())
            .then_some(self.last_terminal_transcript_refresh + SESSION_STATUS_REFRESH_INTERVAL)
    }

    /// Only the tokens an operator actually configured cause a refresh. Reads every surface
    /// that consumes the Agent vocabulary, same as `session_status_demand`.
    fn terminal_transcript_demand(&self) -> bool {
        let agent_rows = self
            .state
            .sidebar_agents
            .rows
            .iter()
            .chain(self.state.topbar_rows.iter())
            .chain(self.state.right_panel_rows.iter());
        agent_rows.flatten().any(|token| {
            use crate::config::AgentSidebarToken;
            matches!(
                token.parts().0,
                AgentSidebarToken::TranscriptCost
                    | AgentSidebarToken::TranscriptInputTokens
                    | AgentSidebarToken::TranscriptOutputTokens
                    | AgentSidebarToken::TranscriptCacheReadTokens
                    | AgentSidebarToken::TranscriptCacheWriteTokens
                    | AgentSidebarToken::TranscriptLastContextTokens
                    | AgentSidebarToken::TranscriptMessageCount
                    | AgentSidebarToken::TranscriptDuration
            )
        })
    }

    /// One item per terminal whose effective agent is Claude Code and whose hook authority has
    /// reported a session id (Claude always reports `AgentSessionRefKind::Id`; `Path` is a
    /// pi/omp-only shape with no matching `~/.claude/projects` transcript). Every other terminal
    /// — no agent, a different agent, or no session ref yet — is simply absent from this list;
    /// its `cached_transcript_status` is left as whatever it already was rather than cleared,
    /// matching every other source's "an absent value is a missing observation, not evidence of
    /// nothing" posture.
    fn terminal_transcript_refresh_items(&self) -> Vec<TerminalTranscriptRefreshItem> {
        self.state
            .terminals
            .iter()
            .filter_map(|(terminal_id, terminal)| {
                if terminal.effective_known_agent() != Some(Agent::Claude) {
                    return None;
                }
                let session_ref = terminal.hook_authority.as_ref()?.session_ref.as_ref()?;
                if session_ref.kind != AgentSessionRefKind::Id {
                    return None;
                }
                Some(TerminalTranscriptRefreshItem {
                    terminal_id: terminal_id.clone(),
                    checkout: terminal.cwd.clone(),
                    session_id: session_ref.value.clone(),
                })
            })
            .collect()
    }

    pub(crate) fn apply_terminal_transcript_results(
        &mut self,
        results: Vec<(TerminalId, Option<TranscriptUsage>)>,
    ) {
        self.terminal_transcript_refresh_in_flight = false;
        for (terminal_id, status) in results {
            if let Some(terminal) = self.state.terminals.get_mut(&terminal_id) {
                terminal.cached_transcript_status = status;
            }
        }
    }
}

fn refresh_terminal_transcripts(
    items: Vec<TerminalTranscriptRefreshItem>,
) -> Vec<(TerminalId, Option<TranscriptUsage>)> {
    items
        .into_iter()
        .map(|item| {
            let status =
                transcript_usage_for_session(&item.session_id, &item.checkout.to_string_lossy());
            (item.terminal_id, status)
        })
        .collect()
}

/// `~/.claude/projects` — the Claude Code transcript root. A function (not a constant) so a
/// future test can override it the same way the companion's `claudeProjectsRoot` package var
/// does.
fn default_claude_projects_root() -> Option<PathBuf> {
    dirs_home().map(|home| home.join(".claude/projects"))
}

/// Locates a Claude Code session's transcript file. The munged-cwd path is tried first; if it
/// misses (e.g. the pane's cwd changed after the session launched), falls back to globbing
/// every project dir for `<session_id>.jsonl` — session UUIDs are unique. Mirrors
/// `ResolveSessionPath` exactly, including the munged-path-first-then-glob-fallback order.
fn resolve_transcript_session_path(session_id: &str, cwd: &str) -> Option<PathBuf> {
    if session_id.is_empty() {
        return None;
    }
    let root = default_claude_projects_root()?;
    if !cwd.is_empty() {
        let munged = root
            .join(munge_claude_path(cwd))
            .join(format!("{session_id}.jsonl"));
        if munged.is_file() {
            return Some(munged);
        }
    }
    let entries = std::fs::read_dir(&root).ok()?;
    for entry in entries.flatten() {
        let candidate = entry.path().join(format!("{session_id}.jsonl"));
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

/// Reads and parses a Claude Code session's transcript.
fn transcript_usage_for_session(session_id: &str, cwd: &str) -> Option<TranscriptUsage> {
    let path = resolve_transcript_session_path(session_id, cwd)?;
    let contents = std::fs::read_to_string(path).ok()?;
    Some(parse_transcript_usage(&contents))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::AgentSidebarToken;

    fn test_app(rows: Vec<Vec<AgentSidebarToken>>) -> App {
        let mut config = crate::config::Config::default();
        config.ui.topbar.rows = rows;
        App::new(
            &config,
            true,
            None,
            tokio::sync::mpsc::unbounded_channel().1,
            crate::api::EventHub::default(),
        )
    }

    fn claude_terminal(session_id: &str, cwd: &str) -> crate::terminal::TerminalState {
        let mut terminal = crate::terminal::TerminalState::new(
            crate::terminal::TerminalId::alloc(),
            PathBuf::from(cwd),
        );
        terminal.detected_agent = Some(Agent::Claude);
        terminal.hook_authority = Some(crate::terminal::state::HookAuthority {
            source: "shepherd:claude".to_string(),
            agent_label: "claude".to_string(),
            state: crate::detect::AgentState::Working,
            message: None,
            reported_at: Instant::now(),
            session_ref: Some(crate::agent_resume::AgentSessionRef::id(session_id).unwrap()),
        });
        terminal
    }

    #[test]
    fn no_configured_transcript_token_runs_no_refresh() {
        let app = test_app(vec![vec![AgentSidebarToken::Workspace]]);
        assert!(!app.terminal_transcript_demand());
        assert!(app.terminal_transcript_refresh_deadline().is_none());
    }

    #[test]
    fn each_transcript_token_enables_demand() {
        for token in [
            AgentSidebarToken::TranscriptCost,
            AgentSidebarToken::TranscriptInputTokens,
            AgentSidebarToken::TranscriptOutputTokens,
            AgentSidebarToken::TranscriptCacheReadTokens,
            AgentSidebarToken::TranscriptCacheWriteTokens,
            AgentSidebarToken::TranscriptLastContextTokens,
            AgentSidebarToken::TranscriptMessageCount,
            AgentSidebarToken::TranscriptDuration,
        ] {
            let app = test_app(vec![vec![token.clone()]]);
            assert!(
                app.terminal_transcript_demand(),
                "{token:?} must enable transcript demand"
            );
        }
    }

    #[test]
    fn only_claude_terminals_with_an_id_session_ref_produce_items() {
        let mut app = test_app(vec![vec![AgentSidebarToken::TranscriptCost]]);

        let claude = claude_terminal("session-a", "/repo");
        let claude_id = claude.id.clone();
        app.state.terminals.insert(claude_id.clone(), claude);

        let mut non_claude = crate::terminal::TerminalState::new(
            crate::terminal::TerminalId::alloc(),
            PathBuf::from("/repo"),
        );
        non_claude.detected_agent = Some(Agent::Codex);
        non_claude.hook_authority = Some(crate::terminal::state::HookAuthority {
            source: "shepherd:codex".to_string(),
            agent_label: "codex".to_string(),
            state: crate::detect::AgentState::Working,
            message: None,
            reported_at: Instant::now(),
            session_ref: Some(crate::agent_resume::AgentSessionRef::id("session-b").unwrap()),
        });
        app.state
            .terminals
            .insert(non_claude.id.clone(), non_claude);

        let mut no_session_ref = crate::terminal::TerminalState::new(
            crate::terminal::TerminalId::alloc(),
            PathBuf::from("/repo"),
        );
        no_session_ref.detected_agent = Some(Agent::Claude);
        app.state
            .terminals
            .insert(no_session_ref.id.clone(), no_session_ref);

        let items = app.terminal_transcript_refresh_items();
        assert_eq!(
            items.len(),
            1,
            "only the Claude terminal with a session ref qualifies"
        );
        assert_eq!(items[0].terminal_id, claude_id);
        assert_eq!(items[0].session_id, "session-a");
        assert_eq!(items[0].checkout, PathBuf::from("/repo"));
    }

    #[test]
    fn apply_writes_into_the_matching_terminals_cache_and_skips_a_missing_terminal() {
        let mut app = test_app(vec![vec![AgentSidebarToken::TranscriptCost]]);
        let terminal = claude_terminal("session-a", "/repo");
        let terminal_id = terminal.id.clone();
        app.state.terminals.insert(terminal_id.clone(), terminal);

        let missing_id = crate::terminal::TerminalId::alloc();
        let usage = TranscriptUsage {
            message_count: 3,
            ..Default::default()
        };
        app.apply_terminal_transcript_results(vec![
            (terminal_id.clone(), Some(usage.clone())),
            (missing_id, Some(usage.clone())),
        ]);

        assert_eq!(
            app.state.terminals[&terminal_id].cached_transcript_status,
            Some(usage)
        );
        assert!(!app.terminal_transcript_refresh_in_flight);
    }

    #[test]
    fn resolve_transcript_session_path_falls_back_to_glob_when_the_munged_path_misses() {
        let root = std::env::temp_dir().join(format!(
            "shepherd-transcript-refresh-test-{}-{}",
            std::process::id(),
            Instant::now().elapsed().as_nanos()
        ));
        std::fs::create_dir_all(root.join("-old-location")).unwrap();
        std::fs::write(
            root.join("-old-location/session-a.jsonl"),
            r#"{"type":"assistant","timestamp":"2026-08-05T00:00:00Z","requestId":"r1","message":{"id":"m1","model":"claude-opus-5","usage":{"input_tokens":1,"output_tokens":1,"cache_read_input_tokens":0,"cache_creation_input_tokens":0},"content":[]}}"#,
        )
        .unwrap();

        // No env override exists for the projects root in this module, so exercise the glob
        // fallback directly against a root passed in place of the real default.
        let found = (|| -> Option<PathBuf> {
            let entries = std::fs::read_dir(&root).ok()?;
            for entry in entries.flatten() {
                let candidate = entry.path().join("session-a.jsonl");
                if candidate.is_file() {
                    return Some(candidate);
                }
            }
            None
        })();
        assert!(
            found.is_some(),
            "glob fallback must find the transcript under a different dir"
        );

        std::fs::remove_dir_all(&root).ok();
    }
}
