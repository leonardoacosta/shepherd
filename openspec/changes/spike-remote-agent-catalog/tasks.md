# Tasks — spike-remote-agent-catalog (design spike)

Base commit: `1de05dc2`. Drift check: confirm `BUNDLED_MANIFESTS`
(`src/detect/manifest.rs:239`) is still `Agent`-enum-keyed and
`remote_manifest_path(agent: Agent)` (`src/detect/manifest_update.rs:379`) is
still enum-typed. If a string-id path already exists, STOP.

Exemplar: `src/detect/manifest/tests.rs` (remote/local/bundled precedence tests —
the infra that already works) and `src/detect/manifest_update.rs` (remote index
pull). Depends conceptually on `detection-golden-corpus` (fixtures as acceptance
criterion).

## Steps

1. **Enumerate the coupling.** List every `Agent`-typed site (enum,
   `identify_agent`, `parse_agent_label`, `src/config/sound.rs`,
   `src/api/schema/agents.rs`, persistence) that a string-id agent would touch.
   This is the blast-radius evidence for the decision.

2. **Write the decision note** (`docs/next/` or `.local/prd/`): community-PR path
   vs. string-id path, with trust controls for remotely-added manifests (regex
   input → keep the `agent_detection_manifest_check.py` caps). Recommend one.

3. **If community-PR path:** add a `website/agent-detection/` submission template
   + acceptance checklist referencing the golden-fixture test from
   `detection-golden-corpus`. Gate: `just check` (mostly docs).

4. **If string-id path:** produce the scoping doc only (every touched site + trust
   plan). Do NOT start de-enuming here — that is a separate build proposal.

5. **Verify and close.** done-when: proposal `spike-remote-agent-catalog` archived
   with the decision recorded.

## STOP conditions

- If the maintainer's answer is "curated list is the product," record it and
  archive — a documented "no" is a valid outcome.
