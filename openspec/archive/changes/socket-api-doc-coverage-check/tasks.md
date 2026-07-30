# Tasks — socket-api-doc-coverage-check

Base commit: `1de05dc2`. Drift check: confirm `docs/next/api/herdr-api.schema.json`
is still generated+committed and `website/src/content/docs/socket-api.mdx` exists.
Confirm `server.live_handoff` and `workspace.move_block` are still absent from that
page. If already documented, adjust the acceptance accordingly.

Exemplar to imitate closely: `scripts/config_reference_check.py` (model-vs-doc
diff, skip-subtree handling) and its test `scripts/test_config_reference_check.py`.

## Ordered steps

1. **Write the checker.** `scripts/socket_api_reference_check.py`: load method
   names from `docs/next/api/herdr-api.schema.json`, scan `socket-api.mdx` for each,
   report any missing. Hardcode a skip list for `#[schemars(skip)]`/`serde(skip)`
   internals (`pane.graphics.stream`, `pane.graphics.stream_*`).
   - Gate: run it — it should fail listing the two known-missing methods.

2. **Document the two missing methods** in `socket-api.mdx` (and the stable page
   if release-docs promotes it) so the checker passes.
   - Gate: `python3 scripts/socket_api_reference_check.py` exits 0.

3. **Add a unittest** `scripts/test_socket_api_reference_check.py` mirroring
   `scripts/test_config_reference_check.py` (feed a fixture missing a method →
   assert failure).
   - Gate: `python3 -m unittest scripts.test_socket_api_reference_check` passes.

4. **Wire into pipelines.** Add the check to `just release-docs-check` and, if
   present, the `maintenance` CI job from `ci-maintenance-gate`.
   - Gate: `just release-docs-check` passes.

5. **Verify and close.**
   - `just check` passes. done-when: proposal `socket-api-doc-coverage-check`
     archived.
