import { cleanup, render, screen, within } from "@testing-library/react";
import { MemoryRouter } from "react-router";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { LeaveBalance } from "@/bindings/LeaveBalance";
import type { MyDashboard as Summary } from "@/bindings/MyDashboard";
import type { MyPayslip } from "@/bindings/MyPayslip";

const call = vi.fn();
vi.mock("@/lib/ipc", async (orig) => ({
  ...(await orig<typeof import("@/lib/ipc")>()),
  call: (cmd: string, args?: Record<string, unknown>) => call(cmd, args),
}));

import { MyDashboard } from "./MyDashboard";

const summary: Summary = {
  today: {
    date: "2026-10-28",
    status: "PRESENT",
    timeIn: "2026-10-28T08:05:00",
    timeOut: null,
    lateMinutes: 5,
    holiday: null,
    leave: null,
  },
  cutoff: {
    periodStart: "2026-10-16",
    periodEnd: "2026-10-31",
    daysPresent: 8,
    lateMinutes: 75,
    overtimeMinutes: 90,
  },
  pendingLeave: 1,
  pendingOvertime: 0,
};

const balance = (code: string, name: string, entitled: number, used: number): LeaveBalance => ({
  id: entitled + used,
  employeeId: 2,
  leaveTypeId: entitled,
  leaveTypeCode: code,
  leaveTypeName: name,
  year: 2026,
  entitledHalfdays: entitled,
  usedHalfdays: used,
});

const slips: MyPayslip[] = [
  {
    id: 9,
    periodStart: "2026-10-01",
    periodEnd: "2026-10-15",
    payDate: "2026-10-20",
    grossCents: 1_250_000,
    netCents: 1_103_450,
  },
  {
    id: 8,
    periodStart: "2026-09-16",
    periodEnd: "2026-09-30",
    payDate: "2026-10-05",
    grossCents: 1_250_000,
    netCents: 1_000_000,
  },
];

function show(
  data: Summary = summary,
  balances = [balance("VL", "Vacation Leave", 10, 3), balance("SL", "Sick Leave", 10, 10)],
  payslips = slips,
) {
  call.mockImplementation(async (cmd: string) => {
    if (cmd === "dashboard_mine") return data;
    if (cmd === "leave_my_balances") return balances;
    if (cmd === "payslip_my_list") return payslips;
    throw new Error(`unexpected ${cmd}`);
  });
  render(
    <MemoryRouter>
      <MyDashboard />
    </MemoryRouter>,
  );
}

const panel = (name: string) => screen.findByRole("region", { name });
const tile = (label: string) => screen.getByRole("group", { name: label });

beforeEach(() => {
  call.mockReset();
});
afterEach(cleanup);

describe("MyDashboard", () => {
  it("shows when they clocked in today and how late", async () => {
    show();

    const today = await panel("Today");
    expect(today.textContent).toContain("Clocked in at 8:05 AM");
    expect(today.textContent).toContain("5m late");
  });

  it("says when they haven't clocked in yet", async () => {
    show({ ...summary, today: { ...summary.today, status: "ABSENT", timeIn: null } });

    expect((await panel("Today")).textContent).toContain("Not clocked in yet");
  });

  it("names the holiday, the leave, or the rest day", async () => {
    const day = (patch: Partial<Summary["today"]>) => ({
      ...summary,
      today: { ...summary.today, timeIn: null, lateMinutes: 0, ...patch },
    });
    const cases: [Partial<Summary["today"]>, string][] = [
      [{ status: "HOLIDAY", holiday: "All Souls' Day" }, "Holiday: All Souls' Day"],
      [{ status: "ON_LEAVE", leave: "Vacation Leave" }, "On leave: Vacation Leave"],
      [{ status: "REST_DAY" }, "Rest day"],
      [{ status: null }, "No work schedule yet"],
    ];
    for (const [patch, text] of cases) {
      show(day(patch));
      expect((await panel("Today")).textContent).toContain(text);
      cleanup();
    }
  });

  it("totals this cutoff so far", async () => {
    show();

    await panel("This cutoff, Oct 16 – 31, 2026");
    expect(tile("Days present").textContent).toContain("8");
    expect(tile("Late").textContent).toContain("1h 15m");
    expect(tile("Approved overtime").textContent).toContain("1h 30m");
  });

  it("shows days left of each leave type for the office clock's year", async () => {
    show({ ...summary, today: { ...summary.today, date: "2027-01-04" } });

    const leave = await panel("Leave left this year");
    const rows = within(leave)
      .getAllByRole("listitem")
      .map((li) => li.textContent);
    expect(rows).toEqual([
      "Vacation Leave3½ days left of 5 days",
      "Sick Leave0 days left of 5 days",
    ]);
    expect(call).toHaveBeenCalledWith("leave_my_balances", { year: 2027 });
  });

  it("sums up the latest payslip with a link to it", async () => {
    show();

    const pay = await panel("Latest payslip");
    expect(pay.textContent).toContain("Oct 1 – 15, 2026");
    expect(pay.textContent).toContain("₱11,034.50");
    expect(within(pay).getByRole("link").getAttribute("href")).toBe("/my-payslips");
  });

  it("says when no payslip is posted yet", async () => {
    show(summary, [], []);

    expect((await panel("Latest payslip")).textContent).toContain("No payslips yet");
  });

  it("lists what is still waiting for HR", async () => {
    show({ ...summary, pendingLeave: 2, pendingOvertime: 1 });

    const waiting = await panel("My requests");
    const links = within(waiting).getAllByRole("link");
    expect(links.map((a) => [a.textContent, a.getAttribute("href")])).toEqual([
      ["2 leave requests waiting for HR", "/my-leave"],
      ["1 overtime request waiting for HR", "/my-attendance"],
    ]);
  });

  it("says when nothing is waiting", async () => {
    show({ ...summary, pendingLeave: 0, pendingOvertime: 0 });

    expect((await panel("My requests")).textContent).toContain("Nothing waiting for HR.");
  });

  it("shows the error when the dashboard can't load", async () => {
    call.mockRejectedValue({ code: "INTERNAL", message: "Database is locked", fields: [] });
    render(
      <MemoryRouter>
        <MyDashboard />
      </MemoryRouter>,
    );

    expect((await screen.findByRole("alert")).textContent).toContain("Database is locked");
  });
});
