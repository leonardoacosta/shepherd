//! Session and provider telemetry that Shepherd resolves for itself.
//!
//! Parsing lives here and stays pure so it can be tested against captured adapter output.
//! Process execution lives in `crate::app::session_provider_refresh`.
//!
//! Shape mirrors `crate::workspace::project_status` layer for layer — see `CONTEXT.md`.

/// Which adapters a refresh must run. Resolved from the configured rows, so an operator who
/// configures no session token pays no process cost. Mirrors `ProjectStatusRefreshDemand`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SessionStatusRefreshDemand {
    pub spend: bool,
}

impl SessionStatusRefreshDemand {
    pub fn is_empty(&self) -> bool {
        !self.spend
    }
}

/// Cumulative spend, in whole cents, as reported by the chrome adapter.
///
/// Cents rather than a float: the value is rendered, compared for change detection, and cached,
/// and none of those want float equality. The adapter's own payload is a float, so the
/// conversion happens once, at the parse boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpendStatus {
    pub cents: u64,
}

/// Each field is independently optional: one unavailable source must not blank another's value.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SessionStatusSnapshot {
    pub spend: Option<SpendStatus>,
}

/// One resolved refresh result, addressed back to the workspace that asked for it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceSessionStatus {
    pub workspace_id: String,
    pub checkout_key: std::path::PathBuf,
    pub snapshot: SessionStatusSnapshot,
}

/// `shepherd-state chrome --once --json` ->
/// `{"version":1,"fields":{"<name>":{"value":…,"source":…,"stale":bool,…}}}`.
///
/// A field the adapter did not resolve is simply absent from `fields`, which is the same
/// answer as a missing adapter: no value for that token.
///
/// A `stale` field yields no value either. The adapter reports staleness precisely so a
/// consumer can tell a current number from one whose producer has stopped answering;
/// rendering a known-stale figure as if it were live is the failure this whole path exists
/// to remove.
pub fn parse_spend_status(stdout: &str) -> Option<SpendStatus> {
    let value: serde_json::Value = serde_json::from_str(stdout).ok()?;
    if value.get("version")?.as_u64()? != 1 {
        return None;
    }
    let field = value.get("fields")?.as_object()?.get("spend_usd")?;
    if field.get("stale")?.as_bool()? {
        return None;
    }
    let dollars = field.get("value")?.as_f64()?;
    if !dollars.is_finite() || dollars < 0.0 {
        return None;
    }
    Some(SpendStatus {
        cents: (dollars * 100.0).round() as u64,
    })
}

/// Compact rendered form, sized like the existing `git_status` and work-queue tokens.
pub fn render_spend_status(spend: SpendStatus) -> String {
    format!("${}.{:02}", spend.cents / 100, spend.cents % 100)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Captured from `shepherd-state chrome --once --json` @ 2026-08-05, trimmed to the
    /// fields this parser reads plus one from another source to prove independence.
    const CHROME_FIXTURE: &str = r#"{
  "version": 1,
  "generation": 1,
  "fields": {
    "spend_usd": {
      "value": 10210.702688,
      "source": "llmtrim",
      "observed_at": "2026-08-05T11:35:04.492320783-05:00",
      "fresh_until": "2026-08-05T11:36:04.492320783-05:00",
      "stale": false
    },
    "session_count": {
      "value": 4,
      "source": "sessions",
      "observed_at": "2026-08-05T11:35:04.492320783-05:00",
      "fresh_until": "2026-08-05T11:36:04.492320783-05:00",
      "stale": true
    }
  }
}"#;

    #[test]
    fn spend_parses_from_captured_adapter_output() {
        let spend = parse_spend_status(CHROME_FIXTURE).expect("parses");
        assert_eq!(spend.cents, 1_021_070, "dollars convert to whole cents");
    }

    #[test]
    fn a_stale_field_resolves_to_none() {
        let stale = CHROME_FIXTURE.replacen(r#""stale": false"#, r#""stale": true"#, 1);
        assert!(
            parse_spend_status(&stale).is_none(),
            "a known-stale figure must not render as if it were live"
        );
    }

    #[test]
    fn an_absent_field_resolves_to_none_without_affecting_the_envelope() {
        let without = r#"{"version":1,"fields":{"session_count":{"value":4,"stale":false}}}"#;
        assert!(parse_spend_status(without).is_none());
    }

    #[test]
    fn malformed_and_unexpected_shapes_resolve_to_none() {
        for body in [
            "",
            "not json",
            "{}",
            r#"{"version":1}"#,
            r#"{"version":2,"fields":{"spend_usd":{"value":1.0,"stale":false}}}"#,
            r#"{"version":1,"fields":[]}"#,
            r#"{"version":1,"fields":{"spend_usd":{"value":"lots","stale":false}}}"#,
            r#"{"version":1,"fields":{"spend_usd":{"value":1.0}}}"#,
            r#"{"version":1,"fields":{"spend_usd":{"value":-1.0,"stale":false}}}"#,
        ] {
            assert!(
                parse_spend_status(body).is_none(),
                "spend parse must reject {body:?}"
            );
        }
    }

    #[test]
    fn zero_spend_parses_to_a_value_rather_than_absent() {
        let zero = r#"{"version":1,"fields":{"spend_usd":{"value":0.0,"stale":false}}}"#;
        assert_eq!(
            parse_spend_status(zero).expect("zero is a real value"),
            SpendStatus { cents: 0 }
        );
    }

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
    fn demand_is_empty_until_a_token_asks_for_it() {
        assert!(SessionStatusRefreshDemand::default().is_empty());
        assert!(!SessionStatusRefreshDemand { spend: true }.is_empty());
        assert!(SessionStatusSnapshot::default().spend.is_none());
    }
}
