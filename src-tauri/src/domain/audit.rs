//! The activity log as the Admin's viewer sees it (plan §6.8).

use serde::{Deserialize, Serialize};
use serde_json::Value;
use ts_rs::TS;

/// Filters and page for `audit_list` and `audit_export_csv`. Dates are the office's local
/// days, inclusive at both ends.
#[derive(Debug, Clone, Default, Deserialize, Serialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export)]
pub struct AuditQuery {
    /// First day, `YYYY-MM-DD`.
    pub from: Option<String>,
    /// Last day, `YYYY-MM-DD`.
    pub to: Option<String>,
    /// The username that did it, exactly.
    pub actor: Option<String>,
    /// An action like `payroll.post`, or an area like `payroll.` for all of its actions.
    pub action: Option<String>,
    pub entity_type: Option<String>,
    #[ts(type = "number | null")]
    pub entity_id: Option<i64>,
    /// 1-based.
    #[ts(type = "number")]
    pub page: i64,
    #[ts(type = "number")]
    pub page_size: i64,
}

/// One recorded sign-in or change. `before` and `after` are what the service that made the
/// change chose to snapshot; most are small JSON objects.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct AuditEntry {
    #[ts(type = "number")]
    pub id: i64,
    /// UTC, like `2026-10-09T05:51:15Z`.
    pub at: String,
    #[ts(type = "number | null")]
    pub actor_user_id: Option<i64>,
    pub actor_username: String,
    pub action: String,
    pub entity_type: Option<String>,
    #[ts(type = "number | null")]
    pub entity_id: Option<i64>,
    #[ts(type = "unknown")]
    pub before: Option<Value>,
    #[ts(type = "unknown")]
    pub after: Option<Value>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct AuditPage {
    pub items: Vec<AuditEntry>,
    #[ts(type = "number")]
    pub total: i64,
    #[ts(type = "number")]
    pub page: i64,
    #[ts(type = "number")]
    pub page_size: i64,
}

/// The values the filters can offer: everything that has ever been recorded, sorted.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct AuditFilters {
    pub actors: Vec<String>,
    pub actions: Vec<String>,
    pub entity_types: Vec<String>,
}
