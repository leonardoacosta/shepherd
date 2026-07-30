# Guard FFI trampolines and the PTY actor thread against panics

Base commit: `1de05dc2` · Route: proposal · Effort: S · Confidence: HIGH · Category: correctness

## Why

Resolves advisory findings CORRECTNESS-04 and CORRECTNESS-06 (audit against
`1de05dc2`). In server mode, an unguarded panic is not one pane dying — it is the
daemon dying, taking every attached client and workspace.

**FFI trampolines** are `extern "C"` functions called from Zig (libghostty-vt)
frames; a panic cannot unwind across the FFI boundary and aborts the process.
Exactly one of four is guarded, which shows the invariant was intended but
incompletely applied:

- `src/ghostty/mod.rs:507` — `clipboard_write_trampoline` correctly wraps its
  body in `std::panic::catch_unwind(AssertUnwindSafe(...))`.
- `src/ghostty/mod.rs:481` — `write_pty_trampoline` calls a user-supplied
  callback with **no** guard.
- `src/ghostty/mod.rs:585` — `pwd_changed_trampoline` allocates and pushes with
  no guard.
- `src/ghostty/mod.rs:618` — `decode_png_trampoline` decodes a PNG whose output
  buffer is sized from an attacker-influenceable header arriving over the Kitty
  graphics protocol, with no guard and no size cap.

**PTY actor thread** — `src/pty/actor/unix.rs:417` (`fn run`) is the per-pane I/O
loop. Its reader-exit notification fires only *after* the loop:
```rust
    if let Some(on_reader_exit) = self.on_reader_exit.take() {
        on_reader_exit();
    }
```
A panic anywhere in `run` (e.g. in the VT parse path invoked via the `on_read`
closure) skips this entirely: the pane's child stays alive, no `PaneDied` is
emitted, and the UI shows a permanently frozen pane indistinguishable from a hung
program. The Windows reader thread (`src/pty/actor.rs:~153`) has the same shape.

## What changes

- Wrap `write_pty_trampoline`, `pwd_changed_trampoline`, and
  `decode_png_trampoline` bodies in `catch_unwind(AssertUnwindSafe(...))`,
  returning a safe no-op / `false` on a caught panic, mirroring
  `clipboard_write_trampoline` at `:507`.
- Add a maximum-pixel guard before the PNG output-buffer allocation in
  `decode_png_trampoline` (reject oversized images rather than allocating from an
  untrusted size).
- In the PTY actor spawn, wrap `runner.run()` in `catch_unwind` and invoke
  `on_reader_exit` unconditionally (both normal and panicking paths) so a pane
  always transitions to dead instead of freezing. Mirror in the Windows reader
  thread.

## Acceptance

- `just check` passes.
- A unit test installs a callback that panics and asserts the trampoline path
  does not abort the process (catches and returns the no-op value).
- A test (or targeted harness) asserts that when the actor loop panics,
  `on_reader_exit` still fires.
- `just test-one pty` and `just test-one ghostty` green.

## Out of scope

- The libghostty-vt Zig source (`vendor/`) — treated as a boundary.
- Changing what the callbacks *do* on the success path — only add the guard.
