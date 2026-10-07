//! System settings (plan §5.3, F-14 Phase 7): the company profile, the idle timeout and
//! where daily backups go. Stored as key/value rows in `settings`.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

pub const DEFAULT_IDLE_MINUTES: i64 = 15;
pub const DEFAULT_BACKUP_KEEP: i64 = 14;

/// What the Settings screen shows.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Settings {
    pub company_name: String,
    pub company_address: String,
    /// Digits only; the screen shows it grouped as 000-000-000-000.
    pub company_tin: String,
    #[ts(type = "number")]
    pub idle_timeout_minutes: i64,
    /// Where daily backups go. Empty means the default folder.
    pub backup_folder: String,
    /// The default folder, shown when `backup_folder` is empty.
    pub default_backup_folder: String,
    /// How many daily backups to keep.
    #[ts(type = "number")]
    pub backup_keep: i64,
}

/// What Admin saves.
#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SettingsInput {
    pub company_name: String,
    pub company_address: String,
    pub company_tin: String,
    #[ts(type = "number")]
    pub idle_timeout_minutes: i64,
    pub backup_folder: String,
    #[ts(type = "number")]
    pub backup_keep: i64,
}
