# Tasks — ffi-panic-guards

Base commit: `1de05dc2`. Drift check: confirm `src/ghostty/mod.rs:507`
(`clipboard_write_trampoline`) still uses `catch_unwind` and that
`write_pty_trampoline` (`:481`), `pwd_changed_trampoline` (`:585`),
`decode_png_trampoline` (`:618`) do NOT. Confirm `src/pty/actor/unix.rs:417`
(`fn run`) still calls `on_reader_exit` only after the loop. If changed, STOP.

Exemplar to imitate exactly: `src/ghostty/mod.rs:507-...`
(`clipboard_write_trampoline`'s `catch_unwind(AssertUnwindSafe(|| { ... }))`
wrapper). Copy its shape for the three unguarded trampolines.

## Ordered steps

1. **Guard the three trampolines.**
   - Wrap the bodies of `write_pty_trampoline`, `pwd_changed_trampoline`,
     `decode_png_trampoline` in `catch_unwind(AssertUnwindSafe(...))`. On a caught
     panic, log at `error!` and return the safe default (no-op / `false`).
   - Gate: `just test-one ghostty` green; `just lint` clean.

2. **Cap PNG decode size.**
   - In `decode_png_trampoline`, before the output-buffer allocation, reject
     images whose pixel count exceeds a sane `MAX_PNG_PIXELS` constant (return the
     no-op path). This bounds the untrusted allocation.
   - Gate: add a test feeding an oversized header → decode returns failure, no
     panic/OOM.

3. **Guard the actor reader thread.**
   - In the actor spawn (`src/pty/actor/unix.rs:~378`), wrap `runner.run()` in
     `catch_unwind`, and move the `on_reader_exit` invocation so it fires
     unconditionally after the run (normal or panicking). Add a
     `fire_reader_exit`-style helper if it makes the ownership clean.
   - Mirror the same guarantee in the Windows reader thread
     (`src/pty/actor.rs:~153`).
   - Gate: `just test-one pty` green; a test where the `on_read` callback panics
     asserts `on_reader_exit` still fires.

4. **Verify and close.**
   - `just check` passes (includes windows-target clippy). done-when: proposal
     `ffi-panic-guards` archived.

## STOP conditions

- If `AssertUnwindSafe` cannot be applied because a trampoline captures a
  non-`UnwindSafe` `&mut` in a way that would be genuinely unsound to catch
  across, STOP and report — the fix then needs the aliasing rework
  (CORRECTNESS-05), which is out of scope here.
