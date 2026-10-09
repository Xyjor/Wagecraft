import { describe, expect, it } from "vitest";
import { changes, show } from "./diff";

describe("changes", () => {
  it("lists only the fields that differ between the snapshots", () => {
    const rows = changes(
      { name: "old", rate: 1, active: true },
      { name: "new", rate: 1, active: false },
    );
    expect(rows).toEqual([
      { field: "name", before: "old", after: "new" },
      { field: "active", before: "true", after: "false" },
    ]);
  });

  it("shows every field when there is no earlier snapshot", () => {
    expect(changes(null, { status: "POSTED", rows: 3 })).toEqual([
      { field: "status", before: null, after: "POSTED" },
      { field: "rows", before: null, after: "3" },
    ]);
  });

  it("shows what was removed when there is no later snapshot", () => {
    expect(changes({ username: "x" }, null)).toEqual([
      { field: "username", before: "x", after: null },
    ]);
  });

  it("keeps a field that only one side has", () => {
    expect(changes({ a: 1 }, { b: 2 })).toEqual([
      { field: "a", before: "1", after: null },
      { field: "b", before: null, after: "2" },
    ]);
  });

  it("compares nested values as a whole", () => {
    const before = { filters: { action: null, page: 1 } };
    const same = { filters: { action: null, page: 1 } };
    const changed = { filters: { action: "auth.", page: 1 } };
    expect(changes(before, same)).toEqual([]);
    expect(changes(before, changed)).toEqual([
      {
        field: "filters",
        before: '{"action":null,"page":1}',
        after: '{"action":"auth.","page":1}',
      },
    ]);
  });

  it("falls back to one row for snapshots that are not objects", () => {
    expect(changes("plain text", null)).toEqual([
      { field: "value", before: "plain text", after: null },
    ]);
    expect(changes(null, [1, 2])).toEqual([{ field: "value", before: null, after: "[1,2]" }]);
  });

  it("returns nothing when both snapshots are missing", () => {
    expect(changes(null, null)).toEqual([]);
    expect(changes(undefined, undefined)).toEqual([]);
  });
});

describe("show", () => {
  it("prints strings bare, nulls as a dash and everything else as JSON", () => {
    expect(show("Juan")).toBe("Juan");
    expect(show(null)).toBe("—");
    expect(show(undefined)).toBe("—");
    expect(show(42)).toBe("42");
    expect(show(false)).toBe("false");
    expect(show({ a: 1 })).toBe('{"a":1}');
  });
});
