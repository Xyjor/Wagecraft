import type { ReactNode } from "react";
import { CircleAlert, Info, TriangleAlert } from "lucide-react";

const NOTICE = {
  info: { box: "border-rule bg-sunken text-ink", icon: Info, iconClass: "text-accent" },
  warn: { box: "border-warn-rule bg-warn-soft text-warn", icon: TriangleAlert, iconClass: "" },
  danger: { box: "border-danger/30 bg-danger-soft text-danger", icon: CircleAlert, iconClass: "" },
};

/**
 * A box of guidance or caution inside a page. Pass `label` to make it a named region, for
 * notices that tests and screen readers find by name.
 */
export function Notice({
  tone = "info",
  label,
  children,
}: {
  tone?: keyof typeof NOTICE;
  label?: string;
  children: ReactNode;
}) {
  const { box, icon: Icon, iconClass } = NOTICE[tone];
  return (
    <section
      aria-label={label}
      className={`flex gap-2.5 rounded-md border px-3 py-2.5 text-sm ${box}`}
    >
      <Icon aria-hidden="true" className={`mt-0.5 size-4 shrink-0 ${iconClass}`} />
      <div className="min-w-0 space-y-2">{children}</div>
    </section>
  );
}

/** "Saved", "Request approved": a short confirmation after an action. */
export function StatusMessage({ children }: { children: ReactNode }) {
  if (!children) return null;
  return (
    <p role="status" className="text-sm text-accent">
      {children}
    </p>
  );
}

/** "Only Admins can …": what a page says to a role that can't use it. */
export function RoleNote({ children }: { children: ReactNode }) {
  return <p className="text-ink-muted">{children}</p>;
}
