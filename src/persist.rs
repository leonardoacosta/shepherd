//! Session persistence — save/restore workspaces, layouts, and working directories.
//!
//! Stored at `~/.config/shepherd/session.json`.
//! Optional pane screen history is stored separately at `session-history.json`.
//! Installed plugins are persisted separately at `plugins.json`.

mod atomic;
mod io;
pub mod plugin_registry;
mod restore;
mod snapshot;

pub(crate) use self::atomic::atomic_write_config;
pub use self::io::{clear, clear_history, load, load_history, save};
pub use self::restore::restore;
#[cfg(unix)]
pub use self::restore::{handoff_pane_aliases, restore_handoff};
#[cfg(any(unix, test))]
pub use self::snapshot::capture;
pub(crate) use self::snapshot::{capture_for_disk, capture_history_for_disk};
pub use self::snapshot::{
    DirectionSnapshot, LayoutSnapshot, SessionHistorySnapshot, SessionSnapshot, TabSnapshot,
    WorkspaceSnapshot,
};
