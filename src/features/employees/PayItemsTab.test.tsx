import { cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { Employee } from "@/bindings/Employee";
import type { RecurringItem } from "@/bindings/RecurringItem";

const call = vi.fn();
vi.mock("@/lib/ipc", async (orig) => ({
  ...(await orig<typeof import("@/lib/ipc")>()),
  call: (cmd: string, args?: Record<string, unknown>) => call(cmd, args),
}));

import { PayItemsTab } from "./PayItemsTab";

const maria = { id: 7, archivedAt: null } as Employee;

const loan: RecurringItem = {
  id: 1,
  employeeId: 7,
  kind: "LOAN",
  label: "SSS salary loan",
  amountCents: 100_000,
  taxable: false,
  schedule: "EVERY_CUTOFF",
  startDate: "2026-03-01",
  endDate: null,
  remainingBalanceCents: 1_200_000,
};

const rice: RecurringItem = {
  ...loan,
  id: 2,
  kind: "ALLOWANCE",
  label: "Rice subsidy",
  amountCents: 150_000,
  taxable: false,
  schedule: "SECOND_CUTOFF",
  startDate: "2026-01-01",
  endDate: "2026-12-31",
  remainingBalanceCents: null,
};

let items: RecurringItem[];

function type(label: string, value: string) {
  fireEvent.change(screen.getByLabelText(label), { target: { value } });
}

describe("PayItemsTab", () => {
  beforeEach(() => {
    items = [rice, loan];
    call.mockImplementation(async (cmd: string, args?: Record<string, unknown>) => {
      if (cmd === "employee_recurring_items") return items;
      if (cmd === "employee_add_recurring_item") {
        const added = { ...(args!.input as RecurringItem), id: 3, employeeId: 7 };
        items = [...items, added];
        return added;
      }
      if (cmd === "recurring_item_update") {
        const saved = { ...(args!.input as RecurringItem), id: args!.id as number, employeeId: 7 };
        items = items.map((i) => (i.id === saved.id ? saved : i));
        return saved;
      }
      if (cmd === "recurring_item_delete") {
        items = items.filter((i) => i.id !== args!.id);
        return null;
      }
      return null;
    });
  });

  afterEach(() => {
    cleanup();
    call.mockReset();
  });

  it("lists each item with its cutoffs, dates and what a loan still owes", async () => {
    render(<PayItemsTab employee={maria} />);
    const riceRow = (await screen.findByText("Rice subsidy")).closest("tr")!;
    expect(within(riceRow).getByText("Allowance")).toBeTruthy();
    expect(within(riceRow).getByText("₱1,500.00")).toBeTruthy();
    expect(within(riceRow).getByText("16th to end of month")).toBeTruthy();
    expect(within(riceRow).getByText("Jan 1, 2026 to Dec 31, 2026")).toBeTruthy();
    const loanRow = screen.getByText("SSS salary loan").closest("tr")!;
    expect(within(loanRow).getByText("Loan")).toBeTruthy();
    expect(within(loanRow).getByText("₱12,000.00")).toBeTruthy();
    expect(within(loanRow).getByText("From Mar 1, 2026")).toBeTruthy();
  });

  it("adds a loan with its balance", async () => {
    render(<PayItemsTab employee={maria} />);
    await screen.findByText("Rice subsidy");
    expect(screen.queryByLabelText("Still owed (₱)")).toBeNull();
    fireEvent.change(screen.getByLabelText("Kind"), { target: { value: "LOAN" } });
    expect(screen.queryByLabelText("Taxable")).toBeNull();
    type("Name", "Company loan");
    type("Amount per cutoff (₱)", "2,000");
    type("Starts on", "2026-11-01");
    type("Still owed (₱)", "24,000");
    fireEvent.click(screen.getByRole("button", { name: "Add item" }));

    await screen.findByText("Company loan");
    expect(call).toHaveBeenCalledWith("employee_add_recurring_item", {
      id: 7,
      input: {
        kind: "LOAN",
        label: "Company loan",
        amountCents: 200_000,
        taxable: false,
        schedule: "EVERY_CUTOFF",
        startDate: "2026-11-01",
        endDate: null,
        remainingBalanceCents: 2_400_000,
      },
    });
  });

  it("shows the server's field errors", async () => {
    call.mockImplementation(async (cmd: string) => {
      if (cmd === "employee_recurring_items") return [];
      throw {
        code: "VALIDATION",
        message: "Check the form",
        fields: [
          { field: "amountCents", message: "Enter an amount above ₱0 and up to ₱10,000,000" },
        ],
      };
    });
    render(<PayItemsTab employee={maria} />);
    await screen.findByText(/No allowances or deductions yet/);
    type("Name", "Bonus");
    type("Amount per cutoff (₱)", "99999999");
    type("Starts on", "2026-11-01");
    fireEvent.click(screen.getByRole("button", { name: "Add item" }));
    expect(await screen.findByText("Enter an amount above ₱0 and up to ₱10,000,000")).toBeTruthy();
  });

  it("edits an item in place, keeping its kind", async () => {
    render(<PayItemsTab employee={maria} />);
    await screen.findByText("SSS salary loan");
    fireEvent.click(screen.getByRole("button", { name: "Edit SSS salary loan" }));
    const form = screen.getByRole("form", { name: "Edit SSS salary loan" });
    expect(within(form).queryByLabelText("Kind")).toBeNull();
    expect((within(form).getByLabelText("Still owed (₱)") as HTMLInputElement).value).toBe(
      "12000.00",
    );
    type("Still owed (₱)", "11,000");
    fireEvent.click(within(form).getByRole("button", { name: "Save changes" }));

    await waitFor(() => expect(screen.getByText("₱11,000.00")).toBeTruthy());
    expect(call).toHaveBeenCalledWith(
      "recurring_item_update",
      expect.objectContaining({
        id: 1,
        input: expect.objectContaining({ kind: "LOAN", remainingBalanceCents: 1_100_000 }),
      }),
    );
  });

  it("removes an item after confirming", async () => {
    render(<PayItemsTab employee={maria} />);
    await screen.findByText("Rice subsidy");
    fireEvent.click(screen.getByRole("button", { name: "Remove Rice subsidy" }));
    fireEvent.click(screen.getByRole("button", { name: "Keep" }));
    expect(call).not.toHaveBeenCalledWith("recurring_item_delete", expect.anything());
    fireEvent.click(screen.getByRole("button", { name: "Remove Rice subsidy" }));
    fireEvent.click(screen.getByRole("button", { name: "Yes, remove" }));
    await waitFor(() => expect(screen.queryByText("Rice subsidy")).toBeNull());
    expect(call).toHaveBeenCalledWith("recurring_item_delete", { id: 2 });
  });

  it("is read-only for archived employees", async () => {
    render(<PayItemsTab employee={{ ...maria, archivedAt: "2026-10-01T00:00:00Z" }} />);
    await screen.findByText("Rice subsidy");
    expect(screen.queryByRole("button", { name: /Edit|Remove|Add item/ })).toBeNull();
  });
});
