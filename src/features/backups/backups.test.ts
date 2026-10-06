import { describe, expect, it } from "vitest";
import { formatBytes, kindLabel } from "./backups";

describe("formatBytes", () => {
  it.each([
    [0, "0 bytes"],
    [512, "512 bytes"],
    [1536, "1.5 KB"],
    [1258291, "1.2 MB"],
    [3 * 1024 ** 3, "3.0 GB"],
  ])("%d → %s", (n, text) => {
    expect(formatBytes(n)).toBe(text);
  });
});

describe("kindLabel", () => {
  it("names each kind the way Admin thinks of it", () => {
    expect(kindLabel("AUTO")).toBe("Daily");
    expect(kindLabel("MANUAL")).toBe("Manual");
    expect(kindLabel("PRE_POST")).toBe("Before posting payroll");
    expect(kindLabel("PRE_RESTORE")).toBe("Before a restore");
  });
});
