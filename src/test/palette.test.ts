import { describe, expect, it } from "vitest";

// Raw Tailwind palette classes (text-zinc-500, bg-white, …) bypass the Ledger tokens in
// src/index.css, so a theme change would miss them. Every screen now uses the tokens; this keeps
// it that way. Use bg-surface, text-ink, border-rule, text-danger and friends instead.
// See ADR-0013.

const RAW =
  /\b(?:bg|text|border|ring|fill|stroke|divide|outline|ring-offset|placeholder|decoration)-(?:(?:zinc|emerald|sky|red|amber|slate|gray|neutral|stone|green|blue)-\d{2,3}|white|black)\b/g;

const sources = import.meta.glob<string>(["/src/**/*.tsx", "!/src/**/*.test.tsx"], {
  query: "?raw",
  import: "default",
  eager: true,
});

describe("raw palette classes", () => {
  it("are recognised, with and without variants", () => {
    const sample = 'className="text-zinc-500 dark:bg-zinc-900 hover:bg-white bg-surface text-ink"';
    expect(sample.match(RAW)).toEqual(["text-zinc-500", "bg-zinc-900", "bg-white"]);
  });

  it("are not used anywhere in the app", () => {
    expect(Object.keys(sources).length).toBeGreaterThan(50);
    const found = Object.entries(sources).flatMap(([path, text]) =>
      (text.match(RAW) ?? []).map((cls) => `${path}: ${cls}`),
    );
    expect(found).toEqual([]);
  });
});
