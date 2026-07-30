# Design spike: in-TUI / API plugin install without losing the trust gate

Base commit: `1de05dc2` · Route: proposal (design spike) · Effort: M (API) / L (with TUI surface) · Confidence: HIGH on the asymmetry; MED on whether install should move into the TUI · Category: direction

## Why

Resolves advisory finding DIRECTION-02, and folds in the related SECURITY-07
consent-gap finding (audit against `1de05dc2`).

The plugin install surface is asymmetric:

- `src/api/schema.rs:216-238` exposes `plugin.link` (local path), `list`,
  `unlink`, `enable`/`disable`, `action.list`/`invoke`, `log.list`,
  `pane.open|focus|close` — but **no `plugin.install`**. The GitHub fetch →
  preview → trust-confirmation flow lives entirely in the CLI
  (`src/cli/plugin.rs:154-260`, `plugin_install`), which clones to a temp dir,
  prints a preview, prompts, and only then calls `Method::PluginLink`.
- `grep -rn 'marketplace' src --include=*.rs` returns **zero hits** — the
  mouse-first TUI has no marketplace surface at all, while
  `website/src/content/docs/marketplace.mdx` documents a browse-and-install story
  whose install step forces the user to leave herdr for a shell command.
- Consent gap (SECURITY-07): the documented safety control is the CLI's install
  preview + `confirm()` (`src/cli/plugin.rs:199-208`). But `plugin.link` /
  `plugin.enable` are ordinary API methods with **no** confirmation step, so a
  local process (including a hosted agent) can already link+enable a plugin
  pointing at an attacker-authored manifest — which then executes on every server
  start via `[[startup]]` hooks — with no preview and no prompt.

## What this spike produces

A design (and optionally a first API method) that resolves the asymmetry **while
preserving the trust gate as a first-class safety property**, not a boolean param:

- Decide whether install belongs in the TUI at all — `marketplace.mdx`
  deliberately frames the index as "not a reviewed catalog", which is an argument
  for keeping install a deliberate, out-of-band act.
- If it moves to the API/TUI: the trust confirmation must become an explicit
  **server-side / foreground-client approval** (a TUI modal — reuse the
  release-notes/announcement overlay machinery in `src/ui/release_notes.rs`), so a
  compromised agent in a pane cannot install silently. A boolean "consented" param
  is not acceptable.
- Independently and with higher priority: close the existing `plugin.link` /
  `plugin.enable` consent gap for API-originated calls, regardless of the
  install-UI decision.

## Acceptance (spike)

- A design note recording the install-location decision with the security
  reasoning.
- The API-side consent gap for `plugin.link`/`plugin.enable` is either closed in
  this change (route through a foreground confirmation or emit a prominent,
  non-suppressible audit event naming the manifest + declared commands) or filed
  as an immediate follow-up with the mechanism specified.
- If any code lands, `just check` passes.

## Explicitly not in scope

- Building a full in-TUI marketplace browser. The spike decides the model; the
  browse UI is a later proposal.
