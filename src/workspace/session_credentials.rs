//! `local-account-signal` — quota remaining and cooldown reset time, read from shepherd-state's
//! in-house credential pool (`shepherd-token-tab`'s account rotation state).
//!
//! Ported from `cmd/shepherd-state/main.go`'s `defaultReadAccountSignals`, the function that
//! actually backs this chrome source in the live system — **not** from `pkg/credentials`'
//! `Mutator.PollUsage`/`Mutator.Refresh`, which decrypt OAuth blobs and call Anthropic live.
//! `defaultReadAccountSignals` does neither: it reads `credentials.jsonl`'s plaintext columns
//! (never `ValueEncrypted`) and `usage.json` (also plaintext, itself a cache some other,
//! currently-unwired process would refresh), and derives `remaining = limit - used`. See
//! `openspec/changes/session-provider-adapters/proposal.md` `## Decisions`
//! "`local-account-signal` polling and refresh" for the full correction history — an earlier
//! version of this decision wrongly assumed live decrypt+HTTP was required.
//!
//! The adapter (I/O: read `credentials.jsonl`/`usage.json`, call the derivation below) lives in
//! `crate::app::session_provider_refresh`; not yet reachable from a token, since that lands with
//! task 6's vocabulary wiring.

use std::collections::HashMap;

use super::session_status::parse_rfc3339_nanos;

/// One row of `credentials.jsonl`, read for exactly the two plaintext columns this source
/// needs. `ValueEncrypted` is never parsed into a field here — not merely unrendered, but
/// structurally absent from this type, so there is no field that could leak it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CredentialRow {
    pub id: String,
    /// Raw RFC3339 string, parsed on demand — mirrors `PersistedSessionRecord`'s convention.
    pub cooldown_until: Option<String>,
}

/// Parses `credentials.jsonl` — one JSON object per line. Strict, all-or-nothing: a single
/// malformed line fails the whole read, mirroring `ReadCredentials`'s own `json.Unmarshal`
/// error propagation (unlike the persisted-session store, which isolates one bad file among
/// many). A missing file is not this function's concern — that is an I/O-layer distinction the
/// adapter makes before ever calling this parser.
pub fn parse_credentials_jsonl(text: &str) -> Option<Vec<CredentialRow>> {
    let mut rows = Vec::new();
    for line in text.split('\n') {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let value: serde_json::Value = serde_json::from_str(line).ok()?;
        let id = value.get("id")?.as_str()?.to_string();
        let cooldown_until = value
            .get("cooldown_until")
            .filter(|v| !v.is_null())
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .map(str::to_string);
        rows.push(CredentialRow { id, cooldown_until });
    }
    Some(rows)
}

/// One credential's volatile poll result, as stored in `usage.json`. Only the two fields
/// `local-account-signal` needs — the 7-day window and its own reset time are out of scope for
/// this source, per the proposal's inventory table.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct UsageEntry {
    pub usage_5h_used: Option<i64>,
    pub usage_5h_limit: Option<i64>,
    /// Raw RFC3339 string.
    pub polled_at: Option<String>,
    pub ttl_seconds: i64,
}

/// Parses `usage.json` — `{"<credential id>": {...}, ...}`. Strict: malformed JSON fails the
/// whole read, mirroring `ReadUsage`. As with [`parse_credentials_jsonl`], a missing file is the
/// adapter's concern, not this parser's — an empty map is a valid parse of an empty object,
/// `{}`, not of a missing file.
pub fn parse_usage_json(text: &str) -> Option<HashMap<String, UsageEntry>> {
    let value: serde_json::Value = serde_json::from_str(text).ok()?;
    let object = value.as_object()?;
    let mut out = HashMap::with_capacity(object.len());
    for (id, entry) in object {
        let usage_5h_used = entry.get("usage_5h_used").and_then(|v| v.as_i64());
        let usage_5h_limit = entry.get("usage_5h_limit").and_then(|v| v.as_i64());
        let polled_at = entry
            .get("polled_at")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .map(str::to_string);
        let ttl_seconds = entry
            .get("ttl_seconds")
            .and_then(|v| v.as_i64())
            .unwrap_or(0);
        out.insert(
            id.clone(),
            UsageEntry {
                usage_5h_used,
                usage_5h_limit,
                polled_at,
                ttl_seconds,
            },
        );
    }
    Some(out)
}

/// Mirrors `Usage.Fresh`: a zero or negative TTL is always stale — an unset TTL must never read
/// as "cached forever".
fn usage_is_fresh(entry: &UsageEntry, now_nanos: i128) -> bool {
    if entry.ttl_seconds <= 0 {
        return false;
    }
    let Some(polled_at) = entry.polled_at.as_deref().and_then(parse_rfc3339_nanos) else {
        return false;
    };
    let fresh_until = polled_at + (entry.ttl_seconds as i128) * 1_000_000_000;
    now_nanos < fresh_until
}

/// `local-account-signal`'s two chrome facts: total remaining tokens across every account whose
/// usage is fresh and complete, and the earliest cooldown among accounts that have one.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LocalAccountStatus {
    pub quota_remaining: Option<i64>,
    pub reset_at: Option<String>,
}

/// Ports `defaultReadAccountSignals` + `DeriveLocalAccountQuotas` + `LocalAccountProducer.Poll`
/// as one function: build a signal per credential row (skipping any without fresh, complete
/// usage), drop signals with neither a usable quota nor a cooldown, then sum the remaining
/// quotas and take the earliest cooldown.
pub fn derive_local_account_status(
    credentials: &[CredentialRow],
    usage: &HashMap<String, UsageEntry>,
    now_nanos: i128,
) -> LocalAccountStatus {
    let mut total: i64 = 0;
    let mut has_quota = false;
    let mut earliest: Option<(i128, String)> = None;

    for credential in credentials {
        let Some(entry) = usage.get(&credential.id) else {
            continue;
        };
        if !usage_is_fresh(entry, now_nanos) {
            continue;
        }
        let (Some(used), Some(limit)) = (entry.usage_5h_used, entry.usage_5h_limit) else {
            continue;
        };
        let remaining = (limit - used).max(0);

        let cooldown_nanos = credential
            .cooldown_until
            .as_deref()
            .and_then(parse_rfc3339_nanos);

        // Mirrors DeriveLocalAccountQuotas's own filter: a signal with zero remaining AND no
        // cooldown is dropped rather than contributing a real-looking zero to the total.
        if remaining == 0 && cooldown_nanos.is_none() {
            continue;
        }

        if remaining != 0 {
            total += remaining;
            has_quota = true;
        }
        if let Some(nanos) = cooldown_nanos {
            if earliest.as_ref().is_none_or(|(at, _)| nanos < *at) {
                earliest = Some((nanos, credential.cooldown_until.clone().unwrap()));
            }
        }
    }

    LocalAccountStatus {
        quota_remaining: has_quota.then_some(total),
        reset_at: earliest.map(|(_, raw)| raw),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn credential(id: &str, cooldown_until: Option<&str>) -> CredentialRow {
        CredentialRow {
            id: id.to_string(),
            cooldown_until: cooldown_until.map(str::to_string),
        }
    }

    fn fresh_entry(used: i64, limit: i64, polled_at: &str, ttl: i64) -> UsageEntry {
        UsageEntry {
            usage_5h_used: Some(used),
            usage_5h_limit: Some(limit),
            polled_at: Some(polled_at.to_string()),
            ttl_seconds: ttl,
        }
    }

    // --- parse_credentials_jsonl ---

    #[test]
    fn credentials_jsonl_parses_id_and_cooldown_and_ignores_other_fields() {
        let text = concat!(
            r#"{"id":"cred-a","name":"a","fingerprint":"abc","status":"available","value_encrypted":"SECRET-ENVELOPE","cooldown_until":"2026-08-05T12:00:00Z"}"#,
            "\n",
            r#"{"id":"cred-b","name":"b","status":"available","value_encrypted":"OTHER-SECRET","cooldown_until":null}"#,
        );
        let rows = parse_credentials_jsonl(text).expect("parses");
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].id, "cred-a");
        assert_eq!(
            rows[0].cooldown_until.as_deref(),
            Some("2026-08-05T12:00:00Z")
        );
        assert_eq!(rows[1].id, "cred-b");
        assert_eq!(rows[1].cooldown_until, None);
    }

    #[test]
    fn credentials_jsonl_never_exposes_value_encrypted_even_by_accident() {
        let text = r#"{"id":"cred-a","value_encrypted":"SECRET-ENVELOPE"}"#;
        let rows = parse_credentials_jsonl(text).expect("parses");
        // Structural guarantee: CredentialRow simply has no field that could hold the secret.
        assert!(!format!("{:?}", rows[0]).contains("SECRET-ENVELOPE"));
    }

    #[test]
    fn credentials_jsonl_skips_blank_lines() {
        let text = "\n\n{\"id\":\"cred-a\"}\n\n";
        let rows = parse_credentials_jsonl(text).expect("parses");
        assert_eq!(rows.len(), 1);
    }

    #[test]
    fn credentials_jsonl_a_single_malformed_line_fails_the_whole_read() {
        let text = "{\"id\":\"cred-a\"}\nnot json\n{\"id\":\"cred-b\"}";
        assert!(
            parse_credentials_jsonl(text).is_none(),
            "one bad line must fail the whole file, matching ReadCredentials"
        );
    }

    #[test]
    fn credentials_jsonl_a_row_with_no_id_fails_the_whole_read() {
        assert!(parse_credentials_jsonl(r#"{"name":"no id"}"#).is_none());
    }

    // --- parse_usage_json ---

    #[test]
    fn usage_json_parses_the_map() {
        let text = r#"{"cred-a":{"usage_5h_used":10,"usage_5h_limit":100,"polled_at":"2026-08-05T12:00:00Z","ttl_seconds":300}}"#;
        let usage = parse_usage_json(text).expect("parses");
        let entry = &usage["cred-a"];
        assert_eq!(entry.usage_5h_used, Some(10));
        assert_eq!(entry.usage_5h_limit, Some(100));
        assert_eq!(entry.ttl_seconds, 300);
    }

    #[test]
    fn usage_json_empty_object_parses_to_an_empty_map() {
        assert_eq!(parse_usage_json("{}").expect("parses").len(), 0);
    }

    #[test]
    fn usage_json_malformed_content_fails_the_whole_read() {
        assert!(parse_usage_json("").is_none());
        assert!(parse_usage_json("not json").is_none());
    }

    // --- usage_is_fresh ---

    #[test]
    fn usage_is_fresh_within_ttl_stale_after() {
        let entry = fresh_entry(1, 2, "2026-08-05T12:00:00Z", 300);
        let polled = parse_rfc3339_nanos("2026-08-05T12:00:00Z").unwrap();
        assert!(usage_is_fresh(&entry, polled + 100 * 1_000_000_000));
        assert!(!usage_is_fresh(&entry, polled + 301 * 1_000_000_000));
    }

    #[test]
    fn usage_is_fresh_zero_or_negative_ttl_is_always_stale() {
        let mut entry = fresh_entry(1, 2, "2026-08-05T12:00:00Z", 0);
        let now = parse_rfc3339_nanos("2026-08-05T12:00:00Z").unwrap();
        assert!(!usage_is_fresh(&entry, now));
        entry.ttl_seconds = -5;
        assert!(!usage_is_fresh(&entry, now));
    }

    // --- derive_local_account_status ---

    #[test]
    fn derives_total_remaining_and_earliest_cooldown_across_accounts() {
        let now = parse_rfc3339_nanos("2026-08-05T12:00:00Z").unwrap();
        let credentials = vec![
            credential("cred-a", Some("2026-08-05T13:00:00Z")),
            credential("cred-b", Some("2026-08-05T12:30:00Z")),
        ];
        let usage: HashMap<String, UsageEntry> = [
            (
                "cred-a".to_string(),
                fresh_entry(10, 100, "2026-08-05T12:00:00Z", 300),
            ),
            (
                "cred-b".to_string(),
                fresh_entry(20, 50, "2026-08-05T12:00:00Z", 300),
            ),
        ]
        .into_iter()
        .collect();

        let status = derive_local_account_status(&credentials, &usage, now);
        assert_eq!(status.quota_remaining, Some(90 + 30));
        assert_eq!(status.reset_at.as_deref(), Some("2026-08-05T12:30:00Z"));
    }

    #[test]
    fn a_credential_with_no_usage_entry_is_skipped_not_zeroed() {
        let now = parse_rfc3339_nanos("2026-08-05T12:00:00Z").unwrap();
        let credentials = vec![credential("cred-a", None)];
        let status = derive_local_account_status(&credentials, &HashMap::new(), now);
        assert_eq!(status.quota_remaining, None);
        assert_eq!(status.reset_at, None);
    }

    #[test]
    fn a_stale_usage_entry_is_skipped() {
        let now = parse_rfc3339_nanos("2026-08-05T13:00:00Z").unwrap(); // 1h after polled_at, ttl 300s
        let credentials = vec![credential("cred-a", None)];
        let usage: HashMap<String, UsageEntry> = [(
            "cred-a".to_string(),
            fresh_entry(10, 100, "2026-08-05T12:00:00Z", 300),
        )]
        .into_iter()
        .collect();
        let status = derive_local_account_status(&credentials, &usage, now);
        assert_eq!(
            status.quota_remaining, None,
            "a stale entry must not contribute"
        );
    }

    #[test]
    fn an_incomplete_usage_entry_is_skipped() {
        let now = parse_rfc3339_nanos("2026-08-05T12:00:00Z").unwrap();
        let credentials = vec![credential("cred-a", None)];
        let mut entry = fresh_entry(10, 100, "2026-08-05T12:00:00Z", 300);
        entry.usage_5h_limit = None; // present but incomplete
        let usage: HashMap<String, UsageEntry> =
            [("cred-a".to_string(), entry)].into_iter().collect();
        let status = derive_local_account_status(&credentials, &usage, now);
        assert_eq!(status.quota_remaining, None);
    }

    #[test]
    fn remaining_never_goes_negative() {
        let now = parse_rfc3339_nanos("2026-08-05T12:00:00Z").unwrap();
        let credentials = vec![credential("cred-a", None)];
        let usage: HashMap<String, UsageEntry> = [(
            "cred-a".to_string(),
            fresh_entry(150, 100, "2026-08-05T12:00:00Z", 300), // used > limit
        )]
        .into_iter()
        .collect();
        let status = derive_local_account_status(&credentials, &usage, now);
        // remaining is 0, no cooldown -> dropped entirely (matches DeriveLocalAccountQuotas).
        assert_eq!(status.quota_remaining, None);
    }

    #[test]
    fn a_zero_remaining_account_with_a_cooldown_still_contributes_its_cooldown() {
        let now = parse_rfc3339_nanos("2026-08-05T12:00:00Z").unwrap();
        let credentials = vec![credential("cred-a", Some("2026-08-05T14:00:00Z"))];
        let usage: HashMap<String, UsageEntry> = [(
            "cred-a".to_string(),
            fresh_entry(100, 100, "2026-08-05T12:00:00Z", 300),
        )]
        .into_iter()
        .collect();
        let status = derive_local_account_status(&credentials, &usage, now);
        assert_eq!(
            status.quota_remaining, None,
            "zero remaining contributes no quota"
        );
        assert_eq!(status.reset_at.as_deref(), Some("2026-08-05T14:00:00Z"));
    }

    #[test]
    fn no_credentials_yields_an_empty_status() {
        let now = parse_rfc3339_nanos("2026-08-05T12:00:00Z").unwrap();
        let status = derive_local_account_status(&[], &HashMap::new(), now);
        assert_eq!(status, LocalAccountStatus::default());
    }
}
