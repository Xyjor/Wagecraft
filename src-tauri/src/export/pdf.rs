//! Checks a PDF the UI built before it is written to disk (plan §6.6, ADR-010). The UI
//! makes the bytes; Rust only makes sure they are a PDF of a sensible size with a safe name.

use crate::error::AppError;

/// Payslips are a few dozen kilobytes; anything near this is not one of ours.
pub const MAX_BYTES: usize = 5 * 1024 * 1024;

/// The file name to suggest in the Save dialog, if `name` and `bytes` are acceptable.
pub fn checked(name: &str, bytes: &[u8]) -> Result<String, AppError> {
    if !bytes.starts_with(b"%PDF-") || bytes.len() > MAX_BYTES {
        return Err(AppError::Conflict(
            "That file isn't a payslip PDF, so it wasn't saved",
        ));
    }
    // Only the last part of a path, so a name can't point the dialog at another folder.
    let base = name.rsplit(['/', '\\']).next().unwrap_or_default();
    let safe: String = base
        .chars()
        .map(|c| match c {
            'a'..='z' | 'A'..='Z' | '0'..='9' | ' ' | '.' | '-' | '_' => c,
            _ => '-',
        })
        .take(100)
        .collect();
    let safe = safe.trim().trim_start_matches('.');
    let safe = if safe.is_empty() { "payslip" } else { safe };
    Ok(if safe.to_ascii_lowercase().ends_with(".pdf") {
        safe.to_string()
    } else {
        format!("{safe}.pdf")
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const PDF: &[u8] = b"%PDF-1.3\n...";

    #[test]
    fn keeps_a_plain_pdf_name() {
        assert_eq!(
            checked("payslip-EMP-0001-2026-10-16.pdf", PDF).expect("ok"),
            "payslip-EMP-0001-2026-10-16.pdf"
        );
    }

    #[test]
    fn strips_folders_and_odd_characters_from_the_name() {
        assert_eq!(
            checked("..\\..\\Windows/evil:name?.pdf", PDF).expect("ok"),
            "evil-name-.pdf"
        );
        assert_eq!(checked("Pay slip.pdf", PDF).expect("ok"), "Pay slip.pdf");
    }

    #[test]
    fn insists_on_the_pdf_extension() {
        assert_eq!(checked("payslip.exe", PDF).expect("ok"), "payslip.exe.pdf");
        assert_eq!(checked("", PDF).expect("ok"), "payslip.pdf");
    }

    #[test]
    fn refuses_bytes_that_are_not_a_pdf_or_too_big() {
        assert!(matches!(
            checked("a.pdf", b"MZ\x90\x00"),
            Err(AppError::Conflict(_))
        ));
        let mut big = PDF.to_vec();
        big.resize(MAX_BYTES + 1, b' ');
        assert!(matches!(checked("a.pdf", &big), Err(AppError::Conflict(_))));
    }
}
