import { cleanup, fireEvent, render, screen, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { Employee } from "@/bindings/Employee";
import type { ScheduleAssignment } from "@/bindings/ScheduleAssignment";

const call = vi.fn();
vi.mock("@/lib/ipc", async (orig) => ({
  ...(await orig<typeof import("@/lib/ipc")>()),
  call: (cmd: string, args?: Record<string, unknown>) => call(cmd, args),
}));

import { ScheduleTab } from "./ScheduleTab";

const juan = { id: 7, scheduleId: 1, archivedAt: null } as Employee;

const schedule = (id: number, name: string, isActive = true) => ({
  id,
  name,
  startTime: id === 1 ? "08:00" : "09:00",
  endTime: id === 1 ? "17:00" : "18:00",
  breakMinutes: 60,
  graceMinutes: 10,
  workDays: id === 1 ? "MON,TUE,WED,THU,FRI" : "MON,TUE,WED,THU,FRI,SAT",
  isActive,
  employeeCount: 1,
});
const schedules = [schedule(1, "Office"), schedule(2, "Six days"), schedule(3, "Old", false)];

const hired: ScheduleAssignment = {
  id: 1,
  scheduleId: 1,
  scheduleName: "Office",
  effectiveFrom: "2025-01-06",
  reason: "Hired",
  createdByName: null,
};
const moved: ScheduleAssignment = {
  id: 2,
  scheduleId: 2,
  scheduleName: "Six days",
  effectiveFrom: "2026-10-12",
  reason: "Moved to the store",
  createdByName: "hr",
};

let history: ScheduleAssignment[];

function type(label: string, value: string) {
  fireEvent.change(screen.getByLabelText(label), { target: { value } });
}

describe("ScheduleTab", () => {
  beforeEach(() => {
    vi.useFakeTimers({ toFake: ["Date"] });
    vi.setSystemTime(new Date(2026, 9, 9, 9, 0));
    history = [hired];
    call.mockImplementation(async (cmd: string) => {
      if (cmd === "employee_schedule_history") return history;
      if (cmd === "schedule_list") return schedules;
      if (cmd === "employee_change_schedule") {
        history = [hired, moved];
        return history;
      }
      return null;
    });
  });

  afterEach(() => {
    cleanup();
    call.mockReset();
    vi.useRealTimers();
  });

  it("lists the history, newest first, with today's schedule marked", async () => {
    history = [hired, moved];
    render(<ScheduleTab employee={juan} />);
    const rows = (await screen.findAllByRole("row")).slice(1);
    expect(rows.map((r) => within(r).getAllByRole("cell")[1].textContent)).toEqual([
      "Six days · 09:00–18:00, Mon–Sat",
      "Office · 08:00–17:00, Mon–Fri",
    ]);
    expect(within(rows[0]).getByText("Oct 12, 2026")).toBeTruthy();
    expect(within(rows[0]).getByText("Upcoming")).toBeTruthy();
    expect(within(rows[1]).getByText("Current")).toBeTruthy();
  });

  it("offers only schedules in use and sends the move", async () => {
    render(<ScheduleTab employee={juan} />);
    await screen.findByRole("cell", { name: "Office · 08:00–17:00, Mon–Fri" });
    const options = within(screen.getByLabelText("New schedule"))
      .getAllByRole("option")
      .map((o) => o.textContent);
    expect(options).toEqual([
      "Choose a schedule",
      "Office · 08:00–17:00, Mon–Fri",
      "Six days · 09:00–18:00, Mon–Sat",
    ]);
    type("New schedule", "2");
    type("Starts on", "2026-10-12");
    type("Reason", "Moved to the store");
    fireEvent.click(screen.getByRole("button", { name: "Save schedule" }));

    expect(
      await screen.findByRole("cell", { name: "Six days · 09:00–18:00, Mon–Sat" }),
    ).toBeTruthy();
    expect(call).toHaveBeenCalledWith("employee_change_schedule", {
      id: 7,
      input: { scheduleId: 2, effectiveFrom: "2026-10-12", reason: "Moved to the store" },
    });
  });

  it("points at missing fields before sending", async () => {
    render(<ScheduleTab employee={juan} />);
    await screen.findByRole("cell", { name: "Office · 08:00–17:00, Mon–Fri" });
    fireEvent.click(screen.getByRole("button", { name: "Save schedule" }));
    expect(await screen.findByText("Pick a schedule")).toBeTruthy();
    expect(screen.getByText("Enter the date the new schedule starts")).toBeTruthy();
    expect(call).not.toHaveBeenCalledWith("employee_change_schedule", expect.anything());
  });

  it("shows the backend's reason under the date", async () => {
    call.mockImplementation(async (cmd: string) => {
      if (cmd === "employee_schedule_history") return history;
      if (cmd === "schedule_list") return schedules;
      throw {
        kind: "validation",
        message: "Check the highlighted fields",
        fields: [
          {
            field: "effectiveFrom",
            message: "Attendance is already recorded up to Oct 7, 2026.",
          },
        ],
      };
    });
    render(<ScheduleTab employee={juan} />);
    await screen.findByRole("cell", { name: "Office · 08:00–17:00, Mon–Fri" });
    type("New schedule", "2");
    type("Starts on", "2026-10-05");
    fireEvent.click(screen.getByRole("button", { name: "Save schedule" }));
    expect(
      await screen.findByText("Attendance is already recorded up to Oct 7, 2026."),
    ).toBeTruthy();
  });

  it("has no form for an archived employee", async () => {
    render(<ScheduleTab employee={{ ...juan, archivedAt: "2026-10-01T00:00:00Z" }} />);
    await screen.findByRole("cell", { name: "Office · 08:00–17:00, Mon–Fri" });
    expect(screen.queryByRole("button", { name: "Save schedule" })).toBeNull();
  });
});
