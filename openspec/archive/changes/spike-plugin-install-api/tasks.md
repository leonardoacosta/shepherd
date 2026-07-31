# Tasks — spike-plugin-install-api (design spike)

Base commit: `1de05dc2`. Drift check: confirm `src/api/schema.rs` still has no
`plugin.install` and that `plugin.link`/`plugin.enable` still have no confirmation
step in `src/api/server.rs`. If a consent gate was added, STOP and re-scope.

Exemplar: the CLI trust flow `src/cli/plugin.rs:154-260` (`plugin_install` →
preview → `confirm()`) is the safety property to preserve; `src/ui/release_notes.rs`
overlay machinery is the existing TUI-modal precedent for a foreground approval.

## Steps

1. **Document the asymmetry + threat model.** Write up what the CLI gate protects
   against and what an API caller can do today without it (link+enable → startup
   hook execution). Design note under `docs/next/` or `.local/prd/`.

2. **Close the API consent gap (priority).** For API-originated
   `plugin.link`/`plugin.enable`, route through a foreground-client confirmation
   modal, OR emit a prominent non-suppressible audit event + toast naming the
   manifest path and the commands it declares. Choose based on the design note.
   - Gate: `just test-one plugin` green; a test asserts an API link without
     confirmation does not silently enable a startup-hook plugin.

3. **Decide install location.** Recommend: keep install CLI-only (out-of-band), or
   add `plugin.install` with a mandatory server-side approval step. Record the
   decision; if adding the method, specify the approval mechanism (not a boolean).

4. **Verify and close.** `just check` passes if code landed. done-when: proposal
   `spike-plugin-install-api` archived with the decision + the consent gap closed
   or filed.

## STOP conditions

- If closing the consent gap would break the CLI's own `--yes` non-interactive
  install (which legitimately pre-consents), capture that path and make the gate
  distinguish CLI-pre-consented calls from raw API calls — do not drop the gate to
  keep `--yes` working.
