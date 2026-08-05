//! Transcript token usage, absorbed from `shepherd-plugins/plugins/shepherd-state/pkg/transcript`
//! (itself vendored from the pinned upstream token dashboard, MIT-licensed;
//! `openspec/changes/session-provider-adapters/tasks.md` task 4.1 requires the path-munging
//! rules preserved exactly). Path resolution and file reading are I/O — they belong in
//! `crate::app::session_provider_refresh`, the adapter layer. Parsing stays here and pure so it
//! can be tested against captured transcript bodies.
//!
//! Not yet wired into any adapter or the render path — task 4 lands the parser and its ported
//! tests ahead of task 6's vocabulary wiring. `dead_code` is allowed at the module level until
//! that wiring lands; remove this attribute in the same change that adds the first non-test
//! caller.
#![allow(dead_code)]

use std::collections::{HashMap, HashSet};

use super::session_pricing::cost as pricing_cost;
use super::session_status::{parse_rfc3339_nanos, round_cents};

/// One Claude Code session transcript's aggregated token usage, estimated cost, and session
/// metadata. Everything except `last_context_tokens` is a cumulative sum across the whole
/// transcript — see its own doc comment for why it is not.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TranscriptUsage {
    /// Estimated cost in whole cents (see `SpendStatus`'s doc comment for why cents, not a
    /// float).
    pub cost_cents: u64,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub cache_read_tokens: i64,
    pub cache_write_tokens: i64,
    /// The final assistant message's input + cache-read + cache-creation tokens — how full the
    /// context window was on the most recent request, NOT a sum over the session. Output tokens
    /// are excluded: they are generated, not resident in the request's context.
    pub last_context_tokens: i64,
    pub model: Option<String>,
    pub provider: Option<String>,
    pub message_count: i64,
    pub started_at: Option<String>,
    pub last_activity_at: Option<String>,
    pub duration_secs: Option<i64>,
    pub tools: HashMap<String, i64>,
    pub tool_total: i64,
}

/// Converts a working directory into the directory name Claude Code uses under
/// `~/.claude/projects`: every character outside `[A-Za-z0-9-]` is replaced by `-`.
pub fn munge_claude_path(cwd: &str) -> String {
    cwd.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' {
                c
            } else {
                '-'
            }
        })
        .collect()
}

#[derive(Debug, Default, Clone)]
struct Turn {
    model: Option<String>,
    input: i64,
    output: i64,
    cache_read: i64,
    cache_write: i64,
}

/// Extracts tokens, estimated cost, model, message count, tool calls, and session duration from
/// a Claude Code session transcript's raw JSONL body.
///
/// Streaming and retries can repeat records for the same assistant message, so usage is
/// aggregated per `(message.id, requestId)` pair — each unique pair counts once, last occurrence
/// wins. A line belonging to a subagent (`agentId` set) is skipped entirely, including for
/// duration tracking. A malformed or unparseable line contributes nothing — never a hard
/// failure, matching the companion's fail-open posture (an empty or garbage transcript yields a
/// zero-value [`TranscriptUsage`], never a panic).
pub fn parse_transcript_usage(jsonl: &str) -> TranscriptUsage {
    let mut turns: HashMap<String, Turn> = HashMap::new();
    let mut seen_tools: HashSet<String> = HashSet::new();
    let mut tools: HashMap<String, i64> = HashMap::new();
    let mut tool_total: i64 = 0;
    let mut first_ts: Option<i128> = None;
    let mut first_ts_raw: Option<String> = None;
    let mut last_ts: Option<i128> = None;
    let mut last_ts_raw: Option<String> = None;
    let mut last_turn_key: Option<String> = None;
    let mut model: Option<String> = None;
    let mut provider: Option<String> = None;

    for line in jsonl.split('\n') {
        if line.is_empty() {
            continue;
        }
        let Ok(entry) = serde_json::from_str::<serde_json::Value>(line) else {
            continue;
        };
        let agent_id = entry.get("agentId").and_then(|v| v.as_str()).unwrap_or("");
        if !agent_id.is_empty() {
            continue;
        }

        if let Some(ts_raw) = entry.get("timestamp").and_then(|v| v.as_str()) {
            if let Some(nanos) = parse_rfc3339_nanos(ts_raw) {
                // First-seen semantics: the FIRST timestamp encountered in file order, not the
                // minimum value. `lastTS` is unconditionally overwritten on every timestamped
                // line, ending up as the last line's timestamp in file order — mirrors the
                // companion exactly.
                if first_ts.is_none() {
                    first_ts = Some(nanos);
                    first_ts_raw = Some(ts_raw.to_string());
                }
                last_ts = Some(nanos);
                last_ts_raw = Some(ts_raw.to_string());
            }
        }

        let entry_type = entry.get("type").and_then(|v| v.as_str()).unwrap_or("");
        let message_id = entry
            .get("message")
            .and_then(|m| m.get("id"))
            .and_then(|v| v.as_str())
            .unwrap_or("");
        if entry_type != "assistant" || message_id.is_empty() {
            continue;
        }
        let Some(message) = entry.get("message") else {
            continue;
        };

        let request_id = entry
            .get("requestId")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let turn_model = message
            .get("model")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty());
        if let Some(m) = turn_model {
            model = Some(m.to_string());
            provider = Some("anthropic".to_string());
        }

        let usage = message.get("usage");
        let key = format!("{message_id}\0{request_id}");
        turns.insert(
            key.clone(),
            Turn {
                model: turn_model.map(str::to_string),
                input: usage
                    .and_then(|u| u.get("input_tokens"))
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0),
                output: usage
                    .and_then(|u| u.get("output_tokens"))
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0),
                cache_read: usage
                    .and_then(|u| u.get("cache_read_input_tokens"))
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0),
                cache_write: usage
                    .and_then(|u| u.get("cache_creation_input_tokens"))
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0),
            },
        );
        last_turn_key = Some(key);

        // Tool calls appear as tool_use content blocks. Blocks carry unique ids, so repeated
        // records for the same message don't double-count globally. A non-array (or absent)
        // content field simply skips tool extraction for this line — the token/model data
        // above is already committed regardless.
        let Some(blocks) = message.get("content").and_then(|c| c.as_array()) else {
            continue;
        };
        for block in blocks {
            let block_type = block.get("type").and_then(|v| v.as_str()).unwrap_or("");
            let name = block.get("name").and_then(|v| v.as_str()).unwrap_or("");
            if block_type != "tool_use" || name.is_empty() {
                continue;
            }
            if let Some(id) = block.get("id").and_then(|v| v.as_str()) {
                if !id.is_empty() && !seen_tools.insert(id.to_string()) {
                    continue;
                }
            }
            *tools.entry(name.to_string()).or_insert(0) += 1;
            tool_total += 1;
        }
    }

    let mut input_tokens = 0i64;
    let mut output_tokens = 0i64;
    let mut cache_read_tokens = 0i64;
    let mut cache_write_tokens = 0i64;
    let mut cost = 0.0f64;
    for turn in turns.values() {
        input_tokens += turn.input;
        output_tokens += turn.output;
        cache_read_tokens += turn.cache_read;
        cache_write_tokens += turn.cache_write;
        cost += pricing_cost(
            turn.model.as_deref().unwrap_or(""),
            turn.input,
            turn.output,
            turn.cache_read,
            turn.cache_write,
        );
    }

    let last_context_tokens = last_turn_key
        .as_ref()
        .and_then(|key| turns.get(key))
        .map(|t| t.input + t.cache_read + t.cache_write)
        .unwrap_or(0);

    let duration_secs = match (first_ts, last_ts) {
        (Some(first), Some(last)) => Some(((last - first) / 1_000_000_000) as i64),
        _ => None,
    };

    TranscriptUsage {
        cost_cents: round_cents(cost).unwrap_or(0),
        input_tokens,
        output_tokens,
        cache_read_tokens,
        cache_write_tokens,
        last_context_tokens,
        model,
        provider,
        message_count: turns.len() as i64,
        started_at: first_ts_raw,
        last_activity_at: last_ts_raw,
        duration_secs,
        tools,
        tool_total,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tools(pairs: &[(&str, i64)]) -> HashMap<String, i64> {
        pairs.iter().map(|(k, v)| (k.to_string(), *v)).collect()
    }

    /// Ported from `transcript_test.go`'s `TestReadClaudeSession` "normal session" case.
    #[test]
    fn normal_session_aggregates_tokens_cost_and_tools() {
        let jsonl = concat!(
            r#"{"type":"user","timestamp":"2026-07-13T10:00:00.000Z","message":{"role":"user","content":"hello"}}"#,
            "\n",
            r#"{"type":"assistant","timestamp":"2026-07-13T10:00:05.000Z","requestId":"req_01","message":{"id":"msg_01","model":"claude-opus-4-8","usage":{"input_tokens":1000,"output_tokens":500,"cache_read_input_tokens":2000,"cache_creation_input_tokens":3000},"content":[{"type":"text","text":"hi"}]}}"#,
            "\n",
            r#"{"type":"assistant","timestamp":"2026-07-13T10:05:00.000Z","requestId":"req_02","message":{"id":"msg_02","model":"claude-opus-4-8","usage":{"input_tokens":100,"output_tokens":50,"cache_read_input_tokens":0,"cache_creation_input_tokens":0},"content":[{"type":"tool_use","id":"toolu_01","name":"Bash"},{"type":"tool_use","id":"toolu_02","name":"Read"}]}}"#,
        );
        let u = parse_transcript_usage(jsonl);

        assert_eq!(u.message_count, 2);
        assert_eq!(u.input_tokens, 1100);
        assert_eq!(u.output_tokens, 550);
        assert_eq!(u.cache_read_tokens, 2000);
        assert_eq!(u.cache_write_tokens, 3000);
        // opus-4-8 is $5/$25 per MTok: (1000*5+500*25+2000*0.5+3000*6.25)/1e6 + (100*5+50*25)/1e6
        assert_eq!(u.cost_cents, round_cents(0.03725 + 0.00175).unwrap());
        assert_eq!(u.model.as_deref(), Some("claude-opus-4-8"));
        assert_eq!(u.provider.as_deref(), Some("anthropic"));
        assert_eq!(u.tools, tools(&[("Bash", 1), ("Read", 1)]));
        assert_eq!(u.tool_total, 2);
        // Duration is from the leading USER message's timestamp (10:00:00) to the last
        // assistant message (10:05:00) -- 5 minutes -- not from the first assistant message.
        assert_eq!(u.duration_secs, Some(5 * 60));
    }

    /// Ported from `transcript_test.go`'s "dedupe repeated message id and requestId" case.
    #[test]
    fn a_retried_turn_dedupes_to_its_last_occurrence() {
        let jsonl = concat!(
            r#"{"type":"assistant","timestamp":"2026-07-13T11:00:00.000Z","requestId":"req_01","message":{"id":"msg_01","model":"claude-sonnet-4-5","usage":{"input_tokens":100,"output_tokens":10,"cache_read_input_tokens":0,"cache_creation_input_tokens":0},"content":[{"type":"tool_use","id":"toolu_x","name":"Bash"}]}}"#,
            "\n",
            r#"{"type":"assistant","timestamp":"2026-07-13T11:00:03.000Z","requestId":"req_01","message":{"id":"msg_01","model":"claude-sonnet-4-5","usage":{"input_tokens":100,"output_tokens":200,"cache_read_input_tokens":0,"cache_creation_input_tokens":0},"content":[{"type":"tool_use","id":"toolu_x","name":"Bash"}]}}"#,
        );
        let u = parse_transcript_usage(jsonl);

        assert_eq!(u.message_count, 1, "same message.id+requestId is one turn");
        assert_eq!(u.input_tokens, 100);
        assert_eq!(
            u.output_tokens, 200,
            "the second (winning) occurrence's value"
        );
        assert_eq!(u.cost_cents, round_cents(0.0033).unwrap());
        assert_eq!(
            u.tools,
            tools(&[("Bash", 1)]),
            "same tool_use id counts once"
        );
        assert_eq!(u.tool_total, 1);
        assert_eq!(u.duration_secs, Some(3));
    }

    /// Ported from `transcript_test.go`'s "unknown model has zero cost but tokens are kept".
    #[test]
    fn an_unknown_model_has_zero_cost_but_keeps_its_tokens() {
        let jsonl = r#"{"type":"assistant","timestamp":"2026-07-13T12:00:00.000Z","requestId":"req_01","message":{"id":"msg_01","model":"claude-zeta-9","usage":{"input_tokens":1000,"output_tokens":1000,"cache_read_input_tokens":0,"cache_creation_input_tokens":0},"content":[{"type":"text","text":"hi"}]}}"#;
        let u = parse_transcript_usage(jsonl);

        assert_eq!(u.input_tokens, 1000);
        assert_eq!(u.output_tokens, 1000);
        assert_eq!(u.cost_cents, 0, "unknown models never fabricate a cost");
        assert_eq!(u.model.as_deref(), Some("claude-zeta-9"));
    }

    /// Ported from `transcript_test.go`'s "malformed lines are skipped" case. Also covers a
    /// truncated final line (task 4.3) — a live transcript exhibits exactly this shape while a
    /// session is still writing to it.
    #[test]
    fn malformed_and_truncated_lines_are_skipped_without_failing_the_parse() {
        let jsonl = concat!(
            "not json at all",
            "\n",
            r#"{"type":"assistant","message":"#,
            "\n", // truncated — mid-write
            r#"{"type":"assistant","timestamp":"2026-07-13T14:00:00.000Z","requestId":"req_01","message":{"id":"msg_01","model":"claude-fable-5","usage":{"input_tokens":10,"output_tokens":20,"cache_read_input_tokens":0,"cache_creation_input_tokens":0},"content":"plain string content"}}"#,
            "\n",
            r#"{"type":"assistant"}"#,
        );
        let u = parse_transcript_usage(jsonl);

        assert_eq!(u.message_count, 1);
        assert_eq!(u.input_tokens, 10);
        assert_eq!(u.output_tokens, 20);
        assert_eq!(u.cost_cents, round_cents(0.0011).unwrap());
        assert_eq!(u.model.as_deref(), Some("claude-fable-5"));
        assert!(
            u.tools.is_empty(),
            "a non-array content field must skip tool extraction, not panic"
        );
    }

    /// Ported from `transcript_test.go`'s `TestReadUsageLastContext` — pins the one field that
    /// is deliberately not a sum.
    #[test]
    fn last_context_reports_only_the_final_message_not_the_cumulative_sum() {
        let jsonl = concat!(
            r#"{"type":"user","timestamp":"2026-07-25T10:00:00.000Z","message":{"role":"user","content":"hello"}}"#,
            "\n",
            r#"{"type":"assistant","timestamp":"2026-07-25T10:00:05.000Z","requestId":"req_01","message":{"id":"msg_01","model":"claude-opus-4-8","usage":{"input_tokens":100,"output_tokens":10,"cache_read_input_tokens":1000,"cache_creation_input_tokens":500},"content":[{"type":"text","text":"a"}]}}"#,
            "\n",
            r#"{"type":"assistant","timestamp":"2026-07-25T10:01:00.000Z","requestId":"req_02","message":{"id":"msg_02","model":"claude-opus-4-8","usage":{"input_tokens":200,"output_tokens":20,"cache_read_input_tokens":4000,"cache_creation_input_tokens":800},"content":[{"type":"text","text":"b"}]}}"#,
            "\n",
            r#"{"type":"assistant","timestamp":"2026-07-25T10:02:00.000Z","requestId":"req_03","message":{"id":"msg_03","model":"claude-opus-4-8","usage":{"input_tokens":300,"output_tokens":30,"cache_read_input_tokens":9000,"cache_creation_input_tokens":700},"content":[{"type":"text","text":"c"}]}}"#,
        );
        let u = parse_transcript_usage(jsonl);

        assert_eq!(u.last_context_tokens, 10_000, "msg_03 alone: 300+9000+700");
        let cumulative = u.input_tokens + u.cache_read_tokens + u.cache_write_tokens;
        assert_eq!(cumulative, 16_600);
        assert_ne!(
            u.last_context_tokens, cumulative,
            "last_context_tokens must not equal the cumulative sum"
        );
    }

    #[test]
    fn last_context_is_zero_when_the_transcript_has_no_assistant_messages() {
        let jsonl = r#"{"type":"user","timestamp":"2026-07-25T12:00:00.000Z","message":{"role":"user","content":"hello"}}"#;
        let u = parse_transcript_usage(jsonl);
        assert_eq!(u.last_context_tokens, 0);
        assert_eq!(u.message_count, 0);
    }

    #[test]
    fn an_empty_transcript_yields_a_zero_value_usage() {
        let u = parse_transcript_usage("");
        assert_eq!(u, TranscriptUsage::default());
    }

    /// Ported from `transcript_test.go`'s `TestMungeClaudePath`.
    #[test]
    fn munge_claude_path_matches_the_companions_rule() {
        assert_eq!(
            munge_claude_path("/home/ajavaherian/projects/TMM-Workflow"),
            "-home-ajavaherian-projects-TMM-Workflow"
        );
        assert_eq!(munge_claude_path("/tmp/a_b.c d"), "-tmp-a-b-c-d");
    }

    /// A line tagged with `agentId` belongs to a subagent transcript intermixed in the same
    /// file and must not contribute tokens, model, or duration tracking.
    #[test]
    fn a_subagent_tagged_line_is_skipped_entirely() {
        let jsonl = concat!(
            r#"{"type":"assistant","timestamp":"2026-07-13T09:00:00.000Z","agentId":"sub-1","requestId":"req_00","message":{"id":"msg_00","model":"claude-haiku-4-5","usage":{"input_tokens":99999,"output_tokens":99999,"cache_read_input_tokens":0,"cache_creation_input_tokens":0},"content":[]}}"#,
            "\n",
            r#"{"type":"assistant","timestamp":"2026-07-13T10:00:00.000Z","requestId":"req_01","message":{"id":"msg_01","model":"claude-haiku-4-5","usage":{"input_tokens":1,"output_tokens":1,"cache_read_input_tokens":0,"cache_creation_input_tokens":0},"content":[]}}"#,
        );
        let u = parse_transcript_usage(jsonl);
        assert_eq!(u.message_count, 1, "the subagent line must not be counted");
        assert_eq!(u.input_tokens, 1);
        assert_eq!(
            u.started_at.as_deref(),
            Some("2026-07-13T10:00:00.000Z"),
            "the subagent line's timestamp must not seed first_ts either"
        );
    }
}
