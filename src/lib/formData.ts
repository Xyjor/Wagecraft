import type { AppError } from "@/lib/ipc";

/** Reads every named input of a form as strings. */
export function formValues(form: HTMLFormElement): Record<string, string> {
  return Object.fromEntries([...new FormData(form)].map(([k, v]) => [k, String(v)]));
}

/** Splits a server AppError into per-field messages and a form-level message. */
export function serverErrors(e: AppError): { fields: Record<string, string>; alert?: string } {
  const fields: Record<string, string> = {};
  for (const f of e.fields ?? []) fields[f.field] ??= f.message;
  return { fields, alert: e.fields?.length ? undefined : e.message };
}
