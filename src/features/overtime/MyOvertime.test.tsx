import { cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { OvertimeRequest } from "@/bindings/OvertimeRequest";

const call = vi.fn();
vi.mock("@/lib/ipc", async (orig) => ({
  ...(await orig<typeof import("@/lib/ipc")>()),
  call: (cmd: string, args?: Record<string, unknown>) => call(cmd, args),
}));

import { MyOvertime } from "./MyOvertime";
import { request } from "./testData";
import { checkOvertime } from "./validation";

function type(label: string, value: string) {
  fireEvent.change(screen.getByLabelText(label), { target: { value } });
}

describe("MyOvertime", () => {
  let mine: OvertimeRequest[];

  beforeEach(() => {
    mine = [
      request({}),
      request({
        id: 2,
        workDate: "2026-10-02",
        startAt: "2026-10-02T20:00:00",
        endAt: "2026-10-03T01:00:00",
        minutes: 300,
        status: "REJECTED",
        decisionNote: "Not needed",
      }),
    ];
    call.mockImplementation(async (cmd: string) => (cmd === "overtime_mine" ? mine : {}));
  });

  afterEach(() => {
    cleanup();
    call.mockReset();
  });

  it("lists recent requests with their status", async () => {
    render(<MyOvertime />);
    const pending = await screen.findByRole("row", { name: /Oct 7, 2026/ });
    expect(within(pending).getByText("5:00 PM – 8:00 PM")).toBeTruthy();
    expect(within(pending).getByText("3h")).toBeTruthy();
    expect(within(pending).getByText("Waiting for HR")).toBeTruthy();
    const rejected = screen.getByRole("row", { name: /Oct 2, 2026/ });
    expect(within(rejected).getByText("8:00 PM – 1:00 AM (next day)")).toBeTruthy();
    expect(within(rejected).getByText(/Not needed/)).toBeTruthy();
    expect(within(rejected).queryByRole("button")).toBeNull();
  });

  it("files overtime", async () => {
    render(<MyOvertime />);
    await screen.findByRole("row", { name: /Oct 7, 2026/ });
    fireEvent.click(screen.getByRole("button", { name: "File overtime" }));
    type("Date", "2026-10-08");
    type("From", "17:00");
    type("To", "19:30");
    type("What it's for", "Inventory");
    fireEvent.click(screen.getByRole("button", { name: "File overtime" }));
    await waitFor(() =>
      expect(call).toHaveBeenCalledWith("overtime_request_create", {
        input: {
          employeeNo: null,
          workDate: "2026-10-08",
          startTime: "17:00",
          endTime: "19:30",
          reason: "Inventory",
        },
      }),
    );
  });

  it("cancels a pending request", async () => {
    render(<MyOvertime />);
    fireEvent.click(await screen.findByRole("button", { name: "Cancel overtime on 2026-10-07" }));
    await waitFor(() => expect(call).toHaveBeenCalledWith("overtime_request_cancel", { id: 1 }));
  });
});

describe("checkOvertime", () => {
  it("needs an employee number only when filing for someone", () => {
    const v = { workDate: "2026-10-08", startTime: "17:00", endTime: "19:00", reason: "Count" };
    expect(checkOvertime(v, false).ok).toBe(true);
    const r = checkOvertime(v, true);
    expect(r.ok).toBe(false);
    if (!r.ok) expect(Object.keys(r.errors)).toEqual(["employeeNo"]);
  });

  it("refuses a block that starts and ends at the same time", () => {
    const r = checkOvertime(
      { workDate: "2026-10-08", startTime: "17:00", endTime: "17:00", reason: "x" },
      false,
    );
    expect(r.ok).toBe(false);
    if (!r.ok) expect(Object.keys(r.errors)).toEqual(["endTime", "reason"]);
  });
});
