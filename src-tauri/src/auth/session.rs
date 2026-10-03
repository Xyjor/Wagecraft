//! Who is signed in right now. Wagecraft runs on one PC, so there is at most one
//! session, held in Rust memory (never in the webview).

use crate::auth::permissions::{Permission, Role};
use crate::error::AppError;
use std::{
    sync::Mutex,
    time::{Duration, Instant},
};

#[derive(Debug, Clone)]
pub struct Session {
    pub user_id: i64,
    /// The employee this user is, if any. "Self" commands read this, never an id from the UI.
    pub employee_id: Option<i64>,
    pub role: Role,
    pub last_activity: Instant,
}

pub struct Auth {
    session: Mutex<Option<Session>>,
    idle_timeout: Duration,
}

impl Auth {
    pub fn new(idle_timeout: Duration) -> Self {
        Self {
            session: Mutex::new(None),
            idle_timeout,
        }
    }

    pub fn sign_in(&self, s: Session) {
        *self.session.lock().expect("session lock") = Some(s);
    }

    pub fn sign_out(&self) {
        *self.session.lock().expect("session lock") = None;
    }

    /// The first line of every command: checks sign-in, idle time and permission.
    pub fn require(&self, p: Permission) -> Result<Session, AppError> {
        self.require_at(p, Instant::now())
    }

    /// `require` with the clock passed in, so tests don't wait 15 minutes.
    pub fn require_at(&self, p: Permission, now: Instant) -> Result<Session, AppError> {
        let mut guard = self.session.lock().expect("session lock");
        let s = guard.as_mut().ok_or(AppError::Unauthenticated)?;
        if now.duration_since(s.last_activity) > self.idle_timeout {
            *guard = None;
            return Err(AppError::SessionExpired);
        }
        if !s.role.allows(p) {
            return Err(AppError::Forbidden);
        }
        s.last_activity = now;
        Ok(s.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const IDLE: Duration = Duration::from_secs(15 * 60);

    fn staff(at: Instant) -> Session {
        Session {
            user_id: 1,
            employee_id: Some(7),
            role: Role::Staff,
            last_activity: at,
        }
    }

    #[test]
    fn nobody_signed_in_is_unauthenticated() {
        let auth = Auth::new(IDLE);
        let r = auth.require_at(Permission::SelfProfile, Instant::now());
        assert!(matches!(r, Err(AppError::Unauthenticated)));
    }

    #[test]
    fn role_decides_what_is_allowed() {
        let auth = Auth::new(IDLE);
        let t0 = Instant::now();
        auth.sign_in(staff(t0));

        let s = auth
            .require_at(Permission::SelfPayslip, t0)
            .expect("allowed");
        assert_eq!(s.employee_id, Some(7));
        assert!(matches!(
            auth.require_at(Permission::EmployeeReadAll, t0),
            Err(AppError::Forbidden)
        ));
    }

    #[test]
    fn idle_too_long_expires_once_then_is_signed_out() {
        let auth = Auth::new(IDLE);
        let t0 = Instant::now();
        auth.sign_in(staff(t0));

        let later = t0 + Duration::from_secs(16 * 60);
        assert!(matches!(
            auth.require_at(Permission::SelfPayslip, later),
            Err(AppError::SessionExpired)
        ));
        assert!(matches!(
            auth.require_at(Permission::SelfPayslip, later),
            Err(AppError::Unauthenticated)
        ));
    }

    #[test]
    fn each_allowed_call_resets_the_idle_clock() {
        let auth = Auth::new(IDLE);
        let t0 = Instant::now();
        auth.sign_in(staff(t0));

        let t10 = t0 + Duration::from_secs(10 * 60);
        auth.require_at(Permission::SelfProfile, t10)
            .expect("active");
        // 20 minutes after sign-in, but only 10 after the last call.
        let t20 = t0 + Duration::from_secs(20 * 60);
        assert!(auth.require_at(Permission::SelfProfile, t20).is_ok());
    }

    #[test]
    fn a_forbidden_call_does_not_keep_the_session_alive() {
        let auth = Auth::new(IDLE);
        let t0 = Instant::now();
        auth.sign_in(staff(t0));

        let t10 = t0 + Duration::from_secs(10 * 60);
        let _ = auth.require_at(Permission::EmployeeReadAll, t10);
        let t16 = t0 + Duration::from_secs(16 * 60);
        assert!(matches!(
            auth.require_at(Permission::SelfProfile, t16),
            Err(AppError::SessionExpired)
        ));
    }

    #[test]
    fn sign_out_ends_the_session() {
        let auth = Auth::new(IDLE);
        let t0 = Instant::now();
        auth.sign_in(staff(t0));
        auth.sign_out();
        assert!(matches!(
            auth.require_at(Permission::SelfProfile, t0),
            Err(AppError::Unauthenticated)
        ));
    }

    #[test]
    fn session_expired_has_its_own_error_code() {
        let value = serde_json::to_value(AppError::SessionExpired).expect("serialize");
        assert_eq!(value["code"], "SESSION_EXPIRED");
    }
}
