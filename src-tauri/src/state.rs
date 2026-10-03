use crate::auth::kiosk_lock::KioskLock;
use crate::auth::permissions::Permission;
use crate::auth::session::{Auth, Refusal, Session};
use crate::error::AppError;
use crate::services;
use chrono::Utc;
use std::time::Instant;

/// Everything the commands share. Tauri hands it to each command as `State<AppState>`.
pub struct AppState {
    pub db: sqlx::SqlitePool,
    pub auth: Auth,
    /// Kiosk PIN lockouts, by employee number.
    pub kiosk: KioskLock,
}

impl AppState {
    /// The first line of every command: checks sign-in, idle time and permission.
    /// When the idle timeout has just ended the session, it also audits that.
    pub async fn require(&self, p: Permission) -> Result<Session, AppError> {
        match self.auth.check_at(p, Instant::now()) {
            Ok(session) => Ok(session),
            Err(Refusal::Expired(session)) => {
                let audited =
                    services::auth::session_expired(&self.db, session.actor(), Utc::now()).await;
                if let Err(e) = audited {
                    // Still sign the user out; a missing audit entry must not keep them in.
                    log::error!("could not audit an expired session: {e}");
                }
                Err(AppError::SessionExpired)
            }
            Err(other) => Err(other.into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audit::test_support as audit_rows;
    use crate::auth::permissions::Role;
    use std::time::Duration;

    #[tokio::test]
    async fn an_idle_timeout_is_audited_once() {
        let dir = tempfile::tempdir().expect("temp dir");
        let db = crate::db::open(&dir.path().join("test.db"))
            .await
            .expect("open");
        let mut conn = db.acquire().await.expect("conn");
        let id = crate::repositories::users::insert(&mut conn, "maria", "x", "HR", false)
            .await
            .expect("user");
        drop(conn);
        let state = AppState {
            db,
            auth: Auth::new(Duration::from_secs(60)),
            kiosk: KioskLock::default(),
        };
        state.auth.sign_in(Session {
            user_id: id,
            username: "maria".into(),
            role: Role::Hr,
            last_activity: Instant::now() - Duration::from_secs(61),
        });

        let first = state.require(Permission::SelfProfile).await.unwrap_err();
        let second = state.require(Permission::SelfProfile).await.unwrap_err();

        assert!(matches!(first, AppError::SessionExpired));
        assert!(matches!(second, AppError::Unauthenticated));
        let rows = audit_rows::all(&state.db).await;
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].action, "auth.session_expired");
        assert_eq!(rows[0].actor_user_id, Some(id));
        assert_eq!(rows[0].actor_username, "maria");
    }
}
