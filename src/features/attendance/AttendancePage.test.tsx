import { cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { AttendanceRecord } from "@/bindings/AttendanceRecord";
import type { DayRow } from "@/bindings/DayRow";
import type { Me } from "@/bindings/Me";
import type { ReviewItem } from "@/bindings/ReviewItem";
import { SessionContext } from "@/features/auth/session";

const call = vi.fn();
vi.mock("@/lib/ipc", async (orig) => ({
  ...(await orig<typeof import("@/lib/ipc")>()),
  call: (cmd: string, args?: Record<string, unknown>) => call(cmd, args),
}));

import { AttendancePage } from "./AttendancePage";

const hr: Me = {
  userId: 2,
  username: "maria",
  role: "HR",
  employeeId: null,
  mustChangePassword: false,
  theme: "SYSTEM",
};

const record = (over: Partial<AttendanceRecord>): AttendanceRecord => ({
  id: 11,
  employeeId: 1,
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

let rows: DayRow[];
let queue: ReviewItem[];

function renderAs(me: Me = hr) {
  render(
    <SessionContext.Provider value={{ me, signOut: async () => {} }}>
      <AttendancePage />
    </SessionContext.Provider>,
  );
}

function type(label: string, value: string) {
  fireEvent.change(screen.getByLabelText(label), { target: { value } });
}

describe("AttendancePage", () => {
  beforeEach(() => {
    rows = [
      {
        employeeId: 1,
        employeeNo: "EMP-1",
        employeeName: "Dela Cruz, Juan",
        departmentName: "Operations",
        status: "PRESENT",
        holiday: null,
        record: record({}),
      },
      {
        employeeId: 2,
        employeeNo: "EMP-2",
        employeeName: "Reyes, Ana",
        departmentName: "Operations",
        status: "ABSENT",
        holiday: null,
        record: null,
      },
      {
        employeeId: 3,
        employeeNo: "EMP-3",
        employeeName: "Santos, Leo",
        departmentName: null,
        status: null,
        holiday: null,
        record: null,
      },
    ];
    queue = [
      {
        employeeNo: "EMP-1",
        employeeName: "Dela Cruz, Juan",
        record: record({
          id: 12,
          workDate: "2026-10-02",
          needsReview: true,
          reviewNote: "PC clock",
        }),
      },
    ];
    call.mockImplementation(async (cmd: string) => {
      if (cmd === "attendance_day") return rows;
      if (cmd === "attendance_review_queue") return queue;
      if (cmd === "attendance_save") return record({ source: "MANUAL" });
      if (cmd === "attendance_mark_reviewed") {
        queue = [];
        return record({});
      }
      return null;
    });
  });

  afterEach(() => {
    cleanup();
    call.mockReset();
  });

  it("shows everyone's day with their status", async () => {
    renderAs();
    const juan = await screen.findByRole("row", { name: /^Dela Cruz, Juan EMP-1/ });
    expect(within(juan).getByText("Present")).toBeTruthy();
    expect(within(juan).getByText("8:25 AM")).toBeTruthy();
    expect(within(juan).getByText("7h 35m")).toBeTruthy();
    expect(
      within(screen.getByRole("row", { name: /^Santos, Leo/ })).getByText("No schedule"),
    ).toBeTruthy();
    const today = call.mock.calls.find(([c]) => c === "attendance_day")?.[1].date;
    expect(today).toMatch(/^\d{4}-\d{2}-\d{2}$/);
    // An absence today is only "not in yet".
    expect(
      within(screen.getByRole("row", { name: /^Reyes, Ana/ })).getByText("No time in yet"),
    ).toBeTruthy();
  });

  it("adds a missing day with times and a reason", async () => {
    renderAs();
    await screen.findByRole("row", { name: /^Reyes, Ana/ });
    fireEvent.click(screen.getByRole("button", { name: "Add Reyes, Ana" }));
    type("Time in", "07:58");
    type("Time out", "17:03");
    type("Reason for the change", "Kiosk was off");
    fireEvent.click(screen.getByRole("button", { name: "Save" }));

    await waitFor(() =>
      expect(call).toHaveBeenCalledWith("attendance_save", {
        input: {
          employeeId: 2,
          workDate: expect.stringMatching(/^\d{4}-\d{2}-\d{2}$/),
          timeIn: "07:58",
          timeOut: "17:03",
          reason: "Kiosk was off",
        },
      }),
    );
  });

  it("needs a reason before saving a correction", async () => {
    renderAs();
    await screen.findByRole("row", { name: /^Dela Cruz, Juan EMP-1/ });
    fireEvent.click(screen.getByRole("button", { name: "Correct Dela Cruz, Juan" }));
    expect((screen.getByLabelText("Time in") as HTMLInputElement).value).toBe("08:25");
    fireEvent.click(screen.getByRole("button", { name: "Save" }));
    expect(await screen.findByText("Say why, in 3 to 200 characters")).toBeTruthy();
    expect(call).not.toHaveBeenCalledWith("attendance_save", expect.anything());
  });

  it("lists flagged records and clears one that looks fine", async () => {
    renderAs();
    expect(await screen.findByRole("heading", { name: "Needs review (1)" })).toBeTruthy();
    expect(screen.getByText("PC clock")).toBeTruthy();
    fireEvent.click(
      screen.getByRole("button", { name: "Mark Dela Cruz, Juan on 2026-10-02 as reviewed" }),
    );
    await waitFor(() => expect(screen.queryByRole("heading", { name: /Needs review/ })).toBeNull());
    expect(call).toHaveBeenCalledWith("attendance_mark_reviewed", { id: 12 });
  });

  it("loads another day from the date picker", async () => {
    renderAs();
    await screen.findByRole("row", { name: /^Reyes, Ana/ });
    type("Date", "2026-10-02");
    await waitFor(() =>
      expect(call).toHaveBeenCalledWith("attendance_day", { date: "2026-10-02" }),
    );
    expect(
      within(screen.getByRole("row", { name: /^Reyes, Ana/ })).getByText("Absent"),
    ).toBeTruthy();
  });

  it("is not for Staff", () => {
    renderAs({ ...hr, role: "STAFF" });
    expect(screen.getByText("Only Admin and HR can see this.")).toBeTruthy();
    expect(call).not.toHaveBeenCalled();
  });

  it("offers only a correction for a forgotten time out", async () => {
    queue = [
      {
        employeeNo: "EMP-2",
        employeeName: "Reyes, Ana",
        record: record({
          id: 13,
          employeeId: 2,
          timeOut: null,
          reviewNote: "No time out was recorded.",
        }),
      },
    ];
    renderAs();
    expect(await screen.findByText("No time out was recorded.")).toBeTruthy();
    expect(screen.getByRole("button", { name: "Correct Reyes, Ana on 2026-10-05" })).toBeTruthy();
    expect(screen.queryByRole("button", { name: /as reviewed/ })).toBeNull();
  });
});
