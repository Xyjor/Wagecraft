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

import { formatDays, parseDays } from "./format";
import { LeavePage } from "./LeavePage";
import { LeaveTab } from "./LeaveTab";
import { MyLeavePage } from "./MyLeavePage";
import { balance, hr, leaveType } from "./testData";
import { checkLeaveType } from "./validation";

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
    call.mockResolvedValue([balance({})]);
    as({ ...hr, role: "STAFF" }, <MyLeavePage year={2026} />);
    const row = await screen.findByRole("row", { name: /Vacation Leave/ });
    expect(within(row).getByText("5 days")).toBeTruthy();
    expect(within(row).getByText("1½ days")).toBeTruthy();
    expect(within(row).getByText("3½ days")).toBeTruthy();
    expect(call).toHaveBeenCalledWith("leave_my_balances", { year: 2026 });
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
      return {};
    });
    as(hr, <LeavePage thisYear={2026} />);
    await screen.findByRole("row", { name: /Service Incentive Leave/ });
    fireEvent.click(screen.getByRole("button", { name: "Grant 2026 leave" }));
    expect(await screen.findByText("Added 412 leave balances for 2026.")).toBeTruthy();
    expect(call).toHaveBeenCalledWith("leave_balance_grant", { year: 2026 });
  });

  it("shows HR the types without changing them", async () => {
    call.mockResolvedValue(types);
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
    call.mockImplementation(async (cmd: string) => (cmd === "leave_types_list" ? types : {}));
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
