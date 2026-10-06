import type { BackupEntry } from "@/bindings/BackupEntry";
import { call } from "@/lib/ipc";

export const listBackups = () => call<BackupEntry[]>("backup_list");
/** Asks for a folder, then backs up there. Resolves to the file's path, or null if cancelled. */
export const backUpNow = () => call<string | null>("backup_create");
