use crate::{error::AppError, state::AppState};
use serde::Serialize;
use sqlx::SqlitePool;
use ts_rs::TS;

#[derive(Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Health {
    pub db_ok: bool,
    // Tauri sends this as a JSON number; without the override ts-rs would say bigint.
    #[ts(type = "number")]
    pub schema_version: i64,
}

async fn health(db: &SqlitePool) -> Result<Health, AppError> {
    let schema_version: i64 =
        sqlx::query_scalar("SELECT COALESCE(MAX(version), 0) FROM _sqlx_migrations")
            .fetch_one(db)
            .await?;
    Ok(Health {
        db_ok: true,
        schema_version,
    })
}

#[tauri::command]
pub async fn system_health(state: tauri::State<'_, AppState>) -> Result<Health, AppError> {
    health(&state.db).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn health_reports_the_applied_schema_version() {
        let dir = tempfile::tempdir().expect("temp dir");
        let pool = crate::db::open(&dir.path().join("test.db"))
            .await
            .expect("open");

        let h = health(&pool).await.expect("health");

        assert!(h.db_ok);
        let newest = sqlx::migrate!("./migrations")
            .migrations
            .iter()
            .map(|m| m.version)
            .max()
            .expect("at least one migration");
        assert_eq!(h.schema_version, newest);
    }

    #[test]
    fn health_serializes_in_camel_case_for_the_frontend() {
        let value = serde_json::to_value(Health {
            db_ok: true,
            schema_version: 1,
        })
        .expect("serialize");
        assert_eq!(
            value,
            serde_json::json!({ "dbOk": true, "schemaVersion": 1 })
        );
    }
}
