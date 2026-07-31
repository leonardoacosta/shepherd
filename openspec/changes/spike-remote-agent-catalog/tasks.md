# Tasks — spike-remote-agent-catalog (design spike)

Base commit: `c000681f`. Drift check: confirm `parse_catalog`
(`src/detect/manifest_update.rs:311-341`) still drops a remote catalog entry
whose `id` doesn't resolve through `parse_agent_label`
(`src/detect/mod.rs:167`) — i.e. the `tracing::warn!("skipping unknown remote
manifest agent")` branch at `src/detect/manifest_update.rs:325` still exists
and unknown ids never reach `parse_remote_manifest_for_agent`. That is the
invariant this spike is actually about; `BUNDLED_MANIFESTS`'s storage shape
(string-keyed since before `c000681f`, `src/detect/manifest.rs:240`) is no
longer the thing to check — a prior version of this file asserted the storage
shape instead of the resolution boundary, and that assertion went stale
first. If `parse_catalog` (or an equivalent gate) now accepts an unresolved
id, STOP: the premise driving this whole spike is gone.

Exemplar: `src/detect/manifest/tests.rs` (remote/local/bundled precedence tests —
the infra that already works) and `src/detect/manifest_update.rs` (remote index
pull). Depends conceptually on `detection-golden-corpus` (fixtures as acceptance
criterion).

## Steps

1. **Coupling inventory — done, see `proposal.md` § Coupling Inventory.** It
   maps every `Agent`-enum-typed site (enum + `Agent::ALL` +
   `SCREEN_MANIFEST_AGENTS`, `parse_agent_label`/`agent_label`/`identify_agent`,
   the manifest-cache/resolution chain in `src/detect/manifest.rs` and
   `manifest_update.rs`, `src/config/sound.rs`, `src/config/sidebar.rs`,
   `src/pane.rs`'s process-identification state machine, `src/cli/spec.rs`)
   against every site already string-keyed (`BUNDLED_MANIFESTS` storage, the
   `Hook*`/`AgentSessionReported` event/authority path, and the entire
   `src/api/schema/agents.rs` wire schema — the latter two were wrongly cited
   as enum-coupled in the pre-drift version of this proposal). Also names the
   conversion seam (`parse_catalog`'s `parse_agent_label` call) a string-id
   path would have to move or delete. Do not re-derive this — read it before
   the decision step below.

2. **Trust controls — done, see `proposal.md` § Trust controls already
   enforced on manifest content.** Regex/gate structural caps
   (`MAX_RULES_PER_MANIFEST` etc., `src/detect/manifest.rs:265-270`) already
   apply to every manifest the binary parses, remote included; `regex::Regex`
   gives a linear-time (ReDoS-free) engine guarantee; the CI script
   (`scripts/agent_detection_manifest_check.py::validate_catalog`) independently
   blocks unknown catalog ids. The uncovered gap is regex *content*
   (over-broad matches, misleading labels), not resource exhaustion — that
   needs human review, not a new automated cap.

3. **Decision — done, see `proposal.md` § What this spike produces.**
   Recommendation: **community-PR path.** Reasoning: the string-id path's
   blast radius, re-measured against the current tree, did not shrink from the
   original estimate — `src/pane.rs`'s process-identification state machine and
   the manifest-cache/resolution chain are still fully enum-native — while the
   community-PR path delivers the same "herdr recognizes my agent" value using
   infrastructure (regex caps, ReDoS-free engine, `detection-golden-corpus`
   fixtures) that is already built and already enforced today.

4. **Community-PR path (chosen): submission template + acceptance checklist.**
   Add `website/agent-detection/SUBMITTING.md` (or equivalent under
   `website/agent-detection/`) covering: manifest TOML shape (`id`, `version`,
   `min_engine_version`, `aliases`, `rules`), the structural caps from
   `proposal.md` § Trust controls (so a submitter self-checks before opening a
   PR), a required golden-fixture test per `detection-golden-corpus`'s
   corpus format, and an explicit maintainer regex-content review step (the
   one gap the automated caps don't close). Add a PR template
   (`.github/PULL_REQUEST_TEMPLATE/agent-detection.md` or a checklist section
   in the submission doc — whichever matches the repo's existing PR-template
   convention) with an acceptance checklist referencing
   `scripts/agent_detection_manifest_check.py` and the golden-fixture test.
   Gate: `just check` (docs-only; the script itself is unchanged, so this
   should pass with no code edits).

   (The other branch, string-id path, is explicitly not chosen — see decision
   above. Had it been chosen, this step would instead produce a scoping doc
   only, per `proposal.md` § Coupling Inventory, and NOT start de-enuming; that
   remains true if a future proposal revisits the decision.)

5. **Verify and close.** done-when: `proposal.md` and this file reflect the
   community-PR recommendation with citations verified against `c000681f`;
   the submission template + acceptance checklist exist under
   `website/agent-detection/` and pass `just check`; proposal
   `spike-remote-agent-catalog` is archived with the decision recorded (record
   via the normal archive step, not a separate note — the decision lives in
   `proposal.md`).

## STOP conditions

- If the maintainer's answer is "curated list is the product," record it and
  archive — a documented "no" is a valid outcome.
- If `parse_catalog` (or an equivalent gate) no longer drops unresolved remote
  agent ids — i.e. the resolution boundary itself has already moved, not just
  the bundled-storage representation — STOP and re-scope again before
  proceeding: the premise driving this spike would be gone a second time.
