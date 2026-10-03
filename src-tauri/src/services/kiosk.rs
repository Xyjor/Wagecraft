//! The kiosk (plan §6.3): clock in and out with an employee number and PIN, no session.
//! Functions take the clock as parameters so tests can set the time.

use crate::audit::{self, Actor, Entry};
use crate::auth::kiosk_lock::KioskLock;
use crate::domain::attendance::{KioskPunch, PunchKind};
use crate::domain::attendance_calc::{self, Shift};
use crate::error::AppError;
use crate::repositories::attendance::{self as repo, KioskEmployee, Review};
use crate::repositories::leave_requests as leave_repo;
use crate::services::auth::{dummy_hash, field, hash_blocking, verify_blocking};
use crate::time;
use chrono::{DateTime, Duration, NaiveDateTime, NaiveTime, Utc};
use serde_json::json;
use sqlx::SqlitePool;
use std::time::Instant;

const LOCKED: &str =
    "Too many wrong PINs. Try again in 15 minutes, or ask HR to set a new PIN for you.";
const ALREADY_IN: &str = "You already clocked in for this shift.";
const NOT_IN: &str =
    "You haven't clocked in yet. If you forgot to, ask HR to correct your attendance.";
/// An open time in older than this is a forgotten time out, which HR fixes by hand.
const LONGEST_SHIFT_HOURS: i64 = 24;
const SEPARATED: [&str; 2] = ["RESIGNED", "TERMINATED"];

/// The kiosk has no signed-in user; audit entries name it instead.
const KIOSK: Actor<'static> = Actor {
    user_id: None,
    username: "kiosk",
};

/// When a punch happens: the office clock for the record, UTC for the audit trail, and a
/// monotonic instant for the lockout.
#[derive(Debug, Clone, Copy)]
pub struct Clock {
    pub local: NaiveDateTime,
    pub utc: DateTime<Utc>,
    pub instant: Instant,
}

impl Clock {
    pub fn now() -> Self {
        Self {
            local: time::local_now(),
            utc: Utc::now(),
            instant: Instant::now(),
        }
    }
}

/// Checks the employee number and PIN, then records a time in or a time out.
pub async fn punch(
    db: &SqlitePool,
    lock: &KioskLock,
    employee_no: &str,
    pin: &str,
    kind: PunchKind,
    clock: Clock,
) -> Result<KioskPunch, AppError> {
    let typed = employee_no.trim();
    let key = KioskLock::key(typed);
    if lock.is_locked(&key, clock.instant) {
        let mut conn = db.acquire().await?;
        audit::record(
            &mut conn,
            clock.utc,
            KIOSK,
            failed(None, typed, "locked", true),
        )
        .await?;
        return Err(AppError::Conflict(LOCKED));
    }

    let mut conn = db.acquire().await?;
    let employee = repo::kiosk_employee(&mut conn, typed).await?;
    drop(conn);
    let checked = employee
        .as_ref()
        .filter(|e| can_punch(e))
        .and_then(|e| e.kiosk_pin_hash.clone());
    // Spend the same time whether or not the number exists, so timing reveals nothing.
    let ok = match checked {
        Some(hash) => verify_blocking(pin.to_string(), hash).await?,
        None => {
            verify_blocking(pin.to_string(), dummy_hash().to_string()).await?;
            false
        }
    };
    let employee = match employee {
        Some(e) if ok => e,
        employee => {
            let reason = match &employee {
                None => "unknown_employee",
                Some(e) if !can_punch(e) => "inactive",
                Some(e) if e.kiosk_pin_hash.is_none() => "no_pin",
                Some(_) => "wrong_pin",
            };
            let locked = lock.fail(&key, clock.instant);
            let mut conn = db.acquire().await?;
            let entry = failed(employee.as_ref().map(|e| e.id), typed, reason, locked);
            audit::record(&mut conn, clock.utc, KIOSK, entry).await?;
            return Err(if locked {
                AppError::Conflict(LOCKED)
            } else {
                AppError::InvalidPin
            });
        }
    };
    lock.clear(&key);

    match kind {
        PunchKind::In => clock_in(db, &employee, clock).await,
        PunchKind::Out => clock_out(db, &employee, clock).await,
    }
}

async fn clock_in(
    db: &SqlitePool,
    e: &KioskEmployee,
    clock: Clock,
) -> Result<KioskPunch, AppError> {
    let shift = shift_of(e);
    let work_date = attendance_calc::work_date_for(shift.as_ref(), clock.local);
    let date = work_date.to_string();
    let at = time::to_local_db(clock.local);

    let mut tx = db.begin().await?;
    if repo::on_date(&mut tx, e.id, &date).await?.is_some() {
        return Err(AppError::Conflict(ALREADY_IN));
    }
    let late_minutes = shift
        .map(|s| attendance_calc::compute(&s, work_date, clock.local, None).late_minutes)
        .unwrap_or(0);
    let mut notes = clock_check(&mut tx, e.id, clock.local).await?;
    if leave_repo::full_day_leave_on(&mut tx, e.id, &date, false).await? {
        notes.push("This employee is on approved leave today. Cancel the leave if they worked.");
    }
    if shift.is_none() {
        notes.push("No work schedule was assigned, so lateness was not worked out.");
    }
    let note = notes.join(" ");
    let review = Review {
        needed: !notes.is_empty(),
        note: (!notes.is_empty()).then_some(note.as_str()),
    };
    let id = repo::insert_clock_in(&mut tx, e.id, &date, &at, late_minutes, &review).await?;
    let entry = Entry {
        action: "attendance.clock_in",
        entity: Some(("attendance_record", id)),
        after: Some(json!({
            "employeeNo": e.employee_no,
            "workDate": date,
            "timeIn": at,
            "lateMinutes": late_minutes,
            "needsReview": review.needed,
        })),
        ..Default::default()
    };
    audit::record(&mut tx, clock.utc, KIOSK, entry).await?;
    tx.commit().await?;
    Ok(KioskPunch {
        first_name: e.first_name.clone(),
        kind: PunchKind::In,
        at,
        work_date: date,
        late_minutes,
    })
}

async fn clock_out(
    db: &SqlitePool,
    e: &KioskEmployee,
    clock: Clock,
) -> Result<KioskPunch, AppError> {
    let at = time::to_local_db(clock.local);
    let mut tx = db.begin().await?;
    let open = repo::latest_open(&mut tx, e.id).await?;
    let open = open.and_then(|r| {
        let time_in = r.time_in.as_deref().and_then(time::from_local_db)?;
        let work_date = r.work_date.parse().ok()?;
        (clock.local - time_in <= Duration::hours(LONGEST_SHIFT_HOURS))
            .then_some((r, work_date, time_in))
    });
    let Some((record, work_date, time_in)) = open else {
        return Err(AppError::Conflict(NOT_IN));
    };

    let computed = shift_of(e)
        .map(|s| attendance_calc::compute(&s, work_date, time_in, Some(clock.local)))
        .unwrap_or_default();
    let notes = clock_check(&mut tx, e.id, clock.local).await?;
    let note = notes.join(" ");
    let review = Review {
        needed: !notes.is_empty(),
        note: (!notes.is_empty()).then_some(note.as_str()),
    };
    repo::set_clock_out(&mut tx, record.id, &at, &computed, &review).await?;
    let entry = Entry {
        action: "attendance.clock_out",
        entity: Some(("attendance_record", record.id)),
        after: Some(json!({
            "employeeNo": e.employee_no,
            "workDate": record.work_date,
            "timeOut": at,
            "workedMinutes": computed.worked_minutes,
            "undertimeMinutes": computed.undertime_minutes,
            "needsReview": review.needed || record.needs_review,
        })),
        ..Default::default()
    };
    audit::record(&mut tx, clock.utc, KIOSK, entry).await?;
    tx.commit().await?;
    Ok(KioskPunch {
        first_name: e.first_name.clone(),
        kind: PunchKind::Out,
        at,
        work_date: record.work_date,
        late_minutes: computed.late_minutes,
    })
}

/// A punch earlier than the employee's last one means the PC clock went back. The punch
/// is still saved, but flagged for HR (plan §6.3).
async fn clock_check(
    conn: &mut sqlx::SqliteConnection,
    employee_id: i64,
    now: NaiveDateTime,
) -> Result<Vec<&'static str>, AppError> {
    let last = repo::last_punch(conn, employee_id)
        .await?
        .and_then(|s| time::from_local_db(&s));
    Ok(if last.is_some_and(|last| now < last) {
        vec!["The PC clock was earlier than this employee's last punch; check the time."]
    } else {
        Vec::new()
    })
}

fn can_punch(e: &KioskEmployee) -> bool {
    e.archived_at.is_none() && !SEPARATED.contains(&e.employment_status.as_str())
}

fn shift_of(e: &KioskEmployee) -> Option<Shift> {
    let t = |s: &Option<String>| {
        s.as_deref()
            .and_then(|s| NaiveTime::parse_from_str(s, "%H:%M").ok())
    };
    Some(Shift {
        start: t(&e.start_time)?,
        end: t(&e.end_time)?,
        break_minutes: e.break_minutes?,
        grace_minutes: e.grace_minutes?,
    })
}

fn failed(employee_id: Option<i64>, typed: &str, reason: &str, locked: bool) -> Entry<'static> {
    let typed: String = typed.chars().take(32).collect();
    Entry {
        action: "kiosk.pin_failed",
        entity: employee_id.map(|id| ("employee", id)),
        after: Some(json!({ "employeeNo": typed, "reason": reason, "lockedOut": locked })),
        ..Default::default()
    }
}

/// Sets or replaces an employee's kiosk PIN (4 to 6 digits). Returns the employee number,
/// so the caller can lift any kiosk lockout on it.
pub async fn set_pin(
    db: &SqlitePool,
    actor: Actor<'_>,
    employee_id: i64,
    pin: &str,
    now: DateTime<Utc>,
) -> Result<String, AppError> {
    if !(4..=6).contains(&pin.len()) || !pin.bytes().all(|b| b.is_ascii_digit()) {
        return Err(AppError::Validation(vec![field(
            "pin",
            "Use 4 to 6 digits",
        )]));
    }
    let employee = crate::repositories::employees::get(db, employee_id)
        .await?
        .ok_or(AppError::NotFound("Employee"))?;
    if employee.archived_at.is_some() {
        return Err(AppError::Conflict(
            "Archived employees can't use the kiosk. Unarchive them first.",
        ));
    }
    let hash = hash_blocking(pin.to_string()).await?;
    let mut tx = db.begin().await?;
    crate::repositories::employees::set_kiosk_pin(&mut tx, employee_id, &hash).await?;
    let entry = Entry {
        action: "employee.set_kiosk_pin",
        entity: Some(("employee", employee_id)),
        // Never the PIN or its hash.
        before: Some(json!({ "hasKioskPin": employee.has_kiosk_pin })),
        after: Some(json!({ "hasKioskPin": true })),
    };
    audit::record(&mut tx, now, actor, entry).await?;
    tx.commit().await?;
    Ok(employee.employee_no)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audit::test_support as audit_rows;
    use crate::domain::attendance::DateRange;
    use crate::services::attendance;
    use chrono::TimeZone;

    const PIN: &str = "4321";

    fn hr() -> Actor<'static> {
        Actor {
            user_id: Some(1),
            username: "hr",
        }
    }

    /// The office clock at `local` ("2026-10-05 08:25").
    fn at(local: &str) -> Clock {
        let local = NaiveDateTime::parse_from_str(local, "%Y-%m-%d %H:%M").expect("time");
        Clock {
            local,
            utc: Utc.from_utc_datetime(&(local - Duration::hours(8))),
            instant: Instant::now(),
        }
    }

    struct Fixture {
        _dir: tempfile::TempDir,
        db: SqlitePool,
        lock: KioskLock,
    }

    impl Fixture {
        async fn punch(
            &self,
            no: &str,
            pin: &str,
            kind: PunchKind,
            clock: Clock,
        ) -> Result<KioskPunch, AppError> {
            punch(&self.db, &self.lock, no, pin, kind, clock).await
        }

        async fn record(&self, employee_id: i64) -> crate::domain::attendance::AttendanceRecord {
            sqlx::query_as(
                "SELECT id, employee_id, work_date, time_in, time_out, status, late_minutes, \
                 undertime_minutes, worked_minutes, night_minutes, source, needs_review, review_note, \
                 0 AS locked FROM attendance_records WHERE employee_id = ? ORDER BY id DESC LIMIT 1",
            )
            .bind(employee_id)
            .fetch_one(&self.db)
            .await
            .expect("record")
        }
    }

    /// An HR user, three schedules and employees: EMP-1 on 08:00–17:00 (10 min grace),
    /// EMP-2 on 22:00–06:00, EMP-3 with no schedule. All have PIN 4321.
    async fn fixture() -> Fixture {
        let dir = tempfile::tempdir().expect("temp dir");
        let db = crate::db::open(&dir.path().join("test.db"))
            .await
            .expect("open");
        let mut conn = db.acquire().await.expect("conn");
        crate::repositories::users::insert(&mut conn, "hr", "x", "HR", false)
            .await
            .expect("user");
        sqlx::query(
            "INSERT INTO work_schedules (id, name, start_time, end_time, break_minutes, \
             grace_minutes, work_days) VALUES \
             (1, 'Office', '08:00', '17:00', 60, 10, 'MON,TUE,WED,THU,FRI'), \
             (2, 'Night', '22:00', '06:00', 60, 0, 'MON,TUE,WED,THU,FRI')",
        )
        .execute(&mut *conn)
        .await
        .expect("schedules");
        sqlx::query(
            "INSERT INTO employees (id, employee_no, first_name, last_name, hire_date, \
             employment_status, schedule_id) VALUES \
             (1, 'EMP-1', 'Juan', 'Dela Cruz', '2025-01-06', 'REGULAR', 1), \
             (2, 'EMP-2', 'Ana', 'Reyes', '2025-01-06', 'REGULAR', 2), \
             (3, 'EMP-3', 'Leo', 'Santos', '2025-01-06', 'PROBATIONARY', NULL)",
        )
        .execute(&mut *conn)
        .await
        .expect("employees");
        drop(conn);
        for id in 1..=3 {
            set_pin(&db, hr(), id, PIN, Utc::now()).await.expect("pin");
        }
        Fixture {
            _dir: dir,
            db,
            lock: KioskLock::default(),
        }
    }

    #[tokio::test]
    async fn clocking_in_and_out_records_the_day() {
        let f = fixture().await;
        let p = f
            .punch(" emp-1 ", PIN, PunchKind::In, at("2026-10-05 08:25"))
            .await
            .expect("in");
        assert_eq!((p.first_name.as_str(), p.kind), ("Juan", PunchKind::In));
        assert_eq!(
            (p.at.as_str(), p.work_date.as_str()),
            ("2026-10-05T08:25:00", "2026-10-05")
        );
        assert_eq!(p.late_minutes, 15);

        let p = f
            .punch("EMP-1", PIN, PunchKind::Out, at("2026-10-05 17:00"))
            .await
            .expect("out");
        assert_eq!(p.kind, PunchKind::Out);
        let r = f.record(1).await;
        assert_eq!(r.time_in.as_deref(), Some("2026-10-05T08:25:00"));
        assert_eq!(r.time_out.as_deref(), Some("2026-10-05T17:00:00"));
        assert_eq!((r.status.as_str(), r.source.as_str()), ("PRESENT", "CLOCK"));
        assert_eq!(
            (r.late_minutes, r.undertime_minutes, r.worked_minutes),
            (15, 0, 455)
        );
        assert!(!r.needs_review);

        let actions = audit_rows::actions(&f.db).await;
        assert!(actions.ends_with(&["attendance.clock_in".into(), "attendance.clock_out".into()]));
    }

    #[tokio::test]
    async fn a_second_clock_in_for_the_same_day_is_refused() {
        let f = fixture().await;
        f.punch("EMP-1", PIN, PunchKind::In, at("2026-10-05 07:58"))
            .await
            .expect("in");
        let again = f
            .punch("EMP-1", PIN, PunchKind::In, at("2026-10-05 08:03"))
            .await;
        assert!(matches!(again, Err(AppError::Conflict(ALREADY_IN))));
        // Still refused after clocking out: one record per day.
        f.punch("EMP-1", PIN, PunchKind::Out, at("2026-10-05 17:00"))
            .await
            .expect("out");
        let again = f
            .punch("EMP-1", PIN, PunchKind::In, at("2026-10-05 18:00"))
            .await;
        assert!(matches!(again, Err(AppError::Conflict(ALREADY_IN))));
        // The next day is fine.
        f.punch("EMP-1", PIN, PunchKind::In, at("2026-10-06 08:00"))
            .await
            .expect("next day");
    }

    #[tokio::test]
    async fn clocking_out_needs_an_open_clock_in() {
        let f = fixture().await;
        let out = f
            .punch("EMP-1", PIN, PunchKind::Out, at("2026-10-05 17:00"))
            .await;
        assert!(matches!(out, Err(AppError::Conflict(NOT_IN))));

        f.punch("EMP-1", PIN, PunchKind::In, at("2026-10-05 08:00"))
            .await
            .expect("in");
        f.punch("EMP-1", PIN, PunchKind::Out, at("2026-10-05 17:00"))
            .await
            .expect("out");
        let twice = f
            .punch("EMP-1", PIN, PunchKind::Out, at("2026-10-05 17:05"))
            .await;
        assert!(matches!(twice, Err(AppError::Conflict(NOT_IN))));
    }

    #[tokio::test]
    async fn a_forgotten_time_out_is_left_for_hr() {
        let f = fixture().await;
        f.punch("EMP-1", PIN, PunchKind::In, at("2026-10-05 08:00"))
            .await
            .expect("in");
        // Two days later the old time in is not closed with a 57-hour shift.
        let out = f
            .punch("EMP-1", PIN, PunchKind::Out, at("2026-10-07 17:00"))
            .await;
        assert!(matches!(out, Err(AppError::Conflict(NOT_IN))));
    }

    #[tokio::test]
    async fn a_night_shift_clocks_out_the_next_morning() {
        let f = fixture().await;
        let p = f
            .punch("EMP-2", PIN, PunchKind::In, at("2026-10-05 21:55"))
            .await
            .expect("in");
        assert_eq!(p.work_date, "2026-10-05");
        let p = f
            .punch("EMP-2", PIN, PunchKind::Out, at("2026-10-06 06:00"))
            .await
            .expect("out");
        assert_eq!(p.work_date, "2026-10-05");
        let r = f.record(2).await;
        assert_eq!(
            (r.worked_minutes, r.night_minutes, r.late_minutes),
            (420, 420, 0)
        );
    }

    #[tokio::test]
    async fn a_clock_that_went_back_is_saved_but_flagged() {
        let f = fixture().await;
        f.punch("EMP-1", PIN, PunchKind::In, at("2026-10-06 08:00"))
            .await
            .expect("in");
        f.punch("EMP-1", PIN, PunchKind::Out, at("2026-10-06 17:00"))
            .await
            .expect("out");
        // Someone sets the PC back a day and clocks in "yesterday".
        f.punch("EMP-1", PIN, PunchKind::In, at("2026-10-05 08:00"))
            .await
            .expect("saved");
        let r = f.record(1).await;
        assert_eq!(r.work_date, "2026-10-05");
        assert!(r.needs_review);
        assert!(r.review_note.as_deref().unwrap_or("").contains("PC clock"));
    }

    #[tokio::test]
    async fn without_a_schedule_the_punch_is_saved_and_flagged() {
        let f = fixture().await;
        let p = f
            .punch("EMP-3", PIN, PunchKind::In, at("2026-10-05 09:40"))
            .await
            .expect("in");
        assert_eq!(p.late_minutes, 0);
        f.punch("EMP-3", PIN, PunchKind::Out, at("2026-10-05 18:00"))
            .await
            .expect("out");
        let r = f.record(3).await;
        assert!(r.needs_review);
        assert_eq!(r.worked_minutes, 0);
        assert!(r
            .review_note
            .as_deref()
            .unwrap_or("")
            .contains("No work schedule"));
    }

    #[tokio::test]
    async fn a_wrong_pin_is_refused_and_audited_without_the_pin() {
        let f = fixture().await;
        let r = f
            .punch("EMP-1", "9999", PunchKind::In, at("2026-10-05 08:00"))
            .await;
        assert!(matches!(r, Err(AppError::InvalidPin)));
        let rows = audit_rows::all(&f.db).await;
        let last = rows.last().expect("audit row");
        assert_eq!(last.action, "kiosk.pin_failed");
        assert_eq!(last.actor_username, "kiosk");
        assert_eq!(last.entity_id, Some(1));
        let after = last.after_json.as_deref().unwrap_or("");
        assert!(after.contains("wrong_pin") && !after.contains("9999"));
    }

    #[tokio::test]
    async fn unknown_numbers_and_missing_pins_get_the_same_answer() {
        let f = fixture().await;
        sqlx::query("UPDATE employees SET kiosk_pin_hash = NULL WHERE id = 3")
            .execute(&f.db)
            .await
            .expect("clear pin");
        let unknown = f
            .punch("EMP-404", PIN, PunchKind::In, at("2026-10-05 08:00"))
            .await;
        let no_pin = f
            .punch("EMP-3", PIN, PunchKind::In, at("2026-10-05 08:00"))
            .await;
        assert!(matches!(unknown, Err(AppError::InvalidPin)));
        assert!(matches!(no_pin, Err(AppError::InvalidPin)));
    }

    #[tokio::test]
    async fn separated_and_archived_employees_cannot_punch() {
        let f = fixture().await;
        sqlx::query("UPDATE employees SET employment_status = 'RESIGNED' WHERE id = 1")
            .execute(&f.db)
            .await
            .expect("resign");
        sqlx::query("UPDATE employees SET archived_at = '2026-10-01T00:00:00Z' WHERE id = 2")
            .execute(&f.db)
            .await
            .expect("archive");
        for no in ["EMP-1", "EMP-2"] {
            let r = f
                .punch(no, PIN, PunchKind::In, at("2026-10-05 08:00"))
                .await;
            assert!(matches!(r, Err(AppError::InvalidPin)), "{no}");
        }
    }

    #[tokio::test]
    async fn five_wrong_pins_lock_out_even_the_right_one_until_hr_sets_a_new_pin() {
        let f = fixture().await;
        let clock = at("2026-10-05 08:00");
        for _ in 0..4 {
            let r = f.punch("EMP-1", "0000", PunchKind::In, clock).await;
            assert!(matches!(r, Err(AppError::InvalidPin)));
        }
        let fifth = f.punch("EMP-1", "0000", PunchKind::In, clock).await;
        assert!(matches!(fifth, Err(AppError::Conflict(LOCKED))));
        let right = f.punch("emp-1", PIN, PunchKind::In, clock).await;
        assert!(matches!(right, Err(AppError::Conflict(LOCKED))));

        // What the set-PIN command does: a new PIN, then lift the lock.
        let no = set_pin(&f.db, hr(), 1, "135790", Utc::now())
            .await
            .expect("new pin");
        f.lock.clear(&KioskLock::key(&no));
        f.punch("EMP-1", "135790", PunchKind::In, clock)
            .await
            .expect("in");
    }

    #[tokio::test]
    async fn a_pin_must_be_4_to_6_digits_and_is_never_audited() {
        let f = fixture().await;
        for bad in ["123", "1234567", "12a4", "", "١٢٣٤"] {
            let r = set_pin(&f.db, hr(), 1, bad, Utc::now()).await;
            assert!(matches!(r, Err(AppError::Validation(_))), "{bad:?}");
        }
        set_pin(&f.db, hr(), 1, "246810", Utc::now())
            .await
            .expect("pin");
        let rows = audit_rows::all(&f.db).await;
        let last = rows.last().expect("audit row");
        assert_eq!(last.action, "employee.set_kiosk_pin");
        let snapshot = format!("{:?}{:?}", last.before_json, last.after_json);
        assert!(!snapshot.contains("246810") && !snapshot.contains("argon2"));
    }

    #[tokio::test]
    async fn own_attendance_lists_the_range_newest_first() {
        let f = fixture().await;
        for day in ["2026-10-05", "2026-10-06", "2026-10-07"] {
            f.punch("EMP-1", PIN, PunchKind::In, at(&format!("{day} 08:00")))
                .await
                .expect("in");
        }
        let range = |from: &str, to: &str| DateRange {
            from: from.into(),
            to: to.into(),
        };
        let rows = attendance::for_employee(&f.db, 1, &range("2026-10-06", "2026-10-31"))
            .await
            .expect("list");
        let dates: Vec<_> = rows.iter().map(|r| r.work_date.as_str()).collect();
        assert_eq!(dates, ["2026-10-07", "2026-10-06"]);
        assert!(
            attendance::for_employee(&f.db, 2, &range("2026-10-01", "2026-10-31"))
                .await
                .expect("list")
                .is_empty()
        );
        for (from, to) in [
            ("2026-10-31", "2026-10-01"),
            ("2026-01-01", "2026-12-31"),
            ("x", "y"),
        ] {
            let r = attendance::for_employee(&f.db, 1, &range(from, to)).await;
            assert!(matches!(r, Err(AppError::Validation(_))), "{from}..{to}");
        }
    }

    #[tokio::test]
    async fn clocking_in_on_approved_leave_is_saved_but_flagged() {
        let f = fixture().await;
        sqlx::query(
            "INSERT INTO leave_requests (employee_id, leave_type_id, start_date, end_date, \
             halfdays, reason, status) SELECT 1, id, '2026-10-06', '2026-10-06', 2, 'Trip', \
             'APPROVED' FROM leave_types WHERE code = 'VL'",
        )
        .execute(&f.db)
        .await
        .expect("leave");
        f.punch("EMP-1", PIN, PunchKind::In, at("2026-10-06 08:00"))
            .await
            .expect("saved");
        let r = f.record(1).await;
        assert!(r.needs_review);
        assert!(r
            .review_note
            .as_deref()
            .unwrap_or("")
            .contains("approved leave"));
    }
}
