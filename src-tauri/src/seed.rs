//! Demo data for trying Wagecraft and for checking speed with a realistic company size
//! (plan §2.3: 200 employees). Run it with `pnpm seed`.
//!
//! Everything goes through the same services as the screens, so the demo data obeys every
//! rule and shows up in the audit log as done by "seed". The numbers come from a fixed
//! pseudo-random sequence, so every run on an empty database produces the same company.

use crate::audit::Actor;
use crate::domain::compensation::{pay_period_start, CompensationInput};
use crate::domain::employee::EmployeeInput;
use crate::domain::org::{DepartmentInput, PositionInput, WorkScheduleInput};
use crate::services::{compensation, employees, org};
use chrono::{Datelike, Days, Months, NaiveDate, Utc};
use sqlx::SqlitePool;
use std::collections::HashSet;
use std::path::Path;

const SEED: Actor<'static> = Actor {
    user_id: None,
    username: "seed",
};

/// What a seed run added.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Summary {
    pub schedules: usize,
    pub departments: usize,
    pub positions: usize,
    pub employees: usize,
    pub rates: usize,
}

/// Opens the database at `path` (creating it if needed) and adds the demo company.
pub fn run(path: &Path, count: usize) -> anyhow::Result<Summary> {
    tauri::async_runtime::block_on(async {
        let db = crate::db::open(path).await?;
        let today = chrono::Local::now().date_naive();
        demo_company(&db, count, today).await
    })
}

/// Monthly salary ranges are in pesos; a `None` range means a daily-paid position.
struct PositionPlan {
    title: &'static str,
    monthly: Option<(i64, i64)>,
    /// Daily rate in pesos for daily-paid positions.
    daily: i64,
    weight: u32,
}

struct DepartmentPlan {
    code: &'static str,
    name: &'static str,
    /// Index into `SCHEDULES`.
    schedule: usize,
    positions: &'static [PositionPlan],
}

/// Work schedules: (name, start, end, work days).
const SCHEDULES: &[(&str, &str, &str, &str)] = &[
    ("Office (Mon–Fri)", "08:00", "17:00", "MON,TUE,WED,THU,FRI"),
    (
        "Warehouse (Mon–Sat)",
        "07:00",
        "16:00",
        "MON,TUE,WED,THU,FRI,SAT",
    ),
    (
        "Night shift (Mon–Fri)",
        "22:00",
        "06:00",
        "MON,TUE,WED,THU,FRI",
    ),
];

const fn monthly(title: &'static str, min: i64, max: i64, weight: u32) -> PositionPlan {
    PositionPlan {
        title,
        monthly: Some((min, max)),
        daily: 0,
        weight,
    }
}

const fn daily(title: &'static str, rate: i64, weight: u32) -> PositionPlan {
    PositionPlan {
        title,
        monthly: None,
        daily: rate,
        weight,
    }
}

const COMPANY: &[DepartmentPlan] = &[
    DepartmentPlan {
        code: "OPS",
        schedule: 1,
        name: "Operations",
        positions: &[
            monthly("Operations Supervisor", 30_000, 42_000, 2),
            monthly("Driver", 18_000, 25_000, 6),
            daily("Warehouse Helper", 695, 10),
            daily("Forklift Operator", 750, 4),
        ],
    },
    DepartmentPlan {
        code: "SAL",
        schedule: 0,
        name: "Sales",
        positions: &[
            monthly("Sales Manager", 45_000, 65_000, 1),
            monthly("Account Executive", 22_000, 35_000, 6),
            monthly("Sales Coordinator", 18_000, 24_000, 3),
        ],
    },
    DepartmentPlan {
        code: "FIN",
        schedule: 0,
        name: "Finance",
        positions: &[
            monthly("Finance Manager", 55_000, 80_000, 1),
            monthly("Accountant", 28_000, 40_000, 2),
            monthly("Accounting Clerk", 18_000, 23_000, 3),
        ],
    },
    DepartmentPlan {
        code: "HR",
        schedule: 0,
        name: "Human Resources",
        positions: &[
            monthly("HR Manager", 50_000, 70_000, 1),
            monthly("HR Associate", 20_000, 28_000, 2),
        ],
    },
    DepartmentPlan {
        code: "IT",
        schedule: 0,
        name: "Information Technology",
        positions: &[
            monthly("IT Lead", 60_000, 90_000, 1),
            monthly("Software Developer", 35_000, 60_000, 2),
            monthly("IT Support", 20_000, 28_000, 2),
        ],
    },
    DepartmentPlan {
        code: "CS",
        schedule: 2,
        name: "Customer Service",
        positions: &[
            monthly("Customer Service Lead", 26_000, 34_000, 1),
            monthly("Customer Service Representative", 17_000, 22_000, 8),
        ],
    },
];

const FIRST_MALE: &[&str] = &[
    "Juan",
    "Jose",
    "Mark",
    "John Paul",
    "Christian",
    "Angelo",
    "Carlo",
    "Rafael",
    "Miguel",
    "Paolo",
    "Jerome",
    "Ramon",
    "Andres",
    "Emmanuel",
    "Joshua",
    "Kenneth",
    "Renato",
    "Ernesto",
    "Danilo",
    "Rodel",
];
const FIRST_FEMALE: &[&str] = &[
    "Maria",
    "Ana",
    "Kristine",
    "Angelica",
    "Jasmine",
    "Patricia",
    "Camille",
    "Rosalie",
    "Maricel",
    "Jocelyn",
    "Liza",
    "Erlinda",
    "Nicole",
    "Bea",
    "Rhea",
    "Grace",
    "Shiela",
    "Joanna",
    "Princess",
    "Ma. Theresa",
];
const LAST: &[&str] = &[
    "Dela Cruz",
    "Santos",
    "Reyes",
    "Garcia",
    "Mendoza",
    "Bautista",
    "Villanueva",
    "Ramos",
    "Aquino",
    "Castillo",
    "Fernandez",
    "Gonzales",
    "Navarro",
    "Torres",
    "Domingo",
    "Mercado",
    "Pascual",
    "Salazar",
    "Aguilar",
    "Peñaflor",
    "Ocampo",
    "Manalo",
    "Soriano",
    "De Guzman",
    "Lopez",
    "Rivera",
    "Tolentino",
    "Cruz",
    "Valdez",
    "Magbanua",
];
const SUFFIXES: &[&str] = &["Jr.", "Sr.", "III"];
const BANKS: &[&str] = &[
    "BDO",
    "BPI",
    "Metrobank",
    "Landbank",
    "UnionBank",
    "Security Bank",
];
const STREETS: &[&str] = &[
    "Rizal St.",
    "Mabini St.",
    "Bonifacio Ave.",
    "Luna St.",
    "Del Pilar St.",
];
const CITIES: &[&str] = &[
    "Quezon City",
    "Pasig City",
    "Makati City",
    "Caloocan City",
    "Marikina City",
    "Taguig City",
    "Valenzuela City",
    "Antipolo City",
];

/// A small linear congruential generator: fixed seed, no extra dependency, same output on
/// every machine.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        self.0 >> 33
    }

    /// 0..n
    fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }

    fn pick<'a, T>(&mut self, items: &'a [T]) -> &'a T {
        &items[self.below(items.len() as u64) as usize]
    }

    fn chance(&mut self, percent: u64) -> bool {
        self.below(100) < percent
    }
}

pub(crate) async fn demo_company(
    db: &SqlitePool,
    count: usize,
    today: NaiveDate,
) -> anyhow::Result<Summary> {
    let existing: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM employees")
        .fetch_one(db)
        .await?;
    anyhow::ensure!(
        existing == 0,
        "this database already has {existing} employees; seed only an empty one"
    );

    let now = Utc::now();
    let mut summary = Summary::default();
    let mut rng = Rng(2026);

    // Work schedules, reusing any that already exist by name.
    let existing_schedules = org::list_schedules(db).await?;
    let mut schedule_ids = Vec::new();
    for (name, start, end, days) in SCHEDULES {
        let id = match existing_schedules.iter().find(|x| x.name == *name) {
            Some(x) => x.id,
            None => {
                summary.schedules += 1;
                let input = WorkScheduleInput {
                    name: (*name).into(),
                    start_time: (*start).into(),
                    end_time: (*end).into(),
                    break_minutes: 60,
                    grace_minutes: 10,
                    work_days: (*days).into(),
                };
                org::create_schedule(db, SEED, input, now).await?.id
            }
        };
        schedule_ids.push(id);
    }

    // Departments and positions, reusing any that already exist by code or title.
    let mut slots = Vec::new(); // (department, position, schedule, plan) repeated by weight
    let departments = org::list_departments(db).await?;
    let positions = org::list_positions(db).await?;
    for d in COMPANY {
        let dept_id = match departments.iter().find(|x| x.code == d.code) {
            Some(x) => x.id,
            None => {
                summary.departments += 1;
                let input = DepartmentInput {
                    code: d.code.into(),
                    name: d.name.into(),
                    description: None,
                };
                org::create_department(db, SEED, input, now).await?.id
            }
        };
        for p in d.positions {
            let existing = positions
                .iter()
                .find(|x| x.department_id == dept_id && x.title == p.title);
            let pos_id = match existing {
                Some(x) => x.id,
                None => {
                    summary.positions += 1;
                    let input = PositionInput {
                        department_id: dept_id,
                        title: p.title.into(),
                        min_rate_cents: p.monthly.map(|(min, _)| min * 100),
                        max_rate_cents: p.monthly.map(|(_, max)| max * 100),
                    };
                    org::create_position(db, SEED, input, now).await?.id
                }
            };
            for _ in 0..p.weight {
                slots.push((dept_id, pos_id, schedule_ids[d.schedule], p));
            }
        }
    }

    let mut emails = HashSet::new();
    for i in 1..=count {
        let (dept_id, pos_id, schedule_id, plan) = *rng.pick(&slots);
        let female = rng.chance(50);
        let first = *rng.pick(if female { FIRST_FEMALE } else { FIRST_MALE });
        let last = *rng.pick(LAST);
        let middle = *rng.pick(LAST);
        let suffix = (!female && rng.chance(4)).then(|| rng.pick(SUFFIXES).to_string());

        // Hired between 2015 and last month; at least 18 on the hire date.
        let earliest = NaiveDate::from_ymd_opt(2015, 1, 5).expect("valid date");
        let span = (today - earliest).num_days().max(31) as u64 - 30;
        let hire = earliest + Days::new(rng.below(span));
        let age_at_hire = 19 + rng.below(30) as u32;
        let birth =
            hire - Months::new(age_at_hire * 12 + rng.below(12) as u32) - Days::new(rng.below(28));

        let tenure_days = (today - hire).num_days();
        let (status, regularized, separated) = if rng.chance(6) && tenure_days > 120 {
            let left = hire + Days::new(90 + rng.below(tenure_days as u64 - 90));
            let status = if rng.chance(80) {
                "RESIGNED"
            } else {
                "TERMINATED"
            };
            (status, None, Some(left))
        } else if tenure_days < 200 {
            ("PROBATIONARY", None, None)
        } else if rng.chance(8) {
            ("CONTRACTUAL", None, None)
        } else {
            ("REGULAR", Some(hire + Months::new(6)), None)
        };

        let handle = format!("{}.{}", first, last)
            .to_lowercase()
            .replace(['ñ'], "n")
            .chars()
            .filter(|c| c.is_ascii_alphanumeric() || *c == '.')
            .collect::<String>();
        let mut email = format!("{handle}@demo.wagecraft.ph");
        if !emails.insert(email.clone()) {
            email = format!("{handle}{i}@demo.wagecraft.ph");
            emails.insert(email.clone());
        }

        let input = EmployeeInput {
            employee_no: format!("EMP-{i:04}"),
            first_name: first.into(),
            middle_name: Some(middle.into()),
            last_name: last.into(),
            suffix,
            birth_date: Some(iso(birth)),
            sex: Some(if female { "FEMALE" } else { "MALE" }.into()),
            civil_status: Some(
                (*rng.pick(&[
                    "SINGLE", "SINGLE", "MARRIED", "MARRIED", "MARRIED", "WIDOWED",
                ]))
                .into(),
            ),
            email: rng.chance(85).then_some(email),
            mobile: Some(format!(
                "09{}{:07}",
                15 + rng.below(85),
                (i as u64 * 37 + rng.below(30)) % 10_000_000
            )),
            address: Some(format!(
                "{} {}, {}",
                1 + rng.below(999),
                rng.pick(STREETS),
                rng.pick(CITIES)
            )),
            hire_date: iso(hire),
            regularization_date: regularized.map(iso),
            separation_date: separated.map(iso),
            employment_status: status.into(),
            department_id: Some(dept_id),
            position_id: Some(pos_id),
            schedule_id: Some(schedule_id),
            // Unique by construction: the employee's index is part of every number.
            tin: Some(format!("{:03}{:06}", 100 + rng.below(800), i)),
            sss_no: Some(format!(
                "{:02}{:04}{:04}",
                10 + rng.below(80),
                i,
                rng.below(10_000)
            )),
            philhealth_no: Some(format!(
                "{:02}{:05}{:05}",
                1 + rng.below(98),
                i,
                rng.below(100_000)
            )),
            pagibig_no: Some(format!("12{:05}{:05}", i, rng.below(100_000))),
            bank_name: Some(rng.pick(BANKS).to_string()),
            bank_account_no: Some(format!("{:010}", rng.below(10_000_000_000))),
        };
        let employee = employees::create(db, SEED, input, today, now).await?;
        summary.employees += 1;

        // Starting rate from the hire period, and a raise for most people past a year.
        let (basis, start) = match plan.monthly {
            Some((min, max)) => {
                let steps = ((max - min) / 500).max(1) as u64;
                (
                    "MONTHLY",
                    (min + 500 * rng.below(steps / 2 + 1) as i64) * 100,
                )
            }
            None => ("DAILY", plan.daily * 100),
        };
        let mut rates = vec![(pay_period_start(hire), start, "Starting rate")];
        let last_day = separated.unwrap_or(today);
        let anniversary = hire + Months::new(12);
        if anniversary < last_day && rng.chance(75) {
            let raise = start + start * (3 + rng.below(8)) as i64 / 100;
            rates.push((
                next_period_start(anniversary),
                raise - raise % 100,
                "Annual review",
            ));
        }
        for (from, rate_cents, reason) in rates {
            if from > last_day {
                continue;
            }
            let input = CompensationInput {
                pay_basis: basis.into(),
                rate_cents,
                effective_from: iso(from),
                reason: Some(reason.into()),
            };
            compensation::add(db, SEED, employee.id, input, now).await?;
            summary.rates += 1;
        }

        // People who left a while ago are archived, as HR would do.
        if separated.is_some_and(|d| (today - d).num_days() > 60) {
            employees::set_archived(db, SEED, employee.id, true, now).await?;
        }
    }
    Ok(summary)
}

fn iso(d: NaiveDate) -> String {
    d.format("%Y-%m-%d").to_string()
}

/// The first pay period that starts on or after `d`.
fn next_period_start(d: NaiveDate) -> NaiveDate {
    let start = pay_period_start(d);
    if start == d {
        d
    } else if d.day() <= 15 {
        d.with_day(16).expect("the 16th exists")
    } else {
        (start + Months::new(1))
            .with_day(1)
            .expect("the 1st exists")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::employee::{Archived, EmployeeQuery};
    use std::time::Instant;

    fn today() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 10, 7).unwrap()
    }

    #[test]
    fn next_period_start_rounds_up_to_the_1st_or_16th() {
        let d = |s| NaiveDate::parse_from_str(s, "%Y-%m-%d").unwrap();
        assert_eq!(next_period_start(d("2026-03-01")), d("2026-03-01"));
        assert_eq!(next_period_start(d("2026-03-02")), d("2026-03-16"));
        assert_eq!(next_period_start(d("2026-03-17")), d("2026-04-01"));
        assert_eq!(next_period_start(d("2026-12-20")), d("2027-01-01"));
    }

    #[tokio::test]
    async fn seeds_200_valid_employees_and_search_stays_fast() {
        let dir = tempfile::tempdir().expect("temp dir");
        let db = crate::db::open(&dir.path().join("test.db"))
            .await
            .expect("open");
        let summary = demo_company(&db, 200, today()).await.expect("seed");
        assert_eq!(summary.schedules, 3);
        assert_eq!(summary.departments, 6);
        assert_eq!(summary.positions, 17);
        assert_eq!(summary.employees, 200);
        assert!(summary.rates >= 200, "{summary:?}");

        // Plan §2.3: employee search returns in under 200 ms with 200 employees.
        let started = Instant::now();
        let page = employees::list(
            &db,
            EmployeeQuery {
                search: Some("santos".into()),
                archived: Archived::Include,
                ..Default::default()
            },
        )
        .await
        .expect("search");
        assert!(
            started.elapsed().as_millis() < 200,
            "{:?}",
            started.elapsed()
        );
        assert!(page.total > 0);

        let again = demo_company(&db, 200, today()).await.unwrap_err();
        assert!(again.to_string().contains("already has 200 employees"));
    }
}
