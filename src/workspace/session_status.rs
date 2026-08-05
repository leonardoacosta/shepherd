//! Session and provider telemetry that Shepherd resolves for itself.
//!
//! Parsing lives here and stays pure so it can be tested against captured adapter output.
//! Process execution and file-system walking live in `crate::app::session_provider_refresh`.
//!
//! Shape mirrors `crate::workspace::project_status` layer for layer — see `CONTEXT.md`.
//!
//! Each source below owns one failure envelope: `SessionStatusSnapshot` holds one `Option`
//! per source, so one unavailable source elides only its own fields. See
//! `openspec/changes/session-provider-adapters/proposal.md` § "Chrome adapter inventory".

/// Which adapters a refresh must run. Resolved from the configured rows, so an operator who
/// configures no session token pays no process cost. Mirrors `ProjectStatusRefreshDemand`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SessionStatusRefreshDemand {
    pub llmtrim: bool,
    pub context_floor: bool,
    pub sessions: bool,
    pub local_account_signal: bool,
}

impl SessionStatusRefreshDemand {
    pub fn is_empty(&self) -> bool {
        !self.llmtrim && !self.context_floor && !self.sessions && !self.local_account_signal
    }
}

/// Cumulative money, in whole cents. Used for both spend and savings.
///
/// Cents rather than a float: the value is rendered, compared for change detection, and cached,
/// and none of those want float equality. Each adapter's own payload is a float, so the
/// conversion happens once, at the parse boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpendStatus {
    pub cents: u64,
}

/// `llmtrim status --json`'s own native shape — no longer the `shepherd-state chrome` wrapper.
/// One adapter, one spawn, one failure envelope: a missing `llmtrim` binary or a malformed
/// payload elides every field here without touching `context_floor` or `sessions`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct LlmTrimStatus {
    pub spend: Option<SpendStatus>,
    pub savings: Option<SpendStatus>,
    pub cache_read_tokens: Option<u64>,
}

/// `context-floor --json`'s own native shape.
///
/// `growth_pct` and `dollars_per_1k_turns` are floats on the wire; both are converted once at
/// the parse boundary to fixed-point integers for the same reason `SpendStatus` uses cents.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ContextFloorStatus {
    pub instruction_total_bytes: Option<u64>,
    pub instruction_floor_warn: Option<bool>,
    /// Growth percentage in basis points (hundredths of a percent).
    pub instruction_growth_bps: Option<i64>,
    /// Dollars per 1,000 turns, in whole cents.
    pub instruction_dollars_per_1k_turns_cents: Option<i64>,
}

/// The persisted session snapshot store's resolved facts, merged across every session file
/// under the state root. Each field independently uses its newest non-empty observation, so a
/// newer bare snapshot cannot erase a still-resolvable model or context value from an older one
/// — mirrors the companion's `persistedSessionFacts` merge exactly.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SessionsStatus {
    pub model: Option<String>,
    pub context_tokens: Option<i64>,
    pub context_window_tokens: Option<i64>,
    pub session_count: u64,
}

/// One session snapshot file's fields, prior to the cross-file merge. `observed_at`/
/// `fresh_until` are kept as their raw RFC3339 strings; only `parse_rfc3339_nanos` needs to
/// understand them, at merge time.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PersistedSessionRecord {
    pub session_id: String,
    pub model: Option<String>,
    pub context_tokens: Option<i64>,
    pub context_window_tokens: Option<i64>,
    pub observed_at: String,
    pub fresh_until: Option<String>,
}

/// Each field is independently optional at the source level: one unavailable source must not
/// blank another's fields.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SessionStatusSnapshot {
    pub llmtrim: Option<LlmTrimStatus>,
    pub context_floor: Option<ContextFloorStatus>,
    pub sessions: Option<SessionsStatus>,
    pub local_account_signal: Option<super::session_credentials::LocalAccountStatus>,
}

/// One resolved refresh result, addressed back to the workspace that asked for it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceSessionStatus {
    pub workspace_id: String,
    pub checkout_key: std::path::PathBuf,
    pub snapshot: SessionStatusSnapshot,
}

pub(crate) fn round_cents(dollars: f64) -> Option<u64> {
    if !dollars.is_finite() || dollars < 0.0 {
        return None;
    }
    Some((dollars * 100.0).round() as u64)
}

fn round_signed_cents(dollars: f64) -> Option<i64> {
    if !dollars.is_finite() {
        return None;
    }
    Some((dollars * 100.0).round() as i64)
}

/// `llmtrim status --json` -> `{"cache_read_tokens":…,"money":{"paid_usd":…,"saved_usd":…},
/// "cost":{"spend_usd":…,"saved_usd":…}}`.
///
/// `money` is the authoritative all-in accounting view when present; `cost` is the fallback for
/// older `llmtrim` versions that do not expose `money` — ported from `MapLLMTrimStatus`
/// unchanged.
pub fn parse_llmtrim_status(stdout: &str) -> Option<LlmTrimStatus> {
    let value: serde_json::Value = serde_json::from_str(stdout).ok()?;
    if !value.is_object() {
        return None;
    }

    let cache_read_tokens = value.get("cache_read_tokens").and_then(|v| v.as_u64());

    let money = value.get("money");
    let cost = value.get("cost");
    let spend = money
        .and_then(|m| m.get("paid_usd"))
        .and_then(|v| v.as_f64())
        .or_else(|| {
            cost.and_then(|c| c.get("spend_usd"))
                .and_then(|v| v.as_f64())
        })
        .and_then(round_cents)
        .map(|cents| SpendStatus { cents });
    let savings = money
        .and_then(|m| m.get("saved_usd"))
        .and_then(|v| v.as_f64())
        .or_else(|| {
            cost.and_then(|c| c.get("saved_usd"))
                .and_then(|v| v.as_f64())
        })
        .and_then(round_cents)
        .map(|cents| SpendStatus { cents });

    if cache_read_tokens.is_none() && spend.is_none() && savings.is_none() {
        return None;
    }
    Some(LlmTrimStatus {
        spend,
        savings,
        cache_read_tokens,
    })
}

/// `context-floor --json` -> `{"total_bytes":…,"floor_warn":…,"growth_pct":…,
/// "dollars_per_1k_turns":…}`. `growth_pct` and `prior_total_bytes` are `null` on a cold start
/// — a JSON `null` and an absent key are both treated as "not yet resolvable", matching
/// `MapContextFloor`.
pub fn parse_context_floor_status(stdout: &str) -> Option<ContextFloorStatus> {
    let value: serde_json::Value = serde_json::from_str(stdout).ok()?;
    if !value.is_object() {
        return None;
    }

    let instruction_total_bytes = value.get("total_bytes").and_then(|v| v.as_u64());
    let instruction_floor_warn = value.get("floor_warn").and_then(|v| v.as_bool());
    let instruction_growth_bps = value
        .get("growth_pct")
        .and_then(|v| v.as_f64())
        .and_then(round_signed_cents);
    let instruction_dollars_per_1k_turns_cents = value
        .get("dollars_per_1k_turns")
        .and_then(|v| v.as_f64())
        .and_then(round_signed_cents);

    if instruction_total_bytes.is_none()
        && instruction_floor_warn.is_none()
        && instruction_growth_bps.is_none()
        && instruction_dollars_per_1k_turns_cents.is_none()
    {
        return None;
    }
    Some(ContextFloorStatus {
        instruction_total_bytes,
        instruction_floor_warn,
        instruction_growth_bps,
        instruction_dollars_per_1k_turns_cents,
    })
}

/// One persisted-session snapshot file -> `{"session_id":…,"model":…,
/// "current_context_tokens":…,"context_window_tokens":…,"observed_at":…,"fresh_until":…}`.
///
/// A record with an empty `session_id` or an unparseable/absent `observed_at` is not a session
/// — mirrors the companion's `SessionID == "" || ObservedAt.IsZero()` skip.
pub fn parse_persisted_session_record(bytes: &str) -> Option<PersistedSessionRecord> {
    let value: serde_json::Value = serde_json::from_str(bytes).ok()?;
    let session_id = value.get("session_id")?.as_str()?.to_string();
    if session_id.is_empty() {
        return None;
    }
    let observed_at = value.get("observed_at")?.as_str()?.to_string();
    parse_rfc3339_nanos(&observed_at)?;

    let model = value
        .get("model")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(str::to_string);
    let context_tokens = value
        .get("current_context_tokens")
        .and_then(|v| v.as_i64())
        .filter(|n| *n != 0);
    let context_window_tokens = value
        .get("context_window_tokens")
        .and_then(|v| v.as_i64())
        .filter(|n| *n != 0);
    let fresh_until = value
        .get("fresh_until")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(str::to_string);

    Some(PersistedSessionRecord {
        session_id,
        model,
        context_tokens,
        context_window_tokens,
        observed_at,
        fresh_until,
    })
}

/// Merges every session record under the store root into one `SessionsStatus`. Each field uses
/// its newest non-empty observation independently — a session with a model but no context
/// tokens does not blank a newer session's context tokens, and vice versa. Ported from
/// `persistedSessionFacts`.
pub fn merge_persisted_sessions(records: &[PersistedSessionRecord]) -> SessionsStatus {
    let mut model: Option<(i128, String)> = None;
    let mut context_tokens: Option<(i128, i64)> = None;
    let mut context_window_tokens: Option<(i128, i64)> = None;

    for record in records {
        let Some(observed_nanos) = parse_rfc3339_nanos(&record.observed_at) else {
            continue;
        };
        if let Some(value) = &record.model {
            if model.as_ref().is_none_or(|(at, _)| observed_nanos > *at) {
                model = Some((observed_nanos, value.clone()));
            }
        }
        if let Some(value) = record.context_tokens {
            if context_tokens
                .as_ref()
                .is_none_or(|(at, _)| observed_nanos > *at)
            {
                context_tokens = Some((observed_nanos, value));
            }
        }
        if let Some(value) = record.context_window_tokens {
            if context_window_tokens
                .as_ref()
                .is_none_or(|(at, _)| observed_nanos > *at)
            {
                context_window_tokens = Some((observed_nanos, value));
            }
        }
    }

    SessionsStatus {
        model: model.map(|(_, v)| v),
        context_tokens: context_tokens.map(|(_, v)| v),
        context_window_tokens: context_window_tokens.map(|(_, v)| v),
        session_count: records.len() as u64,
    }
}

/// Compact rendered form, sized like the existing `git_status` and work-queue tokens.
pub fn render_spend_status(spend: SpendStatus) -> String {
    format!("${}.{:02}", spend.cents / 100, spend.cents % 100)
}

/// Parses an RFC 3339 timestamp into nanoseconds since the Unix epoch, UTC-normalized. Only
/// ordering is needed here (which observation is newest), so this is deliberately not a general
/// calendar library — no existing dependency covers RFC 3339 parsing, and pulling one in for a
/// single comparison would be a heavier addition than a self-contained ~40-line parser.
pub(crate) fn parse_rfc3339_nanos(value: &str) -> Option<i128> {
    let year: i64 = value.get(0..4)?.parse().ok()?;
    if value.as_bytes().get(4)? != &b'-' {
        return None;
    }
    let month: u32 = value.get(5..7)?.parse().ok()?;
    if value.as_bytes().get(7)? != &b'-' {
        return None;
    }
    let day: u32 = value.get(8..10)?.parse().ok()?;
    match value.as_bytes().get(10)? {
        b'T' | b't' | b' ' => {}
        _ => return None,
    }
    let hour: i64 = value.get(11..13)?.parse().ok()?;
    if value.as_bytes().get(13)? != &b':' {
        return None;
    }
    let minute: i64 = value.get(14..16)?.parse().ok()?;
    if value.as_bytes().get(16)? != &b':' {
        return None;
    }
    let second: i64 = value.get(17..19)?.parse().ok()?;

    let mut idx = 19;
    let mut nanos: i64 = 0;
    if value.as_bytes().get(idx) == Some(&b'.') {
        idx += 1;
        let start = idx;
        while value.as_bytes().get(idx).is_some_and(u8::is_ascii_digit) {
            idx += 1;
        }
        let mut digits = value.get(start..idx)?.to_string();
        digits.truncate(9);
        while digits.len() < 9 {
            digits.push('0');
        }
        nanos = digits.parse().ok()?;
    }

    let offset_seconds: i64 = match value.as_bytes().get(idx)? {
        b'Z' | b'z' => 0,
        sign @ (b'+' | b'-') => {
            let rest = value.get(idx + 1..)?;
            let offset_hour: i64 = rest.get(0..2)?.parse().ok()?;
            if rest.as_bytes().get(2)? != &b':' {
                return None;
            }
            let offset_minute: i64 = rest.get(3..5)?.parse().ok()?;
            let total = offset_hour * 3600 + offset_minute * 60;
            if *sign == b'-' {
                -total
            } else {
                total
            }
        }
        _ => return None,
    };

    let days = days_from_civil(year, month, day)?;
    let seconds = days * 86_400 + hour * 3600 + minute * 60 + second - offset_seconds;
    Some(seconds as i128 * 1_000_000_000 + nanos as i128)
}

/// Howard Hinnant's `days_from_civil`: days since the Unix epoch for a proleptic-Gregorian
/// civil date. Widely used, closed-form, and avoids a date-library dependency for one
/// conversion.
fn days_from_civil(y: i64, m: u32, d: u32) -> Option<i64> {
    if !(1..=12).contains(&m) || !(1..=31).contains(&d) {
        return None;
    }
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = (m as i64 + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d as i64 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    Some(era * 146_097 + doe - 719_468)
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- llmtrim ---

    /// Captured from `llmtrim status --json` @ 2026-08-02,
    /// `shepherd-plugins/plugins/shepherd-state/pkg/chrome/testdata/llmtrim-status.json`.
    const LLMTRIM_FIXTURE: &str = r#"{
  "daemon": {"running": true, "health": "healthy", "version": "0.11.11"},
  "requests": 43570,
  "cost": {"saved_usd": 3229.1578250000002, "spend_usd": 9256.76148275, "source": "compressions_live_prices"},
  "money": {"source": "breakdown_turns", "unavailable": false, "paid_usd": 10496.788219, "saved_usd": 2525.497838, "would_have_usd": 13022.286057, "saved_today_usd": 474.977084, "turns": 50000},
  "cache_read_tokens": 3660890700,
  "approximate": true
}"#;

    #[test]
    fn llmtrim_prefers_money_over_cost() {
        let status = parse_llmtrim_status(LLMTRIM_FIXTURE).expect("parses");
        assert_eq!(status.spend, Some(SpendStatus { cents: 1_049_679 }));
        assert_eq!(status.savings, Some(SpendStatus { cents: 252_550 }));
        assert_eq!(status.cache_read_tokens, Some(3_660_890_700));
    }

    #[test]
    fn llmtrim_falls_back_to_cost_when_money_is_absent() {
        let without_money =
            r#"{"cost":{"spend_usd":12.5,"saved_usd":1.25},"cache_read_tokens":10}"#;
        let status = parse_llmtrim_status(without_money).expect("parses");
        assert_eq!(status.spend, Some(SpendStatus { cents: 1250 }));
        assert_eq!(status.savings, Some(SpendStatus { cents: 125 }));
    }

    #[test]
    fn llmtrim_malformed_and_empty_bodies_resolve_to_none() {
        for body in ["", "not json", "[]", "{}", r#"{"daemon":{"running":true}}"#] {
            assert!(parse_llmtrim_status(body).is_none(), "must reject {body:?}");
        }
    }

    // --- context-floor ---

    /// Captured from `context-floor --json` @ 2026-08-02,
    /// `shepherd-plugins/plugins/shepherd-state/pkg/chrome/testdata/context-floor.json`.
    const CONTEXT_FLOOR_FIXTURE: &str = r#"{
  "components": {"claude_md": 1984},
  "total_bytes": 79659,
  "prior_total_bytes": null,
  "growth_pct": null,
  "floor_warn": false,
  "dollars_per_1k_turns": 9.9574
}"#;

    #[test]
    fn context_floor_parses_the_captured_fixture() {
        let status = parse_context_floor_status(CONTEXT_FLOOR_FIXTURE).expect("parses");
        assert_eq!(status.instruction_total_bytes, Some(79_659));
        assert_eq!(status.instruction_floor_warn, Some(false));
        assert_eq!(
            status.instruction_growth_bps, None,
            "a null growth_pct is not fabricated"
        );
        assert_eq!(status.instruction_dollars_per_1k_turns_cents, Some(996));
    }

    #[test]
    fn context_floor_does_not_fabricate_a_model_or_session_field() {
        // context-floor never reports model or context-window facts; the parser must not
        // invent them from an unrelated shape.
        let status = parse_context_floor_status(CONTEXT_FLOOR_FIXTURE).expect("parses");
        assert!(!format!("{status:?}").contains("model"));
    }

    #[test]
    fn context_floor_malformed_and_empty_bodies_resolve_to_none() {
        for body in ["", "not json", "[]", "{}"] {
            assert!(
                parse_context_floor_status(body).is_none(),
                "must reject {body:?}"
            );
        }
    }

    // --- sessions ---

    #[test]
    fn a_session_record_parses_from_captured_shape() {
        let body = r#"{"version":1,"harness":"codex","session_id":"session-a","model":"gpt-5.6-sol","current_context_tokens":58000,"context_window_tokens":200000,"observed_at":"2026-08-02T19:00:00Z"}"#;
        let record = parse_persisted_session_record(body).expect("parses");
        assert_eq!(record.session_id, "session-a");
        assert_eq!(record.model.as_deref(), Some("gpt-5.6-sol"));
        assert_eq!(record.context_tokens, Some(58_000));
        assert_eq!(record.context_window_tokens, Some(200_000));
    }

    #[test]
    fn a_record_with_no_session_id_or_no_observed_at_is_not_a_session() {
        for body in [
            r#"{"session_id":"","observed_at":"2026-08-02T19:00:00Z"}"#,
            r#"{"session_id":"x"}"#,
            r#"{"session_id":"x","observed_at":""}"#,
            r#"{"session_id":"x","observed_at":"not-a-timestamp"}"#,
            "not json",
            "",
        ] {
            assert!(
                parse_persisted_session_record(body).is_none(),
                "must reject {body:?}"
            );
        }
    }

    #[test]
    fn merge_keeps_each_fields_newest_non_empty_observation_independently() {
        let older = PersistedSessionRecord {
            session_id: "a".into(),
            model: Some("older-model".into()),
            context_tokens: None,
            context_window_tokens: Some(50_000),
            observed_at: "2026-08-02T18:00:00Z".into(),
            fresh_until: None,
        };
        let newer_without_model = PersistedSessionRecord {
            session_id: "b".into(),
            model: None,
            context_tokens: Some(12_000),
            context_window_tokens: None,
            observed_at: "2026-08-02T19:00:00Z".into(),
            fresh_until: None,
        };
        let merged = merge_persisted_sessions(&[older, newer_without_model]);

        assert_eq!(
            merged.model.as_deref(),
            Some("older-model"),
            "the newer record's absent model must not blank the older record's model"
        );
        assert_eq!(merged.context_tokens, Some(12_000));
        assert_eq!(merged.context_window_tokens, Some(50_000));
        assert_eq!(merged.session_count, 2);
    }

    #[test]
    fn merge_of_no_records_yields_a_deterministic_empty_count() {
        let merged = merge_persisted_sessions(&[]);
        assert_eq!(merged, SessionsStatus::default());
        assert_eq!(merged.session_count, 0);
    }

    #[test]
    fn a_truly_newer_observation_replaces_an_older_ones_field() {
        let older = PersistedSessionRecord {
            session_id: "a".into(),
            model: Some("gpt-5.6-sol".into()),
            context_tokens: Some(58_000),
            context_window_tokens: Some(200_000),
            observed_at: "2026-08-02T18:59:00Z".into(),
            fresh_until: None,
        };
        let newer = PersistedSessionRecord {
            session_id: "b".into(),
            model: Some("claude-opus-5".into()),
            context_tokens: Some(12_000),
            context_window_tokens: Some(200_000),
            observed_at: "2026-08-02T19:00:00Z".into(),
            fresh_until: None,
        };
        let merged = merge_persisted_sessions(&[older, newer]);
        assert_eq!(merged.model.as_deref(), Some("claude-opus-5"));
        assert_eq!(merged.context_tokens, Some(12_000));
    }

    // --- rfc3339 ordering ---

    #[test]
    fn rfc3339_orders_by_actual_instant_not_lexicographically() {
        // A trailing fractional-second suffix must not corrupt ordering against a bare-seconds
        // timestamp with no fraction: 19:00:00.5Z is after 19:00:00Z, even though 'Z' < '.'
        // in ASCII would order them the other way under naive string comparison.
        let bare = parse_rfc3339_nanos("2026-08-02T19:00:00Z").unwrap();
        let fractional = parse_rfc3339_nanos("2026-08-02T19:00:00.5Z").unwrap();
        assert!(fractional > bare);
    }

    #[test]
    fn rfc3339_normalizes_timezone_offsets_to_utc() {
        let utc = parse_rfc3339_nanos("2026-08-02T19:00:00Z").unwrap();
        let plus_one = parse_rfc3339_nanos("2026-08-02T20:00:00+01:00").unwrap();
        let minus_five = parse_rfc3339_nanos("2026-08-02T14:00:00-05:00").unwrap();
        assert_eq!(utc, plus_one);
        assert_eq!(utc, minus_five);
    }

    #[test]
    fn rfc3339_rejects_malformed_input() {
        for body in ["", "not-a-timestamp", "2026-08-02", "2026-13-02T19:00:00Z"] {
            assert!(parse_rfc3339_nanos(body).is_none(), "must reject {body:?}");
        }
    }

    // --- render / demand ---

    #[test]
    fn rendered_form_is_compact() {
        assert_eq!(
            render_spend_status(SpendStatus { cents: 1_021_070 }),
            "$10210.70"
        );
        assert_eq!(render_spend_status(SpendStatus { cents: 5 }), "$0.05");
        assert_eq!(render_spend_status(SpendStatus { cents: 0 }), "$0.00");
    }

    #[test]
    fn demand_is_empty_until_a_source_asks_for_it() {
        assert!(SessionStatusRefreshDemand::default().is_empty());
        assert!(!SessionStatusRefreshDemand {
            llmtrim: true,
            ..Default::default()
        }
        .is_empty());
        assert!(SessionStatusSnapshot::default().llmtrim.is_none());
    }
}
