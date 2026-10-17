import { cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { MemoryRouter } from "react-router";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

const call = vi.fn();
vi.mock("@/lib/ipc", async (orig) => ({
  ...(await orig<typeof import("@/lib/ipc")>()),
  call: (cmd: string, args?: Record<string, unknown>) => call(cmd, args),
}));

import { SETUP_HIDDEN_KEY, SetupChecklist } from "./SetupChecklist";

type Answers = Record<string, unknown>;

const fresh: Answers = {
  settings_get: { companyName: "Acme", companyAddress: "", companyTin: "" },
  department_list: [],
  position_list: [],
  schedule_list: [],
  holiday_list: [],
  employee_list: { items: [], total: 0, page: 1, pageSize: 1 },
  payroll_period_list: [],
};

function answer(overrides: Answers = {}) {
  const all = { ...fresh, ...overrides };
  call.mockImplementation(async (cmd: string) => {
    if (cmd in all) return all[cmd];
    throw new Error(`unexpected ${cmd}`);
  });
}

function show() {
  return render(
    <MemoryRouter>
      <SetupChecklist year={2026} />
    </MemoryRouter>,
  );
}

beforeEach(() => {
  call.mockReset();
  localStorage.clear();
});
afterEach(cleanup);

describe("SetupChecklist", () => {
  it("lists every step on a fresh install, each linking to where it's done", async () => {
    answer();
    show();

    const panel = await screen.findByRole("region", { name: "Finish setting up" });
    expect(within(panel).getByText("0 of 6 done")).toBeTruthy();
    const link = within(panel).getByRole("link", { name: /Add the company address and TIN/ });
    expect(link.getAttribute("href")).toBe("/settings");
    expect(call).toHaveBeenCalledWith("holiday_list", { year: 2026 });
  });

  it("ticks off what's done", async () => {
    answer({
      settings_get: { companyName: "Acme", companyAddress: "Makati", companyTin: "123" },
      employee_list: { items: [], total: 40, page: 1, pageSize: 1 },
    });
    show();

    const panel = await screen.findByRole("region", { name: "Finish setting up" });
    expect(within(panel).getByText("2 of 6 done")).toBeTruthy();
    const company = within(panel).getByText("Add the company address and TIN").closest("li")!;
    expect(company.textContent).toContain("done");
  });

  it("shows nothing once everything is done", async () => {
    const active = [{ isActive: true }];
    answer({
      settings_get: { companyName: "Acme", companyAddress: "Makati", companyTin: "123" },
      department_list: active,
      position_list: active,
      schedule_list: active,
      holiday_list: [{ id: 1 }],
      employee_list: { items: [], total: 40, page: 1, pageSize: 1 },
      payroll_period_list: [{ id: 1 }],
    });
    const { container } = show();

    await waitFor(() => expect(call).toHaveBeenCalledWith("payroll_period_list", undefined));
    await waitFor(() => expect(container.textContent).toBe(""));
  });

  it("shows nothing if it can't load", async () => {
    call.mockRejectedValue({ code: "FORBIDDEN", message: "No", fields: [] });
    const { container } = show();

    await waitFor(() => expect(call).toHaveBeenCalled());
    expect(container.textContent).toBe("");
  });

  it("can be hidden, and stays hidden", async () => {
    answer();
    show();

    fireEvent.click(await screen.findByRole("button", { name: "Hide setup checklist" }));
    expect(screen.queryByRole("region", { name: "Finish setting up" })).toBeNull();
    expect(localStorage.getItem(SETUP_HIDDEN_KEY)).toBe("1");

    cleanup();
    call.mockClear();
    show();
    expect(screen.queryByRole("region", { name: "Finish setting up" })).toBeNull();
    expect(call).not.toHaveBeenCalled();
  });
});
