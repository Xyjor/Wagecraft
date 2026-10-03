import { cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { AttendanceRecord } from "@/bindings/AttendanceRecord";
import type { Me } from "@/bindings/Me";
import { SessionContext } from "@/features/auth/session";

const call = vi.fn();
vi.mock("@/lib/ipc", async (orig) => ({
  ...(await orig<typeof import("@/lib/ipc")>()),
  call: (cmd: string, args?: Record<string, unknown>) => call(cmd, args),
}));

import { MyAttendancePage } from "./MyAttendancePage";

const staff: Me = {
  userId: 5,
  username: "juan",
  role: "STAFF",
  employeeId: 7,
  mustChangePassword: false,
  theme: "SYSTEM",
};

const record = (over: Partial<AttendanceRecord>): AttendanceRecord => ({
  id: 1,
  employeeId: 7,
  workDate: "2026-10-05",
  timeIn: "2026-10-05T08:25:00",
  timeOut: "2026-10-05T17:00:00",
  status: "PRESENT",
  lateMinutes: 15,
  undertimeMinutes: 0,
  workedMinutes: 455,
  nightMinutes: 0,
  source: "CLOCK",
  needsReview: false,
  reviewNote: null,
  locked: false,
  ...over,
});

function renderAs(me: Me) {
  render(
    <SessionContext.Provider value={{ me, signOut: async () => {} }}>
      <MyAttendancePage />
    </SessionContext.Provider>,
  );
}

describe("MyAttendancePage", () => {
  afterEach(() => {
    cleanup();
    call.mockReset();
  });

  it("lists this month's records with readable times", async () => {
    const records = [
      record({
        id: 2,
        workDate: "2026-10-06",
        timeIn: "2026-10-06T07:58:00",
        timeOut: null,
        lateMinutes: 0,
        workedMinutes: 0,
      }),
      record({ needsReview: true }),
    ];
    call.mockImplementation(async (cmd: string) => (cmd === "attendance_mine" ? records : []));
    renderAs(staff);

    const row = await screen.findByRole("row", { name: /Oct 5, 2026/ });
    expect(within(row).getByText("8:25 AM")).toBeTruthy();
    expect(within(row).getByText("5:00 PM")).toBeTruthy();
    expect(within(row).getByText("15m")).toBeTruthy();
    expect(within(row).getByText("7h 35m")).toBeTruthy();
    expect(within(row).getByText("HR to review")).toBeTruthy();
    const open = screen.getByRole("row", { name: /Oct 6, 2026/ });
    expect(within(open).getByText("Not yet")).toBeTruthy();

    const { from, to } = call.mock.calls.find(([cmd]) => cmd === "attendance_mine")![1].range;
    expect(from).toMatch(/^\d{4}-\d{2}-01$/);
    expect(to.slice(0, 7)).toBe(from.slice(0, 7));
  });

  it("loads the month that is picked", async () => {
    call.mockResolvedValue([]);
    renderAs(staff);
    expect(await screen.findByText("No time records this month.")).toBeTruthy();
    fireEvent.change(screen.getByLabelText("Month"), { target: { value: "2026-02" } });
    await waitFor(() =>
      expect(call).toHaveBeenLastCalledWith("attendance_mine", {
        range: { from: "2026-02-01", to: "2026-02-28" },
      }),
    );
  });

  it("explains when the account has no employee record", () => {
    renderAs({ ...staff, employeeId: null });
    expect(screen.getByText(/isn't linked to an employee record/)).toBeTruthy();
    expect(call).not.toHaveBeenCalled();
  });
});
