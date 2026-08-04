use crate::api::schema::{
    IntegrationInstallResult, IntegrationTarget, IntegrationUninstallResult, ResponseResult,
};
use crate::app::state::{IntegrationOperationKind, IntegrationOperationRecord};
use crate::app::App;

use super::responses::{encode_error, encode_success};

impl App {
    pub(super) fn handle_integration_install(
        &mut self,
        id: String,
        params: crate::api::schema::IntegrationInstallParams,
    ) -> String {
        let target = params.target;
        if self.state.settings.integration_manager.operation_in_flight {
            return encode_error(
                id,
                "integration_operation_in_progress",
                "another integration operation is already running".to_string(),
            );
        }
        match self.run_integration_operation(target, IntegrationOperationKind::Install, |t| {
            crate::integration::install_target(t)
        }) {
            Ok(messages) => encode_success(
                id,
                ResponseResult::IntegrationInstall {
                    target,
                    details: IntegrationInstallResult { messages },
                },
            ),
            Err(err) => encode_error(id, "integration_install_failed", err),
        }
    }

    pub(super) fn handle_integration_uninstall(
        &mut self,
        id: String,
        params: crate::api::schema::IntegrationUninstallParams,
    ) -> String {
        let target = params.target;
        if self.state.settings.integration_manager.operation_in_flight {
            return encode_error(
                id,
                "integration_operation_in_progress",
                "another integration operation is already running".to_string(),
            );
        }
        match self.run_integration_operation(target, IntegrationOperationKind::Uninstall, |t| {
            crate::integration::uninstall_target(t)
        }) {
            Ok(messages) => encode_success(
                id,
                ResponseResult::IntegrationUninstall {
                    target,
                    details: IntegrationUninstallResult { messages },
                },
            ),
            Err(err) => encode_error(id, "integration_uninstall_failed", err),
        }
    }

    /// TUI entry points for the per-target Settings action menu. Thin
    /// wrappers over `run_integration_operation` so the wire API handlers
    /// above and the row-level actions share one single-flight guard, one
    /// history record shape, and one status-refresh path.
    pub(crate) fn install_integration_target(&mut self, target: IntegrationTarget) {
        let _ = self.run_integration_operation(target, IntegrationOperationKind::Install, |t| {
            crate::integration::install_target(t)
        });
    }

    pub(crate) fn update_integration_target(&mut self, target: IntegrationTarget) {
        let _ = self.run_integration_operation(target, IntegrationOperationKind::Update, |t| {
            crate::integration::install_target(t)
        });
    }

    pub(crate) fn uninstall_integration_target(&mut self, target: IntegrationTarget) {
        let _ = self.run_integration_operation(target, IntegrationOperationKind::Uninstall, |t| {
            crate::integration::uninstall_target(t)
        });
    }

    /// Run one target-scoped integration mutation: guards the global
    /// single-flight flag, records a target-keyed history entry (kept for
    /// the scrollable detail view) plus a bounded summary line, refreshes
    /// cached recommendations so status/badges reflect the outcome, and
    /// returns the raw messages/error for wire-API callers.
    fn run_integration_operation(
        &mut self,
        target: IntegrationTarget,
        kind: IntegrationOperationKind,
        op: impl FnOnce(IntegrationTarget) -> std::io::Result<Vec<String>>,
    ) -> Result<Vec<String>, String> {
        self.state.settings.integration_manager.operation_in_flight = true;
        let label = crate::integration::integration_target_label(target);
        let result = op(target);
        self.state.settings.integration_manager.operation_in_flight = false;
        self.refresh_integration_recommendations();

        match result {
            Ok(messages) => {
                let verb = match kind {
                    IntegrationOperationKind::Install => "installed",
                    IntegrationOperationKind::Update => "updated",
                    IntegrationOperationKind::Uninstall => "uninstalled",
                };
                let summary = format!("{verb} {label}");
                self.record_integration_result(target, kind, true, summary);
                for warning in messages.iter().filter(|message| {
                    message.starts_with(crate::integration::INSTALL_WARNING_PREFIX)
                }) {
                    self.record_integration_result(target, kind, true, warning.clone());
                }
                self.state.mark_session_dirty();
                Ok(messages)
            }
            Err(err) => {
                let message = format!("{label}: {err}");
                self.record_integration_result(target, kind, false, message.clone());
                Err(message)
            }
        }
    }

    fn record_integration_result(
        &mut self,
        target: IntegrationTarget,
        kind: IntegrationOperationKind,
        success: bool,
        message: String,
    ) {
        self.state
            .integration_install_messages
            .push(message.clone());
        self.state
            .settings
            .integration_manager
            .history
            .push(IntegrationOperationRecord {
                target,
                kind,
                success,
                message,
            });
    }
}
