use serde::{ser::SerializeStruct, Serialize, Serializer};

/// One invalid input field, shown next to that field in the form.
#[derive(Debug, Clone, Serialize)]
pub struct FieldError {
    pub field: String,
    pub message: String,
}

/// The only error type a Tauri command returns. It reaches the UI as
/// `{ code, message, fields }`, and `message` is always safe to show.
// Unauthenticated, Forbidden, Validation and NotFound are first used by the sign-in work in Week 2.
#[allow(dead_code)]
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("Please sign in")]
    Unauthenticated,
    #[error("You don't have permission to do this")]
    Forbidden,
    #[error("Some fields are invalid")]
    Validation(Vec<FieldError>),
    #[error("{0} not found")]
    NotFound(&'static str),
    #[error("Something went wrong. Please try again")]
    Database(#[from] sqlx::Error),
}

impl AppError {
    fn code(&self) -> &'static str {
        match self {
            AppError::Unauthenticated => "UNAUTHENTICATED",
            AppError::Forbidden => "FORBIDDEN",
            AppError::Validation(_) => "VALIDATION",
            AppError::NotFound(_) => "NOT_FOUND",
            AppError::Database(_) => "DATABASE",
        }
    }
}

impl Serialize for AppError {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        if let AppError::Database(e) = self {
            log::error!("database error: {e}"); // full detail to the log, never to the UI
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
