import { StyleSheet } from "@react-pdf/renderer";

// The Ledger palette for the PDFs. They print, so they use the light values only, and every
// colour still reads in black and white: negatives also carry parentheses, totals a double rule.
// Kept in step with the light tokens in src/index.css by hand (react-pdf can't read CSS).
export const PDF = {
  ink: "#1b2420",
  muted: "#4f5d55",
  faint: "#606d65",
  rule: "#d5dfd4",
  ruleStrong: "#a9b8ab",
  accent: "#1f6b55",
  accentSoft: "#e3efe8",
  danger: "#b3261e",
} as const;

/** react-pdf has no double border, so a double rule is two thin rules 1.5pt apart. */
export const doubleRule = StyleSheet.create({
  above: {
    borderTop: `0.75pt solid ${PDF.ruleStrong}`,
    paddingTop: 1.5,
  },
  aboveInner: {
    borderTop: `0.75pt solid ${PDF.ruleStrong}`,
  },
  left: {
    borderLeft: `0.75pt solid ${PDF.ruleStrong}`,
    paddingLeft: 1.5,
  },
  leftInner: {
    borderLeft: `0.75pt solid ${PDF.ruleStrong}`,
  },
});

/** A negative amount is written in red ink. */
export function amountInk(cents: number) {
  return cents < 0 ? { color: PDF.danger } : {};
}
