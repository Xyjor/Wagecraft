import { describe, expect, it } from "vitest";
import { formatBytes, kindLabel, splitPath } from "./backups";

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

describe("splitPath", () => {
  it("separates the folder from the file name, on Windows and Unix paths", () => {
    expect(
      splitPath(
        "C:\\Users\\ACER\\AppData\\Roaming\\io.github.xyjor.wagecraft\\backups\\wagecraft-2026-10-09.db",
      ),
    ).toEqual({
      folder: "C:\\Users\\ACER\\AppData\\Roaming\\io.github.xyjor.wagecraft\\backups\\",
      name: "wagecraft-2026-10-09.db",
    });
    expect(splitPath("/home/ana/backups/wagecraft-2026-10-09.db")).toEqual({
      folder: "/home/ana/backups/",
      name: "wagecraft-2026-10-09.db",
    });
  });

  it("treats a bare file name as having no folder", () => {
    expect(splitPath("wagecraft.db")).toEqual({ folder: "", name: "wagecraft.db" });
  });
});
