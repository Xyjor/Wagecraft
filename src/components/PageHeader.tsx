import type { ReactNode } from "react";
import { Link } from "react-router";
import { ChevronRight } from "lucide-react";

export type Crumb = { label: string; to: string };

/**
 * The top of every page: breadcrumbs, the title, a short description and the page's actions,
 * closed by a double ledger rule. Presentational only, so pages render it without a router
 * unless they pass breadcrumbs.
 */
export function PageHeader({
  title,
  crumbs,
  description,
  actions,
  meta,
}: {
  title: string;
  crumbs?: Crumb[];
  description?: ReactNode;
  /** Buttons and links for the whole page, at the right. */
  actions?: ReactNode;
  /** A status or badge shown next to the title. */
  meta?: ReactNode;
}) {
  return (
    <header className="mb-6 border-b-[3px] border-double border-rule-strong pb-4">
      {crumbs && crumbs.length > 0 && (
        <nav aria-label="Breadcrumb" className="mb-1.5">
          <ol className="flex flex-wrap items-center gap-1 text-sm text-ink-muted">
            {crumbs.map((c) => (
              <li key={c.to} className="flex items-center gap-1">
                <Link to={c.to} className="rounded hover:text-accent hover:underline">
                  {c.label}
                </Link>
                <ChevronRight aria-hidden="true" className="size-3.5 text-ink-faint" />
              </li>
            ))}
          </ol>
        </nav>
      )}
      <div className="flex flex-wrap items-start justify-between gap-x-6 gap-y-3">
        <div className="min-w-0">
          <div className="flex flex-wrap items-center gap-3">
            <h1 className="text-2xl font-semibold tracking-tight text-balance">{title}</h1>
            {meta}
          </div>
          {description && (
            <div className="mt-1 max-w-2xl text-sm text-ink-muted">{description}</div>
          )}
        </div>
        {actions && <div className="flex shrink-0 flex-wrap items-center gap-2">{actions}</div>}
      </div>
    </header>
  );
}
