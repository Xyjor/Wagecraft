//! Departments and positions (plan §6.2).

use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, FromRow, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Department {
    #[ts(type = "number")]
    pub id: i64,
    pub code: String,
    pub name: String,
    pub description: Option<String>,
    pub is_active: bool,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct DepartmentInput {
    pub code: String,
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, FromRow, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Position {
    #[ts(type = "number")]
    pub id: i64,
    #[ts(type = "number")]
    pub department_id: i64,
    pub department_name: String,
    pub title: String,
    /// Monthly salary range in centavos. Outside it is a warning, never an error.
    #[ts(type = "number | null")]
    pub min_rate_cents: Option<i64>,
    #[ts(type = "number | null")]
    pub max_rate_cents: Option<i64>,
    pub is_active: bool,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct PositionInput {
    #[ts(type = "number")]
    pub department_id: i64,
    pub title: String,
    #[ts(type = "number | null")]
    pub min_rate_cents: Option<i64>,
    #[ts(type = "number | null")]
    pub max_rate_cents: Option<i64>,
}
