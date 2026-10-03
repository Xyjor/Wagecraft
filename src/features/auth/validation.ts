import { z } from "zod";

// These rules mirror services/auth.rs. The Rust side is the real check;
// these only let the form say what's wrong before a round trip.

export const USERNAME = /^[A-Za-z0-9._-]{3,32}$/;
export const MIN_PASSWORD = 10;

export const passwordField = z
  .string()
  .min(MIN_PASSWORD, `Use at least ${MIN_PASSWORD} characters`);

export const setupSchema = z
  .object({
    companyName: z.string().trim().min(1, "Enter your company's name"),
    username: z
      .string()
      .regex(USERNAME, "Use 3 to 32 letters, numbers, dots, dashes or underscores"),
    password: passwordField,
    confirmPassword: z.string(),
  })
  .superRefine((v, ctx) => {
    if (v.password.length < MIN_PASSWORD) return;
    if (v.password.toLowerCase() === v.username.toLowerCase()) {
      ctx.addIssue({
        code: "custom",
        path: ["password"],
        message: "Don't use your username as your password",
      });
    } else if (v.password !== v.confirmPassword) {
      ctx.addIssue({
        code: "custom",
        path: ["confirmPassword"],
        message: "The passwords don't match",
      });
    }
  });

export type SetupForm = z.infer<typeof setupSchema>;

export function changePasswordSchema(username: string) {
  return z
    .object({
      currentPassword: z.string().min(1, "Enter your current password"),
      newPassword: passwordField,
      confirmPassword: z.string(),
    })
    .superRefine((v, ctx) => {
      if (v.newPassword.length < MIN_PASSWORD) return;
      if (v.newPassword.toLowerCase() === username.toLowerCase()) {
        ctx.addIssue({
          code: "custom",
          path: ["newPassword"],
          message: "Don't use your username as your password",
        });
      } else if (v.newPassword === v.currentPassword) {
        ctx.addIssue({
          code: "custom",
          path: ["newPassword"],
          message: "Pick a password you haven't used",
        });
      } else if (v.newPassword !== v.confirmPassword) {
        ctx.addIssue({
          code: "custom",
          path: ["confirmPassword"],
          message: "The passwords don't match",
        });
      }
    });
}

export type ChangePasswordForm = z.infer<ReturnType<typeof changePasswordSchema>>;

/** First error per field, keyed by field name. */
export function fieldErrors(
  issues: { path: PropertyKey[]; message: string }[],
): Record<string, string> {
  const out: Record<string, string> = {};
  for (const i of issues) {
    const key = String(i.path[0]);
    out[key] ??= i.message;
  }
  return out;
}
