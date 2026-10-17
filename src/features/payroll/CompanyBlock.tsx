import { Image, StyleSheet, Text, View, type TextProps } from "@react-pdf/renderer";
import type { CompanyHeader } from "@/bindings/CompanyHeader";
import { companyLines } from "./companyHeader";
import { PDF } from "./pdfTheme";

const styles = StyleSheet.create({
  // A green rule under the company, like the head of a ledger page.
  row: {
    flexDirection: "row",
    alignItems: "center",
    paddingBottom: 8,
    borderBottom: `1.5pt solid ${PDF.accent}`,
  },
  logo: { maxHeight: 40, maxWidth: 120, marginRight: 10, objectFit: "contain" },
  muted: { color: PDF.muted },
});

/** The logo, then the company name with its address and TIN under it, for the PDFs. */
export function CompanyBlock({
  company,
  fallback,
  nameStyle,
}: {
  company: CompanyHeader;
  /** What the heading says when setup never named the company. */
  fallback: string;
  nameStyle: TextProps["style"];
}) {
  return (
    <View style={styles.row}>
      {company.logo ? <Image src={company.logo} style={styles.logo} /> : null}
      <View>
        <Text style={nameStyle}>{company.name || fallback}</Text>
        {companyLines(company).map((line) => (
          <Text key={line} style={styles.muted}>
            {line}
          </Text>
        ))}
      </View>
    </View>
  );
}
