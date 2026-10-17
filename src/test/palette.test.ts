import { describe, expect, it } from "vitest";

// Raw Tailwind palette classes (text-zinc-500, bg-white, …) bypass the Ledger tokens in
// src/index.css. Pages are moving to the tokens one PR at a time; these limits only go down.
// Lower them in the same PR that removes classes, and never raise them. See ADR-0013.
const LIMITS: Record<string, number> = {
  "/src/components/": 73,
  "/src/app/": 0,
  "/src/features/": 483,
};

const RAW =
  /\b(?:bg|text|border|ring|fill|stroke|divide|outline|ring-offset|placeholder|decoration)-(?:(?:zinc|emerald|sky|red|amber|slate|gray|neutral|stone|green|blue)-\d{2,3}|white|black)\b/g;

const sources = import.meta.glob<string>(["/src/**/*.tsx", "!/src/**/*.test.tsx"], {
  query: "?raw",
  import: "default",
  eager: true,
});

function countIn(prefix: string) {
  return Object.entries(sources)
    .filter(([path]) => path.startsWith(prefix))
    .reduce((n, [, text]) => n + (text.match(RAW)?.length ?? 0), 0);
}

describe("raw palette classes", () => {
  it("reads the source files", () => {
    expect(Object.keys(sources).length).toBeGreaterThan(50);
    expect(countIn("/src/")).toBeGreaterThan(0);
  });

  for (const [prefix, limit] of Object.entries(LIMITS)) {
    it(`stay at or under ${limit} in ${prefix}`, () => {
      expect(countIn(prefix)).toBeLessThanOrEqual(limit);
    });
  }
});
