//! Backups (plan §6.7): what the Backups screen lists.

use serde::Serialize;
use ts_rs::TS;

/// One row of the backup history, newest first.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct BackupEntry {
    #[ts(type = "number")]
    pub id: i64,
    /// The full path, so Admin can find the file.
    pub path: String,
    #[ts(type = "number")]
    pub size_bytes: i64,
    #[ts(type = "\"AUTO\" | \"MANUAL\" | \"PRE_RESTORE\" | \"PRE_POST\"")]
    pub kind: String,
    /// Empty for automatic backups, which nobody started.
    pub created_by_name: Option<String>,
    pub created_at: String,
    /// False once the file was moved or deleted, such as a manual backup on a USB drive
    /// that isn't plugged in.
    pub on_disk: bool,
}
