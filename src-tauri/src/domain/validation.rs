//! Field rules from plan §6.11, as pure functions. Government IDs and mobile numbers
//! are accepted with or without dashes and spaces, and stored as digits only.

/// Keeps the digits of `s` if everything else is a dash or space; otherwise `None`.
fn digits_only(s: &str) -> Option<String> {
    if s.chars()
        .all(|c| c.is_ascii_digit() || c == '-' || c == ' ')
    {
        Some(s.chars().filter(char::is_ascii_digit).collect())
    } else {
        None
    }
}

/// TIN: 9 digits plus an optional 3 to 5 digit branch code.
pub fn tin(s: &str) -> Option<String> {
    digits_only(s).filter(|d| matches!(d.len(), 9 | 12..=14))
}

/// SSS number: 10 digits.
pub fn sss(s: &str) -> Option<String> {
    digits_only(s).filter(|d| d.len() == 10)
}

/// PhilHealth number: 12 digits.
pub fn philhealth(s: &str) -> Option<String> {
    digits_only(s).filter(|d| d.len() == 12)
}

/// Pag-IBIG MID: 12 digits.
pub fn pagibig(s: &str) -> Option<String> {
    digits_only(s).filter(|d| d.len() == 12)
}

/// Mobile: `09XXXXXXXXX` or `+639XXXXXXXXX`, stored as `09XXXXXXXXX`.
pub fn mobile(s: &str) -> Option<String> {
    let s = s.trim();
    let rest = s.strip_prefix("+63").map(|r| format!("0{r}"));
    let d = digits_only(rest.as_deref().unwrap_or(s))?;
    (d.len() == 11 && d.starts_with("09")).then_some(d)
}

/// A person's name part: 1 to 60 characters of letters (ñ and accents included),
/// spaces, hyphens, apostrophes and periods, with at least one letter.
pub fn name(s: &str) -> bool {
    let n = s.chars().count();
    (1..=60).contains(&n)
        && s.chars().any(char::is_alphabetic)
        && s.chars()
            .all(|c| c.is_alphabetic() || matches!(c, ' ' | '-' | '\'' | '.'))
}

/// A practical email check: one `@`, something before it, a dotted domain after, no spaces.
pub fn email(s: &str) -> bool {
    let Some((local, domain)) = s.split_once('@') else {
        return false;
    };
    s.len() <= 254
        && !local.is_empty()
        && !domain.contains('@')
        && !s.chars().any(char::is_whitespace)
        && domain.split('.').count() >= 2
        && domain.split('.').all(|part| !part.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tin_accepts_9_digits_or_a_branch_code() {
        assert_eq!(tin("123-456-789").as_deref(), Some("123456789"));
        assert_eq!(tin("123-456-789-000").as_deref(), Some("123456789000"));
        assert_eq!(tin("123 456 789 00000").as_deref(), Some("12345678900000"));
        for bad in [
            "12345678",
            "123-456-789-0",
            "123-456-789-00",
            "123-456-789-000000",
            "12a456789",
        ] {
            assert_eq!(tin(bad), None, "{bad}");
        }
    }

    #[test]
    fn sss_philhealth_and_pagibig_lengths() {
        assert_eq!(sss("34-1234567-8").as_deref(), Some("3412345678"));
        assert_eq!(sss("34-1234567"), None);
        assert_eq!(
            philhealth("12-345678901-2").as_deref(),
            Some("123456789012")
        );
        assert_eq!(philhealth("12-34567890-2"), None);
        assert_eq!(pagibig("1234-5678-9012").as_deref(), Some("123456789012"));
        assert_eq!(pagibig("1234-5678-901"), None);
    }

    #[test]
    fn mobile_accepts_both_ph_forms() {
        assert_eq!(mobile("09171234567").as_deref(), Some("09171234567"));
        assert_eq!(mobile("+639171234567").as_deref(), Some("09171234567"));
        assert_eq!(mobile("0917 123 4567").as_deref(), Some("09171234567"));
        for bad in [
            "9171234567",
            "08171234567",
            "+63917123456",
            "+1 917 123 4567",
        ] {
            assert_eq!(mobile(bad), None, "{bad}");
        }
    }

    #[test]
    fn names_allow_filipino_and_accented_letters() {
        for good in [
            "Juan",
            "Dela Cruz",
            "Peña",
            "O'Neil",
            "Ma. Theresa",
            "José-Luis",
        ] {
            assert!(name(good), "{good}");
        }
        for bad in ["", "J0hn", "Juan!", "---", &"a".repeat(61)] {
            assert!(!name(bad), "{bad}");
        }
    }

    #[test]
    fn email_shape() {
        assert!(email("juan.delacruz@rojyx.ph"));
        for bad in [
            "juan",
            "juan@",
            "@rojyx.ph",
            "juan@rojyx",
            "juan @rojyx.ph",
            "a@b@c.ph",
            "j@x..ph",
        ] {
            assert!(!email(bad), "{bad}");
        }
    }
}
