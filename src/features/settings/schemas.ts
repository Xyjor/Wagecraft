import { z } from "zod";

// Mirrors services/settings.rs. The TIN and the backup folder are checked by Rust only:
// the TIN's grouping is lenient there, and only Rust can see the folder.

const wholeNumber = (min: number, max: number, message: string) =>
  z.coerce.number({ error: message }).int(message).min(min, message).max(max, message);

export const settingsSchema = z.object({
  companyName: z
    .string()
    .trim()
    .min(1, "Enter the company name")
    .max(100, "Use 100 characters or fewer"),
  companyAddress: z.string().trim().max(300, "Use 300 characters or fewer"),
  companyTin: z.string().trim(),
  idleTimeoutMinutes: wholeNumber(5, 120, "Choose between 5 and 120 minutes"),
  backupKeep: wholeNumber(3, 60, "Keep between 3 and 60 daily backups"),
});
