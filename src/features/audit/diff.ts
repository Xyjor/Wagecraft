/** One field of a snapshot as the activity log shows it. */
export type Change = { field: string; before: string | null; after: string | null };

/** A value as one line of text: strings bare, missing values as a dash, the rest as JSON. */
export function show(value: unknown): string {
  if (value === null || value === undefined) return "—";
  if (typeof value === "string") return value;
  return JSON.stringify(value);
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

/** A value as the diff compares and prints it; `null` means the field isn't there. */
function text(value: unknown): string | null {
  if (value === undefined || value === null) return null;
  return typeof value === "string" ? value : JSON.stringify(value);
}

/**
 * The fields that differ between an entry's before and after snapshots, in the order the
 * snapshots list them. A missing snapshot counts as having no fields, so a create shows
 * every field as new and a delete shows every field as gone.
 */
export function changes(before: unknown, after: unknown): Change[] {
  const missing = (v: unknown) => v === null || v === undefined;
  if (missing(before) && missing(after)) return [];
  if ((!isRecord(before) && !missing(before)) || (!isRecord(after) && !missing(after))) {
    return [{ field: "value", before: text(before) ?? null, after: text(after) ?? null }];
  }
  const a = isRecord(before) ? before : {};
  const b = isRecord(after) ? after : {};
  const fields = [...Object.keys(a), ...Object.keys(b).filter((k) => !(k in a))];
  return fields
    .map((field) => ({ field, before: text(a[field]), after: text(b[field]) }))
    .filter((c) => c.before !== c.after);
}
