// How employee values read on screen.

export const STATUS_LABELS: Record<string, string> = {
  PROBATIONARY: "Probationary",
  REGULAR: "Regular",
  CONTRACTUAL: "Contractual",
  RESIGNED: "Resigned",
  TERMINATED: "Terminated",
};

export const SEX_LABELS: Record<string, string> = { MALE: "Male", FEMALE: "Female" };

export const CIVIL_STATUS_LABELS: Record<string, string> = {
  SINGLE: "Single",
  MARRIED: "Married",
  WIDOWED: "Widowed",
  SEPARATED: "Separated",
};

type Named = {
  firstName: string;
  middleName: string | null;
  lastName: string;
  suffix: string | null;
};

/** "Dela Cruz Jr., Juan S." for lists, which sort by last name. */
export function listName(e: Named): string {
  const last = e.suffix ? `${e.lastName} ${e.suffix}` : e.lastName;
  const middle = e.middleName ? ` ${e.middleName[0]}.` : "";
  return `${last}, ${e.firstName}${middle}`;
}

/** "Juan Santos Dela Cruz Jr." for headings. */
export function fullName(e: Named): string {
  return [e.firstName, e.middleName, e.lastName, e.suffix].filter(Boolean).join(" ");
}
