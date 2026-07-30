# Require checksum verification on the stable update and install path

Base commit: `1de05dc2` · Route: proposal · Effort: S · Confidence: HIGH · Category: security

## Why

This change resolves a finding from the advisory audit run against `1de05dc2`
(SECURITY-01).

`herdr`'s self-update and `curl | sh` install verify the downloaded binary's
sha256 **only if the manifest provides one**, and the currently-served stable
manifest provides none:

- `src/update.rs:586` — verification is conditional:
  ```rust
  if let Some(expected) = &release.sha256 {
      if let Err(e) = crate::checksum::verify_sha256(&tmp_path, expected) {
          // ...error
      }
  }
  ```
  When `release.sha256` is `None`, the freshly downloaded binary is installed
  with no integrity check at all.
- `src/update.rs:125-158` — `AssetRef` deserializes either a bare URL string
  (which sets `sha256: None`) or an object with an optional `sha256`. "No
  checksum" is a first-class, silent manifest state.
- `website/latest.json` — the shipped **stable** manifest uses the bare-string
  asset form for all four unix targets, so today's stable channel carries no
  checksums:
  ```json
  "assets": {
    "linux-x86_64": "https://github.com/ogulcancelik/herdr/releases/download/v0.7.5/herdr-linux-x86_64",
    ...
  }
  ```
  `website/preview.json` by contrast carries per-asset `sha256`.
- `website/install.sh:67` — downloads with `curl -fsSL ... -o` then installs,
  with no integrity check of any kind.
- `website/install.ps1:170` — by contrast *throws* when the checksum is missing
  (`"A SHA-256 checksum is required for $Path."`). The unix paths are the
  asymmetric hole.

The manifest (served from herdr.dev) and the artifact (GitHub release assets)
are two different origins. The sha256 in the manifest is what binds them:
without it, anyone who can alter or interpose on the GitHub artifact origin —
but not herdr.dev — gets arbitrary code execution as the user on the next
`herdr update` or fresh install, on every non-Homebrew/mise/Nix unix install.

## What changes

1. The release pipeline emits a per-asset `sha256` into `website/latest.json`
   (matching the object form already used in `preview.json`).
2. `src/update.rs` treats a missing checksum as a hard error rather than a
   silent skip.
3. `website/install.sh` verifies the downloaded binary's sha256 before install.
4. The download hardens its transport: `curl --proto '=https' --proto-redir
   '=https'`, and manifest-supplied URLs are required to be `https://`.

**Ordering is load-bearing:** the manifest must ship checksums (step 1) and be
published *before* the client-side hard-fail (step 2) reaches users, or
existing installs updating against an old bare-string manifest will refuse to
update. See `tasks.md` for the sequencing.

## Acceptance

- `just check` passes.
- A unit test proves `download_update` (or its verification helper) returns an
  error when the resolved asset has no `sha256`.
- `website/latest.json` assets carry `sha256` for all four unix targets.
- `website/install.sh` aborts (non-zero exit) if the downloaded file's hash does
  not match the manifest.
- No secret or key material is introduced; this is integrity, not signing. A
  detached signature over the manifest (to also cover a herdr.dev compromise) is
  noted as follow-up, not in scope here.
