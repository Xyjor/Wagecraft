/** How the dashboard names employees without a department. */
export const NO_DEPARTMENT = "No department";

/** 1 → "1 leave request", 2 → "2 leave requests". */
export const plural = (n: number, one: string, many: string) => `${n} ${n === 1 ? one : many}`;
