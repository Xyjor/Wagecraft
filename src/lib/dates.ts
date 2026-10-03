/** "2024-06-03" → "Jun 3, 2024". Built from parts, so no time zone can shift the day. */
export function formatDate(iso: string | null | undefined): string {
  if (!iso) return "";
  const [y, m, d] = iso.split("-").map(Number);
  if (!y || !m || !d) return iso;
  return new Date(y, m - 1, d).toLocaleDateString("en-PH", { dateStyle: "medium" });
}
