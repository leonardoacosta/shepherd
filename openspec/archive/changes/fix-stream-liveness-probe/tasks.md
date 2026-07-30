# Tasks — fix-stream-liveness-probe

Base commit: `1de05dc2`. Drift check: confirm `src/ipc.rs` `probe_stream_closed`
`#[cfg(unix)]` still contains `Ok(_) => Ok(true)`. If it has changed, STOP and
report.

Exemplar to imitate: the Windows branch in the same file,
`windows_named_pipe_available` (`src/ipc.rs:186+`), which peeks instead of
reading — the intended non-destructive contract.

## Ordered steps

1. **Rewrite the Unix probe to peek.**
   - Replace the `set_nonblocking(true)` + destructive `read` + `set_nonblocking(false)`
     body with a `MSG_PEEK | MSG_DONTWAIT` `recv` on the raw fd (via
     `std::os::unix::io::AsRawFd`). Map: `0` → closed, `>0` → not closed,
     `EWOULDBLOCK`/`EAGAIN` → not closed, connection-closed errors → closed.
   - Do not leave the stream's blocking mode altered on exit.
   - Gate: `just lint` clean (no new clippy warnings; no `unwrap()` in the new
     code per AGENTS.md).

2. **Add regression tests.**
   - New test: open a socketpair/UnixStream, write a byte from the peer, call the
     probe, assert it returns `false` (not closed) AND a subsequent
     `stream.read` still yields that byte.
   - New test: drop the peer, assert the probe returns `true` (closed).
   - Place near existing `src/ipc.rs` `#[cfg(test)]` tests; if none exist there,
     mirror the test style of another `src/*.rs` `#[cfg(all(test, unix))]` module.
   - Gate: `just test-one ipc` (or the module path) green.

3. **Verify no false disconnect at the integration layer.**
   - Confirm `tests/api_ping.rs` / any subscription test still passes; if there's
     an existing "client writes on a subscription" test, it should now be
     un-flaky. If none exists and it's cheap, add one.
   - Gate: `just test-one api` green.

4. **Verify and close.**
   - `just check` passes. done-when: proposal `fix-stream-liveness-probe`
     archived.

## STOP conditions

- If `LocalStream` on unix is not a plain fd-backed stream (e.g. it wraps a
  buffered reader that already consumed bytes), report — a peek on the raw fd
  won't see buffered data and the fix needs rethinking.
