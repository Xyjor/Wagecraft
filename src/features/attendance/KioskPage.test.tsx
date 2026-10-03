import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { KioskPunch } from "@/bindings/KioskPunch";

const call = vi.fn();
vi.mock("@/lib/ipc", async (orig) => ({
  ...(await orig<typeof import("@/lib/ipc")>()),
  call: (cmd: string, args?: Record<string, unknown>) => call(cmd, args),
}));

import { KioskPage } from "./KioskPage";

const punchIn: KioskPunch = {
  firstName: "Juan",
  kind: "IN",
  at: "2026-10-05T08:25:00",
  workDate: "2026-10-05",
  lateMinutes: 15,
};

function type(label: string, value: string) {
  fireEvent.change(screen.getByLabelText(label), { target: { value } });
}

describe("KioskPage", () => {
  beforeEach(() => {
    call.mockImplementation(async (cmd: string) => {
      if (cmd === "kiosk_clock_in") return punchIn;
      if (cmd === "kiosk_clock_out")
        return { ...punchIn, kind: "OUT", at: "2026-10-05T17:02:00", lateMinutes: 15 };
      return null;
    });
  });

  afterEach(() => {
    cleanup();
    call.mockReset();
  });

  it("clocks in with the employee number and PIN, then clears the form", async () => {
    render(<KioskPage onClose={() => {}} />);
    type("Employee no.", " EMP-0001 ");
    type("PIN", "1234");
    fireEvent.click(screen.getByRole("button", { name: "Time in" }));

    const status = await screen.findByRole("status");
    expect(status.textContent).toContain("Time in recorded, Juan.");
    expect(status.textContent).toContain("8:25 AM · 15 minutes late");
    expect(call).toHaveBeenCalledWith("kiosk_clock_in", { employeeNo: "EMP-0001", pin: "1234" });
  });

  it("clocks out without mentioning lateness", async () => {
    render(<KioskPage onClose={() => {}} />);
    type("Employee no.", "EMP-0001");
    type("PIN", "1234");
    fireEvent.click(screen.getByRole("button", { name: "Time out" }));

    const status = await screen.findByRole("status");
    expect(status.textContent).toContain("Time out recorded, Juan.");
    expect(status.textContent).not.toContain("late");
  });

  it("asks for both fields before calling the backend", async () => {
    render(<KioskPage onClose={() => {}} />);
    type("Employee no.", "EMP-0001");
    fireEvent.click(screen.getByRole("button", { name: "Time in" }));
    expect(screen.getByRole("alert").textContent).toBe("Enter your employee number and PIN.");
    expect(call).not.toHaveBeenCalled();
  });

  it("shows the backend's refusal and clears only the PIN", async () => {
    call.mockRejectedValue({
      code: "INVALID_PIN",
      message: "Wrong employee number or PIN",
      fields: [],
    });
    render(<KioskPage onClose={() => {}} />);
    type("Employee no.", "EMP-0001");
    type("PIN", "9999");
    fireEvent.click(screen.getByRole("button", { name: "Time in" }));

    expect((await screen.findByRole("alert")).textContent).toBe("Wrong employee number or PIN");
    expect((screen.getByLabelText("PIN") as HTMLInputElement).value).toBe("");
    expect((screen.getByLabelText("Employee no.") as HTMLInputElement).value).toBe("EMP-0001");
  });

  it("is ready for the next person after a few seconds", async () => {
    render(<KioskPage onClose={() => {}} confirmMs={50} />);
    type("Employee no.", "EMP-0001");
    type("PIN", "1234");
    fireEvent.click(screen.getByRole("button", { name: "Time in" }));
    await screen.findByRole("status");

    await waitFor(() => expect(screen.queryByRole("status")).toBeNull());
    expect((screen.getByLabelText("Employee no.") as HTMLInputElement).value).toBe("");
  });
});
