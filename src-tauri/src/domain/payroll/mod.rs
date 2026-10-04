//! The payroll engine (plan §7): pure functions with no database, clock or files. The
//! service layer gathers the inputs and saves the results.

pub mod engine;
pub mod rates;
pub mod rules;
