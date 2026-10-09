/** "2024-06-03" → "Jun 3, 2024". Built from parts, so no time zone can shift the day. */
export function formatDate(iso: string | null | undefined): string {
  if (!iso) return "";
  const [y, m, d] = iso.split("-").map(Number);
  if (!y || !m || !d) return iso;
  return new Date(y, m - 1, d).toLocaleDateString("en-PH", { dateStyle: "medium" });
}

/** A UTC timestamp as the PC's local date and time: "Oct 6, 2026, 9:00 AM". */
export function formatWhen(iso: string): string {
  return new Date(iso).toLocaleString("en-PH", { dateStyle: "medium", timeStyle: "short" });
}
