use std::fs;
use std::io::{self, Read};
#[cfg(unix)]
use std::os::fd::{AsRawFd, RawFd};
#[cfg(unix)]
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::Path;

#[cfg(unix)]
use interprocess::local_socket::traits::Stream as _;

pub(crate) type LocalListener = interprocess::local_socket::Listener;
pub(crate) type LocalStream = interprocess::local_socket::Stream;

pub(crate) enum LocalStreamRead {
    Data,
    Pending,
    Closed,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct SocketFileIdentity {
    #[cfg(unix)]
    dev: u64,
    #[cfg(unix)]
    ino: u64,
    #[cfg(windows)]
    marker: Vec<u8>,
}

pub(crate) fn connect_local_stream(path: &Path) -> io::Result<LocalStream> {
    #[cfg(unix)]
    {
        use interprocess::local_socket::{prelude::*, GenericFilePath};

        let name = path.to_fs_name::<GenericFilePath>()?;
        LocalStream::connect(name)
    }

    #[cfg(windows)]
    {
        use interprocess::local_socket::{prelude::*, GenericNamespaced};

        let name = path.to_string_lossy().to_string();
        let name = name.to_ns_name::<GenericNamespaced>()?;
        LocalStream::connect(name)
    }
}

pub(crate) fn bind_local_listener(path: &Path) -> io::Result<LocalListener> {
    #[cfg(unix)]
    {
        use interprocess::local_socket::{prelude::*, GenericFilePath, ListenerOptions};

        let name = path.to_fs_name::<GenericFilePath>()?;
        ListenerOptions::new()
            .name(name)
            .reclaim_name(false)
            .create_sync()
    }

    #[cfg(windows)]
    {
        use interprocess::local_socket::{prelude::*, GenericNamespaced, ListenerOptions};

        let name = path.to_string_lossy().to_string();
        let name = name.to_ns_name::<GenericNamespaced>()?;
        let listener = ListenerOptions::new()
            .name(name)
            .reclaim_name(false)
            .create_sync()?;
        fs::write(path, windows_socket_marker())?;
        Ok(listener)
    }
}

pub(crate) fn prepare_socket_path(
    path: &Path,
    busy_message: impl FnOnce(&Path) -> String,
) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    if !path.exists() {
        return Ok(());
    }

    match connect_local_stream(path) {
        Ok(_) => {
            return Err(io::Error::new(io::ErrorKind::AddrInUse, busy_message(path)));
        }
        Err(err) if stale_socket_connect_error(err.kind()) => {}
        Err(err) => return Err(err),
    }

    if let Err(err) = fs::remove_file(path) {
        if err.kind() != io::ErrorKind::NotFound {
            return Err(err);
        }
    }

    Ok(())
}

fn stale_socket_connect_error(kind: io::ErrorKind) -> bool {
    matches!(
        kind,
        io::ErrorKind::ConnectionRefused | io::ErrorKind::NotFound | io::ErrorKind::TimedOut
    ) || (cfg!(windows) && kind == io::ErrorKind::WouldBlock)
}

pub(crate) fn local_stream_peer_closed(stream: &mut LocalStream) -> io::Result<bool> {
    probe_stream_closed(stream)
}

pub(crate) fn set_local_stream_polling(stream: &mut LocalStream, enabled: bool) -> io::Result<()> {
    #[cfg(unix)]
    {
        stream.set_nonblocking(enabled)
    }

    #[cfg(windows)]
    {
        let _ = (stream, enabled);
        Ok(())
    }
}

pub(crate) fn poll_local_stream_read(
    stream: &mut LocalStream,
    buf: &mut [u8],
) -> io::Result<LocalStreamRead> {
    #[cfg(unix)]
    {
        match stream.read(buf) {
            Ok(0) => Ok(LocalStreamRead::Closed),
            Ok(_) => Ok(LocalStreamRead::Data),
            Err(err) if err.kind() == io::ErrorKind::WouldBlock => Ok(LocalStreamRead::Pending),
            Err(err) => Err(err),
        }
    }

    #[cfg(windows)]
    {
        match windows_named_pipe_available(stream)? {
            None => Ok(LocalStreamRead::Closed),
            Some(0) => Ok(LocalStreamRead::Pending),
            Some(_) => match stream.read(buf) {
                Ok(0) => Ok(LocalStreamRead::Closed),
                Ok(_) => Ok(LocalStreamRead::Data),
                Err(err) if is_connection_closed_error(&err) => Ok(LocalStreamRead::Closed),
                Err(err) => Err(err),
            },
        }
    }
}

#[cfg(unix)]
fn probe_stream_closed(stream: &mut LocalStream) -> io::Result<bool> {
    let LocalStream::UdSocket(stream) = stream;
    let mut probe = [0u8; 1];
    let received = unsafe {
        libc::recv(
            stream.inner().as_raw_fd(),
            probe.as_mut_ptr().cast(),
            probe.len(),
            libc::MSG_PEEK | libc::MSG_DONTWAIT,
        )
    };
    match received {
        0 => Ok(true),
        received if received > 0 => Ok(false),
        _ => {
            let err = io::Error::last_os_error();
            if matches!(
                err.kind(),
                io::ErrorKind::WouldBlock | io::ErrorKind::Interrupted
            ) {
                Ok(false)
            } else if is_connection_closed_error(&err) {
                Ok(true)
            } else {
                Err(err)
            }
        }
    }
}

#[cfg(windows)]
fn probe_stream_closed(stream: &mut LocalStream) -> io::Result<bool> {
    Ok(windows_named_pipe_available(stream)?.is_none())
}

#[cfg(windows)]
fn windows_named_pipe_available(stream: &mut LocalStream) -> io::Result<Option<u32>> {
    use std::os::windows::io::{AsHandle, AsRawHandle};

    let LocalStream::NamedPipe(pipe) = stream;
    let mut available = 0;
    let ok = unsafe {
        windows_sys::Win32::System::Pipes::PeekNamedPipe(
            pipe.as_handle().as_raw_handle(),
            std::ptr::null_mut(),
            0,
            std::ptr::null_mut(),
            &mut available,
            std::ptr::null_mut(),
        )
    };
    if ok != 0 {
        return Ok(Some(available));
    }

    let err = io::Error::last_os_error();
    if is_connection_closed_error(&err) || windows_named_pipe_closed_error(&err) {
        return Ok(None);
    }
    Err(err)
}

/// Reports whether the connected peer's uid matches the current process's uid,
/// rejecting the accept-time same-uid trust boundary from resting on file
/// permissions alone (`spike-socket-auth-capability`). Linux uses
/// `SO_PEERCRED`, macOS uses `getpeereid`; both read kernel-verified peer
/// identity off the raw fd, so a peer cannot spoof it.
#[cfg(unix)]
pub(crate) fn peer_uid_authorized(stream: &LocalStream) -> io::Result<bool> {
    let peer_uid = stream_peer_uid(stream)?;
    Ok(peer_uid == unsafe { libc::getuid() })
}

/// Windows named pipes have no dependency-available equivalent to
/// `SO_PEERCRED`/`getpeereid` today: a real check needs
/// `GetNamedPipeClientProcessId` plus opening the peer process token
/// (`OpenProcessToken`/`GetTokenInformation(TokenUser)`) to compare SIDs,
/// which needs a `Win32_Security` feature flag not enabled in `Cargo.toml`,
/// and even then cannot distinguish "same interactive session, different
/// process" from "genuinely different user" without session-enumeration APIs
/// outside the current dependency tree (see
/// `openspec/changes/spike-socket-auth-capability/tasks.md` STOP conditions).
/// Documented gap, matching `restrict_socket_permissions`'s existing Windows
/// no-op precedent: always allow rather than silently rejecting or shipping a
/// check that only looks real.
#[cfg(windows)]
pub(crate) fn peer_uid_authorized(_stream: &LocalStream) -> io::Result<bool> {
    Ok(true)
}

#[cfg(unix)]
fn stream_peer_uid(stream: &LocalStream) -> io::Result<u32> {
    let LocalStream::UdSocket(inner) = stream;
    raw_fd_peer_uid(inner.inner().as_raw_fd())
}

#[cfg(target_os = "linux")]
fn raw_fd_peer_uid(fd: RawFd) -> io::Result<u32> {
    let mut cred: libc::ucred = unsafe { std::mem::zeroed() };
    let mut len = std::mem::size_of::<libc::ucred>() as libc::socklen_t;
    let ret = unsafe {
        libc::getsockopt(
            fd,
            libc::SOL_SOCKET,
            libc::SO_PEERCRED,
            (&mut cred as *mut libc::ucred).cast(),
            &mut len,
        )
    };
    if ret != 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(cred.uid)
}

#[cfg(target_os = "macos")]
fn raw_fd_peer_uid(fd: RawFd) -> io::Result<u32> {
    let mut uid: libc::uid_t = 0;
    let mut gid: libc::gid_t = 0;
    let ret = unsafe { libc::getpeereid(fd, &mut uid, &mut gid) };
    if ret != 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(uid)
}

pub(crate) fn is_connection_closed_error(err: &io::Error) -> bool {
    matches!(
        err.kind(),
        io::ErrorKind::BrokenPipe
            | io::ErrorKind::ConnectionAborted
            | io::ErrorKind::ConnectionReset
            | io::ErrorKind::NotConnected
            | io::ErrorKind::UnexpectedEof
            | io::ErrorKind::WriteZero
    )
}

#[cfg(windows)]
fn windows_named_pipe_closed_error(err: &io::Error) -> bool {
    matches!(err.raw_os_error(), Some(6 | 109 | 232 | 233))
}

pub(crate) fn socket_file_identity(path: &Path) -> io::Result<SocketFileIdentity> {
    #[cfg(windows)]
    {
        Ok(SocketFileIdentity {
            marker: fs::read(path)?,
        })
    }

    #[cfg(unix)]
    {
        let metadata = fs::metadata(path)?;
        Ok(SocketFileIdentity {
            dev: metadata.dev(),
            ino: metadata.ino(),
        })
    }
}

pub(crate) fn remove_socket_file_if_owned(
    path: &Path,
    identity: &SocketFileIdentity,
) -> io::Result<()> {
    let current = match socket_file_identity(path) {
        Ok(current) => current,
        Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(err) => return Err(err),
    };

    if current != *identity {
        return Ok(());
    }

    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(err),
    }
}

#[cfg(windows)]
fn windows_socket_marker() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    format!("{}:{now}", std::process::id())
}

#[cfg(unix)]
pub(crate) fn restrict_socket_permissions(path: &Path, mode: u32) -> io::Result<()> {
    let mut permissions = fs::metadata(path)?.permissions();
    permissions.set_mode(mode);
    fs::set_permissions(path, permissions)
}

#[cfg(windows)]
pub(crate) fn restrict_socket_permissions(_path: &Path, _mode: u32) -> io::Result<()> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use interprocess::local_socket::traits::Listener as _;
    #[cfg(unix)]
    use std::io::Write;
    use std::path::PathBuf;

    #[test]
    fn stale_socket_connect_errors_keep_unix_would_block_strict() {
        assert!(stale_socket_connect_error(io::ErrorKind::ConnectionRefused));
        assert!(stale_socket_connect_error(io::ErrorKind::NotFound));
        assert!(stale_socket_connect_error(io::ErrorKind::TimedOut));
        assert_eq!(
            stale_socket_connect_error(io::ErrorKind::WouldBlock),
            cfg!(windows)
        );
    }

    #[cfg(unix)]
    #[test]
    fn unix_probe_peeks_without_consuming_live_bytes() {
        let path = temp_socket_marker_path("peek-live-byte");
        let _ = fs::remove_file(&path);

        let listener = bind_local_listener(&path).expect("bind listener");
        let mut client = connect_local_stream(&path).expect("connect client");
        let mut server = listener.accept().expect("accept server");

        client.write_all(b"x").expect("client writes probe byte");

        assert!(
            !local_stream_peer_closed(&mut server).expect("probe live stream"),
            "live stream with pending data should not look closed"
        );

        let mut buf = [0u8; 1];
        assert_eq!(server.read(&mut buf).expect("server reads byte"), 1);
        assert_eq!(buf[0], b'x');

        let _ = fs::remove_file(path);
    }

    #[cfg(unix)]
    #[test]
    fn peer_cred_authorizes_same_process_connection() {
        // A same-process client/server pair always shares a uid, so this exercises the real
        // getsockopt(SO_PEERCRED)/getpeereid syscall path end to end. The mismatch path (a
        // different uid rejected before dispatch) needs real privilege drop that CI does not
        // have; it is covered via an injectable authorizer in
        // src/api/server.rs and src/server/client_accept.rs's tests instead.
        let path = temp_socket_marker_path("peer-cred-same-uid");
        let _ = fs::remove_file(&path);

        let listener = bind_local_listener(&path).expect("bind listener");
        let _client = connect_local_stream(&path).expect("connect client");
        let server = listener.accept().expect("accept server");

        assert!(peer_uid_authorized(&server).expect("peer uid check"));

        let _ = fs::remove_file(path);
    }

    #[cfg(unix)]
    #[test]
    fn unix_probe_reports_closed_peer() {
        let path = temp_socket_marker_path("peek-closed-peer");
        let _ = fs::remove_file(&path);

        let listener = bind_local_listener(&path).expect("bind listener");
        let client = connect_local_stream(&path).expect("connect client");
        let mut server = listener.accept().expect("accept server");
        drop(client);

        assert!(local_stream_peer_closed(&mut server).expect("probe closed stream"));

        let _ = fs::remove_file(path);
    }

    #[cfg(windows)]
    #[test]
    fn remove_socket_file_if_owned_compares_windows_marker_contents() {
        let path = temp_socket_marker_path("same-len-marker");
        let _ = fs::remove_file(&path);

        fs::write(&path, b"marker-aa").expect("write first marker");
        let identity = socket_file_identity(&path).expect("read first identity");
        fs::write(&path, b"marker-bb").expect("replace with same-length marker");

        remove_socket_file_if_owned(&path, &identity).expect("remove owned marker");

        assert!(path.exists(), "same-length replacement marker must survive");

        let _ = fs::remove_file(&path);
    }

    #[cfg(windows)]
    #[test]
    fn idle_named_pipe_peer_is_not_treated_as_closed() {
        let path = temp_socket_marker_path("idle-pipe");
        let listener = bind_local_listener(&path).unwrap();
        let _client = connect_local_stream(&path).unwrap();
        let mut server = listener.accept().unwrap();

        assert!(!local_stream_peer_closed(&mut server).unwrap());

        let _ = fs::remove_file(path);
    }

    #[cfg(windows)]
    #[test]
    fn disconnected_named_pipe_peer_is_treated_as_closed() {
        let path = temp_socket_marker_path("disconnected-pipe");
        let listener = bind_local_listener(&path).unwrap();
        let client = connect_local_stream(&path).unwrap();
        let mut server = listener.accept().unwrap();

        drop(client);

        assert!(local_stream_peer_closed(&mut server).unwrap());

        let _ = fs::remove_file(path);
    }

    fn temp_socket_marker_path(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("herdr-{name}-{}.sock", std::process::id()))
    }
}
