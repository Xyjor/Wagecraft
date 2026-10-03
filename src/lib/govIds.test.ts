import { describe, expect, it } from "vitest";
import { formatId } from "./govIds";

describe("formatId", () => {
  it.each([
    ["tin", "123456789", "123-456-789"],
    ["tin", "123456789000", "123-456-789-000"],
    ["tin", "12345678900000", "123-456-789-00000"],
    ["sss", "3412345678", "34-1234567-8"],
    ["philhealth", "123456789012", "12-345678901-2"],
    ["pagibig", "123456789012", "1234-5678-9012"],
  ] as const)("%s %s → %s", (kind, digits, text) => {
    expect(formatId(kind, digits)).toBe(text);
  });

  it("leaves anything unexpected as it is", () => {
    expect(formatId("sss", "12")).toBe("12");
    expect(formatId("tin", null)).toBe("");
  });
});
