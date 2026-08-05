use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema, Default)]
pub struct PingParams {}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct ServerLiveHandoffParams {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub import_exe: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expected_protocol: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expected_version: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct ServerCapabilities {
    pub live_handoff: bool,
    #[serde(default)]
    pub detached_server_daemon: bool,
}

/// Terminal outcome of a `server.config.edit` operation, reported both in the
/// `server.config_edit_finished` completion event and (bounded) in Settings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ConfigEditOutcome {
    /// The editor exited without changing the resolved config file.
    Unchanged,
    /// The file changed and the new bytes were valid TOML; runtime config reloaded.
    Reloaded,
    /// The file changed but the new bytes were invalid or unreadable; the last
    /// valid runtime configuration remains active.
    Invalid,
    /// The editor process exited unsuccessfully and the file did not change.
    EditorFailed,
}
