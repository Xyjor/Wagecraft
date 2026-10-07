import { describe, expect, it } from "vitest";
import { companyLines } from "./companyHeader";

const acme = { name: "Acme Trading", address: "", tin: "", logo: null };

describe("company header", () => {
  it("lists the address, then the TIN grouped", () => {
    expect(companyLines({ ...acme, address: "12 Rizal St, Makati", tin: "123456789000" })).toEqual([
      "12 Rizal St, Makati",
      "TIN 123-456-789-000",
    ]);
  });

  it("leaves out what Settings doesn't have", () => {
    expect(companyLines(acme)).toEqual([]);
    expect(companyLines({ ...acme, tin: "123456789" })).toEqual(["TIN 123-456-789"]);
  });
});
