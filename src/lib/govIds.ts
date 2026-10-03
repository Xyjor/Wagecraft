// Government IDs are stored as digits only. These show them in their official format.

export type IdKind = "tin" | "sss" | "philhealth" | "pagibig";

const GROUPS: Record<IdKind, number[][]> = {
  // TIN: 9 digits plus an optional 3–5 digit branch code.
  tin: [
    [3, 3, 3],
    [3, 3, 3, 3],
    [3, 3, 3, 4],
    [3, 3, 3, 5],
  ],
  sss: [[2, 7, 1]],
  philhealth: [[2, 9, 1]],
  pagibig: [[4, 4, 4]],
};

export function formatId(kind: IdKind, digits: string | null | undefined): string {
  if (!digits) return "";
  const groups = GROUPS[kind].find((g) => g.reduce((a, b) => a + b, 0) === digits.length);
  if (!groups) return digits;
  const parts: string[] = [];
  let at = 0;
  for (const size of groups) {
    parts.push(digits.slice(at, at + size));
    at += size;
  }
  return parts.join("-");
}
