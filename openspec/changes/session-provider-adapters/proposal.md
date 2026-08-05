## Why

`session-provider-status` proved the seven layers for session and provider facts and shipped
exactly one adapter — `shepherd-state chrome --once --json`, backing a `spend` token. It
deliberately left the remaining sources unimplemented so the layering could be proven before it
was multiplied.

The layering held. What it also exposed is a defect in the shape it proved: one adapter now backs
every session field, so a single missing binary blanks all of them at once. `shepherd-chrome`
already requires the opposite — "an all-source failure MUST still serve a valid empty/stale
snapshot", with one source's failure leaving the others current. Shepherd's own copy of that
guarantee does not yet exist, because Shepherd has only one adapter to fail.

This change adds the remaining adapters, each with its own failure envelope, and settles what
`shepherd-state` becomes once Shepherd resolves these facts itself.

## What Changes

- Add one adapter per source, each returning `Option` independently, so one unavailable source
  elides only its own tokens.
- Add the tokens each adapter backs to the Agent vocabulary, following `spend`.
- Extend demand resolution so each token enables only its own adapter, as `spend` already does.
- Record the disposition of `shepherd-state`: whether it remains one aggregating adapter Shepherd
  invokes, or its per-source logic migrates into Shepherd adapter by adapter and the plugin
  dissolves.

## Capabilities

### New Capabilities

- `session-provider-adapters` — per-source adapter independence for session and provider
  telemetry: one failure envelope per source, per-token demand, and the vocabulary each source
  backs.

### Modified Capabilities

None in this repository. The amendment this change requires lands in `shepherd-plugins` — see
`## Impact`.

## Impact

### Adapter inventory

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

### The task list's two axes

`session-provider-status` task 4.1 named the remaining adapters as "llmtrim, anthropic, claude,
codex, pi, openai". That list mixes two axes, and this change resolves it: the seams that can
fail independently are the four **sources** in the table above. `anthropic`, `claude`, `codex`,
`pi`, and `openai` are **harnesses and vendors**, not adapters — they partition the `sessions`
store's contents and the `local-account-signal` credential set. Adding a harness adds rows to an
existing source; it does not add a failure envelope.

Building one adapter per vendor would produce five adapters that fail and recover together
against two underlying sources, which is the opposite of the independence this change exists to
deliver.

### Files

- `src/workspace/session_status.rs` — one parser and one domain field per source.
- `src/app/session_provider_refresh.rs` — one adapter invocation per source; demand gains one
  disjunct per token.
- `src/config/sidebar.rs` — the new token names.
- `src/ui/sidebar/tokens.rs` — resolution for each new token.
- `docs/next/website/src/content/docs/` (EN, JA, ZH) and `docs/next/CHANGELOG.md`.
- `docs/next/website/src/data/config-reference.json` — the token enumeration is gate-checked by
  `scripts/test_config_reference_check`, which `session-provider-status` discovered the hard way.

### Dependency

This change depends on an amendment in `shepherd-plugins` to `shepherd-chrome`'s requirement
"Chrome values reuse metadata transport":

> The feed SHALL project consumer-visible values through the metadata reporting contract
> established by `shepherd-sidebar-agent-metadata` and MUST NOT add a second chrome transport.

Shepherd owning the store contradicts that sentence: Shepherd now resolves these values by
invoking adapters on its own cadence, which is a second path to the same consumer-visible values
and is not the metadata reporting contract. The amendment must state that a consumer resolving a
value for itself is not a transport the feed added, and scope the prohibition to the feed's own
projection path. That amendment is a dependency of **this** proposal, not of
`session-provider-status`, which shipped a single adapter without contradicting the sentence's
intent.

## Preconditions

- premise: the seven layers are implemented and proven for this fact class — verified:
  `session-provider-status` shipped `src/workspace/session_status.rs`,
  `src/app/session_provider_refresh.rs`, `Workspace::session_status()`, and a rendered capture on
  both the available and unavailable paths @ shepherd@b5fc4abe
- premise: the shared provider-adapter helper exists and covers the spawn failure classes —
  verified: `run_provider` in `src/app/provider.rs`, with tests for missing binary, non-zero
  exit, timeout, and oversized output @ shepherd@b5fc4abe
- premise: one adapter currently backs every session field, which is the defect this change
  corrects — verified: `session_status_for_checkout` invokes exactly one adapter,
  `src/app/session_provider_refresh.rs` @ shepherd@b5fc4abe
- premise: `shepherd-chrome` requires per-source failure independence — verified: requirement
  "Chrome sources degrade independently", scenario "One source fails and recovers",
  `shepherd-plugins/openspec/specs/shepherd-chrome/spec.md` @ 2026-08-05
- premise: three of four sources are unavailable in the live system, so the independence this
  change delivers is load-bearing rather than theoretical — verified: `shepherd-state doctor`
  reports `context-floor`, `llmtrim`, and `local-account-signal` unavailable while `sessions` is
  available @ 2026-08-05
- premise: the config reference enumerates agent token names and is gate-checked — verified:
  `scripts/test_config_reference_check` failed on an unlisted `spend` during
  `session-provider-status` @ shepherd@b5fc4abe

## Decisions

- Adapter axis — chosen: one adapter per source (`llmtrim`, `context-floor`, `sessions`,
  `local-account-signal`); rejected: one adapter per vendor or harness (`anthropic`, `claude`,
  `codex`, `pi`, `openai`), because those partition two of the sources rather than failing
  independently, and five adapters over two sources would recover together while pretending not
  to; decided-by: default
- Adapter mechanism — chosen: per adapter, spawn or file read according to what the source is;
  rejected: one global rule, carrying forward `session-provider-status`'s decision unchanged;
  decided-by: default
- `shepherd-state` disposition — **not decided here.** `session-provider-status` task 4.2 requires
  this proposal to record the disposition, and the finding that informs it is above: a single
  aggregating adapter cannot satisfy per-source failure independence, because one absent binary
  elides every token at once. Both options remain open and the choice belongs to the operator:
  - **A — keep it as one aggregating adapter.** Cheapest; already working. Cannot deliver
    per-source independence.
  - **B — dissolve it.** Shepherd gains one adapter per source and the plugin's aggregation logic
    migrates in. Delivers independence; the largest change, and it moves credential-reading code
    into Shepherd.
  - **C — split the difference.** Shepherd invokes `shepherd-state` once per source rather than
    once in total, keeping the plugin as the reader while restoring independent envelopes.
  decided-by: leo

## Done Means

- An operator can configure any one session token and see only its own adapter run.
- With one source unavailable and another healthy, the unavailable source's tokens elide while
  the healthy source's tokens render current values.
- With every source unavailable, every session token elides, no error dialog appears, and every
  non-session token in the same row renders unchanged.
- An operator who configures no session token observes no adapter run at all.
- The recorded `shepherd-state` disposition names one option and the reason it was chosen.

## Testing

- Per-adapter demand tests asserting each token enables only its own adapter.
- An independence test asserting one failing adapter and one succeeding adapter resolve to an
  absent token and a rendered token respectively, in the same snapshot.
- Per-failure-class degradation tests for each adapter, matching its mechanism: the spawn classes
  for the command adapters, the read classes for the file adapters.
- Parser tests over captured output for each source, plus malformed-body and empty cases.
- Render tests asserting each new token elides independently and that a fully unresolved row is
  omitted.
- Config tests asserting each new token name parses, round-trips, and that an unknown name is a
  configuration error.
- `openspec validate session-provider-adapters --strict --no-interactive` passes.
- `just check` passes.
