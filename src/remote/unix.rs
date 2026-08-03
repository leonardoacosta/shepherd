//! Remote thin-client launcher over SSH command stdio.

use std::fs;
use std::io::{self, IsTerminal, Write as _};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

use serde::Deserialize;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

const BRIDGE_ACCEPT_POLL: Duration = Duration::from_millis(50);
const BRIDGE_SOCKET_PERMISSION_MODE: u32 = 0o600;
const REMOTE_SERVER_SHUTDOWN_CONFIRM_TIMEOUT: Duration = Duration::from_secs(5);
const REMOTE_SERVER_SHUTDOWN_POLL_INTERVAL: Duration = Duration::from_millis(100);
const CURRENT_PROTOCOL: u32 = crate::protocol::PROTOCOL_VERSION;
const REMOTE_INSTALLER: &str = "shepherd-install";
const SSH_CONTROL_SOCKET_NAME: &str = "ctl";
pub(crate) const REATTACH_COMMAND_ENV_VAR: &str = "SHEPHERD_REATTACH_COMMAND";

pub(crate) const REMOTE_KEYBINDINGS_ENV_VAR: &str = "SHEPHERD_REMOTE_KEYBINDINGS";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RemoteKeybindings {
    Local,
    Server,
}

impl RemoteKeybindings {
    fn parse(value: &str) -> Result<Self, String> {
        match value {
            "local" => Ok(Self::Local),
            "server" => Ok(Self::Server),
            _ => Err("--remote-keybindings must be 'local' or 'server'".to_string()),
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Local => "local",
            Self::Server => "server",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RemoteLaunch {
    pub(crate) target: String,
    pub(crate) keybindings: RemoteKeybindings,
    pub(crate) live_handoff: bool,
}

pub(crate) fn extract_remote_args(
    args: &[String],
) -> Result<(Vec<String>, Option<RemoteLaunch>), String> {
    let mut cleaned = Vec::with_capacity(args.len());
    if let Some(program) = args.first() {
        cleaned.push(program.clone());
    }

    let mut remote_target = None;
    let mut keybindings = RemoteKeybindings::Local;
    let mut keybindings_seen = false;
    let mut live_handoff = false;
    let mut index = 1;
    while index < args.len() {
        let arg = &args[index];
        if arg == "--" {
            cleaned.extend_from_slice(&args[index..]);
            break;
        }
        if arg == "--handoff" {
            live_handoff = true;
            index += 1;
            continue;
        }
        if arg == "--remote" {
            if remote_target.is_some() {
                return Err("--remote can only be specified once".to_string());
            }
            let Some(value) = args.get(index + 1) else {
                return Err("missing value for --remote".to_string());
            };
            remote_target = Some(validate_remote_target(value)?.to_owned());
            index += 2;
            continue;
        }
        if let Some(value) = arg.strip_prefix("--remote=") {
            if remote_target.is_some() {
                return Err("--remote can only be specified once".to_string());
            }
            remote_target = Some(validate_remote_target(value)?.to_owned());
            index += 1;
            continue;
        }
        if arg == "--remote-keybindings" {
            if keybindings_seen {
                return Err("--remote-keybindings can only be specified once".to_string());
            }
            let Some(value) = args.get(index + 1) else {
                return Err("missing value for --remote-keybindings".to_string());
            };
            keybindings = RemoteKeybindings::parse(value)?;
            keybindings_seen = true;
            index += 2;
            continue;
        }
        if let Some(value) = arg.strip_prefix("--remote-keybindings=") {
            if keybindings_seen {
                return Err("--remote-keybindings can only be specified once".to_string());
            }
            keybindings = RemoteKeybindings::parse(value)?;
            keybindings_seen = true;
            index += 1;
            continue;
        }

        cleaned.push(arg.clone());
        index += 1;
    }

    let remote = remote_target.map(|target| RemoteLaunch {
        target,
        keybindings,
        live_handoff,
    });
    if remote.is_none() && keybindings_seen {
        return Err("--remote-keybindings requires --remote".to_string());
    }
    if remote.is_none() && live_handoff {
        cleaned.push("--handoff".to_string());
    }

    Ok((cleaned, remote))
}

fn validate_remote_target(target: &str) -> Result<&str, String> {
    if target.is_empty() {
        return Err("missing value for --remote".to_string());
    }
    if target.starts_with('-') {
        return Err("--remote target must not start with '-'".to_string());
    }
    Ok(target)
}

pub(crate) fn run_remote(remote: RemoteLaunch) -> io::Result<()> {
    let session_name = crate::session::active_name()
        .unwrap_or_else(|| crate::session::DEFAULT_SESSION_NAME.to_string());
    let local_socket = local_forward_socket_path(&remote.target, &session_name);
    let program = std::env::args()
        .next()
        .unwrap_or_else(|| "shepherd".to_string());
    let reattach_command = reattach_command(
        &program,
        &remote.target,
        &session_name,
        remote.keybindings,
        remote.live_handoff,
    );
    let manage_ssh_config = crate::config::Config::load()
        .config
        .remote
        .manage_ssh_config;
    let remote_ssh = RemoteSsh::new(remote.target.clone(), manage_ssh_config);
    let prepared_remote = prepare_remote_shepherd(&remote_ssh, remote.live_handoff)?;
    ensure_remote_server_ready(
        &remote_ssh,
        &prepared_remote.remote_shepherd,
        prepared_remote.installed_or_replaced,
        prepared_remote.stop_after_install_approved,
        remote.live_handoff,
    )?;

    let _bridge = SshStdioBridge::start(
        remote.target,
        prepared_remote.remote_shepherd,
        local_socket.clone(),
        session_name,
        remote_ssh.options(),
    )?;

    run_client_process(&local_socket, &reattach_command, remote.keybindings)
}

pub(crate) fn run_remote_check(target: &str) -> io::Result<super::RemoteCheck> {
    validate_remote_target(target)
        .map_err(|message| io::Error::new(io::ErrorKind::InvalidInput, message))?;
    let manage_ssh_config = crate::config::Config::load()
        .config
        .remote
        .manage_ssh_config;
    let ssh = RemoteSsh::new(target.to_string(), manage_ssh_config);
    let Some((remote, handshake)) = discover_matching_remote(&ssh)? else {
        return Err(io::Error::other(format!(
            "{target} does not provide Shepherd {} with protocol {CURRENT_PROTOCOL}",
            current_version()
        )));
    };

    Ok(super::RemoteCheck {
        ok: true,
        target: target.to_string(),
        product: handshake.product,
        version: handshake.version,
        protocol: handshake.protocol,
        binary: handshake.binary.unwrap_or(remote.shell_path),
    })
}

pub(crate) fn run_remote_client_bridge() -> io::Result<()> {
    ensure_remote_server_running()?;

    let socket_path = crate::server::socket_paths::client_socket_path();
    let stream = UnixStream::connect(&socket_path).map_err(|err| {
        io::Error::new(
            err.kind(),
            format!(
                "failed to connect to remote Shepherd client socket {}: {err}",
                socket_path.display()
            ),
        )
    })?;

    let mut stdout = io::stdout().lock();
    let mut socket_to_stdout = stream.try_clone()?;
    let mut stdin_to_socket = stream;

    let _upload = thread::spawn(move || {
        let mut stdin = io::stdin();
        let _ = copy_flush(&mut stdin, &mut stdin_to_socket);
        let _ = stdin_to_socket.shutdown(std::net::Shutdown::Write);
    });

    copy_flush(&mut socket_to_stdout, &mut stdout).map(|_| ())
}

fn ensure_remote_server_running() -> io::Result<()> {
    let socket_path = crate::server::socket_paths::client_socket_path();
    if crate::server::autodetect::is_server_listening() {
        let status = crate::api::read_runtime_status_at(
            &crate::api::socket_path(),
            Duration::from_millis(500),
        )?
        .ok_or_else(|| io::Error::other("remote server status API is unavailable"))?;
        if status.protocol == Some(CURRENT_PROTOCOL) {
            return Ok(());
        }
        return Err(io::Error::other(
            "remote shepherd server must restart before this bridge can attach; rerun `shepherd --remote` from an interactive terminal to approve stopping it",
        ));
    }

    crate::server::autodetect::spawn_server_daemon()?;
    crate::server::autodetect::wait_for_server_socket(&socket_path, Duration::from_secs(5))
}

#[derive(Debug, Clone)]
struct RemoteShepherd {
    shell_path: String,
}

impl RemoteShepherd {
    fn managed() -> Self {
        Self {
            shell_path: "\"$HOME/.local/bin/shepherd\"".to_string(),
        }
    }

    fn with_shell_path(mut self, shell_path: String) -> Self {
        self.shell_path = shell_path;
        self
    }
}

fn current_version() -> String {
    crate::build_info::version()
}

struct PreparedRemoteShepherd {
    remote_shepherd: RemoteShepherd,
    installed_or_replaced: bool,
    stop_after_install_approved: bool,
}

#[derive(Clone)]
struct ManagedSshOptions {
    config_path: PathBuf,
    control_path: PathBuf,
}

struct ManagedSshConfig {
    options: ManagedSshOptions,
}

impl Drop for ManagedSshConfig {
    fn drop(&mut self) {
        if let Some(dir) = self.options.config_path.parent() {
            let _ = fs::remove_dir_all(dir);
        }
    }
}

struct RemoteSsh {
    target: String,
    managed_config: Option<ManagedSshConfig>,
}

impl RemoteSsh {
    fn new(target: String, manage_ssh_config: bool) -> Self {
        let managed_config = if manage_ssh_config {
            write_managed_ssh_config()
                .inspect_err(|err| {
                    tracing::debug!(%err, "could not write managed ssh config; using plain ssh");
                })
                .ok()
        } else {
            None
        };

        Self {
            target,
            managed_config,
        }
    }

    fn target(&self) -> &str {
        &self.target
    }

    fn options(&self) -> Option<&ManagedSshOptions> {
        self.managed_config.as_ref().map(|config| &config.options)
    }

    fn command(&self) -> Command {
        let mut command = self.base_command();
        command.arg("-T").arg(&self.target);
        command
    }

    fn base_command(&self) -> Command {
        let mut command = Command::new("ssh");
        apply_managed_ssh_options(&mut command, self.options());
        command
    }

    fn sh_output(&self, script: &str) -> io::Result<Output> {
        let mut child = self
            .command()
            .arg("/bin/sh -s")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        let write_result = if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(script.as_bytes())
        } else {
            Err(io::Error::new(
                io::ErrorKind::BrokenPipe,
                "ssh bootstrap stdin missing",
            ))
        };
        let output = child.wait_with_output()?;
        write_result?;
        Ok(output)
    }

    fn user_shell_output(&self, command: &str) -> io::Result<Output> {
        self.command().arg(command).output()
    }

    fn run_native_installer(&self) -> io::Result<()> {
        let command = native_installer_command();
        let output = self.user_shell_output(&command)?;
        if output.status.success() {
            return Ok(());
        }

        Err(command_failed(
            "peer-native Shepherd installer failed or is not configured",
            &output,
        ))
    }
}

fn native_installer_command() -> String {
    format!(
        "command -v {0} >/dev/null 2>&1 && exec {0}",
        REMOTE_INSTALLER
    )
}

impl Drop for RemoteSsh {
    fn drop(&mut self) {
        if self.managed_config.is_none() {
            return;
        }

        let _ = self
            .base_command()
            .arg("-O")
            .arg("exit")
            .arg("-o")
            .arg("BatchMode=yes")
            .arg(&self.target)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    }
}

fn apply_managed_ssh_options(command: &mut Command, options: Option<&ManagedSshOptions>) {
    let Some(options) = options else {
        return;
    };

    command
        .arg("-F")
        .arg(&options.config_path)
        .arg("-S")
        .arg(&options.control_path)
        .arg("-o")
        .arg("ControlMaster=auto")
        .arg("-o")
        .arg("ControlPersist=yes");
}

fn prepare_remote_shepherd(
    ssh: &RemoteSsh,
    live_handoff_enabled: bool,
) -> io::Result<PreparedRemoteShepherd> {
    if let Some((remote_shepherd, _handshake)) = discover_matching_remote(ssh)? {
        return Ok(PreparedRemoteShepherd {
            remote_shepherd,
            installed_or_replaced: false,
            stop_after_install_approved: false,
        });
    }

    let candidates = remote_binary_candidates(ssh)?;
    let identity_candidate = first_shepherd_identity(ssh, &candidates)?;

    let mut stop_after_install_approved = false;
    if let Some(status_probe_shepherd) = identity_candidate.as_ref() {
        stop_after_install_approved = confirm_remote_install_with_running_server(
            ssh,
            status_probe_shepherd,
            live_handoff_enabled,
        )?;
    }
    confirm_remote_install(ssh.target())?;
    ssh.run_native_installer()?;

    let Some((remote_shepherd, _handshake)) = discover_matching_remote(ssh)? else {
        return Err(io::Error::other(format!(
            "{REMOTE_INSTALLER} completed on {}, but no Shepherd {} with protocol {CURRENT_PROTOCOL} was found",
            ssh.target(),
            current_version()
        )));
    };

    Ok(PreparedRemoteShepherd {
        remote_shepherd,
        installed_or_replaced: true,
        stop_after_install_approved,
    })
}

fn remote_binary_candidates(ssh: &RemoteSsh) -> io::Result<Vec<RemoteShepherd>> {
    let managed = RemoteShepherd::managed();
    let mut candidates = Vec::new();

    if let Some(path_candidate) = remote_binary_on_path_any(ssh, &managed)? {
        push_if_new_remote_binary_candidate(&mut candidates, path_candidate);
    }

    let output = ssh.sh_output(&known_remote_binary_candidate_script())?;
    if !output.status.success() {
        return Err(command_failed("remote binary discovery failed", &output));
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    for candidate in remote_shepherds_from_path_discovery(&managed, &stdout) {
        push_if_new_remote_binary_candidate(&mut candidates, candidate);
    }
    push_if_new_remote_binary_candidate(&mut candidates, managed);

    Ok(candidates)
}

fn push_if_new_remote_binary_candidate(
    candidates: &mut Vec<RemoteShepherd>,
    candidate: RemoteShepherd,
) {
    if !candidates
        .iter()
        .any(|existing| existing.shell_path == candidate.shell_path)
    {
        candidates.push(candidate);
    }
}

fn known_remote_binary_candidate_script() -> String {
    let mut script = String::from(
        r#"home=${HOME:-}
user=${USER:-}
version="#,
    );
    script.push_str(&shell_quote(&current_version()));
    script.push_str(
        r#"
emit() {
    path=$1
    if [ -n "$path" ] && [ -x "$path" ]; then
        printf '%s\n' "$path"
    fi
}
if [ -n "$home" ]; then
    emit "$home/.local/bin/shepherd"
fi
emit "/opt/homebrew/bin/shepherd"
emit "/usr/local/bin/shepherd"
emit "/home/linuxbrew/.linuxbrew/bin/shepherd"
"#,
    );
    script.push_str(
        r#"if [ -n "$home" ]; then
    emit "$home/.local/share/mise/installs/shepherd/$version/bin/shepherd"
    emit "$home/.local/share/mise/installs/shepherd/$version/shepherd"
    emit "$home/.local/share/mise/installs/github-ogulcancelik-shepherd/$version/shepherd"
    emit "$home/.nix-profile/bin/shepherd"
fi
if [ -n "$user" ]; then
    emit "/etc/profiles/per-user/$user/bin/shepherd"
fi
emit "/nix/var/nix/profiles/default/bin/shepherd"
emit "/run/current-system/sw/bin/shepherd"
"#,
    );

    script
}

fn remote_binary_on_path_any(
    ssh: &RemoteSsh,
    remote_shepherd: &RemoteShepherd,
) -> io::Result<Option<RemoteShepherd>> {
    let output = ssh.user_shell_output("command -v shepherd")?;
    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        if let Some(candidate) = remote_shepherd_from_path_discovery(remote_shepherd, &stdout) {
            return Ok(Some(candidate));
        }
    }

    // Non-POSIX login shells such as xonsh reject `command -v`; retry through
    // /bin/sh while retaining the login-shell probe for shell-initialized PATHs.
    let output = ssh.sh_output("command -v shepherd\n")?;
    if !output.status.success() {
        return Ok(None);
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(remote_shepherd_from_path_discovery(
        remote_shepherd,
        &stdout,
    ))
}

fn remote_shepherds_from_path_discovery(
    remote_shepherd: &RemoteShepherd,
    stdout: &str,
) -> Vec<RemoteShepherd> {
    stdout
        .lines()
        .filter_map(|path| remote_shepherd_from_path(remote_shepherd, path))
        .collect()
}

fn remote_shepherd_from_path_discovery(
    remote_shepherd: &RemoteShepherd,
    stdout: &str,
) -> Option<RemoteShepherd> {
    stdout
        .lines()
        .find_map(|path| remote_shepherd_from_path(remote_shepherd, path))
}

fn remote_shepherd_from_path(
    remote_shepherd: &RemoteShepherd,
    path: &str,
) -> Option<RemoteShepherd> {
    let path = path.trim();
    if !path.starts_with('/') {
        return None;
    }
    if is_mise_shim_path(path) {
        return None;
    }
    Some(remote_shepherd.clone().with_shell_path(shell_quote(path)))
}

fn is_mise_shim_path(path: &str) -> bool {
    path.ends_with("/mise/shims/shepherd")
}

fn remote_handshake(
    ssh: &RemoteSsh,
    remote_shepherd: &RemoteShepherd,
) -> io::Result<Option<RemoteClientStatusJson>> {
    let command = format!(
        "test -x {0} && {0} status client --json",
        remote_shepherd.shell_path
    );
    let output = ssh.sh_output(&command)?;
    if !output.status.success() {
        return Ok(None);
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(parse_client_status_json(stdout.trim()))
}

fn handshake_matches(status: &RemoteClientStatusJson) -> bool {
    status.version == current_version() && status.protocol == CURRENT_PROTOCOL
}

fn discover_matching_remote(
    ssh: &RemoteSsh,
) -> io::Result<Option<(RemoteShepherd, RemoteClientStatusJson)>> {
    for candidate in remote_binary_candidates(ssh)? {
        if let Some(handshake) = remote_handshake(ssh, &candidate)? {
            if handshake_matches(&handshake) {
                return Ok(Some((candidate, handshake)));
            }
        }
    }
    Ok(None)
}

fn first_shepherd_identity(
    ssh: &RemoteSsh,
    candidates: &[RemoteShepherd],
) -> io::Result<Option<RemoteShepherd>> {
    for candidate in candidates {
        if remote_handshake(ssh, candidate)?.is_some() {
            return Ok(Some(candidate.clone()));
        }
    }
    Ok(None)
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum RemoteServerStatus {
    Running {
        version: Option<String>,
        protocol: Option<u32>,
        live_handoff: bool,
        detached_server_daemon: bool,
    },
    NotRunning,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RemoteServerRestartReason {
    ProtocolMismatch,
    DaemonDetachMissing,
    BinaryUpdated,
    VersionMismatch,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RemoteInstallRunningServerPlan {
    KeepRunning,
    LiveHandoff,
    StopRequired(RemoteServerRestartReason),
}

fn ensure_remote_server_ready(
    ssh: &RemoteSsh,
    remote_shepherd: &RemoteShepherd,
    remote_binary_changed: bool,
    stop_after_install_approved: bool,
    live_handoff_enabled: bool,
) -> io::Result<()> {
    let status = remote_server_status(ssh, remote_shepherd)?;
    let RemoteServerStatus::Running {
        version,
        protocol,
        live_handoff,
        detached_server_daemon,
    } = status
    else {
        return Ok(());
    };

    let Some(reason) = remote_server_restart_reason(
        version.as_deref(),
        protocol,
        detached_server_daemon,
        remote_binary_changed,
    ) else {
        return Ok(());
    };

    if live_handoff_enabled && live_handoff {
        match live_handoff_remote_server(ssh, remote_shepherd) {
            Ok(()) => return Ok(()),
            Err(err) => {
                eprintln!("remote live handoff failed: {err}");
                eprintln!("falling back to remote server restart.");
            }
        }
    }

    if stop_after_install_approved {
        stop_remote_server(ssh, remote_shepherd)?;
        return Ok(());
    }

    if confirm_remote_server_stop(ssh.target(), version.as_deref(), protocol, reason)? {
        stop_remote_server(ssh, remote_shepherd)?;
    }
    Ok(())
}

fn remote_server_restart_reason(
    version: Option<&str>,
    protocol: Option<u32>,
    detached_server_daemon: bool,
    remote_binary_changed: bool,
) -> Option<RemoteServerRestartReason> {
    if protocol != Some(CURRENT_PROTOCOL) {
        return Some(RemoteServerRestartReason::ProtocolMismatch);
    }
    if !detached_server_daemon {
        return Some(RemoteServerRestartReason::DaemonDetachMissing);
    }
    if version != Some(current_version().as_str()) {
        return Some(RemoteServerRestartReason::VersionMismatch);
    }
    if remote_binary_changed {
        return Some(RemoteServerRestartReason::BinaryUpdated);
    }
    None
}

fn confirm_remote_install_with_running_server(
    ssh: &RemoteSsh,
    remote_shepherd: &RemoteShepherd,
    live_handoff_enabled: bool,
) -> io::Result<bool> {
    let target = ssh.target();
    let status = match remote_server_status(ssh, remote_shepherd) {
        Ok(status) => status,
        Err(err) => {
            if !io::stdin().is_terminal() {
                return Err(io::Error::other(format!(
                    "could not inspect the running remote shepherd server on {target} before installing: {err}; run from an interactive terminal to approve updating the remote binary"
                )));
            }
            eprintln!(
                "could not inspect the running remote shepherd server on {target} before installing: {err}"
            );
            eprint!("continue installing the remote shepherd binary? [y/N] ");
            io::stderr().flush()?;

            let mut answer = String::new();
            io::stdin().read_line(&mut answer)?;
            let answer = answer.trim().to_ascii_lowercase();
            if answer != "y" && answer != "yes" {
                return Err(io::Error::new(
                    io::ErrorKind::Interrupted,
                    "remote shepherd install cancelled",
                ));
            }
            return Ok(false);
        }
    };
    let RemoteServerStatus::Running {
        version,
        protocol,
        live_handoff,
        detached_server_daemon,
    } = &status
    else {
        return Ok(false);
    };
    let plan = remote_install_running_server_plan(
        version.as_deref(),
        *protocol,
        *detached_server_daemon,
        true,
        *live_handoff,
        live_handoff_enabled,
    );

    if plan == RemoteInstallRunningServerPlan::KeepRunning {
        if io::stdin().is_terminal() {
            eprintln!("remote shepherd server on {target} is already compatible:");
            eprintln!("  server: v{}", version_label(version.as_deref()));
            eprintln!(
                "Shepherd will install {} without stopping the running remote server.",
                current_version()
            );
        }
        return Ok(false);
    }

    if !io::stdin().is_terminal() {
        match plan {
            RemoteInstallRunningServerPlan::LiveHandoff => return Ok(false),
            RemoteInstallRunningServerPlan::StopRequired(_) => {
                return Err(io::Error::other(format!(
                    "remote shepherd server on {target} is running v{}; run from an interactive terminal to approve stopping it for the update",
                    version_label(version.as_deref())
                )));
            }
            RemoteInstallRunningServerPlan::KeepRunning => return Ok(false),
        }
    }

    if plan == RemoteInstallRunningServerPlan::LiveHandoff {
        eprintln!("remote shepherd server on {target} is currently running:");
        eprintln!("  server: v{}", version_label(version.as_deref()));
        eprintln!(
            "Shepherd will install {} and hand off live pane processes to the prepared server.",
            current_version()
        );
        return Ok(false);
    }

    eprintln!("remote shepherd server on {target} is currently running:");
    eprintln!("  server: v{}", version_label(version.as_deref()));
    eprintln!(
        "To complete the remote update, Shepherd must stop the running remote server after installing."
    );
    eprintln!("This stops active remote pane processes, including shells, dev servers, and tests.");
    eprintln!();
    eprint!(
        "Install {} and stop the remote server now? [y/N] ",
        current_version()
    );
    io::stderr().flush()?;

    let mut answer = String::new();
    io::stdin().read_line(&mut answer)?;
    let answer = answer.trim().to_ascii_lowercase();
    if answer != "y" && answer != "yes" {
        return Err(io::Error::new(
            io::ErrorKind::Interrupted,
            "remote shepherd install cancelled",
        ));
    }

    Ok(true)
}

fn remote_install_running_server_plan(
    version: Option<&str>,
    protocol: Option<u32>,
    detached_server_daemon: bool,
    remote_binary_changed: bool,
    live_handoff: bool,
    live_handoff_enabled: bool,
) -> RemoteInstallRunningServerPlan {
    let Some(reason) = remote_server_restart_reason(
        version,
        protocol,
        detached_server_daemon,
        remote_binary_changed,
    ) else {
        return RemoteInstallRunningServerPlan::KeepRunning;
    };

    if live_handoff_enabled && live_handoff {
        return RemoteInstallRunningServerPlan::LiveHandoff;
    }

    RemoteInstallRunningServerPlan::StopRequired(reason)
}

fn remote_server_status(
    ssh: &RemoteSsh,
    remote_shepherd: &RemoteShepherd,
) -> io::Result<RemoteServerStatus> {
    let command = format!("{} status server --json", remote_shepherd.shell_path);
    let output = ssh.sh_output(&command)?;
    if !output.status.success() {
        return Err(command_failed("remote server status failed", &output));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    parse_remote_server_status_json(stdout.trim())
}

#[derive(Debug, Deserialize)]
struct RemoteClientStatusJson {
    product: String,
    version: String,
    protocol: u32,
    binary: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RemoteServerStatusJson {
    running: bool,
    version: Option<String>,
    protocol: Option<u32>,
    capabilities: Option<RemoteServerCapabilitiesJson>,
}

#[derive(Debug, Deserialize)]
struct RemoteServerCapabilitiesJson {
    live_handoff: bool,
    #[serde(default)]
    detached_server_daemon: bool,
}

fn parse_client_status_json(status: &str) -> Option<RemoteClientStatusJson> {
    let parsed: RemoteClientStatusJson = serde_json::from_str(status).ok()?;
    (parsed.product == "shepherd").then_some(parsed)
}

fn parse_remote_server_status_json(status: &str) -> io::Result<RemoteServerStatus> {
    let parsed: RemoteServerStatusJson = serde_json::from_str(status).map_err(|err| {
        io::Error::other(format!(
            "could not parse remote server status JSON from `{status}`: {err}"
        ))
    })?;
    if !parsed.running {
        return Ok(RemoteServerStatus::NotRunning);
    }

    let capabilities = parsed.capabilities;

    Ok(RemoteServerStatus::Running {
        version: parsed.version,
        protocol: parsed.protocol,
        live_handoff: capabilities
            .as_ref()
            .is_some_and(|capabilities| capabilities.live_handoff),
        detached_server_daemon: capabilities
            .as_ref()
            .is_some_and(|capabilities| capabilities.detached_server_daemon),
    })
}

fn confirm_remote_server_stop(
    target: &str,
    version: Option<&str>,
    _protocol: Option<u32>,
    reason: RemoteServerRestartReason,
) -> io::Result<bool> {
    if !io::stdin().is_terminal() {
        if reason == RemoteServerRestartReason::ProtocolMismatch {
            return Err(io::Error::other(format!(
                "remote shepherd server on {target} must stop before this client can attach; run from an interactive terminal to approve stopping it"
            )));
        }

        eprintln!(
            "remote shepherd server on {target} is still running v{}; it will use {} after it restarts.",
            version_label(version),
            current_version()
        );
        return Ok(false);
    }

    eprintln!("remote shepherd server on {target} is currently running:");
    eprintln!("  server: v{}", version_label(version));
    eprintln!("  prepared binary: {}", current_version());
    eprintln!();

    match reason {
        RemoteServerRestartReason::ProtocolMismatch => {
            eprintln!("the remote server must stop before this client can attach.");
        }
        RemoteServerRestartReason::DaemonDetachMissing => {
            eprintln!(
                "the remote server was started by a shepherd build that may not survive SSH connection loss. restart it so network drops disconnect only this client."
            );
        }
        RemoteServerRestartReason::BinaryUpdated => {
            eprintln!(
                "the remote shepherd binary was installed or replaced. restart the remote server so it uses the prepared binary."
            );
        }
        RemoteServerRestartReason::VersionMismatch => {
            eprintln!(
                "the remote server is still running a different shepherd version. restart it so it uses the prepared binary."
            );
        }
    }

    let prompt = if reason == RemoteServerRestartReason::ProtocolMismatch {
        "stop the remote server and continue attaching? [Y/n] "
    } else {
        "restart the remote server now? [y/N] "
    };
    eprint!("{prompt}");
    io::stderr().flush()?;

    let mut answer = String::new();
    io::stdin().read_line(&mut answer)?;
    let answer = answer.trim().to_ascii_lowercase();
    if answer == "y" || answer == "yes" {
        return Ok(true);
    }
    if answer.is_empty() && reason == RemoteServerRestartReason::ProtocolMismatch {
        return Ok(true);
    }
    if reason == RemoteServerRestartReason::ProtocolMismatch {
        return Err(io::Error::new(
            io::ErrorKind::Interrupted,
            "remote shepherd server stop cancelled",
        ));
    }

    Ok(false)
}

fn live_handoff_remote_server(ssh: &RemoteSsh, remote_shepherd: &RemoteShepherd) -> io::Result<()> {
    let command = format!(
        "{} server live-handoff --import-exe {} --expected-protocol {} --expected-version {}",
        remote_shepherd.shell_path,
        remote_shepherd.shell_path,
        CURRENT_PROTOCOL,
        current_version()
    );
    let output = ssh.sh_output(&command)?;
    if !output.status.success() {
        return Err(command_failed("remote server live handoff failed", &output));
    }

    eprintln!(
        "handed off the remote shepherd server on {}; reconnecting to the prepared server.",
        ssh.target()
    );
    Ok(())
}

fn stop_remote_server(ssh: &RemoteSsh, remote_shepherd: &RemoteShepherd) -> io::Result<()> {
    let command = format!("{} server stop", remote_shepherd.shell_path);
    let output = ssh.sh_output(&command)?;
    if !output.status.success() {
        return Err(command_failed("remote server stop failed", &output));
    }

    wait_for_remote_server_shutdown(ssh, remote_shepherd)?;
    eprintln!(
        "stopped the remote shepherd server on {}; it will restart when the remote client bridge attaches.",
        ssh.target()
    );
    Ok(())
}

fn wait_for_remote_server_shutdown(
    ssh: &RemoteSsh,
    remote_shepherd: &RemoteShepherd,
) -> io::Result<()> {
    let deadline = Instant::now() + REMOTE_SERVER_SHUTDOWN_CONFIRM_TIMEOUT;
    loop {
        if remote_server_status(ssh, remote_shepherd)? == RemoteServerStatus::NotRunning {
            return Ok(());
        }
        if Instant::now() >= deadline {
            return Err(io::Error::new(
                io::ErrorKind::TimedOut,
                format!(
                    "shutdown was requested, but the old remote shepherd server on {target} is still responding after {} seconds",
                    REMOTE_SERVER_SHUTDOWN_CONFIRM_TIMEOUT.as_secs(),
                    target = ssh.target()
                ),
            ));
        }
        thread::sleep(REMOTE_SERVER_SHUTDOWN_POLL_INTERVAL);
    }
}

fn version_label(version: Option<&str>) -> &str {
    version.unwrap_or("unknown")
}

fn confirm_remote_install(target: &str) -> io::Result<()> {
    if !io::stdin().is_terminal() {
        return Err(io::Error::other(format!(
            "matching Shepherd {} with protocol {CURRENT_PROTOCOL} is not installed on {target}; run `{REMOTE_INSTALLER}` on the peer or retry attach from an interactive terminal",
            current_version()
        )));
    }

    eprintln!(
        "matching Shepherd {} with protocol {CURRENT_PROTOCOL} is not installed on {target}.",
        current_version()
    );
    eprint!("Run the peer-native `{REMOTE_INSTALLER}` on {target}? [Y/n] ");
    io::stderr().flush()?;

    let mut answer = String::new();
    io::stdin().read_line(&mut answer)?;
    let answer = answer.trim().to_ascii_lowercase();
    if answer == "n" || answer == "no" {
        return Err(io::Error::new(
            io::ErrorKind::Interrupted,
            "remote shepherd installation cancelled",
        ));
    }

    Ok(())
}

fn remote_bridge_command(remote_shepherd: &RemoteShepherd, session_name: &str) -> String {
    let mut command = format!("exec {}", remote_shepherd.shell_path);
    if session_name != crate::session::DEFAULT_SESSION_NAME {
        command.push_str(" --session ");
        command.push_str(&shell_quote(session_name));
    }
    command.push_str(" remote-client-bridge");
    command
}

fn reattach_command(
    program: &str,
    target: &str,
    session_name: &str,
    keybindings: RemoteKeybindings,
    live_handoff: bool,
) -> String {
    let program = if program.is_empty() {
        "shepherd"
    } else {
        program
    };
    let mut command = format!("{} --remote {}", shell_quote(program), shell_quote(target));
    if keybindings != RemoteKeybindings::Local {
        command.push_str(" --remote-keybindings ");
        command.push_str(keybindings.as_str());
    }
    if live_handoff {
        command.push_str(" --handoff");
    }
    if session_name != crate::session::DEFAULT_SESSION_NAME {
        command.push_str(" --session ");
        command.push_str(&shell_quote(session_name));
    }
    command
}

fn shell_quote(value: &str) -> String {
    if !value.is_empty()
        && value.chars().all(|ch| {
            ch.is_ascii_alphanumeric()
                || matches!(
                    ch,
                    '@' | '%' | '_' | '+' | '=' | ':' | ',' | '.' | '/' | '-'
                )
        })
    {
        return value.to_string();
    }

    format!("'{}'", value.replace('\'', "'\\''"))
}

fn command_failed(context: &str, output: &Output) -> io::Error {
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stderr = stderr.trim();
    if stderr.is_empty() {
        io::Error::other(format!("{context}: {}", output.status))
    } else {
        io::Error::other(format!("{context}: {stderr}"))
    }
}

struct SshStdioBridge {
    local_socket: PathBuf,
    should_stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl SshStdioBridge {
    fn start(
        target: String,
        remote_shepherd: RemoteShepherd,
        local_socket: PathBuf,
        session_name: String,
        ssh_options: Option<&ManagedSshOptions>,
    ) -> io::Result<Self> {
        let _ = std::fs::remove_file(&local_socket);
        let listener = UnixListener::bind(&local_socket)?;
        crate::ipc::restrict_socket_permissions(&local_socket, BRIDGE_SOCKET_PERMISSION_MODE)?;
        listener.set_nonblocking(true)?;

        let should_stop = Arc::new(AtomicBool::new(false));
        let thread_stop = Arc::clone(&should_stop);
        let thread_ssh_options = ssh_options.cloned();
        let thread = thread::spawn(move || {
            while !thread_stop.load(Ordering::Acquire) {
                match listener.accept() {
                    Ok((stream, _addr)) => {
                        if let Err(err) = stream.set_nonblocking(false) {
                            eprintln!(
                                "shepherd: remote bridge failed to prepare client socket: {err}"
                            );
                            continue;
                        }
                        if let Err(err) = bridge_connection(
                            stream,
                            &target,
                            &remote_shepherd,
                            &session_name,
                            thread_ssh_options.as_ref(),
                        ) {
                            eprintln!("shepherd: remote bridge failed: {err}");
                        }
                    }
                    Err(err) if err.kind() == io::ErrorKind::WouldBlock => {
                        thread::sleep(BRIDGE_ACCEPT_POLL);
                    }
                    Err(err) => {
                        eprintln!("shepherd: remote bridge listener failed: {err}");
                        break;
                    }
                }
            }
        });

        Ok(Self {
            local_socket,
            should_stop,
            thread: Some(thread),
        })
    }
}

impl Drop for SshStdioBridge {
    fn drop(&mut self) {
        self.should_stop.store(true, Ordering::Release);
        let _ = std::fs::remove_file(&self.local_socket);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

/// Creates a fresh user-only (`0700`) directory for the generated ssh config
/// and control socket, returning its path.
///
/// Using a private directory created with fail-if-exists semantics — rather
/// than a predictable file in the world-writable temp dir — stops a local user
/// from pre-planting a symlink or world-writable file that shepherd would write
/// and `ssh -F` would then read.
fn private_ssh_config_dir() -> io::Result<PathBuf> {
    use std::os::unix::fs::DirBuilderExt;

    let mut bases = vec![std::env::temp_dir()];
    let short_tmp = PathBuf::from("/tmp");
    if bases.first() != Some(&short_tmp) {
        bases.push(short_tmp);
    }

    let mut last_error = None;
    for base in bases {
        for attempt in 0..100 {
            let dir = base.join(format!("shepherd-ssh-{}-{attempt}", std::process::id()));
            if !fits_unix_socket_path(&dir.join(SSH_CONTROL_SOCKET_NAME)) {
                continue;
            }
            match fs::DirBuilder::new().mode(0o700).create(&dir) {
                Ok(()) => return Ok(dir),
                Err(err) if err.kind() == io::ErrorKind::AlreadyExists => continue,
                Err(err) => {
                    last_error = Some(err);
                    break;
                }
            }
        }
    }

    Err(last_error.unwrap_or_else(|| {
        io::Error::new(
            io::ErrorKind::AlreadyExists,
            "failed to create private shepherd ssh config directory",
        )
    }))
}

/// Quotes a path for an ssh_config `Include` so a path containing spaces (or
/// glob metacharacters) is treated as one literal token instead of being split
/// or expanded by ssh — otherwise the user's config might not be Included and
/// shepherd's fallback would wrongly take effect.
fn ssh_config_quote(path: &str) -> String {
    format!("\"{path}\"")
}

/// Builds a temporary ssh config for remote attach commands without overriding
/// the user's own settings, returning its path.
///
/// The file `Include`s the user's real ssh config first, so ssh's
/// first-value-wins rule keeps any `ServerAlive*` the user set there (including
/// an explicit `0` to disable it). Shepherd's keepalive values apply only when
/// the user has none.
fn write_managed_ssh_config() -> io::Result<ManagedSshConfig> {
    use std::os::unix::fs::OpenOptionsExt;

    let dir = private_ssh_config_dir()?;
    let path = dir.join("config");
    let control_path = dir.join(SSH_CONTROL_SOCKET_NAME);

    let mut contents = String::new();
    if let Some(home) = std::env::var_os("HOME") {
        let user_config = PathBuf::from(home).join(".ssh").join("config");
        if user_config.is_file() {
            contents.push_str(&format!(
                "Include {}\n",
                ssh_config_quote(&user_config.to_string_lossy())
            ));
        }
    }
    if Path::new("/etc/ssh/ssh_config").is_file() {
        contents.push_str("Include /etc/ssh/ssh_config\n");
    }
    contents.push_str("Host *\n");
    contents.push_str("  ServerAliveInterval 15\n");
    contents.push_str("  ServerAliveCountMax 4\n");

    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(BRIDGE_SOCKET_PERMISSION_MODE)
        .open(&path)?;
    file.write_all(contents.as_bytes())?;
    Ok(ManagedSshConfig {
        options: ManagedSshOptions {
            config_path: path,
            control_path,
        },
    })
}

fn bridge_connection(
    stream: UnixStream,
    target: &str,
    remote_shepherd: &RemoteShepherd,
    session_name: &str,
    ssh_options: Option<&ManagedSshOptions>,
) -> io::Result<()> {
    let mut command = Command::new("ssh");
    apply_managed_ssh_options(&mut command, ssh_options);
    command
        .arg("-T")
        .arg(target)
        .arg(remote_bridge_command(remote_shepherd, session_name));
    command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit());

    let mut child = command
        .spawn()
        .map_err(|err| io::Error::new(err.kind(), format!("failed to start ssh bridge: {err}")))?;
    let mut child_stdin = child
        .stdin
        .take()
        .ok_or_else(|| io::Error::new(io::ErrorKind::BrokenPipe, "ssh bridge stdin missing"))?;
    let mut child_stdout = child
        .stdout
        .take()
        .ok_or_else(|| io::Error::new(io::ErrorKind::BrokenPipe, "ssh bridge stdout missing"))?;
    let mut stream_to_child = stream.try_clone()?;
    let mut child_to_stream = stream;

    let upload = thread::spawn(move || {
        let _ = copy_flush(&mut stream_to_child, &mut child_stdin);
    });
    let download = thread::spawn(move || {
        let _ = copy_flush(&mut child_stdout, &mut child_to_stream);
        let _ = child_to_stream.shutdown(std::net::Shutdown::Write);
    });

    let status = child.wait()?;
    let _ = upload.join();
    let _ = download.join();

    if status.success() {
        Ok(())
    } else {
        Err(io::Error::new(
            io::ErrorKind::ConnectionAborted,
            format!("ssh bridge exited with {status}"),
        ))
    }
}

fn copy_flush<R: io::Read, W: io::Write>(reader: &mut R, writer: &mut W) -> io::Result<u64> {
    let mut buffer = [0_u8; 16 * 1024];
    let mut total = 0;

    loop {
        let bytes_read = match reader.read(&mut buffer) {
            Ok(0) => return Ok(total),
            Ok(bytes_read) => bytes_read,
            Err(err) if err.kind() == io::ErrorKind::Interrupted => continue,
            Err(err) => return Err(err),
        };

        writer.write_all(&buffer[..bytes_read])?;
        writer.flush()?;
        total += bytes_read as u64;
    }
}

fn run_client_process(
    local_socket: &Path,
    reattach_command: &str,
    keybindings: RemoteKeybindings,
) -> io::Result<()> {
    let exe = std::env::current_exe()?;
    let status = Command::new(exe)
        .arg("client")
        .env(
            crate::server::socket_paths::CLIENT_SOCKET_PATH_ENV_VAR,
            local_socket,
        )
        .env("SHEPHERD_RENDER_ENCODING", "terminal-ansi")
        .env(REATTACH_COMMAND_ENV_VAR, reattach_command)
        .env(REMOTE_KEYBINDINGS_ENV_VAR, keybindings.as_str())
        .env_remove(crate::api::SOCKET_PATH_ENV_VAR)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()?;

    if status.success() {
        Ok(())
    } else {
        Err(io::Error::new(
            io::ErrorKind::Interrupted,
            format!("remote client exited with {status}"),
        ))
    }
}

fn local_forward_socket_path(target: &str, session_name: &str) -> PathBuf {
    let pid = std::process::id();
    let target_clean = sanitize_path_component(target);
    let session_clean = sanitize_path_component(session_name);

    let tmpdir = std::env::temp_dir();
    let readable = tmpdir.join(format!(
        "shepherd-remote-{pid}-{target_clean}-{session_clean}.sock"
    ));
    if fits_unix_socket_path(&readable) {
        return readable;
    }

    // macOS' per-user TMPDIR (~49 chars under /var/folders/...) can push the
    // readable name past sun_path's 104-byte ceiling. Fall back to a hashed
    // short name in TMPDIR, then to /tmp as a last resort when TMPDIR itself
    // is longer than the budget. The hash covers the full unsanitized
    // target/session so uniqueness does not depend on the prefix truncation;
    // the prefix is kept only for debuggability.
    let target_prefix: String = target_clean.chars().take(8).collect();
    let hash = short_socket_hash(target, session_name);
    let short_name = format!("shepherd-r-{pid}-{target_prefix}-{hash}.sock");
    let short_in_tmp = tmpdir.join(&short_name);
    if fits_unix_socket_path(&short_in_tmp) {
        return short_in_tmp;
    }
    PathBuf::from("/tmp").join(short_name)
}

fn fits_unix_socket_path(path: &Path) -> bool {
    use std::os::unix::ffi::OsStrExt;
    // sun_path is byte-limited: 104 bytes on macOS, 108 on Linux. Reserve
    // 1 byte for the trailing NUL and use the smaller cap for portability.
    const MAX: usize = 103;
    path.as_os_str().as_bytes().len() <= MAX
}

fn short_socket_hash(target: &str, session: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    target.hash(&mut hasher);
    0u8.hash(&mut hasher);
    session.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

fn sanitize_path_component(input: &str) -> String {
    let sanitized: String = input
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '.' | '_' | '-') {
                ch
            } else {
                '-'
            }
        })
        .collect();

    sanitized.trim_matches('-').chars().take(32).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn remote() -> RemoteShepherd {
        RemoteShepherd::managed()
    }

    #[test]
    fn extract_remote_args_removes_attach_flags() {
        let args = vec![
            "shepherd".into(),
            "--remote".into(),
            "dev".into(),
            "--remote-keybindings=server".into(),
            "--handoff".into(),
        ];
        let (cleaned, launch) = extract_remote_args(&args).unwrap();

        assert_eq!(cleaned, vec!["shepherd"]);
        let launch = launch.unwrap();
        assert_eq!(launch.target, "dev");
        assert_eq!(launch.keybindings, RemoteKeybindings::Server);
        assert!(launch.live_handoff);
    }

    #[test]
    fn extract_remote_args_preserves_child_options_after_separator() {
        let args = vec![
            "shepherd".into(),
            "agent".into(),
            "start".into(),
            "repro".into(),
            "--".into(),
            "child".into(),
            "--remote".into(),
            "dev".into(),
        ];

        let (cleaned, launch) = extract_remote_args(&args).unwrap();

        assert_eq!(cleaned, args);
        assert!(launch.is_none());
    }

    #[test]
    fn extract_remote_args_rejects_invalid_values() {
        assert_eq!(
            extract_remote_args(&["shepherd".into(), "--remote".into()]).unwrap_err(),
            "missing value for --remote"
        );
        assert_eq!(
            extract_remote_args(&[
                "shepherd".into(),
                "--remote".into(),
                "-oProxyCommand=x".into(),
            ])
            .unwrap_err(),
            "--remote target must not start with '-'"
        );
        assert_eq!(
            extract_remote_args(&[
                "shepherd".into(),
                "--remote=dev".into(),
                "--remote=prod".into(),
            ])
            .unwrap_err(),
            "--remote can only be specified once"
        );
    }

    #[test]
    fn remote_bridge_and_reattach_commands_use_shepherd() {
        assert_eq!(
            remote_bridge_command(&remote(), crate::session::DEFAULT_SESSION_NAME),
            "exec \"$HOME/.local/bin/shepherd\" remote-client-bridge"
        );
        assert_eq!(
            reattach_command(
                "target/release/shepherd",
                "host name",
                "work",
                RemoteKeybindings::Server,
                true,
            ),
            "target/release/shepherd --remote 'host name' --remote-keybindings server --handoff --session work"
        );
    }

    #[test]
    fn remote_path_discovery_accepts_absolute_paths_and_ignores_shims() {
        let candidates = remote_shepherds_from_path_discovery(
            &remote(),
            "/usr/bin/shepherd\nrelative/shepherd\n/home/user/.local/share/mise/shims/shepherd\n/opt/shepherd bin/shepherd\n",
        );

        assert_eq!(candidates.len(), 2);
        assert_eq!(candidates[0].shell_path, "/usr/bin/shepherd");
        assert_eq!(candidates[1].shell_path, "'/opt/shepherd bin/shepherd'");
    }

    #[test]
    fn known_candidates_are_shepherd_only_and_have_no_download_fallback() {
        let script = known_remote_binary_candidate_script();
        let old_product = format!("{}{}", "her", "dr");

        assert!(script.contains("$home/.local/bin/shepherd"));
        assert!(script.contains("/opt/homebrew/bin/shepherd"));
        assert!(script.contains("$home/.nix-profile/bin/shepherd"));
        assert!(!script.to_ascii_lowercase().contains(&old_product));
        assert!(!script.contains("curl"));
        assert!(!script.contains("https://"));
    }

    #[test]
    fn native_installer_is_peer_executed_without_streaming() {
        let command = native_installer_command();

        assert_eq!(
            command,
            "command -v shepherd-install >/dev/null 2>&1 && exec shepherd-install"
        );
        assert!(!command.contains("tee "));
        assert!(!command.contains("curl "));
    }

    #[test]
    fn parse_client_status_json_requires_shepherd_identity() {
        let status = parse_client_status_json(
            r#"{"product":"shepherd","version":"0.7.5","protocol":19,"binary":"/bin/shepherd"}"#,
        )
        .expect("Shepherd identity");
        assert_eq!(status.product, "shepherd");
        assert_eq!(status.protocol, 19);

        let old_product = format!("{}{}", "her", "dr");
        let old_binary = format!("/bin/{old_product}");
        let legacy = serde_json::json!({
            "product": old_product,
            "version": "0.7.5",
            "protocol": 19,
            "binary": old_binary,
        });
        assert!(parse_client_status_json(&legacy.to_string()).is_none());
        assert!(parse_client_status_json(r#"{"protocol":"unknown"}"#).is_none());
    }

    #[test]
    fn handshake_requires_exact_version_and_protocol() {
        let current = RemoteClientStatusJson {
            product: "shepherd".into(),
            version: current_version(),
            protocol: CURRENT_PROTOCOL,
            binary: Some("/bin/shepherd".into()),
        };
        assert!(handshake_matches(&current));

        let mut wrong_protocol = RemoteClientStatusJson {
            product: current.product.clone(),
            version: current.version.clone(),
            protocol: CURRENT_PROTOCOL + 1,
            binary: current.binary.clone(),
        };
        assert!(!handshake_matches(&wrong_protocol));
        wrong_protocol.protocol = CURRENT_PROTOCOL;
        wrong_protocol.version = "0.0.0".into();
        assert!(!handshake_matches(&wrong_protocol));
    }

    #[test]
    fn parse_remote_server_status_reads_running_and_stopped() {
        assert_eq!(
            parse_remote_server_status_json(
                r#"{"running":true,"version":"0.7.5","protocol":19,"capabilities":{"live_handoff":true,"detached_server_daemon":true}}"#
            )
            .unwrap(),
            RemoteServerStatus::Running {
                version: Some("0.7.5".into()),
                protocol: Some(19),
                live_handoff: true,
                detached_server_daemon: true,
            }
        );
        assert_eq!(
            parse_remote_server_status_json(r#"{"running":false,"version":null,"protocol":null}"#)
                .unwrap(),
            RemoteServerStatus::NotRunning
        );
    }

    #[test]
    fn restart_reason_protects_protocol_and_runtime_state() {
        assert_eq!(
            remote_server_restart_reason(Some(&current_version()), Some(0), true, false),
            Some(RemoteServerRestartReason::ProtocolMismatch)
        );
        assert_eq!(
            remote_server_restart_reason(
                Some(&current_version()),
                Some(CURRENT_PROTOCOL),
                true,
                false,
            ),
            None
        );
        assert_eq!(
            remote_server_restart_reason(
                Some(&current_version()),
                Some(CURRENT_PROTOCOL),
                false,
                false,
            ),
            Some(RemoteServerRestartReason::DaemonDetachMissing)
        );
    }

    #[test]
    fn managed_ssh_config_is_private_and_preserves_user_precedence() {
        use std::os::unix::fs::PermissionsExt;

        let managed = write_managed_ssh_config().expect("managed config");
        let contents = std::fs::read_to_string(&managed.options.config_path).expect("read config");
        assert!(contents.contains("ServerAliveInterval 15"));
        assert!(!contents.contains("ControlPath"));

        if let Some(home) = std::env::var_os("HOME") {
            let user_config = PathBuf::from(home).join(".ssh/config");
            if user_config.is_file() {
                let include = format!(
                    "Include {}",
                    ssh_config_quote(&user_config.to_string_lossy())
                );
                assert!(contents.find(&include) < contents.find("Host *"));
            }
        }

        let file_mode = std::fs::metadata(&managed.options.config_path)
            .unwrap()
            .permissions()
            .mode()
            & 0o777;
        let dir_mode = std::fs::metadata(managed.options.config_path.parent().unwrap())
            .unwrap()
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(file_mode, BRIDGE_SOCKET_PERMISSION_MODE);
        assert_eq!(dir_mode, 0o700);
    }

    #[test]
    fn local_forward_socket_path_fits_portable_limit() {
        let path = local_forward_socket_path(
            "longish-host.example.com",
            "a-fairly-long-session-name-here",
        );
        assert!(fits_unix_socket_path(&path));
    }

    #[test]
    fn shell_quote_protects_remote_values() {
        assert_eq!(shell_quote("host"), "host");
        assert_eq!(shell_quote("host name"), "'host name'");
        assert_eq!(shell_quote("host's"), "'host'\\''s'");
    }
}
