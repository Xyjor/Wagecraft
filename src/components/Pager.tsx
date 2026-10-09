import { quietButton } from "@/components/ui";

/** "Showing 26–50 of 212" with Previous and Next, for any paged list. */
export function Pager({
  page,
  pageSize,
  total,
  onPage,
}: {
  page: number;
  pageSize: number;
  total: number;
  onPage: (page: number) => void;
}) {
  const from = total === 0 ? 0 : (page - 1) * pageSize + 1;
  const to = Math.min(page * pageSize, total);
  const last = Math.max(1, Math.ceil(total / pageSize));
  return (
    <div className="flex items-center justify-between text-sm text-zinc-600 dark:text-zinc-400">
      <span>{total > 0 && `Showing ${from}–${to} of ${total}`}</span>
      <div className="flex gap-1">
        <button
          type="button"
          className={quietButton}
          aria-label="Previous page"
          disabled={page <= 1}
          onClick={() => onPage(page - 1)}
        >
          Previous
        </button>
        <button
          type="button"
          className={quietButton}
          aria-label="Next page"
          disabled={page >= last}
          onClick={() => onPage(page + 1)}
        >
          Next
        </button>
      </div>
    </div>
  );
}
