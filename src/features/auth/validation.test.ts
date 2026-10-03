import { describe, expect, it } from "vitest";
import { changePasswordSchema, setupSchema } from "./validation";

function fieldsWithErrors(result: {
  success: boolean;
  error?: { issues: { path: PropertyKey[] }[] };
}) {
  return result.success ? [] : result.error!.issues.map((i) => String(i.path[0]));
}

describe("setupSchema (mirrors create_admin in Rust)", () => {
  const good = {
    companyName: "Rojyx Inc.",
    username: "admin",
    password: "correct horse",
    confirmPassword: "correct horse",
  };

  it("accepts a valid first admin", () => {
    expect(setupSchema.safeParse(good).success).toBe(true);
  });

  it("flags each bad field", () => {
    const r = setupSchema.safeParse({
      companyName: "  ",
      username: "a b",
      password: "short",
      confirmPassword: "short",
    });
    expect(fieldsWithErrors(r)).toEqual(["companyName", "username", "password"]);
  });

  it("rejects the username as the password, ignoring case", () => {
    const r = setupSchema.safeParse({
      ...good,
      username: "administrator",
      password: "ADMINISTRATOR",
      confirmPassword: "ADMINISTRATOR",
    });
    expect(fieldsWithErrors(r)).toEqual(["password"]);
  });

  it("requires the confirmation to match", () => {
    const r = setupSchema.safeParse({ ...good, confirmPassword: "correct horsf" });
    expect(fieldsWithErrors(r)).toEqual(["confirmPassword"]);
  });
});

describe("changePasswordSchema", () => {
  it("rejects reusing the current password", () => {
    const r = changePasswordSchema("admin").safeParse({
      currentPassword: "correct horse",
      newPassword: "correct horse",
      confirmPassword: "correct horse",
    });
    expect(fieldsWithErrors(r)).toEqual(["newPassword"]);
  });

  it("accepts a new valid password", () => {
    const r = changePasswordSchema("admin").safeParse({
      currentPassword: "correct horse",
      newPassword: "a brand new password",
      confirmPassword: "a brand new password",
    });
    expect(r.success).toBe(true);
  });
});
