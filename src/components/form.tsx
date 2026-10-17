import type { ReactNode } from "react";
import { primaryButton } from "./ui";

/** A text input or select in a form. Exported for the few inputs that aren't a Field. */
export const inputClass =
  "rounded-md border border-field bg-surface px-3 py-2 text-sm text-ink outline-none placeholder:text-ink-faint focus-visible:border-accent focus-visible:ring-2 focus-visible:ring-focus/40 aria-invalid:border-danger";

/** A compact select or input in a filter row above a table. */
export const filterClass =
  "h-9 rounded-md border border-field bg-surface px-2 py-1.5 text-sm text-ink outline-none focus-visible:border-accent focus-visible:ring-2 focus-visible:ring-focus/40";

const labelClass = "text-sm font-medium text-ink";
const errorClass = "text-sm text-danger";

type FieldProps = {
  name: string;
  label: string;
  type?: "text" | "password" | "date" | "email" | "tel" | "time" | "number" | "month";
  autoComplete?: string;
  /** Which on-screen keyboard to offer, such as "numeric" for a PIN. */
  inputMode?: "numeric" | "text";
  maxLength?: number;
  autoFocus?: boolean;
  defaultValue?: string;
  /** Called on each change, for forms that react while someone types. */
  onChange?: (value: string) => void;
  error?: string;
};

/** A labelled input. Uncontrolled: the parent form reads values through FormData. */
export function Field({
  name,
  label,
  type = "text",
  autoComplete,
  inputMode,
  maxLength,
  autoFocus,
  defaultValue,
  onChange,
  error,
}: FieldProps) {
  const errorId = `${name}-error`;
  return (
    <div className="flex flex-col gap-1">
      <label htmlFor={name} className={labelClass}>
        {label}
      </label>
      <input
        id={name}
        name={name}
        type={type}
        autoComplete={autoComplete}
        inputMode={inputMode}
        maxLength={maxLength}
        autoFocus={autoFocus}
        defaultValue={defaultValue}
        onChange={onChange && ((e) => onChange(e.target.value))}
        aria-invalid={error ? true : undefined}
        aria-describedby={error ? errorId : undefined}
        className={inputClass}
      />
      {error && (
        <p id={errorId} className={errorClass}>
          {error}
        </p>
      )}
    </div>
  );
}

type SelectFieldProps = {
  name: string;
  label: string;
  options: { value: string; label: string }[];
  defaultValue?: string;
  onChange?: (value: string) => void;
  error?: string;
};

export function SelectField({
  name,
  label,
  options,
  defaultValue,
  onChange,
  error,
}: SelectFieldProps) {
  const errorId = `${name}-error`;
  return (
    <div className="flex flex-col gap-1">
      <label htmlFor={name} className={labelClass}>
        {label}
      </label>
      <select
        id={name}
        name={name}
        defaultValue={defaultValue}
        onChange={onChange && ((e) => onChange(e.target.value))}
        aria-invalid={error ? true : undefined}
        aria-describedby={error ? errorId : undefined}
        className={inputClass}
      >
        {options.map((o) => (
          <option key={o.value} value={o.value}>
            {o.label}
          </option>
        ))}
      </select>
      {error && (
        <p id={errorId} className={errorClass}>
          {error}
        </p>
      )}
    </div>
  );
}

/** The card for sign-in, first-run setup and the forced password change. */
export function AuthCard({
  title,
  intro,
  children,
}: {
  title: string;
  intro?: string;
  children: ReactNode;
}) {
  return (
    <main className="flex min-h-screen flex-col items-center justify-center gap-6 bg-canvas p-6 text-ink">
      <p
        aria-hidden="true"
        className="border-b-[3px] border-double border-rule-strong pb-0.5 text-lg font-semibold tracking-tight text-accent"
      >
        Wagecraft
      </p>
      <div className="w-full max-w-sm rounded-sm border border-rule bg-surface p-6 shadow-[0_1px_2px_rgb(0_0_0/0.06)]">
        <h1 className="text-xl font-semibold">{title}</h1>
        {intro && <p className="mt-1 text-sm text-ink-muted">{intro}</p>}
        <div className="mt-6">{children}</div>
      </div>
    </main>
  );
}

export function SubmitButton({ busy, children }: { busy: boolean; children: ReactNode }) {
  return (
    <button type="submit" disabled={busy} className={`${primaryButton} w-full py-2`}>
      {children}
    </button>
  );
}

export function FormAlert({ message }: { message?: string }) {
  if (!message) return null;
  return (
    <p
      role="alert"
      className="rounded-md border border-danger/30 bg-danger-soft px-3 py-2 text-sm text-danger"
    >
      {message}
    </p>
  );
}
