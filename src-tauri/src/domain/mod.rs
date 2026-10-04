//! Types shared by repositories, services and commands. No database or Tauri code here.

pub mod attendance;
pub mod attendance_calc;
pub mod compensation;
pub mod employee;
pub mod leave;
pub mod org;
pub mod overtime;
// The payroll service that calls the engine arrives in a later PR.
#[allow(dead_code)]
pub mod payroll;
pub mod schedule;
pub mod validation;
