import { formatAccounting } from "@/lib/money";

/**
 * An amount in a ledger column. Negatives are in parentheses and red ink, and screen readers
 * hear "minus" because parentheses alone aren't read as a sign.
 */
export function Money({ cents }: { cents: number }) {
  if (cents >= 0) return <>{formatAccounting(cents)}</>;
  return (
    <span className="text-danger">
      <span className="sr-only">minus </span>
      <span>{formatAccounting(cents)}</span>
    </span>
  );
}
