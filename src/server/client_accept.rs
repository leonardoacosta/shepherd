use std::io;
use std::sync::{
    atomic::{AtomicBool, AtomicUsize, Ordering},
    Arc, OnceLock,
};

use interprocess::local_socket::traits::{Listener as _, Stream as _};
use tokio::sync::mpsc;
use tracing::{debug, error, warn};

use crate::ipc::LocalListener;
use crate::protocol::{RenderEncoding, ServerMessage, PROTOCOL_VERSION};
use crate::server::client_transport::{self, ServerEvent};

const MAX_CLIENT_CONNECTIONS: usize = 128;

struct ConnectionSlot {
    active: Arc<AtomicUsize>,
}

impl Drop for ConnectionSlot {
    fn drop(&mut self) {
        self.active.fetch_sub(1, Ordering::AcqRel);
    }
}

/// Accepts pending thin-client connections and starts their handshake readers.
pub(crate) fn accept_pending_client_connections(
    listener: &LocalListener,
    next_client_id: &mut u64,
    should_quit: &Arc<AtomicBool>,
    server_event_tx: &mpsc::Sender<ServerEvent>,
) -> io::Result<()> {
    let active_connections = client_connection_counter();
    accept_pending_client_connections_with_limit(
        listener,
        next_client_id,
        should_quit,
        server_event_tx,
        active_connections,
        MAX_CLIENT_CONNECTIONS,
        crate::ipc::peer_uid_authorized,
    )
}

/// `authorize_peer` is injected so tests can simulate a peer-uid mismatch without real
/// uid-switch privilege (see `openspec/changes/spike-socket-auth-capability/tasks.md` step 2's
/// gate); production always passes `crate::ipc::peer_uid_authorized`.
fn accept_pending_client_connections_with_limit(
    listener: &LocalListener,
    next_client_id: &mut u64,
    should_quit: &Arc<AtomicBool>,
    server_event_tx: &mpsc::Sender<ServerEvent>,
    active_connections: Arc<AtomicUsize>,
    limit: usize,
    authorize_peer: impl Fn(&crate::ipc::LocalStream) -> io::Result<bool>,
) -> io::Result<()> {
    loop {
        match listener.accept() {
            Ok(stream) => {
                let client_id = *next_client_id;
                *next_client_id = next_client_id.saturating_add(1);

                match authorize_peer(&stream) {
                    Ok(true) => {}
                    Ok(false) => {
                        debug!(
                            client_id,
                            "rejected client connection: peer uid does not match server uid"
                        );
                        if let Err(err) = reject_client_for_peer_uid_mismatch(stream) {
                            debug!(client_id, err = %err, "failed to reject client for peer uid mismatch");
                        }
                        continue;
                    }
                    Err(err) => {
                        debug!(client_id, err = %err, "failed to verify client connection peer credentials");
                        if let Err(err) = reject_client_for_peer_uid_mismatch(stream) {
                            debug!(client_id, err = %err, "failed to reject client for peer uid mismatch");
                        }
                        continue;
                    }
                }

                if let Err(err) = stream.set_nonblocking(true) {
                    warn!(err = %err, "failed to set client stream nonblocking");
                    continue;
                }

                let Some(slot) = try_acquire_connection_slot(active_connections.clone(), limit)
                else {
                    if let Err(err) = reject_client_for_connection_limit(stream, limit) {
                        debug!(client_id, err = %err, "failed to reject excess thin client");
                    }
                    continue;
                };

                let should_quit = should_quit.clone();
                let server_event_tx = server_event_tx.clone();
                std::thread::spawn(move || {
                    if let Err(err) = client_transport::handle_client_handshake(
                        stream,
                        client_id,
                        &server_event_tx,
                        &should_quit,
                    ) {
                        debug!(client_id, err = %err, "client handshake failed");
                    }
                    drop(slot);
                });
            }
            Err(ref err) if err.kind() == io::ErrorKind::WouldBlock => break,
            Err(err) => {
                error!(err = %err, "client listener accept failed");
                break;
            }
        }
    }

    Ok(())
}

fn client_connection_counter() -> Arc<AtomicUsize> {
    static ACTIVE: OnceLock<Arc<AtomicUsize>> = OnceLock::new();
    ACTIVE.get_or_init(|| Arc::new(AtomicUsize::new(0))).clone()
}

fn try_acquire_connection_slot(active: Arc<AtomicUsize>, limit: usize) -> Option<ConnectionSlot> {
    loop {
        let current = active.load(Ordering::Acquire);
        if current >= limit {
            return None;
        }
        if active
            .compare_exchange(current, current + 1, Ordering::AcqRel, Ordering::Acquire)
            .is_ok()
        {
            return Some(ConnectionSlot { active });
        }
    }
}

fn reject_client_for_connection_limit(
    mut stream: crate::ipc::LocalStream,
    limit: usize,
) -> io::Result<()> {
    stream.set_nonblocking(false)?;
    crate::protocol::write_message(
        &mut stream,
        &ServerMessage::Welcome {
            version: PROTOCOL_VERSION,
            encoding: RenderEncoding::SemanticFrame,
            error: Some(format!(
                "too many concurrent client connections (limit: {limit})"
            )),
        },
    )
    .map_err(|err| io::Error::other(err.to_string()))
}

fn reject_client_for_peer_uid_mismatch(mut stream: crate::ipc::LocalStream) -> io::Result<()> {
    stream.set_nonblocking(false)?;
    crate::protocol::write_message(
        &mut stream,
        &ServerMessage::Welcome {
            version: PROTOCOL_VERSION,
            encoding: RenderEncoding::SemanticFrame,
            error: Some("connection rejected: peer uid does not match server uid".to_string()),
        },
    )
    .map_err(|err| io::Error::other(err.to_string()))
}

/// Drains pending thin-client connections without starting handshakes.
///
/// During live handoff the old server must not let clients sit in the Unix
/// listener backlog waiting for a welcome frame that will never be sent.
pub(crate) fn reject_pending_client_connections(listener: &LocalListener) -> io::Result<()> {
    loop {
        match listener.accept() {
            Ok(_stream) => {}
            Err(ref err) if err.kind() == io::ErrorKind::WouldBlock => break,
            Err(err) => {
                error!(err = %err, "client listener reject failed");
                break;
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::path::PathBuf;

    fn unique_test_path(name: &str) -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        #[cfg(unix)]
        {
            let _ = name;
            PathBuf::from("/tmp").join(format!(
                "shepherd-client-accept-{}-{nanos}.sock",
                std::process::id()
            ))
        }
        #[cfg(windows)]
        {
            std::env::temp_dir().join(format!(
                "shepherd-client-accept-{name}-{}-{nanos}",
                std::process::id()
            ))
        }
    }

    fn local_stream_pair(
        name: &str,
    ) -> (crate::ipc::LocalStream, crate::ipc::LocalStream, PathBuf) {
        let path = unique_test_path(name);
        let _ = std::fs::remove_file(&path);
        let listener = crate::ipc::bind_local_listener(&path).unwrap();
        let client = crate::ipc::connect_local_stream(&path).unwrap();
        let server = listener.accept().unwrap();
        (client, server, path)
    }

    #[test]
    fn connection_slot_rejects_when_limit_reached_and_releases_on_drop() {
        let active = Arc::new(AtomicUsize::new(0));
        let slot = try_acquire_connection_slot(active.clone(), 1).expect("first slot");
        assert_eq!(active.load(Ordering::Acquire), 1);
        assert!(try_acquire_connection_slot(active.clone(), 1).is_none());
        drop(slot);
        assert_eq!(active.load(Ordering::Acquire), 0);
        assert!(try_acquire_connection_slot(active, 1).is_some());
    }

    #[test]
    fn reject_client_for_connection_limit_sends_welcome_error() {
        let (mut client, server, path) = local_stream_pair("too-many-clients");
        reject_client_for_connection_limit(server, 3).unwrap();

        let message: ServerMessage =
            crate::protocol::read_message(&mut client, crate::protocol::MAX_FRAME_SIZE).unwrap();
        match message {
            ServerMessage::Welcome {
                version,
                encoding,
                error,
            } => {
                assert_eq!(version, PROTOCOL_VERSION);
                assert_eq!(encoding, RenderEncoding::SemanticFrame);
                assert_eq!(
                    error.as_deref(),
                    Some("too many concurrent client connections (limit: 3)")
                );
            }
            other => panic!("expected Welcome, got {other:?}"),
        }

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn peer_cred_mismatch_rejects_client_before_handshake() {
        use interprocess::local_socket::ListenerNonblockingMode;

        // Real uid-switching isn't available in CI, so the mismatch is simulated through the
        // injected `authorize_peer` seam rather than an actual different-uid caller.
        let path = unique_test_path("peer-cred-mismatch");
        let _ = std::fs::remove_file(&path);
        let listener = crate::ipc::bind_local_listener(&path).unwrap();
        listener
            .set_nonblocking(ListenerNonblockingMode::Accept)
            .unwrap();
        let mut client = crate::ipc::connect_local_stream(&path).unwrap();

        let mut next_client_id = 0u64;
        let should_quit = Arc::new(AtomicBool::new(false));
        let (server_event_tx, _server_event_rx) = mpsc::channel(4);
        let active_connections = Arc::new(AtomicUsize::new(0));

        accept_pending_client_connections_with_limit(
            &listener,
            &mut next_client_id,
            &should_quit,
            &server_event_tx,
            active_connections.clone(),
            MAX_CLIENT_CONNECTIONS,
            |_stream| Ok(false),
        )
        .unwrap();

        let message: ServerMessage =
            crate::protocol::read_message(&mut client, crate::protocol::MAX_FRAME_SIZE).unwrap();
        match message {
            ServerMessage::Welcome { error, .. } => {
                assert_eq!(
                    error.as_deref(),
                    Some("connection rejected: peer uid does not match server uid")
                );
            }
            other => panic!("expected peer-uid mismatch Welcome, got {other:?}"),
        }
        assert_eq!(
            active_connections.load(Ordering::Acquire),
            0,
            "a rejected peer must never consume a connection slot"
        );

        let _ = std::fs::remove_file(path);
    }
}
