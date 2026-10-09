import { cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { MemoryRouter } from "react-router";
import type { AuditEntry } from "@/bindings/AuditEntry";
import type { AuditPage } from "@/bindings/AuditPage";
import type { AuditQuery } from "@/bindings/AuditQuery";
import type { Me } from "@/bindings/Me";
import { SessionContext } from "@/features/auth/session";

const call = vi.fn();
vi.mock("@/lib/ipc", async (orig) => ({
  ...(await orig<typeof import("@/lib/ipc")>()),
  call: (cmd: string, args?: Record<string, unknown>) => call(cmd, args),
}));

import { ActivityPage } from "./ActivityPage";

const admin: Me = {
  userId: 1,
  username: "admin",
  role: "ADMIN",
  employeeId: null,
  mustChangePassword: false,
  theme: "SYSTEM",
};

const FIRST_PAGE: AuditQuery = {
  from: null,
  to: null,
  actor: null,
  action: null,
  entityType: null,
  entityId: null,
  page: 1,
  pageSize: 50,
};

function entry(id: number, extra: Partial<AuditEntry>): AuditEntry {
  return {
    id,
    at: "2026-10-06T01:00:00Z",
    actorUserId: 1,
    actorUsername: "admin",
    action: "auth.login",
    entityType: null,
    entityId: null,
    before: null,
    after: null,
    ...extra,
  };
}

const update = entry(2, {
  action: "employee.update",
  entityType: "employee",
  entityId: 7,
  before: { name: "old", rate: 1 },
  after: { name: "new", rate: 1 },
});
const post = entry(3, {
  actorUserId: 2,
  actorUsername: "hr",
  action: "payroll.post",
  entityType: "payroll_period",
  entityId: 3,
  after: { status: "POSTED" },
});

let page: AuditPage;

function renderAs(who: Me = admin) {
  render(
    <SessionContext.Provider value={{ me: who, signOut: async () => {} }}>
      <MemoryRouter>
        <ActivityPage />
      </MemoryRouter>
    </SessionContext.Provider>,
  );
}

/** The query of the latest `audit_list` call. */
function lastQuery(): AuditQuery {
  const calls = call.mock.calls.filter(([cmd]) => cmd === "audit_list");
  return (calls[calls.length - 1][1] as { query: AuditQuery }).query;
}

describe("ActivityPage", () => {
  beforeEach(() => {
    page = { items: [post, update], total: 2, page: 1, pageSize: 50 };
    call.mockImplementation(async (cmd: string) => {
      if (cmd === "audit_filters") {
        return {
          actors: ["admin", "hr"],
          actions: ["auth.login", "employee.update", "payroll.post", "payroll.compute"],
          entityTypes: ["employee", "payroll_period"],
        };
      }
      if (cmd === "audit_list") return page;
      if (cmd === "audit_export_csv") return "C:\\Users\\hr\\Desktop\\activity-log.csv";
      return undefined;
    });
  });

  afterEach(() => {
    cleanup();
    call.mockReset();
  });

  it("tells everyone but Admins that it's not for them", () => {
    renderAs({ ...admin, username: "maria", role: "HR" });

    expect(screen.getByText("Only Admins can see the activity log.")).toBeTruthy();
    expect(call).not.toHaveBeenCalled();
  });

  it("loads the first page with no filters and shows who did what to which record", async () => {
    renderAs();

    const rows = await screen.findAllByRole("row", { name: /employee.update|payroll.post/ });
    expect(rows).toHaveLength(2);
    expect(call).toHaveBeenCalledWith("audit_list", { query: FIRST_PAGE });
    expect(call).toHaveBeenCalledWith("audit_filters", undefined);

    const [first, second] = rows;
    expect(within(first).getByText("hr")).toBeTruthy();
    expect(
      within(first).getByRole("link", { name: "payroll_period #3" }).getAttribute("href"),
    ).toBe("/payroll/3");
    expect(within(second).getByRole("link", { name: "employee #7" }).getAttribute("href")).toBe(
      "/employees/7",
    );
    expect(screen.getByText("Showing 1–2 of 2")).toBeTruthy();
  });

  it("opens an entry to show only the fields that changed", async () => {
    renderAs();
    const row = await screen.findByRole("row", { name: /employee.update/ });

    fireEvent.click(within(row).getByRole("button", { name: /View details/ }));

    const details = screen.getByRole("table", { name: "Changes to employee #7" });
    expect(within(details).getByText("name")).toBeTruthy();
    expect(within(details).getByText("old")).toBeTruthy();
    expect(within(details).getByText("new")).toBeTruthy();
    expect(within(details).queryByText("rate")).toBeNull();

    fireEvent.click(within(row).getByRole("button", { name: /Hide details/ }));
    expect(screen.queryByRole("table", { name: "Changes to employee #7" })).toBeNull();
  });

  it("shows an entry with no record and no earlier snapshot as new values", async () => {
    renderAs();
    const row = await screen.findByRole("row", { name: /payroll.post/ });

    fireEvent.click(within(row).getByRole("button", { name: /View details/ }));

    const details = screen.getByRole("table", { name: "Changes to payroll_period #3" });
    expect(within(details).getByText("status")).toBeTruthy();
    expect(within(details).getByText("POSTED")).toBeTruthy();
  });

  it("offers every user, every action by area and every record type as filters", async () => {
    renderAs();
    await screen.findAllByRole("row", { name: /payroll.post/ });

    const user = screen.getByLabelText("User") as HTMLSelectElement;
    expect([...user.options].map((o) => o.text)).toEqual(["Everyone", "admin", "hr"]);
    const action = screen.getByLabelText("Action") as HTMLSelectElement;
    expect([...action.options].map((o) => o.value)).toEqual([
      "",
      "auth.",
      "auth.login",
      "employee.",
      "employee.update",
      "payroll.",
      "payroll.post",
      "payroll.compute",
    ]);
    const record = screen.getByLabelText("Record") as HTMLSelectElement;
    expect([...record.options].map((o) => o.text)).toEqual([
      "Any record",
      "employee",
      "payroll_period",
    ]);
  });

  it("refines the list with the filters, from page 1", async () => {
    page = { items: [update], total: 101, page: 1, pageSize: 50 };
    renderAs();
    await screen.findAllByRole("row", { name: /employee.update/ });

    fireEvent.click(screen.getByRole("button", { name: "Next page" }));
    await waitFor(() => expect(lastQuery().page).toBe(2));

    fireEvent.change(screen.getByLabelText("User"), { target: { value: "admin" } });
    await waitFor(() => expect(lastQuery()).toEqual({ ...FIRST_PAGE, actor: "admin" }));

    fireEvent.change(screen.getByLabelText("Action"), { target: { value: "payroll." } });
    fireEvent.change(screen.getByLabelText("From"), { target: { value: "2026-10-06" } });
    fireEvent.change(screen.getByLabelText("To"), { target: { value: "2026-10-07" } });
    fireEvent.change(screen.getByLabelText("Record"), { target: { value: "employee" } });
    fireEvent.change(screen.getByLabelText("Record ID"), { target: { value: "7" } });
    await waitFor(() =>
      expect(lastQuery()).toEqual({
        ...FIRST_PAGE,
        actor: "admin",
        action: "payroll.",
        from: "2026-10-06",
        to: "2026-10-07",
        entityType: "employee",
        entityId: 7,
      }),
    );
  });

  it("exports the filtered log and says where it went", async () => {
    renderAs();
    await screen.findAllByRole("row", { name: /payroll.post/ });
    fireEvent.change(screen.getByLabelText("User"), { target: { value: "hr" } });

    fireEvent.click(screen.getByRole("button", { name: "Export CSV" }));

    await screen.findByText("Saved the activity log to C:\\Users\\hr\\Desktop\\activity-log.csv");
    expect(call).toHaveBeenCalledWith("audit_export_csv", {
      query: { ...FIRST_PAGE, actor: "hr" },
    });
  });

  it("says nothing when the export is cancelled", async () => {
    call.mockImplementation(async (cmd: string) => {
      if (cmd === "audit_filters") return { actors: [], actions: [], entityTypes: [] };
      if (cmd === "audit_list") return page;
      return null;
    });
    renderAs();
    await screen.findAllByRole("row", { name: /payroll.post/ });

    fireEvent.click(screen.getByRole("button", { name: "Export CSV" }));

    await waitFor(() => expect(call).toHaveBeenCalledWith("audit_export_csv", expect.anything()));
    expect(screen.queryByText(/Saved the activity log/)).toBeNull();
  });

  it("shows the backend's message when the list can't be loaded", async () => {
    call.mockImplementation(async (cmd: string) => {
      if (cmd === "audit_list") {
        throw { code: "VALIDATION", message: "Use a date like 2026-10-09", fields: [] };
      }
      return { actors: [], actions: [], entityTypes: [] };
    });
    renderAs();

    expect(await screen.findByText("Use a date like 2026-10-09")).toBeTruthy();
  });

  it("explains an empty list differently with and without filters", async () => {
    page = { items: [], total: 0, page: 1, pageSize: 50 };
    renderAs();

    expect(await screen.findByText("Nothing recorded yet.")).toBeTruthy();

    fireEvent.change(screen.getByLabelText("User"), { target: { value: "hr" } });
    expect(await screen.findByText("No activity matches these filters.")).toBeTruthy();
  });
});
