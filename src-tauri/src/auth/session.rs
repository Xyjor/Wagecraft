//! Who is signed in right now. Wagecraft runs on one PC, so there is at most one
//! session, held in Rust memory (never in the webview).

use crate::audit::Actor;
use crate::auth::permissions::{Permission, Role};
use crate::error::AppError;
use std::{
    sync::Mutex,
    time::{Duration, Instant},
};

#[derive(Debug, Clone)]
pub struct Session {
    pub user_id: i64,
    /// Kept for the audit trail, so entries name the user even after the account changes.
    pub username: String,
    pub role: Role,
    pub last_activity: Instant,
}

impl Session {
    pub fn actor(&self) -> Actor<'_> {
        Actor {
            user_id: Some(self.user_id),
            username: &self.username,
        }
    }
}

pub struct Auth {
    session: Mutex<Option<Session>>,
    idle_timeout: Mutex<Duration>,
}

impl Auth {
    pub fn new(idle_timeout: Duration) -> Self {
        Self {
            session: Mutex::new(None),
            idle_timeout: Mutex::new(idle_timeout),
        }
    }

    /// Admin changed the idle timeout in Settings. Applies from the next command on.
    pub fn set_idle_timeout(&self, timeout: Duration) {
        *self.idle_timeout.lock().expect("idle timeout lock") = timeout;
    }

    pub fn sign_in(&self, s: Session) {
        *self.session.lock().expect("session lock") = Some(s);
    }

    /// Ends the session and returns it, so the caller can audit who signed out.
    pub fn sign_out(&self) -> Option<Session> {
        self.session.lock().expect("session lock").take()
    }

    /// `check_at` as an `AppError`, for tests. Commands use `AppState::require`,
    /// which also audits an expired session.
    #[cfg(test)]
    pub fn require_at(&self, p: Permission, now: Instant) -> Result<Session, AppError> {
        self.check_at(p, now).map_err(AppError::from)
    }

    /// Checks sign-in, idle time and permission, with the clock passed in so tests
    /// don't wait 15 minutes. An expired session is ended and handed back for auditing.
    pub fn check_at(&self, p: Permission, now: Instant) -> Result<Session, Refusal> {
        let idle_timeout = *self.idle_timeout.lock().expect("idle timeout lock");
        let mut guard = self.session.lock().expect("session lock");
        let s = guard.as_mut().ok_or(Refusal::Unauthenticated)?;
        if now.duration_since(s.last_activity) > idle_timeout {
            let ended = guard.take().expect("session checked above");
            return Err(Refusal::Expired(ended));
        }
        if !s.role.allows(p) {
            return Err(Refusal::Forbidden);
        }
        s.last_activity = now;
        Ok(s.clone())
    }
}

/// Why `check_at` said no.
#[derive(Debug)]
pub enum Refusal {
    Unauthenticated,
    /// The idle timeout passed. Carries the session that just ended.
    Expired(Session),
    Forbidden,
}

impl From<Refusal> for AppError {
    fn from(r: Refusal) -> Self {
        match r {
            Refusal::Unauthenticated => AppError::Unauthenticated,
            Refusal::Expired(_) => AppError::SessionExpired,
            Refusal::Forbidden => AppError::Forbidden,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const IDLE: Duration = Duration::from_secs(15 * 60);

    fn staff(at: Instant) -> Session {
        Session {
            user_id: 1,
            username: "staff".into(),
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
        assert_eq!(s.user_id, 1);
        assert!(matches!(
            auth.require_at(Permission::EmployeeReadAll, t0),
            Err(AppError::Forbidden)
        ));
    }

    #[test]
    fn a_new_idle_timeout_applies_to_the_session_already_signed_in() {
        let auth = Auth::new(IDLE);
        let t0 = Instant::now();
        auth.sign_in(staff(t0));

        auth.set_idle_timeout(Duration::from_secs(5 * 60));

        let six_minutes = t0 + Duration::from_secs(6 * 60);
        assert!(matches!(
            auth.require_at(Permission::SelfPayslip, six_minutes),
            Err(AppError::SessionExpired)
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
        let ended = auth.sign_out().expect("the session that ended");
        assert_eq!(ended.username, "staff");
        assert!(auth.sign_out().is_none());
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
