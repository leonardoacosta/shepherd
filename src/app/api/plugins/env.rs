use crate::api::schema::InstalledPluginInfo;

pub(super) fn plugin_config_dir(plugin_id: &str) -> std::path::PathBuf {
    crate::plugin_paths::plugin_config_dir(plugin_id)
}

pub(super) fn plugin_state_dir(plugin_id: &str) -> std::path::PathBuf {
    crate::plugin_paths::plugin_state_dir(plugin_id)
}

pub(super) fn ensure_plugin_user_dirs(plugin: &InstalledPluginInfo) -> std::io::Result<()> {
    crate::plugin_paths::ensure_plugin_user_dirs(&plugin.plugin_id)
}

pub(super) fn plugin_path_env(plugin: &InstalledPluginInfo) -> Vec<(String, String)> {
    let config_dir = plugin_config_dir(&plugin.plugin_id);
    let state_dir = plugin_state_dir(&plugin.plugin_id);

    vec![
        (
            "SHEPHERD_PLUGIN_ROOT".to_string(),
            plugin.plugin_root.clone(),
        ),
        (
            "SHEPHERD_PLUGIN_CONFIG_DIR".to_string(),
            config_dir.display().to_string(),
        ),
        (
            "SHEPHERD_PLUGIN_STATE_DIR".to_string(),
            state_dir.display().to_string(),
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plugin_path_environment_uses_only_shepherd_names() {
        let legacy_plugin_prefix = ["HE", "RDR_PLUGIN_"].concat();
        let plugin = serde_json::from_value::<InstalledPluginInfo>(serde_json::json!({
            "plugin_id": "example.shepherd",
            "name": "Shepherd example",
            "version": "0.1.0",
            "min_shepherd_version": "0.1.0",
            "manifest_path": "/tmp/example/shepherd-plugin.toml",
            "plugin_root": "/tmp/example",
            "enabled": true
        }))
        .expect("deserialize plugin fixture");

        let env = plugin_path_env(&plugin);
        assert!(env
            .iter()
            .all(|(name, _)| name.starts_with("SHEPHERD_PLUGIN_")));
        assert!(env
            .iter()
            .all(|(name, _)| !name.starts_with(&legacy_plugin_prefix)));
        assert!(env.iter().any(|(name, _)| name == "SHEPHERD_PLUGIN_ROOT"));
        assert!(env
            .iter()
            .any(|(name, _)| name == "SHEPHERD_PLUGIN_CONFIG_DIR"));
        assert!(env
            .iter()
            .any(|(name, _)| name == "SHEPHERD_PLUGIN_STATE_DIR"));
    }
}
