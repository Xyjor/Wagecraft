import type { BackupEntry } from "@/bindings/BackupEntry";

const UNITS = ["KB", "MB", "GB"];

/** 1258291 → "1.2 MB". */
export function formatBytes(n: number): string {
  if (n < 1024) return `${n} bytes`;
  let value = n / 1024;
  let unit = 0;
  while (value >= 1024 && unit < UNITS.length - 1) {
    value /= 1024;
    unit += 1;
  }
  return `${value.toFixed(1)} ${UNITS[unit]}`;
}

const KINDS: Record<BackupEntry["kind"], string> = {
  AUTO: "Daily",
  MANUAL: "Manual",
  PRE_POST: "Before posting payroll",
  PRE_RESTORE: "Before a restore",
};

export const kindLabel = (kind: BackupEntry["kind"]) => KINDS[kind];

/** "C:\\backups\\wagecraft.db" → the folder, with its trailing separator, and the file name. */
export function splitPath(path: string): { folder: string; name: string } {
  const cut = Math.max(path.lastIndexOf("\\"), path.lastIndexOf("/")) + 1;
  return { folder: path.slice(0, cut), name: path.slice(cut) };
}
