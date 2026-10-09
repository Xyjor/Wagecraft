import { describe, expect, it } from "vitest";
import { compactPesos, formatPesos, parsePesos } from "./money";

describe("formatPesos", () => {
  it.each([
    [0, "₱0.00"],
    [5, "₱0.05"],
    [2500000, "₱25,000.00"],
    [123456789, "₱1,234,567.89"],
    [-150, "-₱1.50"],
  ])("%i centavos → %s", (cents, text) => {
    expect(formatPesos(cents)).toBe(text);
  });
});

describe("parsePesos", () => {
  it.each([
    ["25000", 2500000],
    ["25,000", 2500000],
    [" ₱25,000.00 ", 2500000],
    ["25000.5", 2500050],
    ["0.07", 7],
    [".5", 50],
  ])("%s → %i centavos", (text, cents) => {
    expect(parsePesos(text)).toBe(cents);
  });

  it.each(["", "abc", "1.234", "-5", "1,2,3.4.5", "₱", "1e5"])("rejects %j", (text) => {
    expect(parsePesos(text)).toBeNull();
  });

  it("never goes through floating point", () => {
    // 0.1 + 0.2 style errors would turn 1,234,567.89 into ...88 or ...90.
    expect(parsePesos("1,234,567.89")).toBe(123456789);
  });
});

describe("compactPesos", () => {
  it.each([
    [0, "₱0"],
    [95_000, "₱950"],
    [45_000_000, "₱450k"],
    [45_049_999, "₱450k"],
    [600_000_000, "₱6M"],
    [625_000_000, "₱6.3M"],
  ])("%i centavos read as %s on a chart axis", (cents, text) => {
    expect(compactPesos(cents)).toBe(text);
  });
});
