## Why

`session-provider-status` proved the seven layers for session and provider facts and shipped
exactly one adapter — `shepherd-state chrome --once --json`, backing a `spend` token. It
deliberately left the remaining sources unimplemented so the layering could be proven before it
was multiplied.

The layering held. Two things it exposed now need fixing together.

**One adapter backs every session field.** A single missing binary blanks all of them at once.
`shepherd-chrome` already requires the opposite — "an all-source failure MUST still serve a valid
empty/stale snapshot", with one source's failure leaving the others current. Shepherd's own copy
of that guarantee does not yet exist, because Shepherd has only one adapter to fail.

**`shepherd-state` does two unrelated jobs in one binary.** One is hook ingest: `hooks.json`
registers it as a Claude Code and codex plugin hook on `SessionStart`, `UserPromptSubmit`,
`PermissionRequest`, `Stop`, `Notification`, and `SessionEnd`. The harness invokes it in-process,
on events a terminal multiplexer cannot observe; `internal/engine` applies those envelopes under
an authority matrix, and `pkg/snapshot` persists the result with atomic writes, per-stripe locks,
and a merge that keeps an older non-empty observation so a newer bare snapshot cannot erase a
resolvable value. The other job is reading: `pkg/chrome`, `pkg/credentials`, `pkg/accounts`,
`pkg/pricing`, `pkg/transcript`, and `pkg/severity` read files, run programs, and derive display
values.

Only the first job requires a companion. The second sits in a separate process because it grew up
next to the first, and it costs Shepherd exactly what `CONTEXT.md` exists to protect: values
Shepherd renders but does not own, cached somewhere it cannot inspect, on a cadence it does not
control.

This change gives every source its own failure envelope and moves the reading half into Shepherd,
leaving the companion as only what a companion alone can be.

## What Changes

- Add one adapter per chrome source, each returning `Option` independently, so one unavailable
  source elides only its own tokens.
- Absorb each remaining reader as a provider adapter with its cached value in `AppState`, its
  scheduling on the runtime `App`, and its parsing pure: credentials, accounts, transcript usage.
- Absorb each derivation as a pure function over an already-cached value: pricing rates and cost,
  severity tiers. Neither reads a source, so neither gets an adapter.
- Add the tokens all of these back to the Agent vocabulary, following `spend`, with demand
  resolved per token so each enables only its own adapter.
- Shrink `shepherd-state` to the hook-ingest path: `hooks.json`, `internal/engine`,
  `pkg/snapshot`, and the socket reporter. Remove the absorbed packages and the subcommands that
  existed only to expose them.
- Keep the snapshot store on disk. It is the handoff between the two halves: the companion writes
  it, Shepherd reads it as a provider source.

## Capabilities

### New Capabilities

- `session-provider-adapters` — per-source adapter independence for session and provider
  telemetry, Shepherd-owned resolution of the facts the companion used to read, and the boundary
  that keeps hook ingest in the companion.

### Modified Capabilities

None in this repository. The companion-side removals and the spec amendment land in
`shepherd-plugins` — see `## Impact`.

## Impact

### Chrome adapter inventory

Recorded from the running system @ 2026-08-05, `shepherd-plugins/plugins/shepherd-state/pkg/chrome`.

| Adapter | Source it reads | Tokens it backs | Mechanism | Failure modes |
| --- | --- | --- | --- | --- |
| `llmtrim` | `llmtrim status --json` | spend, savings, cache-read tokens | spawns a command | binary absent, non-zero exit, timeout, unparseable body |
| `context-floor` | `context-floor --json` | instruction total bytes, floor warning, growth percentage, dollars per 1k turns | spawns a command | binary absent, non-zero exit, timeout, unparseable body |
| `sessions` | the persisted session snapshot store under the state root, one JSON file per session, partitioned by harness | model, context tokens, context window tokens, session count | reads files directly | root absent, entry unreadable, entry unparseable, partially-written entry |
| `local-account-signal` | local credential stores, read-only | quota remaining, reset time | reads files directly | credential absent, unreadable, unparseable, expired |

The mechanism column is per adapter and deliberately mixed, per `session-provider-status`
`## Decisions`. Two adapters spawn a command because their source is a program; two read files
because their source is a file. Neither is the global rule.

### What else moves, and what it becomes

| Package | Responsibility today | Becomes in Shepherd |
| --- | --- | --- |
| `pkg/credentials` | reads the credential, usage, active, and swap files under the state dir | one provider adapter + one pure parser |
| `pkg/accounts` | dedups the credential pool into active-resolved account rows | a domain type over the credentials adapter's output |
| `pkg/transcript` | resolves a session's transcript path and reads token usage from its `.jsonl` | one provider adapter + one pure parser |
| `pkg/pricing` | model rate table; cost from input/output/cache-read/cache-write counts | a pure derivation over already-cached token counts |
| `pkg/severity` | classifies context-window tokens into six tiers and flags the handoff threshold | a pure derivation over an already-cached token count |

### What stays in the companion

| Component | Why it cannot move |
| --- | --- |
| `hooks.json` | the harness invokes it; Shepherd is not in the hook path and cannot be |
| `internal/engine` | applies the authority matrix at ingest time, before Shepherd sees anything |
| `pkg/snapshot` writer | the ingest half's persistence, including the merge that protects an older non-empty observation |
| `internal/shepherd` reporter | reports lifecycle and identity metadata that only the hook path observes |

### The vendor axis is not an adapter axis

`session-provider-status` task 4.1 named the remaining adapters as "llmtrim, anthropic, claude,
codex, pi, openai". That list mixes two axes, and this change resolves it: the seams that can fail
independently are the four **sources** in the chrome table above. `anthropic`, `claude`, `codex`,
`pi`, and `openai` are **harnesses and vendors**, not adapters — they partition the `sessions`
store's contents and the `local-account-signal` credential set. Adding a harness adds rows to an
existing source; it does not add a failure envelope.

Building one adapter per vendor would produce five adapters that fail and recover together against
two underlying sources, which is the opposite of the independence this change exists to deliver.

### Security surface

Shepherd gains the ability to read credential material it does not touch today. This was surfaced
before the decision and accepted; see `## Decisions`. The adapter reads the same files
`pkg/credentials` reads today, read-only, and no credential value is ever rendered — only derived
counts, quotas, and times.

### Files

- `src/workspace/session_status.rs` — one parser and one domain field per source. In practice
  split across sibling files (`session_status.rs`, `session_credentials.rs`,
  `session_pricing.rs`, `session_severity.rs`, `session_transcript.rs`) once the true size of
  seven sources' worth of parsers, domain types, and derivations became clear during
  implementation — one file housing all of it would violate this repo's "no god objects"
  principle. `session_status.rs` stays the shared home for `SessionStatusSnapshot`/
  `SessionStatusRefreshDemand` themselves and the RFC3339/cents helpers the siblings reuse.
- `src/app/session_provider_refresh.rs` — one adapter invocation per source; demand gains one
  disjunct per token.
- `src/config/sidebar.rs`, `src/ui/sidebar/tokens.rs` — the new token names and their resolution.
- `docs/next/website/src/content/docs/` (EN, JA, ZH), `docs/next/CHANGELOG.md`.
- `docs/next/website/src/data/config-reference.json` — the token enumeration is gate-checked by
  `scripts/test_config_reference_check`, which `session-provider-status` discovered the hard way.
- `shepherd-plugins/plugins/shepherd-state/` — package removals and subcommand removals.

### Dependency

This change depends on an amendment in `shepherd-plugins` to `shepherd-chrome`'s requirement
"Chrome values reuse metadata transport":

> The feed SHALL project consumer-visible values through the metadata reporting contract
> established by `shepherd-sidebar-agent-metadata` and MUST NOT add a second chrome transport.

Shepherd owning the store contradicts that sentence: Shepherd now resolves these values by
invoking adapters on its own cadence, which is a second path to the same consumer-visible values
and is not the metadata reporting contract. The amendment must state that a consumer resolving a
value for itself is not a transport the feed added, and scope the prohibition to the feed's own
projection path.

## Preconditions

- premise: the seven layers are implemented and proven for this fact class — verified:
  `session-provider-status` shipped `src/workspace/session_status.rs`,
  `src/app/session_provider_refresh.rs`, `Workspace::session_status()`, and a rendered capture on
  both the available and unavailable paths @ shepherd@89bb29e2
- premise: the shared provider-adapter helper exists and covers the spawn failure classes —
  verified: `run_provider` in `src/app/provider.rs`, with tests for missing binary, non-zero exit,
  timeout, and oversized output @ shepherd@89bb29e2
- premise: one adapter currently backs every session field, which is the defect this change
  corrects — verified: `session_status_for_checkout` invokes exactly one adapter,
  `src/app/session_provider_refresh.rs` @ shepherd@89bb29e2
- premise: `shepherd-chrome` requires per-source failure independence — verified: requirement
  "Chrome sources degrade independently", scenario "One source fails and recovers",
  `shepherd-plugins/openspec/specs/shepherd-chrome/spec.md` @ 2026-08-05
- premise: three of four sources are unavailable in the live system, so the independence this
  change delivers is load-bearing rather than theoretical — verified: `shepherd-state doctor`
  reports `context-floor`, `llmtrim`, and `local-account-signal` unavailable while `sessions` is
  available @ 2026-08-05
- premise: the reading half and the ingest half are separable, and the ingest half is invoked by
  the harness rather than by Shepherd — verified: `hooks.json` registers `SessionStart`,
  `UserPromptSubmit`, `PermissionRequest`, and `Stop` against `bin/shepherd-state-hook`
  @ shepherd-plugins 2026-08-05
- premise: the ingest half enforces an authority matrix Shepherd does not implement — verified:
  `internal/engine/engine.go` header, "Vendor integrations own session identity for every harness;
  companions can only report lifecycle, metadata, and supported releases" @ shepherd-plugins
  2026-08-05
- premise: the snapshot writer carries merge semantics that protect an older non-empty observation
  — verified: `mergeSnapshots` and `shouldKeepOld`, `pkg/snapshot/store.go` @ shepherd-plugins
  2026-08-05
- premise: every package proposed for absorption is a reader or a pure derivation, with no
  ingest-time responsibility — ~~verified: `pkg/credentials/store.go` read/write helpers,
  `pkg/transcript.ReadUsage`, `pkg/pricing.Rates`/`Cost`, `pkg/severity` package doc
  @ shepherd-plugins 2026-08-05~~ **FALSE, found during section-7 implementation (2026-08-05,
  same day) — see `## Decisions` "Section 7 companion-shrink scope" below.** This premise was
  checked by reading each package's own doc comment, not by checking its callers; the caller
  graph tells a different story for four of the six.
- premise: the config reference enumerates agent token names and is gate-checked — verified:
  `scripts/test_config_reference_check` failed on an unlisted `spend` during
  `session-provider-status` @ shepherd@89bb29e2

## Decisions

- `shepherd-state` disposition — chosen: **hollow it out.** Shepherd absorbs every reader and
  derivation the plugin owns — the four chrome sources, `pkg/credentials`, `pkg/accounts`,
  `pkg/pricing`, `pkg/transcript`, `pkg/severity` — as provider adapters and domain types whose
  caches live in `AppState`. The plugin retains only the path it alone can serve: `hooks.json`,
  `internal/engine`'s authority matrix, and `pkg/snapshot`'s writer. Rejected: keeping one
  aggregating adapter, because a single absent binary elides every token at once and cannot
  satisfy `shepherd-chrome`'s per-source independence. Also rejected: full dissolution, because
  the harness invokes the companion in-process on `SessionStart`, `UserPromptSubmit`,
  `PermissionRequest`, `Stop`, `Notification`, and `SessionEnd` — events a terminal multiplexer
  cannot observe — so no amount of Shepherd-side code replaces it. decided-by: leo
- Change granularity — chosen: one change covering both the adapter split and the reader
  absorption. Rejected: splitting the security surface into a dependent proposal, which was
  authored and then merged back on the operator's instruction; decided-by: leo
- Adapter axis — chosen: one adapter per source (`llmtrim`, `context-floor`, `sessions`,
  `local-account-signal`); rejected: one adapter per vendor or harness (`anthropic`, `claude`,
  `codex`, `pi`, `openai`), because those partition two of the sources rather than failing
  independently, and five adapters over two sources would recover together while pretending not
  to; decided-by: default
- Adapter mechanism — chosen: per adapter, spawn or file read according to what the source is;
  rejected: one global rule, carrying forward `session-provider-status`'s decision unchanged;
  decided-by: default
- Snapshot store on disk — chosen: it stays, as the handoff between the two halves. The companion
  writes it; Shepherd reads it as a provider source, exactly as it reads `openspec list --json`
  output. `CONTEXT.md`'s "not on disk" rule governs where Shepherd caches a fact, and that cache
  lives in `AppState`; it does not forbid a source file produced by a process Shepherd cannot be
  inside. decided-by: leo
- Credential reading — chosen: accepted. Shepherd gains the ability to read credential material it
  does not touch today. Surfaced before the decision rather than discovered during it;
  decided-by: leo
- `local-account-signal` polling and refresh — **superseded, same day.** The entry below records
  what actually happened; the struck-through reasoning above it is kept rather than deleted
  because the correction is itself load-bearing (a later reader hitting the same wrong premise
  should find out why it's wrong, not silently see a different answer).

  ~~Chosen: Shepherd fully absorbs `Mutator.PollUsage` and `Mutator.Refresh` — decrypt, poll the
  provider's usage endpoint, refresh an expired token, and persist the refreshed value back to
  `credentials.jsonl`, re-encrypted. Reasoning at decision time: `quota_remaining`/`reset_at`
  looked like they required decrypting the active credential's OAuth blob and calling the
  provider live.~~ **That reasoning was wrong.** Reading `cmd/shepherd-state/main.go`'s
  `defaultReadAccountSignals` (the function that actually backs `local-account-signal` in the
  live system) shows it does neither: it reads `credentials.jsonl` (plaintext columns only —
  `ID`, `CooldownUntil` — never touching `ValueEncrypted`) and `usage.json` (also plaintext),
  and computes `remaining = usage_5h_limit - usage_5h_used` from whatever is already cached
  there. No decrypt, no HTTP call, in the adapter's own path. `Mutator.PollUsage` and
  `Mutator.Refresh` exist in `pkg/credentials` but are called from nowhere else in either
  `shepherd-plugins` repo (`shepherd-state` or `shepherd-token-tab`) — unwired maintenance code
  the package doc marks "(in later tasks)", not something `local-account-signal` depends on
  today.

  **Corrected decision — chosen: read-only, matching task 3.1's original wording.** Shepherd
  ports `defaultReadAccountSignals`'s exact logic: read `credentials.jsonl`'s plaintext columns
  and `usage.json`, compute `remaining`/`cooldown_until`, done. No decryption, no HTTP client, no
  write path, for this source. `NEXUS_ENCRYPTION_KEY` and the OAuth refresh/poll HTTP flow are
  out of scope for `session-provider-adapters` entirely — they would only matter to a future
  proposal that also decides who re-wires `PollUsage`/`Refresh` into a live caller now that
  section 7 removes the companion's copy, which this proposal does not attempt to answer.
  decided-by: leo
- Section 7 companion-shrink scope — **blocked, not decided.** Task 7.1 lists six packages to
  remove (`pkg/chrome`, `pkg/credentials`, `pkg/accounts`, `pkg/pricing`, `pkg/transcript`,
  `pkg/severity`). Checking the actual import graph in `shepherd-plugins` (2026-08-05, during
  section-7 implementation) instead of trusting each package's own doc comment found that five
  of the six are load-bearing somewhere task 7.1/7.2 did not account for:
  - `pkg/credentials`, `pkg/accounts` — imported by `shepherd-token-tab/cmd/shepherd-token-tab/
    main.go`, a **separate live tool** in the same monorepo that this proposal never analyzes.
    Deleting them breaks `shepherd-token-tab`'s build outright.
  - `pkg/transcript`, `pkg/severity` — **also** imported by `shepherd-token-tab/main.go`, same
    problem.
  - `pkg/transcript`, `pkg/pricing` — imported by `internal/adapter/claude/claude.go`
    (`Reader.Read`), which is the **ingest-side** `adapter.Reader` implementation that
    populates `snapshot.SessionSnapshot`'s token/cost/context fields from a live transcript read
    during hook ingest. This directly falsifies the Preconditions premise "every package
    proposed for absorption is a reader or a pure derivation, with no ingest-time
    responsibility" — `pkg/transcript`/`pkg/pricing` have ingest-time responsibility, on the
    exact path task 7.2 says stays untouched.
  - `pkg/chrome` — imported by `internal/engine/engine.go`'s `ProjectChromeSnapshot` (and
    friends), which projects chrome facts into reported metadata tokens. `internal/engine` is
    one of the four things task 7.2 explicitly protects ("untouched... not this change's to
    alter"), so task 7.1 ("remove `pkg/chrome`") and task 7.2 ("leave `internal/engine`
    untouched") **directly contradict each other** as written — `pkg/chrome` cannot be deleted
    without editing the file task 7.2 forbids editing.

  Only `pkg/pricing`'s claim of shepherd-token-tab-independence holds (nothing in
  `shepherd-token-tab` imports it directly) — but it is still blocked transitively through
  `pkg/transcript`, and through the ingest-path entanglement above.

  This is not a scope correction with an obvious safe default the way the two decisions above
  were — every candidate default (delete anyway; delete only the truly-exclusive subset; keep
  everything) has a real, different blast radius on code this proposal did not analyze, in a
  repository with its own release cadence and its own test suite this session did not run. Left
  **unimplemented** rather than guessed. A follow-up pass needs to: (1) decide whether
  `shepherd-token-tab` also migrates off these packages (a second consumer this proposal never
  scoped) or keeps them, (2) decide whether `internal/adapter/claude` and `internal/engine`'s
  chrome-projection code move, get deleted, or stay, and (3) only then re-derive which packages
  (if any) are actually safe to delete. decided-by: leo
- Pricing table ownership — chosen: Shepherd carries the model rate table and it goes stale on
  Shepherd's release cadence rather than the companion's. Rejected: leaving pricing in the
  companion, which would split one derivation across two processes for one table; decided-by: leo

## Done Means

- An operator can configure any one session token and see only its own adapter run.
- With one source unavailable and another healthy, the unavailable source's tokens elide while the
  healthy source's tokens render current values.
- With every source unavailable, every session token elides, no error dialog appears, and every
  non-session token in the same row renders unchanged.
- An operator sees account, usage, and context-severity values rendered from facts Shepherd
  resolved itself, with no `shepherd-state` subcommand invoked for any of them.
- With the companion's binary absent entirely, every absorbed token still resolves, because
  Shepherd reads the same sources directly.
- `shepherd-state` retains only the hook-ingest path; its removed subcommands exit with a usage
  error naming Shepherd as the owner.
- Hook ingest continues to work unchanged: a session start still persists a snapshot and reports
  lifecycle metadata.

## Testing

- Per-adapter demand tests asserting each token enables only its own adapter.
- An independence test asserting one failing adapter and one succeeding adapter resolve to an
  absent token and a rendered token respectively, in the same snapshot.
- Per-failure-class degradation tests for each adapter, matching its mechanism: the spawn classes
  for the command adapters, the read classes for the file adapters, including a partially-malformed
  file source.
- Parser tests over captured output for each source — chrome, credentials, accounts, transcript
  usage — plus malformed-body and empty cases.
- A test asserting no credential secret reaches a rendered token, a log, or an error message.
- A test asserting a credential source is opened read-only and is byte-identical afterwards.
- Derivation tests for pricing cost and severity tiers, ported from the companion's existing tests
  rather than rewritten, so a divergence in rates or tiers is caught as a diff.
- Render tests asserting each new token elides independently and that a fully unresolved row is
  omitted.
- Config tests asserting each new token name parses, round-trips, and that an unknown name is a
  configuration error.
- An end-to-end check that hook ingest still persists a snapshot after the companion shrinks.
- `openspec validate session-provider-adapters --strict --no-interactive` passes.
- `just check` passes.
