import type { Settings } from "@/bindings/Settings";
import type { SettingsInput } from "@/bindings/SettingsInput";
import { call } from "@/lib/ipc";

export const getSettings = () => call<Settings>("settings_get");
export const saveSettings = (input: SettingsInput) => call<Settings>("settings_update", { input });
/** Opens the folder picker. Resolves to the folder, or null if cancelled. */
export const pickBackupFolder = () => call<string | null>("settings_pick_backup_folder");
/** Opens the image picker and saves the logo at once. Resolves to the settings as stored. */
export const pickLogo = () => call<Settings>("settings_pick_logo");
export const clearLogo = () => call<Settings>("settings_clear_logo");
