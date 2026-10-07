//! Payroll periods (plan §5.3, §6.5): the types the Payroll screen uses, and the rule for
//! which days a semi-monthly cutoff covers.

use crate::domain::settings::CompanyHeader;
use chrono::{Datelike, Days, NaiveDate};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, FromRow, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct PayrollPeriod {
    #[ts(type = "number")]
    pub id: i64,
    pub period_start: String,
    pub period_end: String,
    pub pay_date: String,
    /// 1 for the 1st to the 15th, 2 for the 16th to the end of the month.
    #[ts(type = "1 | 2")]
    pub cutoff_no: i64,
    #[ts(type = "\"DRAFT\" | \"COMPUTED\" | \"APPROVED\" | \"POSTED\"")]
    pub status: String,
    pub rule_pack_code: String,
    pub created_by_name: Option<String>,
    /// UTC, `YYYY-MM-DDTHH:MM:SSZ`.
    pub created_at: String,
}

/// Creating a period. It is named by its first day, the 1st or the 16th.
#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct PayrollPeriodInput {
    pub period_start: String,
    pub pay_date: String,
    #[ts(type = "number")]
    pub rule_pack_id: i64,
}

/// A rule pack HR can pick for a period.
#[derive(Debug, Clone, Serialize, FromRow, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct RulePackSummary {
    #[ts(type = "number")]
    pub id: i64,
    pub code: String,
    pub name: String,
    pub effective_from: String,
    pub effective_to: Option<String>,
}

/// Everything in a rule pack, for the read-only viewer (plan §6.5). Money is in centavos
/// and rates in basis points, as stored; the screen formats them.
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct RulePackDetail {
    pub summary: RulePackSummary,
    pub settings: RulePackSettings,
    pub sss_brackets: Vec<SssBracketRow>,
    /// Semi-monthly brackets first, each lowest first.
    pub tax_brackets: Vec<TaxBracketRow>,
    pub premiums: Vec<PremiumRow>,
}

/// `rule_packs.settings_json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct RulePackSettings {
    /// Paid days per year for a 5-day and a 6-day work week.
    pub factor_five_day: u32,
    pub factor_six_day: u32,
    #[ts(type = "number")]
    pub sss_employee_bp: i64,
    #[ts(type = "number")]
    pub sss_employer_bp: i64,
    pub philhealth: PhilHealthSettings,
    pub pagibig: PagIbigSettings,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct PhilHealthSettings {
    #[ts(type = "number")]
    pub rate_bp: i64,
    #[ts(type = "number")]
    pub floor_cents: i64,
    #[ts(type = "number")]
    pub ceiling_cents: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct PagIbigSettings {
    /// Monthly pay up to this uses the low rate for the employee's share.
    #[ts(type = "number")]
    pub low_pay_limit_cents: i64,
    #[ts(type = "number")]
    pub low_rate_bp: i64,
    #[ts(type = "number")]
    pub employee_bp: i64,
    #[ts(type = "number")]
    pub employer_bp: i64,
    #[ts(type = "number")]
    pub max_base_cents: i64,
}

/// One SSS Monthly Salary Credit bracket. `to_cents` is inclusive; `None` on the top one.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, FromRow, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SssBracketRow {
    #[ts(type = "number")]
    pub from_cents: i64,
    #[ts(type = "number | null")]
    pub to_cents: Option<i64>,
    #[ts(type = "number")]
    pub msc_cents: i64,
    #[ts(type = "number")]
    pub ee_cents: i64,
    #[ts(type = "number")]
    pub er_cents: i64,
    /// Employees' Compensation, paid by the employer.
    #[ts(type = "number")]
    pub ec_cents: i64,
}

/// One withholding tax bracket: tax is `base_tax` plus `rate` of the amount over `over`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, FromRow, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct TaxBracketRow {
    #[ts(type = "\"SEMI_MONTHLY\" | \"MONTHLY\"")]
    pub frequency: String,
    #[ts(type = "number")]
    pub over_cents: i64,
    #[ts(type = "number | null")]
    pub not_over_cents: Option<i64>,
    #[ts(type = "number")]
    pub base_tax_cents: i64,
    #[ts(type = "number")]
    pub rate_bp: i64,
}

/// Pay for work, overtime and night hours on one kind of day, as a share of the hourly rate.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, FromRow, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct PremiumRow {
    pub day_type: String,
    #[ts(type = "number")]
    pub work_bp: i64,
    #[ts(type = "number")]
    pub ot_bp: i64,
    #[ts(type = "number")]
    pub night_diff_bp: i64,
}

/// Unfinished work inside a period that would change its payslips (plan §6.5). These are
/// warnings: HR can still create the period and sort them out before computing.
#[derive(Debug, Clone, Default, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct PeriodChecks {
    pub pending_leave: Vec<PendingItem>,
    pub pending_overtime: Vec<PendingItem>,
    pub missing_time_outs: Vec<PendingItem>,
}

#[derive(Debug, Clone, Serialize, FromRow, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct PendingItem {
    pub employee_no: String,
    pub employee_name: String,
    /// The day it is about, or the first day of a leave request.
    pub date: String,
}

/// A computed period: one row per payslip, plus anyone left out and why.
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct PayrollRegister {
    pub period: PayrollPeriod,
    pub rows: Vec<RegisterRow>,
    pub skipped: Vec<SkippedEmployee>,
}

/// The payroll register as a report (plan §6.8): each deduction in its own column, and a
/// totals row. Deductions are positive amounts.
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct RegisterReport {
    pub period: PayrollPeriod,
    pub company: CompanyHeader,
    pub rows: Vec<RegisterReportRow>,
    /// Every column added up; the name is "Total" and the number is empty.
    pub totals: RegisterReportRow,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, FromRow, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct RegisterReportRow {
    pub employee_no: String,
    pub employee_name: String,
    #[ts(type = "number")]
    pub gross_cents: i64,
    #[ts(type = "number")]
    pub sss_cents: i64,
    #[ts(type = "number")]
    pub philhealth_cents: i64,
    #[ts(type = "number")]
    pub pagibig_cents: i64,
    #[ts(type = "number")]
    pub tax_cents: i64,
    /// Loans and other deductions.
    #[ts(type = "number")]
    pub other_deductions_cents: i64,
    #[ts(type = "number")]
    pub net_cents: i64,
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct RegisterRow {
    #[ts(type = "number")]
    pub payslip_id: i64,
    pub employee_no: String,
    pub employee_name: String,
    #[ts(type = "number")]
    pub gross_cents: i64,
    #[ts(type = "number")]
    pub statutory_ee_cents: i64,
    #[ts(type = "number")]
    pub tax_cents: i64,
    #[ts(type = "number")]
    pub other_deductions_cents: i64,
    #[ts(type = "number")]
    pub net_cents: i64,
    pub warnings: Vec<String>,
}

/// Someone employed during the period who has no payslip.
#[derive(Debug, Clone, Serialize, FromRow, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SkippedEmployee {
    pub employee_no: String,
    pub employee_name: String,
    pub reason: String,
}

/// One of the signed-in employee's posted payslips, for their list.
#[derive(Debug, Clone, Serialize, FromRow, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct MyPayslip {
    #[ts(type = "number")]
    pub id: i64,
    pub period_start: String,
    pub period_end: String,
    pub pay_date: String,
    #[ts(type = "number")]
    pub gross_cents: i64,
    #[ts(type = "number")]
    pub net_cents: i64,
}

/// One payslip, line by line.
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct PayslipDetail {
    #[ts(type = "number")]
    pub id: i64,
    pub period_start: String,
    pub period_end: String,
    pub pay_date: String,
    /// For the payslip header.
    pub company: CompanyHeader,
    pub employee_no: String,
    pub employee_name: String,
    pub department: Option<String>,
    pub position: Option<String>,
    #[ts(type = "\"MONTHLY\" | \"DAILY\"")]
    pub pay_basis: String,
    #[ts(type = "number")]
    pub rate_cents: i64,
    pub lines: Vec<PayslipLine>,
    #[ts(type = "number")]
    pub gross_cents: i64,
    #[ts(type = "number")]
    pub taxable_cents: i64,
    #[ts(type = "number")]
    pub statutory_ee_cents: i64,
    #[ts(type = "number")]
    pub tax_cents: i64,
    #[ts(type = "number")]
    pub other_deductions_cents: i64,
    #[ts(type = "number")]
    pub net_cents: i64,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, FromRow, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct PayslipLine {
    #[ts(type = "\"EARNING\" | \"DEDUCTION\" | \"EMPLOYER_SHARE\"")]
    pub kind: String,
    pub code: String,
    pub label: String,
    /// Decimal text, such as "1.5".
    pub quantity: String,
    #[ts(type = "\"DAYS\" | \"HOURS\" | \"MINUTES\" | null")]
    pub unit: Option<String>,
    /// Deductions are negative.
    #[ts(type = "number")]
    pub amount_cents: i64,
    pub taxable: bool,
}

/// The cutoff number and last day of the semi-monthly period starting on `start`, or `None`
/// when `start` isn't the 1st or the 16th.
pub fn cutoff(start: NaiveDate) -> Option<(i64, NaiveDate)> {
    match start.day() {
        1 => Some((1, start.with_day(15)?)),
        16 => {
            let next_month = start
                .with_day(1)?
                .checked_add_months(chrono::Months::new(1))?;
            Some((2, next_month.checked_sub_days(Days::new(1))?))
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(s: &str) -> NaiveDate {
        NaiveDate::parse_from_str(s, "%Y-%m-%d").unwrap()
    }

    #[test]
    fn the_first_cutoff_runs_to_the_15th() {
        assert_eq!(cutoff(d("2026-10-01")), Some((1, d("2026-10-15"))));
    }

    #[test]
    fn the_second_cutoff_runs_to_the_last_day_of_the_month() {
        assert_eq!(cutoff(d("2026-10-16")), Some((2, d("2026-10-31"))));
        assert_eq!(cutoff(d("2026-11-16")), Some((2, d("2026-11-30"))));
        assert_eq!(cutoff(d("2027-02-16")), Some((2, d("2027-02-28"))));
        assert_eq!(cutoff(d("2028-02-16")), Some((2, d("2028-02-29"))));
        assert_eq!(cutoff(d("2026-12-16")), Some((2, d("2026-12-31"))));
    }

    #[test]
    fn other_days_do_not_start_a_period() {
        for day in ["2026-10-02", "2026-10-15", "2026-10-17", "2026-10-31"] {
            assert_eq!(cutoff(d(day)), None, "{day}");
        }
    }
}
