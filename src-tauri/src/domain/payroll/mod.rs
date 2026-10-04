//! The payroll engine (plan §7): pure functions with no database, clock or files. The
//! service layer gathers the inputs and saves the results.

pub mod contributions;
pub mod engine;
#[cfg(test)]
pub mod fixtures;
pub mod payslip;
pub mod rates;
pub mod rules;
pub mod tax;
