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

- [ ] 2.1 Replace `session_status_for_checkout`'s single adapter invocation with one invocation
      per source, each gated on its own demand disjunct.
- [ ] 2.2 Add one parser per chrome source in `src/workspace/session_status.rs`, each pure and
      testable against captured output. Reuse `run_provider` for the command sources; do not copy
      it.
- [ ] 2.3 Add one optional field per source to the snapshot so each elides independently.
- [ ] 2.4 Add the independence test: one failing adapter and one succeeding adapter in the same
      snapshot resolve to an absent token and a present token respectively.
- [ ] 2.5 Add per-failure-class degradation tests matching each adapter's mechanism — the spawn
      classes for command sources, the read classes for file sources, including a
      partially-malformed file source.
- [ ] 2.6 Run `cargo nextest run session` and paste the passing output.

## 3. Absorb the credential and account readers

Per `proposal.md` `## Decisions` "`local-account-signal` polling and refresh": tasks 3.1-3.4
below are read-only (`credentials.jsonl`/`usage.json`/`active` parsing and the accounts domain
type). Tasks 3.5-3.9 are the exception that decision carves out — Shepherd absorbing
`Mutator.PollUsage` and `Mutator.Refresh` is a write path, not a reader.

- [ ] 3.1 Port `pkg/credentials`' file layout and parsing (`credentials.jsonl`, `usage.json`,
      `active`) into a pure parser in `src/workspace/session_status.rs`, plus `Decrypt`'s
      AES-256-GCM envelope (key from `NEXUS_ENCRYPTION_KEY`, ported from `crypto.go` unchanged).
- [ ] 3.2 Port `pkg/accounts`' dedup and active-resolution into a domain type over that parser's
      output. Its input is the credential parser's result, not a second read.
- [ ] 3.3 Add a test asserting no credential secret reaches a rendered token, a log, or an error
      message — only derived counts, quotas, and times.
- [ ] 3.4 Add degradation tests: source absent, unreadable, unparseable, partially malformed, and
      a `NEXUS_ENCRYPTION_KEY` that is unset, non-hex, or the wrong size.
- [ ] 3.5 Port `Mutator.PollUsage`'s live usage-endpoint call (`probeUsage`, `usageURL`, response
      parsing) as a provider adapter — same `run_provider`-adjacent shape as the spawn-based
      chrome sources, but an HTTP call instead of a subprocess.
- [ ] 3.6 Port `Mutator.Refresh`'s OAuth token-endpoint call (`callRefreshGrant`, `needsRefresh`)
      so an expired access token is refreshed before the usage poll, not elided.
- [ ] 3.7 Port the write-back: a successful refresh re-encrypts the new token and persists it to
      `credentials.jsonl` via the same atomic-write contract as `writeFileAtomic` (temp file in
      the same dir, fsync, rename). Do not touch `usage.json`'s or `active`'s write paths — those
      remain the companion's (`pkg/snapshot`'s neighbors, not `pkg/credentials`').
- [ ] 3.8 Add the adapter, its demand disjunct, and its snapshot field.
- [ ] 3.9 Add tests for the poll/refresh/write-back path: a refresh call that succeeds and
      persists, one that fails and leaves the prior token intact, and a test asserting the
      encrypted envelope round-trips (encrypt-then-decrypt, not just decrypt-a-fixture).
- [ ] 3.10 Run `cargo nextest run credential` and paste the passing output.

## 4. Absorb the transcript reader

- [ ] 4.1 Port `pkg/transcript`'s session-path resolution and `.jsonl` usage parsing into pure
      functions, with the path-munging rules preserved exactly.
- [ ] 4.2 Add the adapter, its demand disjunct, and its snapshot field.
- [ ] 4.3 Add parser tests over captured transcript bodies, including a truncated final line, which
      a live transcript exhibits while a session is running.
- [ ] 4.4 Run `cargo nextest run transcript` and paste the passing output.

## 5. Absorb the pure derivations

- [ ] 5.1 Port `pkg/pricing`'s rate table and cost computation as a pure derivation over
      already-cached token counts. It reads no source and gets no adapter.
- [ ] 5.2 Port `pkg/severity`'s six-tier classification and handoff threshold the same way.
- [ ] 5.3 Port the companion's existing tests for both rather than writing new ones, so a
      divergence in tiers or rates is caught as a diff.
- [ ] 5.4 Run `cargo nextest run pricing severity` and paste the passing output.

## 6. Expose the vocabulary

- [ ] 6.1 Add the token names for every source and derivation to `AgentSidebarToken`, following
      `spend`.
- [ ] 6.2 Extend demand resolution so each token enables only its own adapter, and add a demand
      test per token.
- [ ] 6.3 Resolve each token in `src/ui/sidebar/tokens.rs` through the accessor. Render is a pure
      read; no adapter runs on the render path.
- [ ] 6.4 Add config tests: each name parses, round-trips, and an unknown name is a configuration
      error rather than a silent elision.
- [ ] 6.5 Add render tests asserting independent elision and omission of a fully unresolved row.
- [ ] 6.6 Update `docs/next/website/src/data/config-reference.json` with every new token name.
      `scripts/test_config_reference_check` gates this and `session-provider-status` was caught by
      it.
- [ ] 6.7 Run `cargo nextest run` and paste the passing output.

## 7. Shrink the companion

- [ ] 7.1 In `shepherd-plugins`, remove `pkg/chrome`, `pkg/credentials`, `pkg/accounts`,
      `pkg/pricing`, `pkg/transcript`, and `pkg/severity`, plus the subcommands that existed only
      to expose them.
- [ ] 7.2 Leave `hooks.json`, `internal/engine`, `pkg/snapshot`, and `internal/shepherd` untouched.
      The authority matrix and the snapshot merge semantics are not this change's to alter.
- [ ] 7.3 Make each removed subcommand exit with a usage error naming Shepherd as the owner of that
      fact, rather than vanishing silently.
- [ ] 7.4 Run the companion's own test suite and paste the passing output.

## 8. Land the dependency amendment

- [ ] 8.1 In `shepherd-plugins`, amend `shepherd-chrome`'s requirement "Chrome values reuse
      metadata transport" so its prohibition scopes to the feed's own projection path and does not
      forbid a consumer resolving a value for itself. Cite this proposal.
- [ ] 8.2 Confirm the amended requirement and this change agree: the feed still MUST NOT add a
      second transport of its own, and Shepherd pulling is not the feed adding one.

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
