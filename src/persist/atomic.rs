use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[cfg(unix)]
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};

pub(super) fn ensure_private_parent_dir(path: &Path) -> io::Result<()> {
    let Some(parent) = path.parent() else {
        return Ok(());
    };
    fs::create_dir_all(parent)?;
    #[cfg(unix)]
    fs::set_permissions(parent, fs::Permissions::from_mode(0o700))?;
    Ok(())
}

pub(crate) fn atomic_write(path: &Path, bytes: &[u8]) -> io::Result<()> {
    ensure_private_parent_dir(path)?;
    atomic_write_in_parent(path, bytes)
}

/// Atomically replace a user-facing config file without changing permissions
/// on an existing parent directory (which may be shared, such as `/tmp`).
pub(crate) fn atomic_write_config(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| io::Error::other(format!("path {} has no parent", path.display())))?;
    fs::create_dir_all(parent)?;
    atomic_write_in_parent(path, bytes)
}

fn atomic_write_in_parent(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| io::Error::other(format!("path {} has no parent", path.display())))?;
    let tmp_path = unique_tmp_path(path);

    {
        let mut options = fs::OpenOptions::new();
        options.create_new(true).write(true);
        #[cfg(unix)]
        options.mode(0o600);
        let mut file = options.open(&tmp_path)?;
        if let Err(err) = file.write_all(bytes).and_then(|_| file.sync_all()) {
            let _ = fs::remove_file(&tmp_path);
            return Err(err);
        }
    }

    #[cfg(windows)]
    if path.exists() {
        if let Err(err) = fs::remove_file(path) {
            let _ = fs::remove_file(&tmp_path);
            return Err(err);
        }
    }

    if let Err(err) = fs::rename(&tmp_path, path) {
        let _ = fs::remove_file(&tmp_path);
        return Err(err);
    }

    sync_parent_dir(parent)?;
    Ok(())
}

fn unique_tmp_path(path: &Path) -> PathBuf {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("persist");
    parent.join(format!(
        ".{}.{}.{}.tmp",
        file_name,
        std::process::id(),
        now_nanos()
    ))
}

fn now_nanos() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos()
}

fn sync_parent_dir(parent: &Path) -> io::Result<()> {
    let dir = match fs::File::open(parent) {
        Ok(dir) => dir,
        Err(err) if directory_sync_unsupported(&err) => return Ok(()),
        Err(err) => return Err(err),
    };
    match dir.sync_all() {
        Ok(()) => Ok(()),
        Err(err) if directory_sync_unsupported(&err) => Ok(()),
        Err(err) => Err(err),
    }
}

fn directory_sync_unsupported(err: &io::Error) -> bool {
    #[cfg(windows)]
    {
        err.kind() == io::ErrorKind::PermissionDenied
    }
    #[cfg(not(windows))]
    {
        matches!(err.raw_os_error(), Some(libc::EINVAL) | Some(libc::ENOTSUP))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_path(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "shepherd-atomic-tests-{name}-{}-{}",
            std::process::id(),
            now_nanos()
        ))
    }

    #[test]
    fn unique_tmp_path_changes_per_call() {
        let target = temp_path("unique").join("session.json");
        let first = unique_tmp_path(&target);
        let second = unique_tmp_path(&target);
        assert_ne!(first, second);
        assert_eq!(first.parent(), second.parent());
    }

    #[test]
    fn atomic_write_round_trips_bytes() {
        let target = temp_path("roundtrip").join("session.json");
        atomic_write(&target, br#"{"version":1}"#).unwrap();
        assert_eq!(fs::read(&target).unwrap(), br#"{"version":1}"#);
    }

    #[test]
    fn atomic_config_write_supports_shared_parent_directory() {
        let target = std::env::temp_dir().join(format!(
            "shepherd-atomic-config-{}-{}.toml",
            std::process::id(),
            now_nanos()
        ));
        atomic_write_config(&target, b"[ui]\nmouse_capture = true\n").unwrap();
        assert_eq!(
            fs::read_to_string(&target).unwrap(),
            "[ui]\nmouse_capture = true\n"
        );
        let _ = fs::remove_file(target);
    }

    #[cfg(unix)]
    #[test]
    fn atomic_write_sets_private_modes() {
        let target = temp_path("modes").join("session.json");
        atomic_write(&target, b"{}").unwrap();

        let file_mode = fs::metadata(&target).unwrap().permissions().mode() & 0o777;
        assert_eq!(file_mode, 0o600);

        let dir_mode = fs::metadata(target.parent().unwrap())
            .unwrap()
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(dir_mode, 0o700);
    }
}
