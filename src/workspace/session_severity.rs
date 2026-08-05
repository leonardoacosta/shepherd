//! Context-window severity is a pure derivation over an already-cached token count — it reads
//! no source, so it gets no adapter. Ported verbatim from
//! `shepherd-plugins/plugins/shepherd-state/pkg/severity` per
//! `openspec/changes/session-provider-adapters/tasks.md` task 5.3, which itself ports
//! `apps/cc-tmux/src/cc_tmux/render.py`'s `_context_color_pair` (tier boundaries + base/pulse
//! color pairs) and `_SES_HANDOFF_THRESHOLD` (the 0.63 ratio) — this file is the fleet's sole
//! home for the context-severity ramp going forward.
//!
//! `classify_tier`/`classify_ratio`/`resolve_color`/`base_color`/`pulse_color` are not yet
//! wired into a token — a colored, pulsing tier badge needs a `ResolvedTokenKind` variant that
//! can carry `Tier`'s base/pulse colors, which is new rendering-surface work beyond wiring an
//! existing `Custom(String)` value (see `openspec/changes/session-provider-adapters/tasks.md`
//! section 6's note). `past_handoff`/`render_occupancy_pct` render as plain text and are wired.
#![allow(dead_code)]

use std::time::{SystemTime, UNIX_EPOCH};

/// One of the six severity levels `_context_color_pair` classifies raw context tokens into,
/// plus the zero-value `Dim` "safe zone, no signal" case — render.py's fail-open default for
/// both `raw_tokens<=100k` and `raw_tokens==None`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Tier {
    /// The safe zone: 100k raw tokens or fewer, or context data unavailable.
    #[default]
    Dim,
    /// Over 100k raw tokens.
    Green,
    /// Over 200k raw tokens.
    Yellow,
    /// Over 300k raw tokens.
    Orange,
    /// Over 500k raw tokens, steady (no pulse).
    Red,
    /// Over 600k raw tokens, alternates RED <-> BRIGHT_RED.
    RedPulsing,
    /// Over 750k raw tokens, alternates DARK_RED <-> RED — a visually darker, more urgent pulse
    /// than `RedPulsing`, not just a repeat of it.
    DarkRedPulsing,
}

// Hex color values ported verbatim from `apps/cc_tmux/usage.py` (lines 41-61) — the same
// constants render.py's `_context_color_pair` imports and uses to build its (base, pulse) tier
// pairs.
const COLOR_DIM: &str = "#454D54";
const COLOR_GREEN: &str = "#00ac3a";
const COLOR_YELLOW: &str = "#FAC760";
const COLOR_ORANGE: &str = "#FF8C00";
const COLOR_RED: &str = "#E61F44";
const COLOR_BRIGHT_RED: &str = "#FF6B6B";
const COLOR_DARK_RED: &str = "#8B0000";

/// Matches render.py's `FRAME_PERIOD_SEC` (1.0 second) — the wall-clock cadence pulsing tiers
/// alternate base/pulse on. Since the period is exactly one second, `now.Unix() % 2` is the
/// whole computation.
const FRAME_PERIOD_SECS: u64 = 1;

/// Ports render.py's `_context_color_pair` boundary logic exactly: exclusive `>` comparisons
/// against 100_000, 200_000, 300_000, 500_000, 600_000, 750_000, checked from the top down.
pub fn classify_tier(raw_tokens: i64) -> Tier {
    if raw_tokens > 750_000 {
        Tier::DarkRedPulsing
    } else if raw_tokens > 600_000 {
        Tier::RedPulsing
    } else if raw_tokens > 500_000 {
        Tier::Red
    } else if raw_tokens > 300_000 {
        Tier::Orange
    } else if raw_tokens > 200_000 {
        Tier::Yellow
    } else if raw_tokens > 100_000 {
        Tier::Green
    } else {
        Tier::Dim
    }
}

/// Grades an occupancy ratio (0..1) against the 0.25 / 0.45 / 0.60 / 0.75 / 0.85 / 0.95
/// boundaries, checked from the top down with the same exclusive `>` comparisons
/// [`classify_tier`] uses. A ratio at or below 0.25 (and any negative input) is `Dim` — reserved
/// for the low band because it is also what an unresolvable ratio grades to.
pub fn classify_ratio(ratio: f64) -> Tier {
    if ratio > 0.95 {
        Tier::DarkRedPulsing
    } else if ratio > 0.85 {
        Tier::RedPulsing
    } else if ratio > 0.75 {
        Tier::Red
    } else if ratio > 0.60 {
        Tier::Orange
    } else if ratio > 0.45 {
        Tier::Yellow
    } else if ratio > 0.25 {
        Tier::Green
    } else {
        Tier::Dim
    }
}

impl Tier {
    /// The steady (non-pulsing) color for the tier — render.py's `_context_color_pair` first
    /// tuple element.
    pub fn base_color(self) -> &'static str {
        match self {
            Tier::Green => COLOR_GREEN,
            Tier::Yellow => COLOR_YELLOW,
            Tier::Orange => COLOR_ORANGE,
            Tier::Red | Tier::RedPulsing => COLOR_RED,
            Tier::DarkRedPulsing => COLOR_DARK_RED,
            Tier::Dim => COLOR_DIM,
        }
    }

    /// The alternate color a pulsing tier flashes against — render.py's `_context_color_pair`
    /// second tuple element. Non-pulsing tiers return `None` — render.py's equivalent
    /// "no animation" case is `pulse is None`.
    pub fn pulse_color(self) -> Option<&'static str> {
        match self {
            Tier::RedPulsing => Some(COLOR_BRIGHT_RED),
            Tier::DarkRedPulsing => Some(COLOR_RED),
            _ => None,
        }
    }
}

/// Mirrors `resolve_context_color`: the single concrete color to render `tier` at wall-clock
/// `now`. Non-pulsing tiers always return their base color. Pulsing tiers alternate base/pulse
/// on `FRAME_PERIOD_SECS` parity.
pub fn resolve_color(tier: Tier, now: SystemTime) -> &'static str {
    let Some(pulse) = tier.pulse_color() else {
        return tier.base_color();
    };
    let unix_secs = now
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    if (unix_secs / FRAME_PERIOD_SECS).is_multiple_of(2) {
        tier.base_color()
    } else {
        pulse
    }
}

/// The ratio (0..1) of a pane's context window at or above which [`past_handoff`] flags a
/// session-handoff warning — ported from render.py's `_SES_HANDOFF_THRESHOLD` (0.63 — 70% of
/// cc's default `CLAUDE_AUTOCOMPACT_PCT_OVERRIDE` of 0.90).
pub const HANDOFF_THRESHOLD: f64 = 0.63;

/// The literal suffix text render.py's `render_session_bar` renders past [`HANDOFF_THRESHOLD`]
/// (tmux color-code wrapping and the leading space stripped — this is the plain text content of
/// that suffix).
pub const HANDOFF_LABEL: &str = "!handoff:/workflow:handoff";

/// Reports whether `raw_tokens`/`window_size` is at or above [`HANDOFF_THRESHOLD`].
/// `window_size <= 0` returns `false` (fail-open, no division by zero).
pub fn past_handoff(raw_tokens: i64, window_size: i64) -> bool {
    if window_size <= 0 {
        return false;
    }
    (raw_tokens as f64) / (window_size as f64) >= HANDOFF_THRESHOLD
}

/// Renders `raw_tokens`/`window_size` as a whole-percent occupancy string, e.g. `"62%"`.
/// `None` when `window_size <= 0` — fail-open, the same convention [`past_handoff`] uses.
pub fn render_occupancy_pct(raw_tokens: i64, window_size: i64) -> Option<String> {
    if window_size <= 0 {
        return None;
    }
    let pct = (raw_tokens as f64 / window_size as f64 * 100.0).round();
    Some(format!("{}%", pct as i64))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    /// Ported from `severity_test.go`'s `TestClassifyTier`.
    #[test]
    fn classify_tier_matches_the_companions_boundaries() {
        let cases: &[(i64, Tier)] = &[
            (99_999, Tier::Dim),
            (100_000, Tier::Dim),
            (100_001, Tier::Green),
            (199_999, Tier::Green),
            (200_000, Tier::Green),
            (200_001, Tier::Yellow),
            (299_999, Tier::Yellow),
            (300_000, Tier::Yellow),
            (300_001, Tier::Orange),
            (499_999, Tier::Orange),
            (500_000, Tier::Orange),
            (500_001, Tier::Red),
            (599_999, Tier::Red),
            (600_000, Tier::Red),
            (600_001, Tier::RedPulsing),
            (749_999, Tier::RedPulsing),
            (750_000, Tier::RedPulsing),
            (750_001, Tier::DarkRedPulsing),
        ];
        for (raw_tokens, want) in cases {
            assert_eq!(
                classify_tier(*raw_tokens),
                *want,
                "classify_tier({raw_tokens})"
            );
        }
    }

    /// Ported from `severity_test.go`'s `TestClassifyRatio`.
    #[test]
    fn classify_ratio_matches_the_companions_boundaries() {
        let cases: &[(f64, Tier)] = &[
            (0.25, Tier::Dim),
            (0.251, Tier::Green),
            (0.45, Tier::Green),
            (0.451, Tier::Yellow),
            (0.60, Tier::Yellow),
            (0.601, Tier::Orange),
            (0.75, Tier::Orange),
            (0.751, Tier::Red),
            (0.85, Tier::Red),
            (0.851, Tier::RedPulsing),
            (0.95, Tier::RedPulsing),
            (0.951, Tier::DarkRedPulsing),
            (0.0, Tier::Dim),
            (1.0, Tier::DarkRedPulsing),
            (1.5, Tier::DarkRedPulsing),
            (-0.5, Tier::Dim),
        ];
        for (ratio, want) in cases {
            assert_eq!(classify_ratio(*ratio), *want, "classify_ratio({ratio})");
        }
    }

    /// Ported from `severity_test.go`'s `TestPastHandoff`.
    #[test]
    fn past_handoff_matches_the_companions_threshold() {
        let cases: &[(i64, i64, bool)] = &[
            (62, 100, false),
            (63, 100, true),
            (70, 100, true),
            (100, 0, false),
            (100, -1, false),
        ];
        for (raw_tokens, window_size, want) in cases {
            assert_eq!(
                past_handoff(*raw_tokens, *window_size),
                *want,
                "past_handoff({raw_tokens}, {window_size})"
            );
        }
    }

    #[test]
    fn resolve_color_alternates_for_a_pulsing_tier() {
        let tier = Tier::RedPulsing;
        let base = tier.base_color();
        let pulse = tier.pulse_color().expect("pulsing tier has a pulse color");

        let t0 = UNIX_EPOCH; // Unix()=0, even parity -> base
        let t1 = UNIX_EPOCH + Duration::from_secs(1); // Unix()=1, odd parity -> pulse

        assert_eq!(resolve_color(tier, t0), base);
        assert_eq!(resolve_color(tier, t1), pulse);
        assert_ne!(resolve_color(tier, t0), resolve_color(tier, t1));
    }

    /// `HANDOFF_LABEL` isn't consumed until task 6.3 wires the render path, but the constant
    /// itself is a verbatim port (task 5.2) worth pinning against a regression now.
    #[test]
    fn handoff_label_matches_the_companions_suffix_text() {
        assert_eq!(HANDOFF_LABEL, "!handoff:/workflow:handoff");
    }

    #[test]
    fn resolve_color_is_always_base_for_a_non_pulsing_tier() {
        let tier = Tier::Green;
        let base = tier.base_color();
        assert_eq!(resolve_color(tier, UNIX_EPOCH), base);
        assert_eq!(
            resolve_color(tier, UNIX_EPOCH + Duration::from_secs(1)),
            base
        );
    }

    #[test]
    fn render_occupancy_pct_rounds_to_a_whole_percent() {
        assert_eq!(render_occupancy_pct(62, 100), Some("62%".to_string()));
        assert_eq!(render_occupancy_pct(1, 3), Some("33%".to_string()));
        assert_eq!(render_occupancy_pct(2, 3), Some("67%".to_string()));
    }

    #[test]
    fn render_occupancy_pct_fails_open_on_a_non_positive_window() {
        assert_eq!(render_occupancy_pct(100, 0), None);
        assert_eq!(render_occupancy_pct(100, -1), None);
    }
}
