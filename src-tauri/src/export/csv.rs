//! CSV files that open cleanly in Excel (plan §6.8, §10): UTF-8 with a byte-order mark so
//! "₱" and "ñ" show correctly, and cells that start like a formula made inert.

const BOM: &[u8] = b"\xEF\xBB\xBF";

/// A cell starting with `=`, `+`, `-` or `@` gets a leading `'`, so Excel shows it as text
/// instead of running it as a formula (CSV injection).
pub fn safe_cell(value: &str) -> std::borrow::Cow<'_, str> {
    if value.starts_with(['=', '+', '-', '@']) {
        format!("'{value}").into()
    } else {
        value.into()
    }
}

/// Writes a header row and data rows, quoting as needed.
pub fn build<R, C>(headers: &[&str], rows: R) -> anyhow::Result<Vec<u8>>
where
    R: IntoIterator<Item = Vec<C>>,
    C: AsRef<str>,
{
    let mut out = BOM.to_vec();
    {
        let mut w = csv::Writer::from_writer(&mut out);
        w.write_record(headers)?;
        for row in rows {
            w.write_record(row.iter().map(|c| safe_cell(c.as_ref()).into_owned()))?;
        }
        w.flush()?;
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn starts_with_a_bom_and_quotes_commas() {
        let bytes = build(
            &["Name", "Rate"],
            vec![vec!["Dela Cruz, Juan", "₱25,000.00"]],
        )
        .unwrap();
        assert!(bytes.starts_with(BOM));
        let text = std::str::from_utf8(&bytes[3..]).unwrap();
        assert_eq!(text, "Name,Rate\n\"Dela Cruz, Juan\",\"₱25,000.00\"\n");
    }

    #[test]
    fn neutralizes_formula_like_cells() {
        for evil in ["=HYPERLINK(\"x\")", "+1", "-2+3", "@SUM(A1)"] {
            assert_eq!(safe_cell(evil), format!("'{evil}"));
        }
        assert_eq!(safe_cell("Peñaflor"), "Peñaflor");
    }
}
