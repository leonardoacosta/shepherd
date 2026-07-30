# Tasks — update-checksum-integrity

Base commit: `1de05dc2`. Drift check first: confirm `src/update.rs:586` still
reads `if let Some(expected) = &release.sha256` and `website/latest.json` assets
are still bare strings. If either has changed, STOP and report.

Exemplar to imitate: `website/install.ps1:133-184` (correct required-checksum
handling) and `website/preview.json` (correct object-form assets with `sha256`).

## Ordered steps

1. **Emit checksums into the stable manifest generator.**
   - Find the code that writes `website/latest.json` (the release pipeline —
     search `scripts/` for `latest.json` / `sync-latest-json`, and
     `.github/workflows/release.yml`; the preview path in
     `.github/workflows/preview.yml` already computes per-asset sha256 — reuse
     that step's shape).
   - Make it write each asset as `{ "url": ..., "sha256": ... }` for all four
     unix targets, matching `website/preview.json`.
   - Gate: the generator's own test or a `python3 -m unittest scripts.test_preview`-style
     run passes; a manual dry-run produces objects with non-empty `sha256`.
   - **STOP condition:** if the artifacts' true hashes are not available to the
     generator at manifest-write time, report — do not fabricate placeholder
     hashes.

2. **Publish a checksummed stable manifest before enforcing on the client.**
   - This is a release/ops step, not a code change: `website/latest.json` served
     from herdr.dev must carry checksums before step 3 ships to users. Record in
     the proposal that this ordering was honored.

3. **Make verification mandatory in `src/update.rs`.**
   - Change the `if let Some(expected) = &release.sha256` block (`~:586`) so a
     `None` checksum is an error (`"stable update asset has no checksum"`),
     keeping the existing `verify_sha256` call for the `Some` case.
   - Consider a narrow escape hatch only if preview/local-dev flows legitimately
     lack a hash — if so, gate the bypass behind an explicit channel check, not a
     silent skip.
   - Gate: `just test-one update` passes; add a test asserting the error path.

4. **Harden the download transport.**
   - Add `--proto '=https' --proto-redir '=https'` to the `curl` invocation in
     `download_update` (`src/update.rs:~574`) and reject any `download_url` whose
     scheme is not `https` before spawning curl.
   - Gate: `just lint` + `just test-one update`.

5. **Add integrity verification to `website/install.sh`.**
   - After the `curl ... -o "${TMP}/${BIN}"` at `:67`, read the asset's `sha256`
     from the already-fetched manifest and compare with `shasum -a 256` /
     `sha256sum`; abort non-zero on mismatch or missing hash. Mirror the
     `install.ps1` "required checksum" semantics.
   - Gate: `bash -n website/install.sh` parses; a manual run against a real
     release verifies and installs; a run with a corrupted file aborts.

6. **Verify and close.**
   - `just check` passes.
   - done-when: proposal `update-checksum-integrity` archived after the
     maintainer confirms the manifest ships checksums and the client enforces.
