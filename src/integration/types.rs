use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug)]
pub(crate) struct ClaudeInstallPaths {
    pub hook_path: PathBuf,
    pub settings_path: PathBuf,
}

#[derive(Debug)]
pub(crate) struct CodexInstallPaths {
    pub hook_path: PathBuf,
    pub hooks_path: PathBuf,
    pub config_path: PathBuf,
}

#[derive(Debug)]
pub(crate) struct KimiInstallPaths {
    pub hook_path: PathBuf,
    pub config_path: PathBuf,
}

#[derive(Debug)]
pub(crate) struct CopilotInstallPaths {
    pub hook_path: PathBuf,
    pub settings_path: PathBuf,
}

#[derive(Debug)]
pub(crate) struct DevinInstallPaths {
    pub hook_path: PathBuf,
    pub settings_path: PathBuf,
}

#[derive(Debug)]
pub(crate) struct DroidInstallPaths {
    pub hook_path: PathBuf,
    pub hooks_path: PathBuf,
    pub settings_path: PathBuf,
    pub updated_legacy_hooks: bool,
}

#[derive(Debug)]
pub(crate) struct OpenCodeInstallPaths {
    pub plugin_path: PathBuf,
}

#[derive(Debug)]
pub(crate) struct KiloInstallPaths {
    pub plugin_path: PathBuf,
}

#[derive(Debug)]
pub(crate) struct OmpInstallPaths {
    pub extension_path: PathBuf,
    pub removed_legacy_pi_extension: bool,
}

#[derive(Debug)]
pub(crate) struct HermesInstallPaths {
    pub plugin_dir: PathBuf,
    pub config_path: PathBuf,
}

#[derive(Debug)]
pub(crate) struct QodercliInstallPaths {
    pub hook_path: PathBuf,
    pub settings_path: PathBuf,
}

#[derive(Debug)]
pub(crate) struct CursorInstallPaths {
    pub hook_path: PathBuf,
    pub hooks_path: PathBuf,
}

#[derive(Debug)]
pub(crate) struct CursorUninstallResult {
    pub hook_path: PathBuf,
    pub hooks_path: PathBuf,
    pub removed_hook_file: bool,
    pub updated_hooks: bool,
}

#[derive(Debug)]
pub(crate) struct MastracodeInstallPaths {
    pub hook_path: PathBuf,
    pub hooks_path: PathBuf,
}

#[derive(Debug)]
pub(crate) struct MastracodeUninstallResult {
    pub hook_path: PathBuf,
    pub hooks_path: PathBuf,
    pub removed_hook_file: bool,
    pub updated_hooks: bool,
}

#[derive(Debug)]
pub(crate) struct GrokInstallPaths {
    pub hook_path: PathBuf,
    pub config_path: PathBuf,
}

#[derive(Debug)]
pub(crate) struct GrokUninstallResult {
    pub hook_path: PathBuf,
    pub config_path: PathBuf,
    pub removed_hook_file: bool,
    pub removed_config_file: bool,
}

#[derive(Debug)]
pub(crate) struct QodercliUninstallResult {
    pub hook_path: PathBuf,
    pub settings_path: PathBuf,
    pub removed_hook_file: bool,
    pub updated_settings: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct IntegrationStatus {
    pub target: crate::api::schema::IntegrationTarget,
    pub path: PathBuf,
    pub state: IntegrationStatusKind,
    pub installed_version: Option<u32>,
    pub expected_version: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum IntegrationStatusKind {
    NotInstalled,
    Current,
    Outdated,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct IntegrationRecommendation {
    pub target: crate::api::schema::IntegrationTarget,
    pub label: &'static str,
    pub command: &'static str,
    /// Whether this target is supported on the current platform at all
    /// (independent of whether its CLI/plugin is currently found).
    pub supported: bool,
    pub available: bool,
    pub path: PathBuf,
    pub state: IntegrationStatusKind,
    pub installed_version: Option<u32>,
    pub expected_version: u32,
}

/// One mutating action a Settings row can offer for a target, ordered by
/// `IntegrationRecommendation::available_actions` to match the row's state:
/// absent -> Install, outdated -> Update (plus Uninstall), installed -> Uninstall.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum IntegrationAction {
    Install,
    Update,
    Uninstall,
}

impl IntegrationAction {
    pub fn label(self) -> &'static str {
        match self {
            Self::Install => "install",
            Self::Update => "update",
            Self::Uninstall => "uninstall",
        }
    }
}

impl IntegrationRecommendation {
    pub fn needs_install(&self) -> bool {
        self.state == IntegrationStatusKind::Outdated
            || (self.available && self.state == IntegrationStatusKind::NotInstalled)
    }

    pub fn status_label(&self) -> &'static str {
        match (self.available, self.state) {
            (_, IntegrationStatusKind::Current) => "installed",
            (_, IntegrationStatusKind::Outdated) => "update available",
            (true, IntegrationStatusKind::NotInstalled) => "available",
            (false, IntegrationStatusKind::NotInstalled) => "not found",
        }
    }

    /// Actions offered for this target's current state; empty when the
    /// target is unsupported or simply not found (see `unavailable_reason`).
    pub fn available_actions(&self) -> Vec<IntegrationAction> {
        if !self.supported {
            return Vec::new();
        }
        match self.state {
            IntegrationStatusKind::NotInstalled if self.available => {
                vec![IntegrationAction::Install]
            }
            IntegrationStatusKind::NotInstalled => Vec::new(),
            IntegrationStatusKind::Outdated => {
                vec![IntegrationAction::Update, IntegrationAction::Uninstall]
            }
            IntegrationStatusKind::Current => vec![IntegrationAction::Uninstall],
        }
    }

    /// Explanation shown when `available_actions()` is empty, so an
    /// unsupported/unfound target stays visible without a dead-end control.
    pub fn unavailable_reason(&self) -> Option<&'static str> {
        if !self.supported {
            Some("not supported on this platform")
        } else if !self.available && self.state == IntegrationStatusKind::NotInstalled {
            Some("not found on PATH")
        } else {
            None
        }
    }
}

#[derive(Debug)]
pub(crate) struct PiUninstallResult {
    pub extension_path: PathBuf,
    pub removed_extension: bool,
}

#[derive(Debug)]
pub(crate) struct OmpUninstallResult {
    pub extension_path: PathBuf,
    pub removed_extension: bool,
}

#[derive(Debug)]
pub(crate) struct ClaudeUninstallResult {
    pub hook_path: PathBuf,
    pub settings_path: PathBuf,
    pub removed_hook_file: bool,
    pub updated_settings: bool,
}

#[derive(Debug)]
pub(crate) struct CodexUninstallResult {
    pub hook_path: PathBuf,
    pub hooks_path: PathBuf,
    pub config_path: PathBuf,
    pub removed_hook_file: bool,
    pub updated_hooks: bool,
}

#[derive(Debug)]
pub(crate) struct KimiUninstallResult {
    pub hook_path: PathBuf,
    pub config_path: PathBuf,
    pub removed_hook_file: bool,
    pub updated_config: bool,
}

#[derive(Debug)]
pub(crate) struct CopilotUninstallResult {
    pub hook_path: PathBuf,
    pub settings_path: PathBuf,
    pub removed_hook_file: bool,
    pub updated_settings: bool,
}

#[derive(Debug)]
pub(crate) struct DevinUninstallResult {
    pub hook_path: PathBuf,
    pub settings_path: PathBuf,
    pub removed_hook_file: bool,
    pub updated_settings: bool,
}

#[derive(Debug)]
pub(crate) struct DroidUninstallResult {
    pub hook_path: PathBuf,
    pub hooks_path: PathBuf,
    pub settings_path: PathBuf,
    pub removed_hook_file: bool,
    pub updated_hooks: bool,
    pub updated_settings: bool,
}

#[derive(Debug)]
pub(crate) struct OpenCodeUninstallResult {
    pub plugin_path: PathBuf,
    pub removed_plugin: bool,
}

#[derive(Debug)]
pub(crate) struct KiloUninstallResult {
    pub plugin_path: PathBuf,
    pub removed_plugin: bool,
}

#[derive(Debug)]
pub(crate) struct HermesUninstallResult {
    pub plugin_dir: PathBuf,
    pub config_path: PathBuf,
    pub removed_plugin_dir: bool,
    pub updated_config: bool,
}

/// Exact-source runtime observation for one integration target: how many
/// terminals currently report state through its canonical source, and how
/// many carry that source's session identity (session-identity-only
/// integrations like Hermes never populate `reporting_terminals`). Absence
/// is zero on both counts, never a health verdict.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct IntegrationObservation {
    pub reporting_terminals: usize,
    pub session_identity_terminals: usize,
}

impl IntegrationObservation {
    pub fn is_absent(&self) -> bool {
        self.reporting_terminals == 0 && self.session_identity_terminals == 0
    }
}

/// Aggregates exact canonical-source observations across all terminals in
/// one pass. Only a `TerminalState::agent_state_evidence()` reporter or a
/// session-ref source that exactly matches a registered `IntegrationTarget`
/// contributes; unmatched/custom sources contribute to no target and never
/// surface a session identifier here.
pub(crate) fn integration_observations<'a>(
    terminals: impl Iterator<Item = &'a crate::terminal::TerminalState>,
) -> HashMap<crate::api::schema::IntegrationTarget, IntegrationObservation> {
    let mut observations: HashMap<crate::api::schema::IntegrationTarget, IntegrationObservation> =
        HashMap::new();
    for terminal in terminals {
        if let Some(source) = terminal
            .agent_state_evidence()
            .and_then(|evidence| evidence.reporter)
            .map(|reporter| reporter.source)
        {
            if let Some(target) = super::registry::integration_target_for_source(&source) {
                observations.entry(target).or_default().reporting_terminals += 1;
            }
        }
        if let Some(source) = terminal_session_identity_source(terminal) {
            if let Some(target) = super::registry::integration_target_for_source(&source) {
                observations
                    .entry(target)
                    .or_default()
                    .session_identity_terminals += 1;
            }
        }
    }
    observations
}

/// Session identity alone, independent of current reporting authority —
/// mirrors `terminal_agent_session_info`'s hook-authority-then-persisted
/// precedence in `src/app/creation.rs` without exposing the session
/// identifier itself, only its owning source.
fn terminal_session_identity_source(terminal: &crate::terminal::TerminalState) -> Option<String> {
    if let Some(authority) = terminal.hook_authority.as_ref() {
        if authority.session_ref.is_some() {
            return Some(authority.source.clone());
        }
    }
    terminal
        .persisted_agent_session
        .as_ref()
        .map(|session| session.source.clone())
}
