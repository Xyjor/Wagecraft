import type { BackupEntry } from "@/bindings/BackupEntry";
import type { RestorePreview } from "@/bindings/RestorePreview";
import { call } from "@/lib/ipc";

export const listBackups = () => call<BackupEntry[]>("backup_list");
/** Asks for a folder, then backs up there. Resolves to the file's path, or null if cancelled. */
export const backUpNow = () => call<string | null>("backup_create");
/** Opens the file picker and checks the chosen backup. Resolves to null if cancelled. */
export const chooseRestore = () => call<RestorePreview | null>("backup_restore_choose");
/** Restores the chosen backup. The app restarts, so this normally never resolves. */
export const restoreBackup = () => call<void>("backup_restore", { confirm: "RESTORE" });
