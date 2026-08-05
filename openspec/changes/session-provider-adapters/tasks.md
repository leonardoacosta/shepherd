Read `CONTEXT.md` and `openspec/changes/archive/*-session-provider-status/` first. This change
multiplies a layering that is already proven; it does not redesign it. Section 2 gives every
chrome source its own failure envelope; sections 3-6 move the companion's readers into Shepherd;
section 7 shrinks the companion to what only it can do.

## 1. Settle the `shepherd-state` disposition

- [x] 1.1 Disposition settled: hollow out `shepherd-state`. Recorded in `proposal.md`
      `## Decisions` as `decided-by: leo`, with the rejected alternatives and the reason full
      dissolution is impossible (the harness invokes the companion in-process on events a terminal
      multiplexer cannot observe).
- [x] 1.2 Scope settled: one change covering both the adapter split and the reader absorption,
      including the credential surface. Recorded as `decided-by: leo`.

## 2. Split the single adapter into per-source adapters

- [x] 2.1 Replace `session_status_for_checkout`'s single adapter invocation with one invocation
      per source, each gated on its own demand disjunct. All four sources (`llmtrim`,
      `context-floor`, `sessions`, `local-account-signal`) as of section 3 landing.
- [x] 2.2 Add one parser per chrome source in `src/workspace/session_status.rs`, each pure and
      testable against captured output. Reuse `run_provider` for the command sources; do not copy
      it. `local-account-signal`'s parser lives in `src/workspace/session_credentials.rs`
      alongside the other per-source files (`session_pricing.rs`, `session_severity.rs`,
      `session_transcript.rs`) rather than all four crowding one file — a deliberate deviation
      from this task's literal file name, in the spirit of the repo's "no god objects" rule; see
      `proposal.md`'s Files list note.
- [x] 2.3 Add one optional field per source to the snapshot so each elides independently.
- [x] 2.4 Add the independence test: one failing adapter and one succeeding adapter in the same
      snapshot resolve to an absent token and a present token respectively.
- [x] 2.5 Add per-failure-class degradation tests matching each adapter's mechanism — the spawn
      classes for command sources, the read classes for file sources, including a
      partially-malformed file source.
- [x] 2.6 Run `cargo nextest run session` and paste the passing output. 3342 tests run: 3342
      passed, 0 failed (2026-08-05, full suite; the `session` filter alone also passes clean).

## 3. Absorb the credential and account readers

Per `proposal.md` `## Decisions` "`local-account-signal` polling and refresh" (corrected
2026-08-05): this section is entirely read-only, matching task 3.1's original wording. Shepherd
ports `cmd/shepherd-state/main.go`'s `defaultReadAccountSignals` exactly — read
`credentials.jsonl`'s plaintext columns (never `ValueEncrypted`) and `usage.json`, compute
`remaining = usage_5h_limit - usage_5h_used` and `cooldown_until`. No decryption, no HTTP client,
no write path. `crypto.go`/`NEXUS_ENCRYPTION_KEY`/`Mutator.PollUsage`/`Mutator.Refresh` are out
of scope for this proposal.

- [x] 3.1 Port `pkg/credentials`' file layout and parsing (`credentials.jsonl`'s plaintext
      columns, `usage.json`) into a pure parser in `src/workspace/session_status.rs`. Read-only:
      no write path moves, and no decryption — `local-account-signal` never reads
      `ValueEncrypted`. Landed in `src/workspace/session_credentials.rs` (see task 2.2's file-
      location note); the I/O adapter is in `src/app/session_provider_refresh.rs`.
- [x] 3.2 Port `defaultReadAccountSignals`' derivation (`remaining = usage_5h_limit -
      usage_5h_used`, clamped at 0; `cooldown_until` passed through) as a domain type over that
      parser's output. Its input is the credential parser's result, not a second read.
- [x] 3.3 Add a test asserting no credential secret reaches a rendered token, a log, or an error
      message — only derived counts and times. (`ValueEncrypted` is never even parsed into a
      field this package touches, so this is also a structural guarantee, not just a render-path
      one.)
- [x] 3.4 Add degradation tests: `credentials.jsonl` absent/unreadable/unparseable,
      `usage.json` absent/unreadable/unparseable, a `usage.json` entry that is stale (past its
      TTL) or missing `usage_5h_used`/`usage_5h_limit` (both must elide that credential's signal,
      not the whole source).
- [x] 3.5 Add the adapter, its demand disjunct, and its snapshot field. Unlike `transcript`
      (task 4.2), this source is system-wide rather than per-session, so it fits the existing
      checkout-keyed refresh loop directly — fully wired, not deferred to task 6.
- [x] 3.6 Run `cargo nextest run credential` and paste the passing output. 21 tests run: 21
      passed, 0 failed (2026-08-05).

## 4. Absorb the transcript reader

- [x] 4.1 Port `pkg/transcript`'s session-path resolution and `.jsonl` usage parsing into pure
      functions, with the path-munging rules preserved exactly.
      `src/workspace/session_transcript.rs` (`munge_claude_path`, `parse_transcript_usage`,
      pure); `resolve_transcript_session_path` (I/O: stat + glob fallback) in
      `src/app/session_provider_refresh.rs`.
- [ ] 4.2 Add the adapter, its demand disjunct, and its snapshot field. The adapter function
      (`transcript_usage_for_session`) is landed but not yet called: transcript usage is keyed
      by (checkout, session id), not by checkout alone like every other source in
      `SessionStatusRefreshDemand`/`SessionStatusSnapshot` — wiring its demand disjunct and
      snapshot field needs the refresh-loop's per-session-item restructuring, which task 6 does
      alongside the vocabulary wiring rather than churning `SessionStatusRefreshItem`'s shape
      twice. `#[allow(dead_code)]` marks the one still-unreferenced function.
- [x] 4.3 Add parser tests over captured transcript bodies, including a truncated final line, which
      a live transcript exhibits while a session is running.
- [x] 4.4 Run `cargo nextest run transcript` and paste the passing output. 10 tests run: 10
      passed, 0 failed (2026-08-05) — 9 new `session_transcript` tests plus one pre-existing
      unrelated match.

## 5. Absorb the pure derivations

- [x] 5.1 Port `pkg/pricing`'s rate table and cost computation as a pure derivation over
      already-cached token counts. It reads no source and gets no adapter.
      `src/workspace/session_pricing.rs`. Not yet wired into a token (task 6.3).
- [x] 5.2 Port `pkg/severity`'s six-tier classification and handoff threshold the same way.
      `src/workspace/session_severity.rs`. Not yet wired into a token (task 6.3).
- [x] 5.3 Port the companion's existing tests for both rather than writing new ones, so a
      divergence in tiers or rates is caught as a diff.
- [x] 5.4 Run `cargo nextest run pricing severity` and paste the passing output. 9 tests run: 9
      passed, 0 failed (2026-08-05).

## 6. Expose the vocabulary

Landed for the four chrome sources' 12 facts (`savings`, `cache_read_tokens`,
`instruction_total_bytes`, `instruction_floor_warn`, `instruction_growth_pct`,
`instruction_dollars_per_1k_turns`, `model`, `context_tokens`, `context_window_tokens`,
`session_count`, `quota_remaining`, `reset_at`) — the ones the proposal's own "Chrome adapter
inventory" table enumerates. **Not landed**: tokens for transcript usage (still blocked on the
per-session refresh-loop restructuring noted at task 4.2) or for the pricing/severity pure
derivations (`session_pricing.rs`/`session_severity.rs` exist and are tested but have no token
name or render path yet — a render token for e.g. context-window occupancy would need a new
`ResolvedTokenKind` capable of carrying `severity::Tier`'s pulsing base/pulse colors, which is
new rendering-surface work beyond wiring an existing `Custom(String)` value). Follow-up scope,
not silently dropped.

- [ ] 6.1 Add the token names for every source and derivation to `AgentSidebarToken`, following
      `spend`. Done for the four chrome sources' 12 facts; transcript and pricing/severity
      deferred per the note above.
- [ ] 6.2 Extend demand resolution so each token enables only its own adapter, and add a demand
      test per token. Done for the 12 landed tokens (one `each_token_enables_only_its_own_adapter`
      case per token).
- [ ] 6.3 Resolve each token in `src/ui/sidebar/tokens.rs` through the accessor. Render is a pure
      read; no adapter runs on the render path. Done for the 12 landed tokens.
- [x] 6.4 Add config tests: each name parses, round-trips, and an unknown name is a configuration
      error rather than a silent elision. `session_provider_tokens_parse_and_round_trip` in
      `src/config/sidebar.rs`; the existing `spend_token_parses_round_trips_and_is_rejected_when_misspelled`
      already covers the unknown-name-is-an-error case for this vocabulary.
- [x] 6.5 Add render tests asserting independent elision and omission of a fully unresolved row.
      One test per source (`llmtrims_other_tokens_...`, `context_floor_tokens_...`,
      `sessions_tokens_...`, `local_account_signal_tokens_...`) in `src/ui/sidebar/tokens.rs`.
- [x] 6.6 Update `docs/next/website/src/data/config-reference.json` with every new token name.
      `scripts/test_config_reference_check` gates this and `session-provider-status` was caught by
      it. Note: the reference's "values" arrays use the checker's own naive
      lowercase-no-separator transform of the Rust variant name (e.g. `cachereadtokens`), not the
      real snake_case TOML value (`cache_read_tokens`) — a pre-existing quirk of this checker
      (every existing entry, e.g. `stateicon` for the real `state_icon`, has the same shape), not
      something this change introduced.
- [x] 6.7 Run `cargo nextest run` and paste the passing output. 3348 tests run: 3348 passed, 0
      failed (2026-08-05). `just check` also passes in full, including the Windows clippy target
      and `scripts.test_config_reference_check`.

## 7. Shrink the companion

**BLOCKED — do not implement from the tasks below as written.** Per `proposal.md` `## Decisions`
"Section 7 companion-shrink scope" (2026-08-05): checking the actual `shepherd-plugins` import
graph instead of trusting each package's doc comment found `pkg/credentials`, `pkg/accounts`,
`pkg/transcript`, and `pkg/severity` imported by the separate `shepherd-token-tab` tool;
`pkg/transcript` and `pkg/pricing` imported by `internal/adapter/claude.Reader.Read`, part of the
ingest path task 7.2 protects; and `pkg/chrome` imported by `internal/engine.go`, one of the four
files task 7.2 explicitly says stays untouched — task 7.1 and 7.2 contradict each other for that
package. Only `pkg/pricing` has no direct `shepherd-token-tab` import, and even it is blocked
transitively through `pkg/transcript`. A follow-up pass must resolve `shepherd-token-tab`'s
status and the ingest/engine entanglement before any deletion is safe; see the Decisions entry
for the three open questions.

- [ ] 7.1 In `shepherd-plugins`, remove `pkg/chrome`, `pkg/credentials`, `pkg/accounts`,
      `pkg/pricing`, `pkg/transcript`, and `pkg/severity`, plus the subcommands that existed only
      to expose them. **Not executed — blocked, see note above.**
- [ ] 7.2 Leave `hooks.json`, `internal/engine`, `pkg/snapshot`, and `internal/shepherd` untouched.
      The authority matrix and the snapshot merge semantics are not this change's to alter.
- [ ] 7.3 Make each removed subcommand exit with a usage error naming Shepherd as the owner of that
      fact, rather than vanishing silently. **Not executed — blocked, see note above.**
- [ ] 7.4 Run the companion's own test suite and paste the passing output. **Not run — nothing to
      verify until 7.1 is unblocked.**

## 8. Land the dependency amendment

Independent of section 7's blocker in principle (the metadata-transport requirement amendment
doesn't require deleting any package), but left undone alongside it this pass — landing a spec
amendment in a repo whose own companion-shrink plan turned out to need rework is premature; the
amendment's wording may need to change too once section 7's real scope is known.

- [ ] 8.1 In `shepherd-plugins`, amend `shepherd-chrome`'s requirement "Chrome values reuse
      metadata transport" so its prohibition scopes to the feed's own projection path and does not
      forbid a consumer resolving a value for itself. Cite this proposal. **Not executed.**
- [ ] 8.2 Confirm the amended requirement and this change agree: the feed still MUST NOT add a
      second transport of its own, and Shepherd pulling is not the feed adding one. **Not
      executed.**

## 9. Verify the boundary holds

- [ ] 9.1 With the companion binary removed from `PATH` entirely, capture a rendered row showing
      every absorbed token still resolving.
- [ ] 9.2 Capture a rendered row with one source available and another unavailable, proving
      independent elision in production rather than only in tests.
- [ ] 9.3 Trigger a real harness session-start hook and confirm a snapshot is still persisted and
      lifecycle metadata still reported — the ingest half must be unaffected.
- [ ] 9.4 Run `just check` and paste the passing output.

## 10. Document

- [ ] 10.1 Document every new token in `docs/next/website/src/content/docs/` (EN, JA, ZH together),
      stating that Shepherd resolves it itself and that an unavailable source elides only its own
      tokens.
- [ ] 10.2 Add a `docs/next/CHANGELOG.md` entry naming the per-source independence, the facts
      Shepherd now resolves itself, and that the companion remains required for harness hook
      ingest.
- [ ] 10.3 Update the companion's own README and plugin description so it no longer claims the
      responsibilities that moved.
