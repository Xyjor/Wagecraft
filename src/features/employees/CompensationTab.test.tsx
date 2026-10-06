import { cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { Compensation } from "@/bindings/Compensation";
import type { Employee } from "@/bindings/Employee";

const call = vi.fn();
vi.mock("@/lib/ipc", async (orig) => ({
  ...(await orig<typeof import("@/lib/ipc")>()),
  call: (cmd: string, args?: Record<string, unknown>) => call(cmd, args),
}));

import { CompensationTab } from "./CompensationTab";

const maria = {
  id: 7,
  positionId: 10,
  archivedAt: null,
} as Employee;

const positions = [
  {
    id: 10,
    departmentId: 1,
    departmentName: "Operations",
    title: "Driver",
    minRateCents: 1_800_000,
    maxRateCents: 2_500_000,
    isActive: true,
  },
];

const first: Compensation = {
  id: 1,
  employeeId: 7,
  payBasis: "MONTHLY",
  rateCents: 2_000_000,
  effectiveFrom: "2025-01-01",
  effectiveTo: null,
  reason: "Hired",
  minimumWageEarner: false,
  createdByName: "hr",
  createdAt: "2025-01-02T00:00:00Z",
};

let history: Compensation[];

function type(label: string, value: string) {
  fireEvent.change(screen.getByLabelText(label), { target: { value } });
}

describe("CompensationTab", () => {
  beforeEach(() => {
    history = [first];
    call.mockImplementation(async (cmd: string) => {
      if (cmd === "employee_compensation_history") return history;
      if (cmd === "position_list") return positions;
      if (cmd === "employee_add_compensation") {
        const added = { ...first, id: 2, rateCents: 2_600_000, effectiveFrom: "2026-03-01" };
        history = [added, { ...first, effectiveTo: "2026-02-28" }];
        return added;
      }
      return null;
    });
  });

  afterEach(() => {
    cleanup();
    call.mockReset();
  });

  it("lists the history with the current rate marked", async () => {
    render(<CompensationTab employee={maria} />);
    const row = (await screen.findByText("₱20,000.00")).closest("tr")!;
    expect(within(row).getByText("Current")).toBeTruthy();
    expect(within(row).getByText("Monthly")).toBeTruthy();
    expect(screen.getByRole("heading", { name: "Change pay" })).toBeTruthy();
  });

  it("adds a raise and shows the closed previous rate", async () => {
    render(<CompensationTab employee={maria} />);
    await screen.findByText("₱20,000.00");
    type("Rate (₱)", "26,000");
    type("Starts on", "2026-03-01");
    type("Reason", "Annual review");
    fireEvent.click(screen.getByRole("button", { name: "Save rate" }));

    await waitFor(() =>
      expect(call).toHaveBeenCalledWith("employee_add_compensation", {
        id: 7,
        input: {
          payBasis: "MONTHLY",
          rateCents: 2_600_000,
          effectiveFrom: "2026-03-01",
          reason: "Annual review",
          minimumWageEarner: false,
        },
      }),
    );
    expect(await screen.findByText("₱26,000.00")).toBeTruthy();
    expect(screen.getByText("Feb 28, 2026")).toBeTruthy();
    expect(screen.getByText(/above the Driver range/)).toBeTruthy();
  });

  it("saves a rate for a minimum wage earner and marks it in the history", async () => {
    call.mockImplementation(async (cmd: string, args?: Record<string, unknown>) => {
      if (cmd === "employee_compensation_history") return history;
      if (cmd === "position_list") return positions;
      if (cmd === "employee_add_compensation") {
        const input = args!.input as { minimumWageEarner: boolean };
        const added = { ...first, id: 2, rateCents: 1_500_000, effectiveFrom: "2026-03-01" };
        history = [{ ...added, minimumWageEarner: input.minimumWageEarner }, first];
        return added;
      }
      return null;
    });
    render(<CompensationTab employee={maria} />);
    const old = (await screen.findByText("₱20,000.00")).closest("tr")!;
    expect(within(old).queryByText("Minimum wage earner")).toBeNull();
    type("Rate (₱)", "15,000");
    type("Starts on", "2026-03-01");
    fireEvent.click(screen.getByRole("checkbox", { name: /Minimum wage earner/ }));
    fireEvent.click(screen.getByRole("button", { name: "Save rate" }));

    await waitFor(() =>
      expect(call).toHaveBeenCalledWith("employee_add_compensation", {
        id: 7,
        input: expect.objectContaining({ rateCents: 1_500_000, minimumWageEarner: true }),
      }),
    );
    const row = (await screen.findByText("₱15,000.00")).closest("tr")!;
    expect(within(row).getByText("Minimum wage earner")).toBeTruthy();
  });

  it("starts the next rate with the current rate's minimum wage setting", async () => {
    history = [{ ...first, minimumWageEarner: true }];
    render(<CompensationTab employee={maria} />);
    const box = await screen.findByRole("checkbox", { name: /Minimum wage earner/ });
    expect((box as HTMLInputElement).checked).toBe(true);
  });

  it("asks for a pay-period start before saving", async () => {
    render(<CompensationTab employee={maria} />);
    await screen.findByText("₱20,000.00");
    type("Rate (₱)", "26000");
    type("Starts on", "2026-03-05");
    fireEvent.click(screen.getByRole("button", { name: "Save rate" }));
    expect(
      await screen.findByText("Start on the 1st or 16th, the first day of a pay period"),
    ).toBeTruthy();
    expect(call).not.toHaveBeenCalledWith("employee_add_compensation", expect.anything());
  });

  it("puts the backend's rate message under the Rate field", async () => {
    call.mockImplementation(async (cmd: string) => {
      if (cmd === "employee_compensation_history") return history;
      if (cmd === "position_list") return positions;
      throw {
        code: "VALIDATION",
        message: "Some fields are invalid",
        fields: [{ field: "rateCents", message: "Enter an amount above ₱0 and up to ₱10,000,000" }],
      };
    });
    render(<CompensationTab employee={maria} />);
    await screen.findByText("₱20,000.00");
    type("Rate (₱)", "99999999");
    type("Starts on", "2026-03-01");
    fireEvent.click(screen.getByRole("button", { name: "Save rate" }));
    const message = await screen.findByText("Enter an amount above ₱0 and up to ₱10,000,000");
    expect(message.id).toBe("rate-error");
  });

  it("hides the form for archived employees", async () => {
    render(<CompensationTab employee={{ ...maria, archivedAt: "2026-10-01T00:00:00Z" }} />);
    await screen.findByText("₱20,000.00");
    expect(screen.queryByRole("button", { name: "Save rate" })).toBeNull();
  });
});
