//! Pricing is a pure derivation over already-cached token counts — it reads no source, so it
//! gets no adapter. Ported from `shepherd-plugins/plugins/shepherd-state/pkg/pricing`
//! (vendored there from the pinned upstream token dashboard, MIT-licensed) rather than
//! rewritten, per `openspec/changes/session-provider-adapters/tasks.md` task 5.3: a divergence
//! in rates should show up as a diff against the companion, not get silently re-derived.
//!
//! Rates synced with cc's own rate table (`~/.claude/scripts/lib/cost-rates.sh`) as of
//! 2026-08-05 — prefer that file as the upstream when a rate changes.
//!
//! Not yet wired into any adapter or the render path — task 5 lands the derivation and its
//! ported tests ahead of task 6's vocabulary wiring. `dead_code` is allowed at the module level
//! until that wiring lands; remove this attribute in the same change that adds the first
//! non-test caller.
#![allow(dead_code)]

struct ClaudeRate {
    substr: &'static str,
    input_per_mtok: f64,
    output_per_mtok: f64,
}

// Opus dropped to $5/$25 at 4.5 — the bare "opus-4" fallback keeps the Opus-4.1-era $15/$75 for
// genuinely old ids, while the explicit 4-6/4-7/4-8 rows (longest-match-first) stop the ~3x
// overcount on current Opus. sonnet-5 is on intro pricing ($2/$10) through 2026-08-31, then
// $3/$15 — cc tracks the revert as bead cc-vd8wf; update both tables together.
const CLAUDE_PRICING: &[ClaudeRate] = &[
    ClaudeRate {
        substr: "opus-5",
        input_per_mtok: 5.0,
        output_per_mtok: 25.0,
    },
    ClaudeRate {
        substr: "opus-4-8",
        input_per_mtok: 5.0,
        output_per_mtok: 25.0,
    },
    ClaudeRate {
        substr: "opus-4-7",
        input_per_mtok: 5.0,
        output_per_mtok: 25.0,
    },
    ClaudeRate {
        substr: "opus-4-6",
        input_per_mtok: 5.0,
        output_per_mtok: 25.0,
    },
    ClaudeRate {
        substr: "opus-4",
        input_per_mtok: 15.0,
        output_per_mtok: 75.0,
    },
    ClaudeRate {
        substr: "sonnet-5",
        input_per_mtok: 2.0,
        output_per_mtok: 10.0,
    },
    ClaudeRate {
        substr: "sonnet-4-5",
        input_per_mtok: 3.0,
        output_per_mtok: 15.0,
    },
    ClaudeRate {
        substr: "sonnet-4", // also covers sonnet-4-6 ($3/$15)
        input_per_mtok: 3.0,
        output_per_mtok: 15.0,
    },
    ClaudeRate {
        substr: "haiku-4-5",
        input_per_mtok: 1.0,
        output_per_mtok: 5.0,
    },
    ClaudeRate {
        substr: "fable-5",
        input_per_mtok: 10.0,
        output_per_mtok: 50.0,
    },
];

/// Estimated per-MTok input/output USD rates for `model`, matching pricing-table substrings
/// longest-first so a more specific entry (`"sonnet-4-5"`) wins over a broader one
/// (`"sonnet-4"`). Returns `None` for an unknown model rather than a guessed rate.
pub fn rates(model: &str) -> Option<(f64, f64)> {
    let mut best: Option<(usize, f64, f64)> = None;
    for rate in CLAUDE_PRICING {
        if model.contains(rate.substr)
            && best.is_none_or(|(best_len, _, _)| rate.substr.len() > best_len)
        {
            best = Some((rate.substr.len(), rate.input_per_mtok, rate.output_per_mtok));
        }
    }
    best.map(|(_, input, output)| (input, output))
}

/// Estimated USD cost of one assistant turn given its token counts. Cache reads are billed at
/// 0.1x the input rate, cache writes at 1.25x the input rate — same convention as `rates`.
/// Unknown models return 0 (never a guessed/fabricated number).
pub fn cost(model: &str, input: i64, output: i64, cache_read: i64, cache_write: i64) -> f64 {
    let Some((input_rate, output_rate)) = rates(model) else {
        return 0.0;
    };
    (input as f64 * input_rate
        + output as f64 * output_rate
        + cache_read as f64 * 0.1 * input_rate
        + cache_write as f64 * 1.25 * input_rate)
        / 1_000_000.0
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Ported from `pricing_test.go`'s `TestRates` — logic and cases unchanged, ported rather
    /// than rewritten so a divergence in rates is caught as a diff.
    #[test]
    fn rates_match_the_companions_table() {
        let cases: &[(&str, Option<f64>)] = &[
            ("claude-opus-5", Some(5.0)),
            ("claude-sonnet-5", Some(2.0)),
            ("claude-opus-4-8", Some(5.0)),
            ("claude-opus-4-7", Some(5.0)),
            ("claude-opus-4-6", Some(5.0)),
            ("claude-opus-4-1", Some(15.0)),
            ("claude-sonnet-4-6", Some(3.0)),
            ("claude-sonnet-4-5", Some(3.0)),
            ("claude-sonnet-4-20250514", Some(3.0)),
            ("claude-haiku-4-5", Some(1.0)),
            ("claude-fable-5", Some(10.0)),
            ("some-future-model", None),
            ("", None),
        ];
        for (model, want_in) in cases {
            let got = rates(model).map(|(input, _)| input);
            assert_eq!(got, *want_in, "rates({model:?})");
        }
    }

    #[test]
    fn cost_of_an_unknown_model_is_zero_not_fabricated() {
        assert_eq!(cost("some-future-model", 1000, 1000, 0, 0), 0.0);
    }

    #[test]
    fn cost_applies_the_cache_read_and_write_multipliers() {
        // sonnet-4-5: $3/$15 per MTok.
        let input_only = cost("claude-sonnet-4-5", 1_000_000, 0, 0, 0);
        assert!((input_only - 3.0).abs() < 1e-9);

        let cache_read_only = cost("claude-sonnet-4-5", 0, 0, 1_000_000, 0);
        assert!((cache_read_only - 0.3).abs() < 1e-9, "0.1x input rate");

        let cache_write_only = cost("claude-sonnet-4-5", 0, 0, 0, 1_000_000);
        assert!((cache_write_only - 3.75).abs() < 1e-9, "1.25x input rate");
    }
}
