// Money is integer centavos everywhere (plan §5.1). These convert to and from what people
// read and type, using string and integer math only, never floating point.

/** 2500000 → "₱25,000.00" */
export function formatPesos(cents: number): string {
  const sign = cents < 0 ? "-" : "";
  const abs = Math.abs(cents);
  const pesos = Math.trunc(abs / 100).toLocaleString("en-US");
  const centavos = String(abs % 100).padStart(2, "0");
  return `${sign}₱${pesos}.${centavos}`;
}

/** "₱25,000.50" or "25000.5" → 2500050. Returns null for anything that isn't a plain amount. */
export function parsePesos(text: string): number | null {
  const cleaned = text.trim().replace(/^₱/, "").replace(/,/g, "");
  const match = /^(\d*)(?:\.(\d{1,2}))?$/.exec(cleaned);
  if (!match || (match[1] === "" && match[2] === undefined)) return null;
  const pesos = Number(match[1] || "0");
  const centavos = Number((match[2] ?? "0").padEnd(2, "0"));
  return pesos * 100 + centavos;
}
