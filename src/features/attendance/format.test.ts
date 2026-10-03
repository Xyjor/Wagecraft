import { describe, expect, it } from "vitest";
import { formatClock, formatMinutes, monthRange, thisMonth } from "./format";

describe("attendance format", () => {
  it("shows local times on a 12-hour clock", () => {
    expect(formatClock("2026-10-05T08:25:00")).toBe("8:25 AM");
    expect(formatClock("2026-10-05T00:05:00")).toBe("12:05 AM");
    expect(formatClock("2026-10-05T12:00:00")).toBe("12:00 PM");
    expect(formatClock("2026-10-05T22:30:59")).toBe("10:30 PM");
    expect(formatClock(null)).toBe("");
  });

  it("shows minutes as hours and minutes, and nothing for zero", () => {
    expect(formatMinutes(455)).toBe("7h 35m");
    expect(formatMinutes(15)).toBe("15m");
    expect(formatMinutes(120)).toBe("2h");
    expect(formatMinutes(0)).toBe("");
  });

  it("finds a month's first and last day", () => {
    expect(monthRange("2026-02")).toEqual({ from: "2026-02-01", to: "2026-02-28" });
    expect(monthRange("2028-02")).toEqual({ from: "2028-02-01", to: "2028-02-29" });
    expect(monthRange("2026-10")).toEqual({ from: "2026-10-01", to: "2026-10-31" });
    expect(thisMonth(new Date(2026, 0, 9))).toBe("2026-01");
  });
});
