//! The activity log viewer (plan §6.8): filter the audit log by day, user, action and
//! record, page through it, and export the filtered list as CSV. Reading is Admin-only
//! (`Permission::AuditRead`); nothing here writes except the export's own audit entry.

use crate::audit::{self, Actor, Entry};
use crate::domain::audit::{AuditEntry, AuditFilters, AuditPage, AuditQuery};
use crate::error::AppError;
use crate::export::csv;
use crate::repositories::audit::{self as repo, AuditRow, Filter};
use crate::services::auth::field;
use crate::time;
use chrono::{DateTime, Duration, FixedOffset, NaiveDate, NaiveTime, Utc};
use serde_json::{json, Value};
use sqlx::SqlitePool;

/// Rows per page when the query says 0 or less.
pub const DEFAULT_PAGE_SIZE: i64 = 50;
/// The most rows one page may hold.
pub const MAX_PAGE_SIZE: i64 = 200;

const CSV_HEADERS: &[&str] = &[
    "Time",
    "User",
    "Action",
    "Record",
    "Record ID",
    "Before",
    "After",
];

/// One page of the log, newest first. `offset` is the office's UTC offset, which turns the
/// query's local days into the UTC bounds the log is stored in.
pub async fn list(
    db: &SqlitePool,
    q: &AuditQuery,
    offset: FixedOffset,
) -> Result<AuditPage, AppError> {
    let filter = filter_of(q, offset)?;
    let page_size = if q.page_size <= 0 {
        DEFAULT_PAGE_SIZE
    } else {
        q.page_size.min(MAX_PAGE_SIZE)
    };
    let page = q.page.max(1);
    let mut conn = db.acquire().await?;
    let (rows, total) = repo::list(&mut conn, &filter, page_size, (page - 1) * page_size).await?;
    Ok(AuditPage {
        items: rows.into_iter().map(entry_of).collect(),
        total,
        page,
        page_size,
    })
}

/// The usernames, actions and record types that appear in the log, for the filter boxes.
pub async fn filters(db: &SqlitePool) -> Result<AuditFilters, AppError> {
    let mut conn = db.acquire().await?;
    Ok(repo::filters(&mut conn).await?)
}

/// The filtered log as a CSV file with local times, newest first. Records the export.
pub async fn export_csv(
    db: &SqlitePool,
    actor: Actor<'_>,
    q: &AuditQuery,
    now: DateTime<Utc>,
    offset: FixedOffset,
) -> Result<Vec<u8>, AppError> {
    let filter = filter_of(q, offset)?;
    let mut tx = db.begin().await?;
    let rows = repo::all(&mut tx, &filter).await?;
    let count = rows.len();
    let bytes = csv::build(CSV_HEADERS, rows.into_iter().map(|r| csv_row(r, offset)))?;
    let entry = Entry {
        action: "report.export",
        after: Some(json!({ "report": "audit_log", "rows": count, "filters": q })),
        ..Default::default()
    };
    audit::record(&mut tx, now, actor, entry).await?;
    tx.commit().await?;
    Ok(bytes)
}

/// Turns the query into SQL terms. A day filter becomes the UTC moment the local day
/// starts (and, for `to`, the moment the next day starts).
fn filter_of(q: &AuditQuery, offset: FixedOffset) -> Result<Filter, AppError> {
    let from = day("from", q.from.as_deref())?;
    let to = day("to", q.to.as_deref())?;
    let action = clean(q.action.as_deref());
    let (action, action_like) = match action {
        Some(area) if area.ends_with('.') => (None, Some(format!("{}%", escape_like(&area)))),
        other => (other, None),
    };
    Ok(Filter {
        from_utc: from.map(|d| start_of_day_utc(d, offset)),
        to_utc: to.map(|d| start_of_day_utc(d + Duration::days(1), offset)),
        actor: clean(q.actor.as_deref()),
        action,
        action_like,
        entity_type: clean(q.entity_type.as_deref()),
        entity_id: q.entity_id,
    })
}

fn clean(s: Option<&str>) -> Option<String> {
    s.map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

fn day(name: &str, s: Option<&str>) -> Result<Option<NaiveDate>, AppError> {
    match s.map(str::trim).filter(|s| !s.is_empty()) {
        None => Ok(None),
        Some(s) => NaiveDate::parse_from_str(s, "%Y-%m-%d")
            .map(Some)
            .map_err(|_| AppError::Validation(vec![field(name, "Use a date like 2026-10-09")])),
    }
}

fn start_of_day_utc(day: NaiveDate, offset: FixedOffset) -> String {
    let local_midnight = day.and_time(NaiveTime::MIN);
    let utc = local_midnight - Duration::seconds(i64::from(offset.local_minus_utc()));
    time::to_db(DateTime::<Utc>::from_naive_utc_and_offset(utc, Utc))
}

/// `%`, `_` and `\` mean themselves in the pattern.
fn escape_like(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
}

fn entry_of(r: AuditRow) -> AuditEntry {
    AuditEntry {
        id: r.id,
        at: r.at,
        actor_user_id: r.actor_user_id,
        actor_username: r.actor_username,
        action: r.action,
        entity_type: r.entity_type,
        entity_id: r.entity_id,
        before: r.before_json.map(parse_json),
        after: r.after_json.map(parse_json),
    }
}

/// A snapshot that somehow isn't JSON is shown as the text it is, not dropped.
fn parse_json(s: String) -> Value {
    serde_json::from_str(&s).unwrap_or(Value::String(s))
}

fn csv_row(r: AuditRow, offset: FixedOffset) -> Vec<String> {
    let local = time::from_db(&r.at)
        .map(|t| {
            t.with_timezone(&offset)
                .format("%Y-%m-%d %H:%M:%S")
                .to_string()
        })
        .unwrap_or(r.at);
    vec![
        local,
        r.actor_username,
        r.action,
        r.entity_type.unwrap_or_default(),
        r.entity_id.map(|id| id.to_string()).unwrap_or_default(),
        r.before_json.unwrap_or_default(),
        r.after_json.unwrap_or_default(),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audit::{self, test_support, Actor, Entry};
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

    fn manila() -> FixedOffset {
        FixedOffset::east_opt(8 * 3600).unwrap()
    }

    fn at(d: u32, h: u32, m: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 10, d, h, m, 0).unwrap()
    }

    const ADMIN: Actor<'static> = Actor {
        user_id: Some(1),
        username: "admin",
    };
    const HR: Actor<'static> = Actor {
        user_id: Some(2),
        username: "hr",
    };
    const NOBODY: Actor<'static> = Actor {
        user_id: None,
        username: "x",
    };

    /// Four entries, oldest first, in UTC. In Manila (+8) they fall on Oct 5, Oct 6, Oct 7
    /// and Oct 7.
    async fn seeded() -> (tempfile::TempDir, SqlitePool) {
        let (dir, pool) = db().await;
        let mut conn = pool.acquire().await.expect("conn");
        for (name, role) in [("admin", "ADMIN"), ("hr", "HR")] {
            crate::repositories::users::insert(&mut conn, name, "x", role, false)
                .await
                .expect("user");
        }
        let entries = [
            (
                at(5, 8, 30),
                ADMIN,
                Entry {
                    action: "auth.login",
                    entity: Some(("user", 1)),
                    ..Default::default()
                },
            ),
            (
                at(6, 1, 0),
                ADMIN,
                Entry {
                    action: "employee.update",
                    entity: Some(("employee", 7)),
                    before: Some(json!({ "name": "old", "rate": 1 })),
                    after: Some(json!({ "name": "new", "rate": 1 })),
                },
            ),
            (
                at(6, 23, 30),
                HR,
                Entry {
                    action: "payroll.post",
                    entity: Some(("payroll_period", 3)),
                    after: Some(json!({ "status": "POSTED" })),
                    ..Default::default()
                },
            ),
            (
                at(7, 10, 0),
                NOBODY,
                Entry {
                    action: "auth.login_failed",
                    after: Some(json!({ "username": "x" })),
                    ..Default::default()
                },
            ),
        ];
        for (when, who, what) in entries {
            audit::record(&mut conn, when, who, what)
                .await
                .expect("record");
        }
        (dir, pool)
    }

    fn query(change: impl FnOnce(&mut AuditQuery)) -> AuditQuery {
        let mut q = AuditQuery::default();
        change(&mut q);
        q
    }

    async fn actions_of(db: &SqlitePool, q: AuditQuery) -> Vec<String> {
        list(db, &q, manila())
            .await
            .expect("list")
            .items
            .into_iter()
            .map(|e| e.action)
            .collect()
    }

    #[tokio::test]
    async fn lists_newest_first_with_the_snapshots_parsed() {
        let (_dir, db) = seeded().await;

        let page = list(&db, &AuditQuery::default(), manila())
            .await
            .expect("list");

        assert_eq!(
            (page.total, page.page, page.page_size),
            (4, 1, DEFAULT_PAGE_SIZE)
        );
        let actions: Vec<_> = page.items.iter().map(|e| e.action.as_str()).collect();
        assert_eq!(
            actions,
            [
                "auth.login_failed",
                "payroll.post",
                "employee.update",
                "auth.login"
            ]
        );
        let update = &page.items[2];
        assert_eq!(update.at, "2026-10-06T01:00:00Z");
        assert_eq!(
            (update.actor_user_id, update.actor_username.as_str()),
            (Some(1), "admin")
        );
        assert_eq!(
            (update.entity_type.as_deref(), update.entity_id),
            (Some("employee"), Some(7))
        );
        assert_eq!(update.before, Some(json!({ "name": "old", "rate": 1 })));
        assert_eq!(update.after, Some(json!({ "name": "new", "rate": 1 })));
        assert_eq!(page.items[3].before, None);
        assert_eq!(page.items[0].actor_user_id, None);
    }

    #[tokio::test]
    async fn filters_by_the_office_s_local_days() {
        let (_dir, db) = seeded().await;

        // Oct 6 in Manila is Oct 5 16:00Z to Oct 6 16:00Z: only the update.
        let one_day = query(|q| {
            q.from = Some("2026-10-06".into());
            q.to = Some("2026-10-06".into());
        });
        assert_eq!(actions_of(&db, one_day).await, ["employee.update"]);

        // From Oct 7 on: the post at 23:30Z on the 6th is already the 7th in Manila.
        let from = query(|q| q.from = Some("2026-10-07".into()));
        assert_eq!(
            actions_of(&db, from).await,
            ["auth.login_failed", "payroll.post"]
        );

        let to = query(|q| q.to = Some("2026-10-05".into()));
        assert_eq!(actions_of(&db, to).await, ["auth.login"]);
    }

    #[tokio::test]
    async fn a_day_ends_the_moment_the_next_one_starts() {
        let (_dir, db) = seeded().await;
        {
            // Exactly midnight on Oct 7 in Manila: 16:00Z on the 6th.
            let mut conn = db.acquire().await.expect("conn");
            audit::record(
                &mut conn,
                at(6, 16, 0),
                ADMIN,
                Entry {
                    action: "settings.update",
                    ..Default::default()
                },
            )
            .await
            .expect("record");
        }

        let through_the_6th = query(|q| q.to = Some("2026-10-06".into()));
        assert_eq!(
            actions_of(&db, through_the_6th).await,
            ["employee.update", "auth.login"]
        );
        let from_the_7th = query(|q| q.from = Some("2026-10-07".into()));
        assert_eq!(
            actions_of(&db, from_the_7th).await,
            ["auth.login_failed", "payroll.post", "settings.update"]
        );
    }

    #[tokio::test]
    async fn refuses_a_bad_date() {
        let (_dir, db) = seeded().await;

        for (from, to) in [(Some("yesterday"), None), (None, Some("2026-13-01"))] {
            let q = query(|q| {
                q.from = from.map(Into::into);
                q.to = to.map(Into::into);
            });
            let err = list(&db, &q, manila()).await.expect_err("bad date");
            let AppError::Validation(fields) = err else {
                panic!("expected a validation error, got {err:?}");
            };
            let expected = if from.is_some() { "from" } else { "to" };
            assert_eq!(fields[0].field, expected);
        }
    }

    #[tokio::test]
    async fn filters_by_user_action_area_and_record() {
        let (_dir, db) = seeded().await;

        let by_admin = query(|q| q.actor = Some("admin".into()));
        assert_eq!(
            actions_of(&db, by_admin).await,
            ["employee.update", "auth.login"]
        );

        let exact = query(|q| q.action = Some("auth.login".into()));
        assert_eq!(actions_of(&db, exact).await, ["auth.login"]);

        let area = query(|q| q.action = Some("auth.".into()));
        assert_eq!(
            actions_of(&db, area).await,
            ["auth.login_failed", "auth.login"]
        );

        let record = query(|q| {
            q.entity_type = Some("employee".into());
            q.entity_id = Some(7);
        });
        assert_eq!(actions_of(&db, record).await, ["employee.update"]);

        let other_record = query(|q| {
            q.entity_type = Some("employee".into());
            q.entity_id = Some(8);
        });
        assert!(actions_of(&db, other_record).await.is_empty());

        let any_employee = query(|q| q.entity_type = Some("employee".into()));
        assert_eq!(actions_of(&db, any_employee).await, ["employee.update"]);
    }

    #[tokio::test]
    async fn an_area_filter_is_not_a_wildcard() {
        let (_dir, db) = seeded().await;
        {
            let mut conn = db.acquire().await.expect("conn");
            audit::record(
                &mut conn,
                at(8, 0, 0),
                ADMIN,
                Entry {
                    action: "leave_type.update",
                    ..Default::default()
                },
            )
            .await
            .expect("record");
        }

        // `_` would match any character in a LIKE pattern; it must mean an underscore.
        let area = query(|q| q.action = Some("leave_type.".into()));
        assert_eq!(actions_of(&db, area).await, ["leave_type.update"]);
        let wrong_area = query(|q| q.action = Some("leaveXtype.".into()));
        assert!(actions_of(&db, wrong_area).await.is_empty());
    }

    #[tokio::test]
    async fn pages_and_clamps_the_page_size() {
        let (_dir, db) = seeded().await;

        let first = list(&db, &query(|q| q.page_size = 3), manila())
            .await
            .expect("page 1");
        assert_eq!((first.items.len(), first.total, first.page), (3, 4, 1));

        let second = list(
            &db,
            &query(|q| {
                q.page = 2;
                q.page_size = 3;
            }),
            manila(),
        )
        .await
        .expect("page 2");
        assert_eq!(second.items.len(), 1);
        assert_eq!(second.items[0].action, "auth.login");

        let huge = list(&db, &query(|q| q.page_size = 10_000), manila())
            .await
            .expect("huge");
        assert_eq!(huge.page_size, MAX_PAGE_SIZE);

        let before_the_start = list(&db, &query(|q| q.page = -3), manila())
            .await
            .expect("page -3");
        assert_eq!(before_the_start.page, 1);
        assert_eq!(before_the_start.items.len(), 4);
    }

    #[tokio::test]
    async fn filter_options_are_distinct_and_sorted() {
        let (_dir, db) = seeded().await;

        let f = filters(&db).await.expect("filters");

        assert_eq!(f.actors, ["admin", "hr", "x"]);
        assert_eq!(
            f.actions,
            [
                "auth.login",
                "auth.login_failed",
                "employee.update",
                "payroll.post"
            ]
        );
        assert_eq!(f.entity_types, ["employee", "payroll_period", "user"]);
    }

    #[tokio::test]
    async fn exports_the_filtered_log_with_local_times_and_records_it() {
        let (_dir, db) = seeded().await;

        let q = query(|q| q.action = Some("employee.".into()));
        let bytes = export_csv(&db, HR, &q, at(9, 2, 0), manila())
            .await
            .expect("export");

        assert!(bytes.starts_with(b"\xEF\xBB\xBF"));
        let text = std::str::from_utf8(&bytes[3..]).expect("utf-8");
        let mut lines = text.lines();
        assert_eq!(
            lines.next(),
            Some("Time,User,Action,Record,Record ID,Before,After")
        );
        assert_eq!(
            lines.next(),
            Some(
                r#"2026-10-06 09:00:00,admin,employee.update,employee,7,"{""name"":""old"",""rate"":1}","{""name"":""new"",""rate"":1}""#
            )
        );
        assert_eq!(lines.next(), None);

        let logged = test_support::all(&db).await;
        let export = logged.last().expect("an entry");
        assert_eq!(
            (export.action.as_str(), export.actor_username.as_str()),
            ("report.export", "hr")
        );
        let after: serde_json::Value =
            serde_json::from_str(export.after_json.as_deref().unwrap()).unwrap();
        assert_eq!(after["report"], "audit_log");
        assert_eq!(after["rows"], 1);
        assert_eq!(after["filters"]["action"], "employee.");
    }

    #[tokio::test]
    async fn the_export_takes_every_matching_row_not_one_page() {
        let (_dir, db) = seeded().await;

        let q = query(|q| q.page_size = 1);
        let bytes = export_csv(&db, HR, &q, at(9, 2, 0), manila())
            .await
            .expect("export");
        let text = std::str::from_utf8(&bytes[3..]).expect("utf-8");

        assert_eq!(text.lines().count(), 1 + 4);
        assert!(text
            .lines()
            .nth(1)
            .unwrap()
            .starts_with("2026-10-07 18:00:00,x,auth.login_failed,,,,"));
    }
}
