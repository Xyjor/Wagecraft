//! The audit trail (plan §8, ADR-008). Services call `record` inside the same
//! transaction as the change, so the change and its entry succeed or fail together.

use crate::time;
use chrono::{DateTime, Utc};
use serde_json::Value;
use sqlx::SqliteConnection;

/// Who did it. `user_id` is `None` when nobody is signed in, such as a failed login.
#[derive(Debug, Clone, Copy)]
pub struct Actor<'a> {
    pub user_id: Option<i64>,
    pub username: &'a str,
}

/// What happened. `before` and `after` are snapshots for the audit viewer; never put a
/// password hash or PIN hash in them.
#[derive(Debug, Default)]
pub struct Entry<'a> {
    /// `area.verb`, like `auth.login` or `employee.update`.
    pub action: &'a str,
    /// The record it touched, like `("user", 3)`.
    pub entity: Option<(&'a str, i64)>,
    pub before: Option<Value>,
    pub after: Option<Value>,
}

/// Longest actor name stored. A failed login records whatever was typed, which could be huge.
const MAX_ACTOR_CHARS: usize = 64;

pub async fn record(
    conn: &mut SqliteConnection,
    at: DateTime<Utc>,
    actor: Actor<'_>,
    entry: Entry<'_>,
) -> sqlx::Result<i64> {
    let username: String = actor.username.chars().take(MAX_ACTOR_CHARS).collect();
    let (entity_type, entity_id) = entry.entity.unzip();
    let res = sqlx::query(
        "INSERT INTO audit_logs \
         (at, actor_user_id, actor_username, action, entity_type, entity_id, before_json, after_json) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(time::to_db(at))
    .bind(actor.user_id)
    .bind(username)
    .bind(entry.action)
    .bind(entity_type)
    .bind(entity_id)
    .bind(entry.before.map(|v| v.to_string()))
    .bind(entry.after.map(|v| v.to_string()))
    .execute(conn)
    .await?;
    Ok(res.last_insert_rowid())
}

#[cfg(test)]
pub mod test_support {
    //! Reads the audit log back in tests.
    use sqlx::{FromRow, SqlitePool};

    #[derive(Debug, FromRow)]
    pub struct Row {
        pub at: String,
        pub actor_user_id: Option<i64>,
        pub actor_username: String,
        pub action: String,
        pub entity_type: Option<String>,
        pub entity_id: Option<i64>,
        pub before_json: Option<String>,
        pub after_json: Option<String>,
    }

    pub async fn all(db: &SqlitePool) -> Vec<Row> {
        sqlx::query_as(
            "SELECT at, actor_user_id, actor_username, action, entity_type, entity_id, \
             before_json, after_json FROM audit_logs ORDER BY id",
        )
        .fetch_all(db)
        .await
        .expect("read audit log")
    }

    pub async fn actions(db: &SqlitePool) -> Vec<String> {
        all(db).await.into_iter().map(|r| r.action).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::test_support::all;
    use super::*;
    use chrono::TimeZone;
    use serde_json::json;
    use sqlx::SqlitePool;

    async fn db() -> (tempfile::TempDir, SqlitePool) {
        let dir = tempfile::tempdir().expect("temp dir");
        let pool = crate::db::open(&dir.path().join("test.db"))
            .await
            .expect("open");
        (dir, pool)
    }

    fn at() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 10, 5, 8, 30, 0).unwrap()
    }

    async fn write_one(db: &SqlitePool) {
        let mut conn = db.acquire().await.expect("conn");
        record(
            &mut conn,
            at(),
            Actor {
                user_id: None,
                username: "someone",
            },
            Entry {
                action: "test.thing",
                entity: Some(("thing", 9)),
                before: Some(json!({ "name": "old" })),
                after: Some(json!({ "name": "new" })),
            },
        )
        .await
        .expect("record");
    }

    #[tokio::test]
    async fn records_every_column() {
        let (_dir, db) = db().await;
        write_one(&db).await;

        let rows = all(&db).await;
        assert_eq!(rows.len(), 1);
        let r = &rows[0];
        assert_eq!(r.at, "2026-10-05T08:30:00Z");
        assert_eq!(r.actor_user_id, None);
        assert_eq!(r.actor_username, "someone");
        assert_eq!(r.action, "test.thing");
        assert_eq!(r.entity_type.as_deref(), Some("thing"));
        assert_eq!(r.entity_id, Some(9));
        assert_eq!(r.before_json.as_deref(), Some(r#"{"name":"old"}"#));
        assert_eq!(r.after_json.as_deref(), Some(r#"{"name":"new"}"#));
    }

    #[tokio::test]
    async fn caps_the_actor_name() {
        let (_dir, db) = db().await;
        let long = "x".repeat(500);
        let mut conn = db.acquire().await.expect("conn");
        let entry = Entry {
            action: "auth.login_failed",
            ..Default::default()
        };
        let actor = Actor {
            user_id: None,
            username: &long,
        };
        record(&mut conn, at(), actor, entry).await.expect("record");
        drop(conn);

        assert_eq!(all(&db).await[0].actor_username.len(), MAX_ACTOR_CHARS);
    }

    #[tokio::test]
    async fn the_database_refuses_edits_and_deletes() {
        let (_dir, db) = db().await;
        write_one(&db).await;

        let update = sqlx::query("UPDATE audit_logs SET action = 'covered.up'")
            .execute(&db)
            .await;
        let delete = sqlx::query("DELETE FROM audit_logs").execute(&db).await;

        for result in [update, delete] {
            let err = result
                .expect_err("audit_logs must be append-only")
                .to_string();
            assert!(err.contains("append-only"), "unexpected error: {err}");
        }
        assert_eq!(all(&db).await.len(), 1);
    }

    #[tokio::test]
    async fn rolls_back_with_its_transaction() {
        let (_dir, db) = db().await;
        let mut tx = db.begin().await.expect("begin");
        let entry = Entry {
            action: "test.thing",
            ..Default::default()
        };
        let actor = Actor {
            user_id: None,
            username: "someone",
        };
        record(&mut tx, at(), actor, entry).await.expect("record");
        tx.rollback().await.expect("rollback");

        assert!(all(&db).await.is_empty());
    }
}
