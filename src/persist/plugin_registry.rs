use std::fs::OpenOptions;
use std::path::{Path, PathBuf};

use tracing::warn;

use super::atomic::{atomic_write, ensure_private_parent_dir};
use crate::api::schema::InstalledPluginInfo;

pub const MANIFEST_UNAVAILABLE_WARNING_PREFIX: &str = "manifest unavailable: ";
const REGISTRY_LOCK_FILE: &str = ".plugins.lock";

fn registry_path() -> PathBuf {
    crate::config::config_dir().join("plugins.json")
}

fn registry_lock_path() -> PathBuf {
    crate::config::config_dir().join(REGISTRY_LOCK_FILE)
}

fn with_registry_lock<T>(operation: impl FnOnce() -> std::io::Result<T>) -> std::io::Result<T> {
    let lock_path = registry_lock_path();
    ensure_private_parent_dir(&lock_path)?;
    let lock = OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(lock_path)?;
    lock.lock()?;
    operation()
}

fn save_json_to_path<T: serde::Serialize + ?Sized>(path: &Path, value: &T) -> std::io::Result<()> {
    ensure_private_parent_dir(path)?;
    let json = serde_json::to_string_pretty(value)?;
    atomic_write(path, json.as_bytes())?;
    Ok(())
}

pub fn save_to_path(path: &Path, plugins: &[InstalledPluginInfo]) -> std::io::Result<()> {
    save_json_to_path(path, plugins)
}

pub fn update<T>(
    mutation: impl FnOnce(&mut Vec<InstalledPluginInfo>) -> T,
) -> std::io::Result<(T, Vec<InstalledPluginInfo>)> {
    with_registry_lock(|| {
        let mut plugins = load_from_path_strict(&registry_path())?;
        let result = mutation(&mut plugins);
        plugins.sort_by(|left, right| left.plugin_id.cmp(&right.plugin_id));
        save_to_path(&registry_path(), &plugins)?;
        Ok((result, plugins))
    })
}

pub fn try_load() -> std::io::Result<Vec<InstalledPluginInfo>> {
    with_registry_lock(|| load_from_path_strict(&registry_path()))
}

/// Load the global registry. Returns an empty vec on failure so a corrupt or
/// missing file never blocks server startup; mutations still use strict reads.
pub fn load() -> Vec<InstalledPluginInfo> {
    match try_load() {
        Ok(plugins) => plugins,
        Err(err) => {
            warn!(path = %registry_path().display(), err = %err, "failed to load plugin registry");
            Vec::new()
        }
    }
}

#[cfg(test)]
pub fn load_from_path(path: &Path) -> Vec<InstalledPluginInfo> {
    match load_from_path_strict(path) {
        Ok(entries) => entries,
        Err(err) => {
            warn!(path = %path.display(), err = %err, "failed to read plugin registry");
            Vec::new()
        }
    }
}

fn load_from_path_strict(path: &Path) -> std::io::Result<Vec<InstalledPluginInfo>> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    let content = std::fs::read_to_string(path)?;
    serde_json::from_str::<Vec<InstalledPluginInfo>>(&content)
        .map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidData, err))
}

/// Re-read each entry's manifest from disk using the provided reload function.
///
/// If the manifest parses successfully, replace cached fields but keep the
/// stored `enabled` flag.  If the file is gone or unparseable, keep the stored
/// entry and append a warning so `plugin.list` surfaces it.
pub fn reload_manifests(
    mut entries: Vec<InstalledPluginInfo>,
    reload_fn: impl Fn(&str, bool) -> Result<InstalledPluginInfo, String>,
) -> Vec<InstalledPluginInfo> {
    for entry in &mut entries {
        entry.warnings.clear();
        match reload_fn(&entry.manifest_path, entry.enabled) {
            Ok(mut fresh) => {
                fresh.enabled = entry.enabled;
                fresh.source = entry.source.clone();
                *entry = fresh;
            }
            Err(warn_msg) => {
                entry
                    .warnings
                    .push(format!("{MANIFEST_UNAVAILABLE_WARNING_PREFIX}{warn_msg}"));
            }
        }
    }
    entries
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_registry_path(name: &str) -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        std::env::temp_dir()
            .join(format!(
                "shepherd-registry-{name}-{}-{nanos}",
                std::process::id()
            ))
            .join("plugins.json")
    }

    fn sample_plugin(id: &str) -> InstalledPluginInfo {
        InstalledPluginInfo {
            plugin_id: id.to_string(),
            name: "Test Plugin".to_string(),
            version: "0.1.0".to_string(),
            min_shepherd_version: crate::build_info::BASE_VERSION.to_string(),
            description: None,
            manifest_path: format!("/tmp/{id}/shepherd-plugin.toml"),
            plugin_root: format!("/tmp/{id}"),
            enabled: true,
            platforms: None,
            build: vec![],
            startup: vec![],
            actions: vec![],
            events: vec![],
            panes: vec![],
            link_handlers: vec![],
            source: Default::default(),
            warnings: vec![],
        }
    }

    #[test]
    fn save_and_load_roundtrip() {
        let path = temp_registry_path("roundtrip");
        let plugins = vec![sample_plugin("example.a"), sample_plugin("example.b")];

        save_to_path(&path, &plugins).unwrap();

        let loaded = load_from_path(&path);
        assert_eq!(loaded.len(), 2);
        let ids: Vec<_> = loaded.iter().map(|p| p.plugin_id.as_str()).collect();
        assert!(ids.contains(&"example.a"));
        assert!(ids.contains(&"example.b"));
    }

    /// Registries written before `InstalledPluginInfo::source` existed have no
    /// `source` key at all; they must still load instead of tripping the strict
    /// read that every mutation goes through.
    #[test]
    fn legacy_registry_without_source_key_still_loads() {
        let path = temp_registry_path("legacy-no-source");
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(
            &path,
            br#"[
  {
    "plugin_id": "example.legacy",
    "name": "Legacy Plugin",
    "version": "0.1.0",
    "manifest_path": "/tmp/example.legacy/shepherd-plugin.toml",
    "plugin_root": "/tmp/example.legacy",
    "enabled": true
  }
]"#,
        )
        .unwrap();

        let loaded = load_from_path_strict(&path).unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].plugin_id, "example.legacy");
        assert_eq!(loaded[0].min_shepherd_version, "");
        assert_eq!(
            loaded[0].source,
            crate::api::schema::PluginSourceInfo::default()
        );
        assert_eq!(
            loaded[0].source.kind,
            crate::api::schema::PluginSourceKind::Local
        );
        assert!(loaded[0].source.resolved_commit.is_none());
        assert!(loaded[0].source.installed_unix_ms.is_none());
    }

    #[test]
    fn missing_file_returns_empty() {
        let path = temp_registry_path("missing");
        let loaded = load_from_path(&path);
        assert!(loaded.is_empty());
    }

    #[test]
    fn corrupt_file_returns_empty_without_panic() {
        let path = temp_registry_path("corrupt");
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        let corrupt = b"this is not valid json {{{{";
        std::fs::write(&path, corrupt).unwrap();

        assert!(load_from_path_strict(&path).is_err());
        assert!(load_from_path(&path).is_empty());
        assert_eq!(std::fs::read(path).unwrap(), corrupt);
    }

    #[test]
    fn reload_manifests_keeps_entry_with_warning_on_missing_manifest() {
        let entry = sample_plugin("example.missing");
        let entries = vec![entry];

        let result = reload_manifests(entries, |path, _enabled| {
            Err(format!("manifest not found at {path}"))
        });

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].plugin_id, "example.missing");
        assert!(!result[0].warnings.is_empty());
        assert!(result[0].warnings[0].contains("manifest not found"));
    }

    #[test]
    fn reload_manifests_uses_fresh_parse_and_keeps_enabled_flag() {
        let mut entry = sample_plugin("example.reload");
        entry.enabled = false;
        entry.source = crate::api::schema::PluginSourceInfo {
            kind: crate::api::schema::PluginSourceKind::Github,
            owner: Some("ogulcancelik".into()),
            repo: Some("shepherd-plugin-examples".into()),
            subdir: Some("worktree-bootstrap".into()),
            requested_ref: Some("main".into()),
            resolved_commit: Some("abc123".into()),
            managed_path: Some("/tmp/shepherd/plugins/github/example.reload".into()),
            installed_unix_ms: Some(42),
        };

        let result = reload_manifests(vec![entry], |_path, _enabled| {
            Ok(InstalledPluginInfo {
                plugin_id: "example.reload".to_string(),
                name: "Fresh Name".to_string(),
                version: "0.2.0".to_string(),
                min_shepherd_version: crate::build_info::BASE_VERSION.to_string(),
                description: Some("refreshed".to_string()),
                manifest_path: "/tmp/example.reload/shepherd-plugin.toml".to_string(),
                plugin_root: "/tmp/example.reload".to_string(),
                enabled: true, // caller would pass stored enabled; fresh parse returns true
                platforms: None,
                build: vec![],
                startup: vec![],
                actions: vec![],
                events: vec![],
                panes: vec![],
                link_handlers: vec![],
                source: Default::default(),
                warnings: vec![],
            })
        });

        assert_eq!(result[0].name, "Fresh Name");
        assert_eq!(result[0].version, "0.2.0");
        // enabled preserved from stored entry
        assert!(!result[0].enabled);
        assert_eq!(
            result[0].source.kind,
            crate::api::schema::PluginSourceKind::Github
        );
        assert_eq!(result[0].source.owner.as_deref(), Some("ogulcancelik"));
        assert!(result[0].warnings.is_empty());
    }

    #[test]
    fn save_replaces_existing_registry_file() {
        let path = temp_registry_path("replace-existing");
        save_to_path(&path, &[sample_plugin("example.first")]).unwrap();
        save_to_path(&path, &[sample_plugin("example.second")]).unwrap();

        let loaded = load_from_path(&path);
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].plugin_id, "example.second");
    }
}
