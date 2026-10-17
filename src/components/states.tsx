import type { ReactNode } from "react";
import { quietButton } from "./ui";

/**
 * Skeleton lines while a page loads. It marks itself busy but has no status role: pages use
 * role="status" for their own messages, and tests look those up by role.
 */
export function Loading({ label = "Loading…", lines = 3 }: { label?: string; lines?: number }) {
  return (
    <div aria-busy="true" className="space-y-2.5 py-2">
      <span className="sr-only">{label}</span>
      {Array.from({ length: lines }, (_, i) => (
        <div
          key={i}
          aria-hidden="true"
          className="h-3.5 animate-pulse rounded bg-sunken motion-reduce:animate-none"
          style={{ width: `${90 - i * 18}%` }}
        />
      ))}
    </div>
  );
}

/** What an empty list says, and what to do about it. The message stays one paragraph. */
export function EmptyState({ message, action }: { message: string; action?: ReactNode }) {
  return (
    <div className="flex flex-wrap items-center gap-3 py-6 text-sm text-ink-muted">
      <p>{message}</p>
      {action}
    </div>
  );
}

/** A load failure: the message in an alert, with Retry beside it rather than inside it. */
export function ErrorState({ message, onRetry }: { message: string; onRetry?: () => void }) {
  return (
    <div className="flex flex-wrap items-center gap-3 rounded-md border border-danger/30 bg-danger-soft px-3 py-2 text-sm">
      <p role="alert" className="text-danger">
        {message}
      </p>
      {onRetry && (
        <button type="button" onClick={onRetry} className={`${quietButton} text-danger`}>
          Retry
        </button>
      )}
    </div>
  );
}
