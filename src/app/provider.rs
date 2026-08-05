//! Provider-adapter invocation shared by the demand-driven refresh loops.
//!
//! This is `CONTEXT.md`'s provider-adapter layer: talk to exactly one source and return
//! `Option<T>`. Every failure — binary absent, spawn error, non-zero exit, timeout — is the
//! same answer, an absent value. Callers never see an error to surface.
//!
//! Lives here rather than inside one refresh module because more than one business layer
//! invokes adapters this way, and a second copy would drift from the pipe-drain fix below.

use std::path::Path;
use std::time::{Duration, Instant};

/// A provider that has not answered by now is not worth a frame's staleness. Kills the child
/// rather than letting a hung tool pin a thread for the process lifetime.
pub(crate) const PROVIDER_TIMEOUT: Duration = Duration::from_secs(5);
const PROVIDER_POLL_INTERVAL: Duration = Duration::from_millis(25);

/// Every failure — binary absent, spawn error, non-zero exit, timeout — returns `None`.
/// Never panics and never surfaces an error to the user; an absent value is the signal.
pub(crate) fn run_provider(cwd: &Path, program: &str, args: &[&str]) -> Option<String> {
    let mut child = std::process::Command::new(program)
        .args(args)
        .current_dir(cwd)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
        .ok()?;

    // stdout must be drained concurrently with the wait. A provider whose output exceeds the
    // pipe buffer blocks on write until someone reads, so polling `try_wait` alone deadlocks
    // until the timeout — observed with `bd list --json` over a few hundred issues.
    let stdout = child.stdout.take()?;
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let mut buffer = Vec::new();
        let mut stdout = stdout;
        let read = std::io::Read::read_to_end(&mut stdout, &mut buffer);
        let _ = tx.send(read.map(|_| buffer));
    });

    let deadline = Instant::now() + PROVIDER_TIMEOUT;
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                if !status.success() {
                    return None;
                }
                break;
            }
            Ok(None) => {
                if Instant::now() >= deadline {
                    let _ = child.kill();
                    let _ = child.wait();
                    tracing::debug!(program, "status provider timed out");
                    return None;
                }
                std::thread::sleep(PROVIDER_POLL_INTERVAL);
            }
            Err(_) => return None,
        }
    }

    let remaining = deadline.saturating_duration_since(Instant::now());
    let stdout = rx.recv_timeout(remaining).ok()?.ok()?;
    String::from_utf8(stdout).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_missing_binary_resolves_to_none_rather_than_panicking() {
        let resolved = run_provider(
            Path::new("."),
            "shepherd-provider-that-does-not-exist",
            &["--json"],
        );
        assert!(resolved.is_none());
    }

    #[test]
    fn a_nonzero_exit_resolves_to_none() {
        assert!(run_provider(Path::new("."), "false", &[]).is_none());
    }

    #[test]
    fn a_zero_exit_returns_stdout() {
        let out = run_provider(Path::new("."), "echo", &["hello"]);
        assert_eq!(out.as_deref().map(str::trim), Some("hello"));
    }

    /// A provider that never returns is abandoned at the timeout rather than pinning the
    /// refresh thread for the process lifetime.
    #[test]
    fn a_provider_that_never_terminates_is_abandoned_at_the_timeout() {
        let started = Instant::now();
        let resolved = run_provider(
            Path::new("."),
            "sh",
            &["-c", "sleep 3600 & wait; echo unreachable"],
        );

        assert!(resolved.is_none(), "a timed-out provider yields no value");
        assert!(
            started.elapsed() < PROVIDER_TIMEOUT * 3,
            "the wait loop must not outlive the timeout by an order of magnitude"
        );
    }

    /// Regression: stdout must be drained while waiting. Polling `try_wait` against an
    /// undrained pipe deadlocks once the child's output exceeds the pipe buffer, which
    /// silently turned every real issue database into a timeout.
    #[test]
    fn output_larger_than_the_pipe_buffer_is_returned_whole() {
        let payload_bytes = 512 * 1024;
        let out = run_provider(
            Path::new("."),
            "sh",
            &[
                "-c",
                &format!("head -c {payload_bytes} /dev/zero | tr '\\0' 'x'"),
            ],
        )
        .expect("a large payload must not deadlock the wait loop");

        assert_eq!(out.len(), payload_bytes, "output must not be truncated");
    }
}
