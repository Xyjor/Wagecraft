use serde::Serialize;
use ts_rs::TS;

/// Who someone is in Wagecraft. Stored in `users.role` as ADMIN, HR or STAFF.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "UPPERCASE")]
#[ts(export)]
pub enum Role {
    Admin,
    Hr,
    Staff,
}

/// Everything a command can ask for. Every command checks one of these in Rust;
/// the UI hiding a button is only a convenience.
// Each variant starts being used when the feature it guards lands.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Permission {
    UserManage,
    SettingsManage,
    BackupManage,
    AuditRead,
    OrgManage,
    EmployeeReadAll,
    EmployeeWrite,
    AttendanceReadAll,
    AttendanceEdit,
    OvertimeDecide,
    LeaveDecide,
    PayrollCompute,
    PayrollApprove,
    PayrollPost,
    PayslipReadAll,
    ReportExport,
    SelfProfile,
    SelfAttendance,
    SelfLeave,
    SelfPayslip,
}

impl Role {
    /// The value stored in `users.role`.
    pub fn as_db(self) -> &'static str {
        match self {
            Role::Admin => "ADMIN",
            Role::Hr => "HR",
            Role::Staff => "STAFF",
        }
    }

    pub fn from_db(s: &str) -> Option<Role> {
        match s {
            "ADMIN" => Some(Role::Admin),
            "HR" => Some(Role::Hr),
            "STAFF" => Some(Role::Staff),
            _ => None,
        }
    }

    /// The permission matrix from §3.2 of the plan.
    pub fn allows(self, p: Permission) -> bool {
        use Permission::*;
        match self {
            Role::Admin => true,
            Role::Hr => !matches!(p, UserManage | SettingsManage | BackupManage | AuditRead),
            Role::Staff => matches!(p, SelfProfile | SelfAttendance | SelfLeave | SelfPayslip),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Permission::*, Role};

    #[test]
    fn staff_gets_only_self_permissions() {
        assert!(Role::Staff.allows(SelfPayslip));
        assert!(!Role::Staff.allows(EmployeeReadAll));
        assert!(!Role::Staff.allows(PayrollCompute));
    }

    #[test]
    fn hr_cannot_manage_system() {
        assert!(Role::Hr.allows(PayrollPost));
        for p in [UserManage, SettingsManage, BackupManage, AuditRead] {
            assert!(!Role::Hr.allows(p), "HR should not have {p:?}");
        }
    }

    #[test]
    fn admin_has_everything() {
        assert!(Role::Admin.allows(UserManage));
        assert!(Role::Admin.allows(PayrollPost)); // decision Q6: Admin can do HR work
    }

    #[test]
    fn role_round_trips_through_the_database_value() {
        for r in [Role::Admin, Role::Hr, Role::Staff] {
            assert_eq!(Role::from_db(r.as_db()), Some(r));
        }
        assert_eq!(Role::from_db("admin"), None);
        assert_eq!(serde_json::to_value(Role::Hr).expect("json"), "HR");
    }

    /// The whole §3.2 matrix, row by row: (permission, admin, hr, staff).
    #[test]
    fn matches_the_permission_matrix_in_the_plan() {
        let matrix = [
            (UserManage, true, false, false),
            (SettingsManage, true, false, false),
            (BackupManage, true, false, false),
            (AuditRead, true, false, false),
            (OrgManage, true, true, false),
            (EmployeeReadAll, true, true, false),
            (EmployeeWrite, true, true, false),
            (AttendanceReadAll, true, true, false),
            (AttendanceEdit, true, true, false),
            (OvertimeDecide, true, true, false),
            (LeaveDecide, true, true, false),
            (PayrollCompute, true, true, false),
            (PayrollApprove, true, true, false),
            (PayrollPost, true, true, false),
            (PayslipReadAll, true, true, false),
            (ReportExport, true, true, false),
            (SelfProfile, true, true, true),
            (SelfAttendance, true, true, true),
            (SelfLeave, true, true, true),
            (SelfPayslip, true, true, true),
        ];
        for (p, admin, hr, staff) in matrix {
            assert_eq!(Role::Admin.allows(p), admin, "Admin / {p:?}");
            assert_eq!(Role::Hr.allows(p), hr, "HR / {p:?}");
            assert_eq!(Role::Staff.allows(p), staff, "Staff / {p:?}");
        }
    }
}
