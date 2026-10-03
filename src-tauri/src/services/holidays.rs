//! The holiday calendar (plan §6.3). HR enters each year's holidays from the presidential
//! proclamation. Holidays can be deleted, since a wrong entry is just a typo, but never on a
//! date that is in a posted payroll period. Every change is audited.

use crate::audit::{self, Actor, Entry};
use crate::domain::org::{Holiday, HolidayInput, HOLIDAY_KINDS};
use crate::error::{AppError, FieldError};
use crate::repositories::holidays as repo;
use crate::services::auth::field;
use crate::time;
use chrono::{DateTime, Datelike, NaiveDate, Utc};
use serde_json::json;
use sqlx::{SqliteConnection, SqlitePool};

/// Years the calendar accepts. Anything outside is almost surely a typo.
const YEARS: std::ops::RangeInclusive<i32> = 2000..=2100;
const NAME_MAX: usize = 80;
/// A double regular holiday is two rows; three on one date is a mistake.
const MAX_PER_DATE: i64 = 2;

const LOCKED: &str = "This date is in a posted payroll period, so its holidays can't be changed.";

pub async fn list(db: &SqlitePool, year: i32) -> Result<Vec<Holiday>, AppError> {
    if !YEARS.contains(&year) {
        return Err(AppError::Validation(vec![field(
            "year",
            "Pick a year from 2000 to 2100",
        )]));
    }
    Ok(repo::in_year(db, year).await?)
}

/// Trimmed and checked holiday fields.
struct Clean {
    date: String,
    name: String,
    kind: &'static str,
}

fn clean(input: &HolidayInput) -> Result<Clean, Vec<FieldError>> {
    let mut errors = Vec::new();
    let date = NaiveDate::parse_from_str(input.date.trim(), "%Y-%m-%d")
        .ok()
        .filter(|d| YEARS.contains(&d.year()));
    if date.is_none() {
        errors.push(field("date", "Enter a date from 2000 to 2100"));
    }
    let name = input.name.split_whitespace().collect::<Vec<_>>().join(" ");
    if name.chars().count() < 2 || name.chars().count() > NAME_MAX {
        errors.push(field("name", "Enter a name (2 to 80 characters)"));
    }
    let kind = HOLIDAY_KINDS.iter().find(|k| **k == input.kind);
    if kind.is_none() {
        errors.push(field("kind", "Pick a holiday type"));
    }
    match (date, kind) {
        (Some(d), Some(k)) if errors.is_empty() => Ok(Clean {
            date: d.format("%Y-%m-%d").to_string(),
            name,
            kind: k,
        }),
        _ => Err(errors),
    }
}

/// The date rules shared by create and update: not locked, no repeated name, at most two.
async fn check_date(
    conn: &mut SqliteConnection,
    h: &Clean,
    except_id: Option<i64>,
) -> Result<(), AppError> {
    if repo::date_locked(conn, &h.date).await? {
        return Err(AppError::Conflict(LOCKED));
    }
    if repo::name_taken(conn, &h.date, &h.name, except_id).await? {
        return Err(AppError::Validation(vec![field(
            "name",
            "This date already has a holiday with this name",
        )]));
    }
    if repo::count_on(conn, &h.date, except_id).await? >= MAX_PER_DATE {
        return Err(AppError::Validation(vec![field(
            "date",
            "This date already has two holidays",
        )]));
    }
    Ok(())
}

pub async fn create(
    db: &SqlitePool,
    actor: Actor<'_>,
    input: HolidayInput,
    now: DateTime<Utc>,
) -> Result<Holiday, AppError> {
    let h = clean(&input).map_err(AppError::Validation)?;
    let mut tx = db.begin().await?;
    check_date(&mut tx, &h, None).await?;
    let id = repo::insert(&mut tx, &h.date, &h.name, h.kind).await?;
    let created = repo::by_id(&mut *tx, id)
        .await?
        .ok_or(AppError::NotFound("Holiday"))?;
    let entry = Entry {
        action: "holiday.create",
        entity: Some(("holiday", id)),
        after: Some(json!(created)),
        ..Default::default()
    };
    audit::record(&mut tx, now, actor, entry).await?;
    tx.commit().await?;
    Ok(created)
}

pub async fn update(
    db: &SqlitePool,
    actor: Actor<'_>,
    id: i64,
    input: HolidayInput,
    now: DateTime<Utc>,
) -> Result<Holiday, AppError> {
    let h = clean(&input).map_err(AppError::Validation)?;
    let mut tx = db.begin().await?;
    let before = repo::by_id(&mut *tx, id)
        .await?
        .ok_or(AppError::NotFound("Holiday"))?;
    // Moving a holiday changes pay on both dates, so both must be open.
    if before.locked {
        return Err(AppError::Conflict(LOCKED));
    }
    check_date(&mut tx, &h, Some(id)).await?;
    repo::update(&mut tx, id, &h.date, &h.name, h.kind, &time::to_db(now)).await?;
    let after = repo::by_id(&mut *tx, id)
        .await?
        .ok_or(AppError::NotFound("Holiday"))?;
    let entry = Entry {
        action: "holiday.update",
        entity: Some(("holiday", id)),
        before: Some(json!(before)),
        after: Some(json!(after)),
    };
    audit::record(&mut tx, now, actor, entry).await?;
    tx.commit().await?;
    Ok(after)
}

pub async fn delete(
    db: &SqlitePool,
    actor: Actor<'_>,
    id: i64,
    now: DateTime<Utc>,
) -> Result<(), AppError> {
    let mut tx = db.begin().await?;
    let before = repo::by_id(&mut *tx, id)
        .await?
        .ok_or(AppError::NotFound("Holiday"))?;
    if before.locked {
        return Err(AppError::Conflict(LOCKED));
    }
    repo::delete(&mut tx, id).await?;
    let entry = Entry {
        action: "holiday.delete",
        entity: Some(("holiday", id)),
        before: Some(json!(before)),
        ..Default::default()
    };
    audit::record(&mut tx, now, actor, entry).await?;
    tx.commit().await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audit::test_support as audit_rows;
    use chrono::TimeZone;

    fn t0() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 10, 7, 9, 0, 0).unwrap()
    }

    fn hr() -> Actor<'static> {
        Actor {
            user_id: Some(1),
            username: "hr",
        }
    }

    async fn db() -> (tempfile::TempDir, SqlitePool) {
        let dir = tempfile::tempdir().expect("temp dir");
        let pool = crate::db::open(&dir.path().join("test.db"))
            .await
            .expect("open");
        let mut conn = pool.acquire().await.expect("conn");
        crate::repositories::users::insert(&mut conn, "hr", "x", "HR", false)
            .await
            .expect("user");
        drop(conn);
        (dir, pool)
    }

    fn day(date: &str, name: &str, kind: &str) -> HolidayInput {
        HolidayInput {
            date: date.into(),
            name: name.into(),
            kind: kind.into(),
        }
    }

    fn fields(e: AppError) -> Vec<String> {
        match e {
            AppError::Validation(f) => f.into_iter().map(|f| f.field).collect(),
            other => panic!("expected a validation error, got {other:?}"),
        }
    }

    /// Puts a posted (locked) attendance record on `date`.
    async fn lock(db: &SqlitePool, date: &str) {
        sqlx::query(
            "INSERT INTO employees (id, employee_no, first_name, last_name, hire_date, \
             employment_status) VALUES (1, 'EMP-1', 'Juan', 'Dela Cruz', '2025-01-06', 'REGULAR') \
             ON CONFLICT DO NOTHING",
        )
        .execute(db)
        .await
        .expect("employee");
        sqlx::query(
            "INSERT INTO attendance_records (employee_id, work_date, time_in, status, source, \
             locked_by_period_id) VALUES (1, ?, ?, 'PRESENT', 'CLOCK', 1)",
        )
        .bind(date)
        .bind(format!("{date}T08:00:00"))
        .execute(db)
        .await
        .expect("locked record");
    }

    #[tokio::test]
    async fn adds_a_holiday_with_a_tidy_name() {
        let (_d, db) = db().await;
        let h = create(
            &db,
            hr(),
            day(" 2026-11-30 ", "  Bonifacio   Day ", "REGULAR"),
            t0(),
        )
        .await
        .expect("create");
        assert_eq!(h.date, "2026-11-30");
        assert_eq!(h.name, "Bonifacio Day");
        assert_eq!(h.kind, "REGULAR");
        assert!(!h.locked);
    }

    #[tokio::test]
    async fn holiday_fields_are_checked() {
        let (_d, db) = db().await;
        let err = create(&db, hr(), day("2026-02-30", "X", "FIESTA"), t0())
            .await
            .unwrap_err();
        assert_eq!(fields(err), ["date", "name", "kind"]);
        let err = create(&db, hr(), day("1999-12-31", "Old year", "REGULAR"), t0())
            .await
            .unwrap_err();
        assert_eq!(fields(err), ["date"]);
        let long = "x".repeat(81);
        let err = create(&db, hr(), day("2026-01-01", &long, "REGULAR"), t0())
            .await
            .unwrap_err();
        assert_eq!(fields(err), ["name"]);
    }

    #[tokio::test]
    async fn a_date_can_hold_a_double_holiday_but_not_three() {
        let (_d, db) = db().await;
        create(
            &db,
            hr(),
            day("2026-04-09", "Araw ng Kagitingan", "REGULAR"),
            t0(),
        )
        .await
        .expect("first");
        create(
            &db,
            hr(),
            day("2026-04-09", "Maundy Thursday", "REGULAR"),
            t0(),
        )
        .await
        .expect("second");
        let err = create(
            &db,
            hr(),
            day("2026-04-09", "araw ng kagitingan", "REGULAR"),
            t0(),
        )
        .await
        .unwrap_err();
        assert_eq!(fields(err), ["name"]);
        let err = create(
            &db,
            hr(),
            day("2026-04-09", "Third", "SPECIAL_WORKING"),
            t0(),
        )
        .await
        .unwrap_err();
        assert_eq!(fields(err), ["date"]);
    }

    #[tokio::test]
    async fn a_holiday_keeps_its_own_name_and_slot_when_edited() {
        let (_d, db) = db().await;
        let a = create(&db, hr(), day("2026-04-09", "Day A", "REGULAR"), t0())
            .await
            .expect("a");
        create(&db, hr(), day("2026-04-09", "Day B", "REGULAR"), t0())
            .await
            .expect("day b");
        let a = update(
            &db,
            hr(),
            a.id,
            day("2026-04-09", "Day A", "SPECIAL_NON_WORKING"),
            t0(),
        )
        .await
        .expect("edit in place");
        assert_eq!(a.kind, "SPECIAL_NON_WORKING");
        let err = update(&db, hr(), a.id, day("2026-04-09", "day b", "REGULAR"), t0())
            .await
            .unwrap_err();
        assert_eq!(fields(err), ["name"]);
    }

    #[tokio::test]
    async fn lists_one_year_in_date_order() {
        let (_d, db) = db().await;
        for (date, name) in [
            ("2026-12-25", "Christmas Day"),
            ("2027-01-01", "New Year's Day"),
            ("2026-01-01", "New Year's Day"),
        ] {
            create(&db, hr(), day(date, name, "REGULAR"), t0())
                .await
                .expect("create");
        }
        let dates: Vec<_> = list(&db, 2026)
            .await
            .expect("list")
            .into_iter()
            .map(|h| h.date)
            .collect();
        assert_eq!(dates, ["2026-01-01", "2026-12-25"]);
        let err = list(&db, 1900).await.unwrap_err();
        assert_eq!(fields(err), ["year"]);
    }

    #[tokio::test]
    async fn posted_dates_are_locked() {
        let (_d, db) = db().await;
        let h = create(
            &db,
            hr(),
            day("2026-08-31", "National Heroes Day", "REGULAR"),
            t0(),
        )
        .await
        .expect("create");
        lock(&db, "2026-08-31").await;
        lock(&db, "2026-08-21").await;

        assert!(list(&db, 2026).await.expect("list")[0].locked);
        let edit = day("2026-08-31", "Heroes Day", "REGULAR");
        let r = update(&db, hr(), h.id, edit, t0()).await;
        assert!(matches!(r, Err(AppError::Conflict(LOCKED))));
        let r = delete(&db, hr(), h.id, t0()).await;
        assert!(matches!(r, Err(AppError::Conflict(LOCKED))));
        // Nor can a holiday be added to, or moved onto, a posted date.
        let r = create(
            &db,
            hr(),
            day("2026-08-21", "Ninoy Aquino Day", "SPECIAL_NON_WORKING"),
            t0(),
        )
        .await;
        assert!(matches!(r, Err(AppError::Conflict(LOCKED))));
    }

    #[tokio::test]
    async fn every_change_is_audited_and_unknown_ids_are_not_found() {
        let (_d, db) = db().await;
        let h = create(&db, hr(), day("2026-12-30", "Rizal Day", "REGULAR"), t0())
            .await
            .expect("create");
        update(
            &db,
            hr(),
            h.id,
            day("2026-12-30", "Rizal Day", "REGULAR"),
            t0(),
        )
        .await
        .expect("update");
        delete(&db, hr(), h.id, t0()).await.expect("delete");
        assert!(list(&db, 2026).await.expect("list").is_empty());

        let rows = audit_rows::all(&db).await;
        let actions: Vec<_> = rows.iter().map(|r| r.action.as_str()).collect();
        assert_eq!(
            actions,
            ["holiday.create", "holiday.update", "holiday.delete"]
        );
        let deleted = rows[2].before_json.as_deref().expect("before");
        assert!(deleted.contains("Rizal Day"), "{deleted}");
        assert_eq!(rows[2].entity_id, Some(h.id));

        let r = delete(&db, hr(), h.id, t0()).await;
        assert!(matches!(r, Err(AppError::NotFound("Holiday"))));
        let r = update(&db, hr(), h.id, day("2026-12-30", "X1", "REGULAR"), t0()).await;
        assert!(matches!(r, Err(AppError::NotFound("Holiday"))));
    }
}
