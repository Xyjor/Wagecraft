import type { ReactNode } from "react";

/** A ruled box on the page: settings sections, side panels. */
export const cardClass = "rounded-lg border border-rule bg-surface p-4";

export function Card({ children, className = "" }: { children: ReactNode; className?: string }) {
  return <div className={`${cardClass} ${className}`}>{children}</div>;
}

/** The narrow box most add and edit forms sit in. */
export const formCardClass = `${cardClass} grid max-w-md gap-4`;
