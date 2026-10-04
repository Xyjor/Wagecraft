import { cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { Employee } from "@/bindings/Employee";
import type { Me } from "@/bindings/Me";
import { SessionContext } from "@/features/auth/session";

const call = vi.fn();
vi.mock("@/lib/ipc", async (orig) => ({
  ...(await orig<typeof import("@/lib/ipc")>()),
  call: (cmd: string, args?: Record<string, unknown>) => call(cmd, args),
}));

import { formatDays, leaveDates, monthWeeks, parseDays } from "./format";
import { LeaveCalendarPage } from "./LeaveCalendarPage";
import { LeavePage } from "./LeavePage";
import { LeaveTab } from "./LeaveTab";
import { MyLeavePage } from "./MyLeavePage";
import { balance, hr, leaveRequest, leaveType } from "./testData";
import { checkLeaveRequest, checkLeaveType } from "./validation";

function as(me: Me, ui: React.ReactNode) {
  render(
    <SessionContext.Provider value={{ me, signOut: async () => {} }}>{ui}</SessionContext.Provider>,
  );
}

function type(label: string, value: string) {
  fireEvent.change(screen.getByLabelText(label), { target: { value } });
}

afterEach(() => {
  cleanup();
  call.mockReset();
});

describe("formatDays and parseDays", () => {
  it("shows half days as fractions", () => {
    expect(formatDays(10)).toBe("5 days");
    expect(formatDays(3)).toBe("1½ days");
    expect(formatDays(2)).toBe("1 day");
    expect(formatDays(1)).toBe("½ day");
    expect(formatDays(0)).toBe("0 days");
  });

  it("reads whole and half days only", () => {
    expect(parseDays("2.5")).toBe(5);
    expect(parseDays(" 5 ")).toBe(10);
    expect(parseDays("2.25")).toBeNull();
    expect(parseDays("-1")).toBeNull();
    expect(parseDays("366")).toBeNull();
  });
});

describe("MyLeavePage", () => {
  it("shows this year's balances", async () => {
    call.mockImplementation(async (cmd: string) =>
      cmd === "leave_my_balances" ? [balance({})] : [],
    );
    as({ ...hr, role: "STAFF" }, <MyLeavePage year={2026} />);
    const row = await screen.findByRole("row", { name: /Vacation Leave/ });
    expect(within(row).getByText("5 days")).toBeTruthy();
    expect(within(row).getByText("1½ days")).toBeTruthy();
    expect(within(row).getByText("3½ days")).toBeTruthy();
    expect(call).toHaveBeenCalledWith("leave_my_balances", { year: 2026 });
  });

  it("files leave and lists it with a cancel button until HR decides", async () => {
    const filed: unknown[] = [];
    call.mockImplementation(async (cmd: string, args?: Record<string, unknown>) => {
      if (cmd === "leave_types_list")
        return [leaveType({ id: 2, code: "VL", name: "Vacation Leave" })];
      if (cmd === "leave_my_requests") return filed.length ? [leaveRequest({})] : [];
      if (cmd === "leave_request_create") filed.push(args);
      return [];
    });
    as({ ...hr, role: "STAFF" }, <MyLeavePage year={2026} />);
    expect(await screen.findByText("You haven't filed leave for 2026.")).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "File leave" }));
    const form = screen.getByRole("form", { name: "File leave" });
    // No type is picked for the employee, so unpaid leave is never filed by accident.
    fireEvent.click(within(form).getByRole("button", { name: "File leave" }));
    expect(await within(form).findByText("Pick a leave type")).toBeTruthy();
    expect(call).not.toHaveBeenCalledWith("leave_request_create", expect.anything());
    fireEvent.change(screen.getByLabelText("Leave type"), { target: { value: "2" } });
    type("First day", "2026-10-12");
    type("Last day", "2026-10-16");
    type("What it's for", "Family trip");
    fireEvent.click(
      within(screen.getByRole("form", { name: "File leave" })).getByRole("button", {
        name: "File leave",
      }),
    );
    await waitFor(() =>
      expect(call).toHaveBeenCalledWith("leave_request_create", {
        input: {
          employeeNo: null,
          leaveTypeId: 2,
          startDate: "2026-10-12",
          endDate: "2026-10-16",
          halfDay: false,
          reason: "Family trip",
        },
      }),
    );
    const row = await screen.findByRole("row", { name: /Family trip/ });
    expect(within(row).getByText("4 days")).toBeTruthy();
    expect(within(row).getByText("Waiting for HR")).toBeTruthy();
    fireEvent.click(within(row).getByRole("button", { name: /Cancel leave/ }));
    await waitFor(() => expect(call).toHaveBeenCalledWith("leave_request_cancel", { id: 1 }));
  });

  it("hides cancel once leave is approved", async () => {
    call.mockImplementation(async (cmd: string) =>
      cmd === "leave_my_requests" ? [leaveRequest({ status: "APPROVED" })] : [],
    );
    as({ ...hr, role: "STAFF" }, <MyLeavePage year={2026} />);
    const row = await screen.findByRole("row", { name: /Family trip/ });
    expect(within(row).queryByRole("button")).toBeNull();
  });

  it("explains when the account has no employee record", () => {
    as({ ...hr, employeeId: null }, <MyLeavePage year={2026} />);
    expect(screen.getByText(/isn't linked to an employee record/)).toBeTruthy();
    expect(call).not.toHaveBeenCalled();
  });
});

describe("LeavePage", () => {
  const types = [
    leaveType({}),
    leaveType({
      id: 4,
      code: "LWOP",
      name: "Leave Without Pay",
      isPaid: false,
      defaultHalfdaysPerYear: 0,
      minServiceMonths: 0,
    }),
  ];

  it("grants a year and says how many balances were added", async () => {
    call.mockImplementation(async (cmd: string) => {
      if (cmd === "leave_types_list") return types;
      if (cmd === "leave_balance_grant") return 412;
      return [];
    });
    as(hr, <LeavePage thisYear={2026} />);
    await screen.findByRole("row", { name: /Service Incentive Leave/ });
    fireEvent.click(screen.getByRole("button", { name: "Grant 2026 leave" }));
    expect(await screen.findByText("Added 412 leave balances for 2026.")).toBeTruthy();
    expect(call).toHaveBeenCalledWith("leave_balance_grant", { year: 2026 });
  });

  it("shows HR the types without changing them", async () => {
    call.mockImplementation(async (cmd: string) => (cmd === "leave_types_list" ? types : []));
    as(hr, <LeavePage thisYear={2026} />);
    const sil = await screen.findByRole("row", { name: /Service Incentive Leave/ });
    expect(within(sil).getByText("12 months")).toBeTruthy();
    expect(
      within(screen.getByRole("row", { name: /Leave Without Pay/ })).getByText("No limit"),
    ).toBeTruthy();
    expect(screen.queryByRole("button", { name: "Add leave type" })).toBeNull();
    expect(screen.getByText("Only Admin can change leave types.")).toBeTruthy();
  });

  it("lets Admin add a type", async () => {
    call.mockImplementation(async (cmd: string) => (cmd === "leave_types_list" ? types : []));
    as({ ...hr, role: "ADMIN" }, <LeavePage thisYear={2026} />);
    await screen.findByRole("row", { name: /Service Incentive Leave/ });
    fireEvent.click(screen.getByRole("button", { name: "Add leave type" }));
    type("Code", "bday");
    type("Name", "Birthday Leave");
    type("Days each year", "1");
    fireEvent.click(screen.getByRole("button", { name: "Save leave type" }));
    await waitFor(() =>
      expect(call).toHaveBeenCalledWith("leave_type_create", {
        input: {
          code: "BDAY",
          name: "Birthday Leave",
          isPaid: true,
          defaultHalfdaysPerYear: 2,
          minServiceMonths: 0,
        },
      }),
    );
  });
});

describe("LeavePage requests", () => {
  function setUp(
    pending = [
      leaveRequest({}),
      leaveRequest({ id: 9, employeeId: 3, employeeName: "Santos, Maria" }),
    ],
  ) {
    call.mockImplementation(async (cmd: string) => {
      if (cmd === "leave_request_pending") return pending;
      if (cmd === "leave_request_list") {
        return [
          leaveRequest({ id: 4, status: "APPROVED", employeeName: "Reyes, Ana" }),
          leaveRequest({ id: 5, status: "APPROVED", employeeName: "Cruz, Leo", locked: true }),
        ];
      }
      return [];
    });
    as(hr, <LeavePage thisYear={2026} />);
  }

  it("approves, and never offers HR their own request", async () => {
    setUp();
    const own = await screen.findByRole("row", { name: /Santos, Maria/ });
    expect(within(own).getByText("Your own: someone else decides")).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "Approve Dela Cruz, Juan's leave" }));
    await waitFor(() =>
      expect(call).toHaveBeenCalledWith("leave_request_decide", {
        id: 1,
        approve: true,
        note: null,
      }),
    );
  });

  it("rejects only with a note", async () => {
    setUp([leaveRequest({})]);
    fireEvent.click(await screen.findByRole("button", { name: "Reject Dela Cruz, Juan's leave" }));
    const form = screen.getByRole("form", { name: "Reject Dela Cruz, Juan's leave" });
    fireEvent.click(within(form).getByRole("button", { name: "Reject" }));
    expect(await screen.findByText("Say why, in 3 to 200 characters")).toBeTruthy();
    fireEvent.change(within(form).getByLabelText("Why it's rejected"), {
      target: { value: "Peak season" },
    });
    fireEvent.click(within(form).getByRole("button", { name: "Reject" }));
    await waitFor(() =>
      expect(call).toHaveBeenCalledWith("leave_request_decide", {
        id: 1,
        approve: false,
        note: "Peak season",
      }),
    );
  });

  it("cancels approved leave, except where payroll is posted", async () => {
    setUp([]);
    expect(await screen.findByText("Nothing to decide.")).toBeTruthy();
    const posted = await screen.findByRole("row", { name: /Cruz, Leo/ });
    expect(within(posted).getByText("Payroll posted")).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: /Cancel Reyes, Ana's leave/ }));
    await waitFor(() => expect(call).toHaveBeenCalledWith("leave_request_cancel", { id: 4 }));
  });
});

describe("LeaveTab", () => {
  const employee = { id: 7 } as Employee;

  it("adjusts a balance with a reason, not below what's used", async () => {
    call.mockImplementation(async (cmd: string) =>
      cmd === "leave_balances_for" ? [balance({})] : {},
    );
    as(hr, <LeaveTab employee={employee} initialYear={2026} />);
    fireEvent.click(await screen.findByRole("button", { name: "Adjust Vacation Leave" }));
    type("Vacation Leave days for 2026", "1");
    type("Reason for the change", "Carry-over");
    fireEvent.click(screen.getByRole("button", { name: "Save balance" }));
    expect(await screen.findByText("That's less than what's already used")).toBeTruthy();

    type("Vacation Leave days for 2026", "8");
    fireEvent.click(screen.getByRole("button", { name: "Save balance" }));
    await waitFor(() =>
      expect(call).toHaveBeenCalledWith("leave_balance_adjust", {
        id: 1,
        input: { entitledHalfdays: 16, reason: "Carry-over" },
      }),
    );
  });

  it("moves between years", async () => {
    call.mockResolvedValue([]);
    as(hr, <LeaveTab employee={employee} initialYear={2026} />);
    expect(await screen.findByText(/No leave balances for 2026/)).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "Previous year" }));
    await waitFor(() =>
      expect(call).toHaveBeenLastCalledWith("leave_balances_for", { employeeId: 7, year: 2025 }),
    );
  });
});

describe("checkLeaveType", () => {
  it("keeps unpaid leave at zero days", () => {
    const r = checkLeaveType({
      code: "UL",
      name: "Unpaid",
      defaultDays: "2",
      minServiceMonths: "0",
    });
    expect(r.ok).toBe(false);
    if (!r.ok) expect(r.errors.defaultDays).toMatch(/Unpaid/);
  });
});

describe("checkLeaveRequest", () => {
  const base = { leaveTypeId: "2", startDate: "2026-10-12", reason: "Family trip" };

  it("treats an empty last day as a one-day leave", () => {
    const r = checkLeaveRequest(base, false);
    expect(r.ok && r.value.endDate).toBe("2026-10-12");
  });

  it("keeps a half day to one date and a request to one year", () => {
    const half = checkLeaveRequest({ ...base, endDate: "2026-10-13", halfDay: "on" }, false);
    expect(!half.ok && half.errors.halfDay).toMatch(/same date/);
    const across = checkLeaveRequest(
      { ...base, startDate: "2026-12-28", endDate: "2027-01-02" },
      false,
    );
    expect(!across.ok && across.errors.endDate).toMatch(/New Year/);
  });

  it("needs the employee number when HR files", () => {
    const r = checkLeaveRequest(base, true);
    expect(!r.ok && r.errors.employeeNo).toBe("Enter the employee number");
  });
});

describe("leaveDates", () => {
  it("shows one date for a single day and marks half days", () => {
    expect(leaveDates({ startDate: "2026-10-12", endDate: "2026-10-12", halfDay: true })).toMatch(
      /2026 \(half day\)$/,
    );
    expect(
      leaveDates({ startDate: "2026-10-12", endDate: "2026-10-16", halfDay: false }),
    ).toContain(" – ");
  });
});

describe("monthWeeks", () => {
  it("starts weeks on Monday and pads the edges", () => {
    // October 2026 starts on a Thursday and ends on a Saturday.
    const weeks = monthWeeks("2026-10");
    expect(weeks[0]).toEqual([
      null,
      null,
      null,
      "2026-10-01",
      "2026-10-02",
      "2026-10-03",
      "2026-10-04",
    ]);
    expect(weeks[weeks.length - 1]).toEqual([
      "2026-10-26",
      "2026-10-27",
      "2026-10-28",
      "2026-10-29",
      "2026-10-30",
      "2026-10-31",
      null,
    ]);
    expect(weeks.flat().filter(Boolean)).toHaveLength(31);
  });
});

describe("LeaveCalendarPage", () => {
  it("puts approved and waiting leave on each day it covers", async () => {
    call.mockImplementation(async (cmd: string) =>
      cmd === "leave_request_list"
        ? [
            leaveRequest({ startDate: "2026-10-12", endDate: "2026-10-13", status: "APPROVED" }),
            leaveRequest({
              id: 2,
              employeeName: "Reyes, Ana",
              leaveTypeCode: "SL",
              startDate: "2026-10-13",
              endDate: "2026-10-13",
              halfDay: true,
            }),
            leaveRequest({
              id: 3,
              employeeName: "Cruz, Leo",
              startDate: "2026-10-14",
              endDate: "2026-10-14",
              status: "REJECTED",
            }),
          ]
        : [],
    );
    as(hr, <LeaveCalendarPage initialMonth="2026-10" />);
    const tuesday = await screen.findByRole("cell", { name: "2026-10-13: 2 away" });
    expect(within(tuesday).getByText("Dela Cruz · VL")).toBeTruthy();
    expect(within(tuesday).getByText(/Reyes · SL ½ \(waiting\)/)).toBeTruthy();
    // Rejected leave isn't on the calendar.
    expect(screen.getByRole("cell", { name: "2026-10-14: nobody away" })).toBeTruthy();
    expect(call).toHaveBeenCalledWith("leave_request_list", {
      status: null,
      range: { from: "2026-10-01", to: "2026-10-31" },
    });
  });

  it("keeps staff out", () => {
    as({ ...hr, role: "STAFF" }, <LeaveCalendarPage initialMonth="2026-10" />);
    expect(screen.getByText(/Only Admin and HR can see everyone's leave/)).toBeTruthy();
    expect(call).not.toHaveBeenCalled();
  });
});
