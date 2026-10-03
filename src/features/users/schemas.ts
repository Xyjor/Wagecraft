import { z } from "zod";
import { MIN_PASSWORD, USERNAME, passwordField } from "@/features/auth/validation";

// Mirrors services/users.rs. Staff accounts are created from an employee's profile
// (Phase 2), so this form offers only Admin and HR.

const notUsername = (username: string, password: string) =>
  password.length < MIN_PASSWORD || password.toLowerCase() !== username.trim().toLowerCase();

export const newUserSchema = z
  .object({
    username: z
      .string()
      .trim()
      .regex(USERNAME, "Use 3 to 32 letters, numbers, dots, dashes or underscores"),
    role: z.enum(["ADMIN", "HR"]),
    password: passwordField,
  })
  .refine((v) => notUsername(v.username, v.password), {
    path: ["password"],
    message: "Don't use the username as the password",
  });

export function temporaryPasswordSchema(username: string) {
  return passwordField.refine((pw) => notUsername(username, pw), {
    message: "Don't use the username as the password",
  });
}
