import { describe, expect, it } from "vitest";
import { cutoffOf, periodEnd, periodLabel, periodStart } from "./periods";

describe("pay periods", () => {
  it("starts the first cutoff on the 1st and the second on the 16th", () => {
    expect(periodStart("2026-10", 1)).toBe("2026-10-01");
    expect(periodStart("2026-10", 2)).toBe("2026-10-16");
  });

  it("ends the first cutoff on the 15th and the second on the month's last day", () => {
    expect(periodEnd("2026-10-01")).toBe("2026-10-15");
    expect(periodEnd("2026-10-16")).toBe("2026-10-31");
    expect(periodEnd("2026-11-16")).toBe("2026-11-30");
    expect(periodEnd("2027-02-16")).toBe("2027-02-28");
    expect(periodEnd("2028-02-16")).toBe("2028-02-29");
  });

  it("finds the cutoff a day falls in", () => {
    expect(cutoffOf("2026-10-15")).toEqual({ month: "2026-10", cutoff: 1 });
    expect(cutoffOf("2026-10-16")).toEqual({ month: "2026-10", cutoff: 2 });
  });

  it("names a period by its days", () => {
    expect(periodLabel("2026-10-16", "2026-10-31")).toBe("Oct 16 – 31, 2026");
    expect(periodLabel("2026-11-01", "2026-11-15")).toBe("Nov 1 – 15, 2026");
  });
});
