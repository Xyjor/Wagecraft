use serde::{ser::SerializeStruct, Serialize, Serializer};

/// One invalid input field, shown next to that field in the form.
#[derive(Debug, Clone, Serialize)]
pub struct FieldError {
    pub field: String,
    pub message: String,
}

/// The only error type a Tauri command returns. It reaches the UI as
/// `{ code, message, fields }`, and `message` is always safe to show.
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("Please sign in")]
    Unauthenticated,
    #[error("You were signed out after a period of inactivity. Please sign in again")]
    SessionExpired,
    #[error("You don't have permission to do this")]
    Forbidden,
    #[error("Some fields are invalid")]
    Validation(Vec<FieldError>),
    #[error("{0} not found")]
    NotFound(&'static str),
    #[error("Wrong username or password")]
    InvalidCredentials,
    /// The kiosk's version of `InvalidCredentials`: never says which part was wrong.
    #[error("Wrong employee number or PIN")]
    InvalidPin,
    /// A business rule refused the change. The message says which rule, in plain words.
    #[error("{0}")]
    Conflict(&'static str),
    #[error("Something went wrong. Please try again")]
    Database(#[from] sqlx::Error),
    #[error("Something went wrong. Please try again")]
    Internal(#[from] anyhow::Error),
}

impl AppError {
    fn code(&self) -> &'static str {
        match self {
            AppError::Unauthenticated => "UNAUTHENTICATED",
            AppError::SessionExpired => "SESSION_EXPIRED",
            AppError::Forbidden => "FORBIDDEN",
            AppError::Validation(_) => "VALIDATION",
            AppError::NotFound(_) => "NOT_FOUND",
            AppError::InvalidCredentials => "INVALID_CREDENTIALS",
            AppError::InvalidPin => "INVALID_PIN",
            AppError::Conflict(_) => "CONFLICT",
            AppError::Database(_) => "DATABASE",
            AppError::Internal(_) => "INTERNAL",
        }
    }
}

impl Serialize for AppError {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        // Full detail goes to the log, never to the UI.
        match self {
            AppError::Database(e) => log::error!("database error: {e}"),
            AppError::Internal(e) => log::error!("internal error: {e:#}"),
            _ => {}
        }
        let fields: &[FieldError] = match self {
            AppError::Validation(f) => f,
            _ => &[],
        };
        let mut st = s.serialize_struct("AppError", 3)?;
        st.serialize_field("code", self.code())?;
        st.serialize_field("message", &self.to_string())?;
        st.serialize_field("fields", fields)?;
        st.end()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn forbidden_serializes_to_code_message_fields() {
        let value = serde_json::to_value(AppError::Forbidden).expect("serialize");
        assert_eq!(
            value,
            json!({
                "code": "FORBIDDEN",
                "message": "You don't have permission to do this",
                "fields": []
            })
        );
    }

    #[test]
    fn validation_carries_its_field_errors() {
        let err = AppError::Validation(vec![FieldError {
            field: "tin".into(),
            message: "TIN must have 9 or 12 digits".into(),
        }]);
        let value = serde_json::to_value(err).expect("serialize");
        assert_eq!(value["code"], "VALIDATION");
        assert_eq!(
            value["fields"],
            json!([{ "field": "tin", "message": "TIN must have 9 or 12 digits" }])
        );
    }

    #[test]
    fn database_error_hides_its_details_from_the_ui() {
        let err = AppError::from(sqlx::Error::Protocol("table users is corrupt".into()));
        let value = serde_json::to_value(err).expect("serialize");
        assert_eq!(value["code"], "DATABASE");
        assert_eq!(value["message"], "Something went wrong. Please try again");
    }
}
