//! SQL for payroll rule packs (plan §5.3, ADR-005): the rates, brackets and premiums a
//! payroll period is computed with, loaded into the engine's [`RulePack`].

use crate::domain::payroll::rules::{
    DayType, PagIbigRule, PhilHealthRule, PremiumRate, RulePack, SssBracket, TaxBracket,
};
use crate::domain::payroll_period::{
    PremiumRow, RulePackDetail, RulePackSettings as Settings, RulePackSummary,
};
use rust_decimal::Decimal;
use sqlx::{SqliteConnection, SqliteExecutor};

const SUMMARY: &str =
    "SELECT id, code, name, effective_from, effective_to FROM rule_packs WHERE is_active = 1";

/// The packs HR can pick, newest first.
pub async fn active<'e>(db: impl SqliteExecutor<'e>) -> sqlx::Result<Vec<RulePackSummary>> {
    sqlx::query_as(&format!("{SUMMARY} ORDER BY effective_from DESC"))
        .fetch_all(db)
        .await
}

/// The pack with this id, if it is active.
pub async fn active_by_id<'e>(
    db: impl SqliteExecutor<'e>,
    id: i64,
) -> sqlx::Result<Option<RulePackSummary>> {
    sqlx::query_as(&format!("{SUMMARY} AND id = ?"))
        .bind(id)
        .fetch_optional(db)
        .await
}

/// Every value in the pack with this id, active or not, for the viewer.
pub async fn detail(
    conn: &mut SqliteConnection,
    id: i64,
) -> anyhow::Result<Option<RulePackDetail>> {
    let Some((settings,)): Option<(String,)> =
        sqlx::query_as("SELECT settings_json FROM rule_packs WHERE id = ?")
            .bind(id)
            .fetch_optional(&mut *conn)
            .await?
    else {
        return Ok(None);
    };
    let summary = sqlx::query_as(
        "SELECT id, code, name, effective_from, effective_to FROM rule_packs WHERE id = ?",
    )
    .bind(id)
    .fetch_one(&mut *conn)
    .await?;
    let sss_brackets = sqlx::query_as(
        "SELECT range_from_cents AS from_cents, range_to_cents AS to_cents, \
         base_cents AS msc_cents, ee_cents, er_cents, ec_cents FROM contribution_brackets \
         WHERE rule_pack_id = ? AND agency = 'SSS' ORDER BY range_from_cents",
    )
    .bind(id)
    .fetch_all(&mut *conn)
    .await?;
    let tax_brackets = sqlx::query_as(
        "SELECT frequency, over_cents, not_over_cents, base_tax_cents, rate_bp FROM tax_brackets \
         WHERE rule_pack_id = ? ORDER BY frequency = 'MONTHLY', over_cents",
    )
    .bind(id)
    .fetch_all(&mut *conn)
    .await?;
    let mut premiums: Vec<PremiumRow> = sqlx::query_as(
        "SELECT day_type, work_bp, ot_bp, night_diff_bp FROM premium_rates WHERE rule_pack_id = ?",
    )
    .bind(id)
    .fetch_all(&mut *conn)
    .await?;
    premiums.sort_by_key(|r| DAY_TYPES.iter().position(|(name, _)| *name == r.day_type));

    Ok(Some(RulePackDetail {
        summary,
        settings: serde_json::from_str(&settings)?,
        sss_brackets,
        tax_brackets,
        premiums,
    }))
}

fn pesos(cents: i64) -> Decimal {
    Decimal::new(cents, 2)
}

/// Basis points as a fraction: 12,500 bp is 1.25.
fn rate(bp: i64) -> Decimal {
    Decimal::new(bp, 4)
}

const DAY_TYPES: [(&str, DayType); 8] = [
    ("ORDINARY", DayType::Ordinary),
    ("REST_DAY", DayType::RestDay),
    ("SPECIAL", DayType::Special),
    ("SPECIAL_REST_DAY", DayType::SpecialRestDay),
    ("REGULAR", DayType::Regular),
    ("REGULAR_REST_DAY", DayType::RegularRestDay),
    ("DOUBLE_REGULAR", DayType::DoubleRegular),
    ("DOUBLE_REGULAR_REST_DAY", DayType::DoubleRegularRestDay),
];

/// The rule pack with this code, such as `PH-2026`.
pub async fn by_code(conn: &mut SqliteConnection, code: &str) -> anyhow::Result<Option<RulePack>> {
    let Some((id, settings)): Option<(i64, String)> =
        sqlx::query_as("SELECT id, settings_json FROM rule_packs WHERE code = ?")
            .bind(code)
            .fetch_optional(&mut *conn)
            .await?
    else {
        return Ok(None);
    };
    let s: Settings = serde_json::from_str(&settings)?;

    let sss: Vec<(i64, i64, i64)> = sqlx::query_as(
        "SELECT range_from_cents, base_cents, ec_cents FROM contribution_brackets \
         WHERE rule_pack_id = ? AND agency = 'SSS' ORDER BY range_from_cents",
    )
    .bind(id)
    .fetch_all(&mut *conn)
    .await?;
    let tax: Vec<(i64, i64, i64)> = sqlx::query_as(
        "SELECT over_cents, base_tax_cents, rate_bp FROM tax_brackets \
         WHERE rule_pack_id = ? AND frequency = 'SEMI_MONTHLY' ORDER BY over_cents",
    )
    .bind(id)
    .fetch_all(&mut *conn)
    .await?;
    let rows: Vec<(String, i64, i64, i64)> = sqlx::query_as(
        "SELECT day_type, work_bp, ot_bp, night_diff_bp FROM premium_rates WHERE rule_pack_id = ?",
    )
    .bind(id)
    .fetch_all(&mut *conn)
    .await?;
    let premium = |name: &str| {
        rows.iter()
            .find(|r| r.0 == name)
            .ok_or_else(|| anyhow::anyhow!("rule pack {code} has no {name} premium rate"))
    };
    let mut premiums = Vec::with_capacity(DAY_TYPES.len());
    for (name, day) in DAY_TYPES {
        let (_, work, ot, _) = premium(name)?;
        premiums.push((
            day,
            PremiumRate {
                work: rate(*work),
                overtime: rate(*ot),
            },
        ));
    }
    let night = premium("ORDINARY")?.3;
    anyhow::ensure!(
        rows.iter().all(|r| r.3 == night),
        "rule pack {code} has a different night differential for some day types"
    );

    Ok(Some(RulePack {
        factor_five_day: s.factor_five_day,
        factor_six_day: s.factor_six_day,
        night_differential: rate(night),
        premiums: premiums.try_into().expect("one rate per day type"),
        sss_brackets: sss
            .into_iter()
            .map(|(from, msc, ec)| SssBracket {
                from: pesos(from),
                msc: pesos(msc),
                ec: pesos(ec),
            })
            .collect(),
        sss_employee_rate: rate(s.sss_employee_bp),
        sss_employer_rate: rate(s.sss_employer_bp),
        philhealth: PhilHealthRule {
            rate: rate(s.philhealth.rate_bp),
            floor: pesos(s.philhealth.floor_cents),
            ceiling: pesos(s.philhealth.ceiling_cents),
        },
        pagibig: PagIbigRule {
            low_pay_limit: pesos(s.pagibig.low_pay_limit_cents),
            low_rate: rate(s.pagibig.low_rate_bp),
            employee_rate: rate(s.pagibig.employee_bp),
            employer_rate: rate(s.pagibig.employer_bp),
            max_base: pesos(s.pagibig.max_base_cents),
        },
        tax_semi_monthly: tax
            .into_iter()
            .map(|(over, base_tax, bp)| TaxBracket {
                over: pesos(over),
                base_tax: pesos(base_tax),
                rate: rate(bp),
            })
            .collect(),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn db() -> (tempfile::TempDir, sqlx::SqlitePool) {
        let dir = tempfile::tempdir().expect("temp dir");
        let pool = crate::db::open(&dir.path().join("test.db"))
            .await
            .expect("open");
        (dir, pool)
    }

    /// The seeded pack must hold exactly the values the 20 golden cases were checked
    /// against, so a payroll computed from the database matches them.
    #[tokio::test]
    async fn the_seeded_ph_2026_pack_has_the_appendix_a_values() {
        let (_d, db) = db().await;
        let pack = by_code(&mut db.acquire().await.unwrap(), "PH-2026")
            .await
            .unwrap();
        assert_eq!(pack, Some(RulePack::ph_2026()));
    }

    #[tokio::test]
    async fn an_unknown_code_has_no_pack() {
        let (_d, db) = db().await;
        assert_eq!(
            by_code(&mut db.acquire().await.unwrap(), "PH-1999")
                .await
                .unwrap(),
            None
        );
    }
}
