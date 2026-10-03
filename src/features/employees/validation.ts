import type { EmployeeInput } from "@/bindings/EmployeeInput";

// Quick checks so the form can point at mistakes before saving. The full rules, including
// uniqueness, ages and dates, live in services/employees.rs, and those are the ones that count.

type Result = { ok: true; value: EmployeeInput } | { ok: false; errors: Record<string, string> };

const ID_RULES: [field: string, lengths: number[], hint: string][] = [
  ["tin", [9, 12, 13, 14], "Use 000-000-000, with an optional branch code"],
  ["sssNo", [10], "Use 00-0000000-0 (10 digits)"],
  ["philhealthNo", [12], "Use 00-000000000-0 (12 digits)"],
  ["pagibigNo", [12], "Use 0000-0000-0000 (12 digits)"],
];

const REQUIRED: [field: string, message: string][] = [
  ["employeeNo", "Enter the employee number"],
  ["firstName", "Enter the first name"],
  ["lastName", "Enter the last name"],
  ["hireDate", "Enter the hire date"],
  ["employmentStatus", "Pick a status"],
];

/** Blank text becomes null, as the backend stores it. */
const text = (v: string | undefined) => v?.trim() || null;
const id = (v: string | undefined) => Number(v) || null;

export function checkEmployee(v: Record<string, string>): Result {
  const errors: Record<string, string> = {};
  for (const [field, message] of REQUIRED) {
    if (!text(v[field])) errors[field] = message;
  }
  for (const [field, lengths, hint] of ID_RULES) {
    const value = text(v[field]);
    if (!value) continue;
    const digits = value.replace(/[-\s]/g, "");
    if (!/^\d+$/.test(digits) || !lengths.includes(digits.length)) errors[field] = hint;
  }
  const mobile = text(v.mobile)?.replace(/[-\s]/g, "");
  if (mobile && !/^(09\d{9}|\+639\d{9})$/.test(mobile)) {
    errors.mobile = "Use 09XXXXXXXXX or +639XXXXXXXXX";
  }
  const email = text(v.email);
  if (email && !/^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(email)) {
    errors.email = "Enter an email like juan@company.ph";
  }
  if (Object.keys(errors).length) return { ok: false, errors };

  return {
    ok: true,
    value: {
      employeeNo: v.employeeNo.trim(),
      firstName: v.firstName.trim(),
      middleName: text(v.middleName),
      lastName: v.lastName.trim(),
      suffix: text(v.suffix),
      birthDate: text(v.birthDate),
      sex: text(v.sex),
      civilStatus: text(v.civilStatus),
      email,
      mobile: text(v.mobile),
      address: text(v.address),
      hireDate: v.hireDate.trim(),
      regularizationDate: text(v.regularizationDate),
      separationDate: text(v.separationDate),
      employmentStatus: v.employmentStatus,
      departmentId: id(v.departmentId),
      positionId: id(v.positionId),
      scheduleId: id(v.scheduleId),
      tin: text(v.tin),
      sssNo: text(v.sssNo),
      philhealthNo: text(v.philhealthNo),
      pagibigNo: text(v.pagibigNo),
      bankName: text(v.bankName),
      bankAccountNo: text(v.bankAccountNo),
    },
  };
}

/** Checks the kiosk PIN form: 4 to 6 digits, typed the same twice. */
export function checkPin(
  v: Record<string, string>,
): { ok: true; value: string } | { ok: false; errors: Record<string, string> } {
  const pin = v.pin ?? "";
  if (!/^[0-9]{4,6}$/.test(pin)) return { ok: false, errors: { pin: "Use 4 to 6 digits" } };
  if (pin !== (v.confirmPin ?? "")) {
    return { ok: false, errors: { confirmPin: "The two PINs don't match" } };
  }
  return { ok: true, value: pin };
}
