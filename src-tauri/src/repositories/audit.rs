//! Reads the audit log. Writing is `crate::audit::record`, and the database refuses
//! updates and deletes (migration 0002).

use crate::domain::audit::AuditFilters;
use sqlx::sqlite::SqliteArguments;
use sqlx::{FromRow, Sqlite, SqliteConnection};

#[derive(Debug, Clone, FromRow)]
pub struct AuditRow {
    pub id: i64,
    pub at: String,
    pub actor_user_id: Option<i64>,
    pub actor_username: String,
    pub action: String,
    pub entity_type: Option<String>,
    pub entity_id: Option<i64>,
    pub before_json: Option<String>,
    pub after_json: Option<String>,
}

/// The filters as SQL sees them: UTC bounds (`from` inclusive, `to` exclusive) and either
/// an exact action or an escaped `LIKE` pattern for an area.
#[derive(Debug, Clone, Default)]
pub struct Filter {
    pub from_utc: Option<String>,
    pub to_utc: Option<String>,
    pub actor: Option<String>,
    pub action: Option<String>,
    pub action_like: Option<String>,
    pub entity_type: Option<String>,
    pub entity_id: Option<i64>,
}

const COLUMNS: &str = "id, at, actor_user_id, actor_username, action, entity_type, entity_id, \
                       before_json, after_json";

/// Every filter is optional: a NULL parameter switches its clause off. Uses `?1`..`?7`.
const WHERE: &str = "FROM audit_logs WHERE (?1 IS NULL OR at >= ?1) AND (?2 IS NULL OR at < ?2) \
                     AND (?3 IS NULL OR actor_username = ?3) AND (?4 IS NULL OR action = ?4) \
                     AND (?5 IS NULL OR action LIKE ?5 ESCAPE '\\') \
                     AND (?6 IS NULL OR entity_type = ?6) AND (?7 IS NULL OR entity_id = ?7)";

type Query<'q, O> = sqlx::query::QueryAs<'q, Sqlite, O, SqliteArguments<'q>>;

fn bind_filter<'q, O>(query: Query<'q, O>, f: &'q Filter) -> Query<'q, O> {
    query
        .bind(f.from_utc.as_deref())
        .bind(f.to_utc.as_deref())
        .bind(f.actor.as_deref())
        .bind(f.action.as_deref())
        .bind(f.action_like.as_deref())
        .bind(f.entity_type.as_deref())
        .bind(f.entity_id)
}

/// One page, newest first (ties broken by insertion order), and how many rows match in all.
pub async fn list(
    conn: &mut SqliteConnection,
    filter: &Filter,
    limit: i64,
    offset: i64,
) -> sqlx::Result<(Vec<AuditRow>, i64)> {
    let count_sql = format!("SELECT COUNT(*) {WHERE}");
    let (total,): (i64,) = bind_filter(sqlx::query_as(&count_sql), filter)
        .fetch_one(&mut *conn)
        .await?;

    let page_sql = format!("SELECT {COLUMNS} {WHERE} ORDER BY at DESC, id DESC LIMIT ?8 OFFSET ?9");
    let rows = bind_filter(sqlx::query_as(&page_sql), filter)
        .bind(limit)
        .bind(offset)
        .fetch_all(&mut *conn)
        .await?;
    Ok((rows, total))
}

/// Every matching row, newest first, for the CSV export.
pub async fn all(conn: &mut SqliteConnection, filter: &Filter) -> sqlx::Result<Vec<AuditRow>> {
    let sql = format!("SELECT {COLUMNS} {WHERE} ORDER BY at DESC, id DESC");
    bind_filter(sqlx::query_as(&sql), filter)
        .fetch_all(&mut *conn)
        .await
}

pub async fn filters(conn: &mut SqliteConnection) -> sqlx::Result<AuditFilters> {
    let actors = sqlx::query_scalar("SELECT DISTINCT actor_username FROM audit_logs ORDER BY 1")
        .fetch_all(&mut *conn)
        .await?;
    let actions = sqlx::query_scalar("SELECT DISTINCT action FROM audit_logs ORDER BY 1")
        .fetch_all(&mut *conn)
        .await?;
    let entity_types = sqlx::query_scalar(
        "SELECT DISTINCT entity_type FROM audit_logs WHERE entity_type IS NOT NULL ORDER BY 1",
    )
    .fetch_all(&mut *conn)
    .await?;
    Ok(AuditFilters {
        actors,
        actions,
        entity_types,
    })
}
