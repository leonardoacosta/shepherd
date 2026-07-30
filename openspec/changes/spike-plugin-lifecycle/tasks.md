# Tasks — spike-plugin-lifecycle (design spike)

Base commit: `1de05dc2`. Drift check: confirm there is still no `plugin upgrade`/
`outdated` in `src/cli/spec.rs` and that `src/persist/plugin_registry.rs` records
`installed_unix_ms` but no upstream ref. If a lifecycle path already exists, STOP.

Exemplar to imitate: `src/detect/manifest_update.rs` (scheduled remote-index pull,
version comparison, cache fallback) — the proven in-repo pattern for the same
problem class. Registry schema: `src/persist/plugin_registry.rs`.

## Steps

1. **Record provenance at install.** Add the resolved upstream ref/commit and
   declared version to the plugin registry record at install time
   (`src/cli/plugin.rs` install path writes the registry). Keep the change
   migration-safe (old records without the field still load).
   - Gate: `just test-one plugin` green; registry round-trips old + new records.

2. **Implement `plugin outdated` (read-only).** Compare each installed plugin's
   recorded ref/version against the marketplace index (`plugins/index.json` from
   `workers/plugin-marketplace`) or the upstream ref, and report drift. No fetch of
   plugin code, no execution.
   - Gate: `just test-one plugin` green; manual run lists an outdated plugin.

3. **Write the auto-update decision note** under `docs/next/` or `.local/prd/`:
   conservative (outdated-only) vs. opt-in auto-update, with the unreviewed-index
   security tradeoff spelled out. Recommend one.

4. **Verify and close.** `just check` passes. done-when: proposal
   `spike-plugin-lifecycle` archived with the decision recorded.

## STOP conditions

- If recording upstream ref requires network access at install time that the
  install flow deliberately avoids, report — provenance may need to come from the
  already-fetched clone metadata instead.
