import { cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { Me } from "@/bindings/Me";
import type { OvertimeRequest } from "@/bindings/OvertimeRequest";
import { SessionContext } from "@/features/auth/session";

const call = vi.fn();
vi.mock("@/lib/ipc", async (orig) => ({
  ...(await orig<typeof import("@/lib/ipc")>()),
  call: (cmd: string, args?: Record<string, unknown>) => call(cmd, args),
}));

import { OvertimePage } from "./OvertimePage";
import { request } from "./testData";

const hr: Me = {
  userId: 2,
  username: "maria",
  role: "HR",
  employeeId: 3,
  mustChangePassword: false,
  theme: "SYSTEM",
};

function renderAs(who: Me = hr) {
  render(
    <SessionContext.Provider value={{ me: who, signOut: async () => {} }}>
      <OvertimePage />
    </SessionContext.Provider>,
  );
}

describe("OvertimePage", () => {
  let pending: OvertimeRequest[];
  let history: OvertimeRequest[];

  beforeEach(() => {
    pending = [
      request({}),
      request({ id: 9, employeeId: 3, employeeName: "Santos, Maria", employeeNo: "EMP-0003" }),
    ];
    history = [
      request({ id: 4, status: "APPROVED", employeeName: "Reyes, Ana" }),
      request({ id: 5, status: "APPROVED", employeeName: "Cruz, Leo", locked: true }),
    ];
    call.mockImplementation(async (cmd: string) => {
      if (cmd === "overtime_pending") return pending;
      if (cmd === "overtime_list") return history;
      return {};
    });
  });

  afterEach(() => {
    cleanup();
    call.mockReset();
  });

  function pendingSection() {
    return within(screen.getByRole("region", { name: "Waiting for a decision" }));
  }

  it("tells Staff to file from My attendance", () => {
    renderAs({ ...hr, role: "STAFF" });
    expect(screen.getByText(/File your own from My attendance/)).toBeTruthy();
    expect(call).not.toHaveBeenCalled();
  });

  it("approves a request", async () => {
    renderAs();
    fireEvent.click(
      await screen.findByRole("button", { name: "Approve Dela Cruz, Juan's overtime" }),
    );
    await waitFor(() =>
      expect(call).toHaveBeenCalledWith("overtime_decide", { id: 1, approve: true, note: null }),
    );
  });

  it("needs a reason to reject", async () => {
    renderAs();
    fireEvent.click(
      await screen.findByRole("button", { name: "Reject Dela Cruz, Juan's overtime" }),
    );
    const form = within(screen.getByRole("form", { name: "Reject Dela Cruz, Juan's overtime" }));
    fireEvent.click(form.getByRole("button", { name: "Reject" }));
    expect(await form.findByText("Say why, in 3 to 200 characters")).toBeTruthy();
    expect(call).not.toHaveBeenCalledWith("overtime_decide", expect.anything());

    fireEvent.change(form.getByLabelText("Why it's rejected"), { target: { value: "Not needed" } });
    fireEvent.click(form.getByRole("button", { name: "Reject" }));
    await waitFor(() =>
      expect(call).toHaveBeenCalledWith("overtime_decide", {
        id: 1,
        approve: false,
        note: "Not needed",
      }),
    );
  });

  it("doesn't offer a decision on your own overtime", async () => {
    renderAs();
    await screen.findByRole("row", { name: /Santos, Maria/ });
    const own = within(pendingSection().getByRole("row", { name: /Santos, Maria/ }));
    expect(own.getByText("Your own: someone else decides")).toBeTruthy();
    expect(own.queryByRole("button")).toBeNull();
  });

  it("filters history by status and cancels an approved request", async () => {
    renderAs();
    await screen.findByRole("row", { name: /Reyes, Ana/ });
    fireEvent.change(screen.getByLabelText("Status"), { target: { value: "APPROVED" } });
    await waitFor(() =>
      expect(call).toHaveBeenLastCalledWith("overtime_list", {
        status: "APPROVED",
        range: expect.objectContaining({ from: expect.stringMatching(/-01$/) }),
      }),
    );
    expect(
      within(screen.getByRole("row", { name: /Cruz, Leo/ })).getByText("Payroll posted"),
    ).toBeTruthy();
    fireEvent.click(
      screen.getByRole("button", { name: "Cancel Reyes, Ana's overtime on 2026-10-07" }),
    );
    await waitFor(() => expect(call).toHaveBeenCalledWith("overtime_request_cancel", { id: 4 }));
  });

  it("files for an employee by number", async () => {
    renderAs();
    await screen.findByRole("row", { name: /Reyes, Ana/ });
    fireEvent.click(screen.getByRole("button", { name: "File for an employee" }));
    fireEvent.change(screen.getByLabelText("Employee number"), { target: { value: "EMP-0007" } });
    fireEvent.change(screen.getByLabelText("Date"), { target: { value: "2026-10-10" } });
    fireEvent.change(screen.getByLabelText("From"), { target: { value: "08:00" } });
    fireEvent.change(screen.getByLabelText("To"), { target: { value: "12:00" } });
    fireEvent.change(screen.getByLabelText("What it's for"), {
      target: { value: "Saturday delivery" },
    });
    fireEvent.click(screen.getByRole("button", { name: "File overtime" }));
    await waitFor(() =>
      expect(call).toHaveBeenCalledWith("overtime_request_create", {
        input: {
          employeeNo: "EMP-0007",
          workDate: "2026-10-10",
          startTime: "08:00",
          endTime: "12:00",
          reason: "Saturday delivery",
        },
      }),
    );
  });
});
