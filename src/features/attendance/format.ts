/** "2026-10-05T08:25:00" → "8:25 AM". Read from the text, so no time zone can shift it. */
export function formatClock(localTime: string | null | undefined): string {
  if (!localTime) return "";
  const [h, m] = localTime.slice(11, 16).split(":").map(Number);
  if (Number.isNaN(h) || Number.isNaN(m)) return localTime;
  const suffix = h < 12 ? "AM" : "PM";
  return `${h % 12 || 12}:${String(m).padStart(2, "0")} ${suffix}`;
}

/** 455 → "7h 35m", 15 → "15m", 0 → "". */
export function formatMinutes(minutes: number): string {
  if (minutes <= 0) return "";
  const h = Math.floor(minutes / 60);
  const m = minutes % 60;
  if (h === 0) return `${m}m`;
  return m === 0 ? `${h}h` : `${h}h ${m}m`;
}

/** First and last day of a `YYYY-MM` month, as `YYYY-MM-DD`. */
export function monthRange(month: string): { from: string; to: string } {
  const [y, m] = month.split("-").map(Number);
  const last = new Date(y, m, 0).getDate();
  return { from: `${month}-01`, to: `${month}-${String(last).padStart(2, "0")}` };
}

/** This month as `YYYY-MM`, on the PC's calendar. */
export function thisMonth(today = new Date()): string {
  return `${today.getFullYear()}-${String(today.getMonth() + 1).padStart(2, "0")}`;
}
