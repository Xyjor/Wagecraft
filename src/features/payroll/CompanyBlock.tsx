import { Image, StyleSheet, Text, View, type TextProps } from "@react-pdf/renderer";
import type { CompanyHeader } from "@/bindings/CompanyHeader";
import { companyLines } from "./companyHeader";

const styles = StyleSheet.create({
  row: { flexDirection: "row", alignItems: "center" },
  logo: { maxHeight: 40, maxWidth: 120, marginRight: 10, objectFit: "contain" },
  muted: { color: "#52525b" },
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
