import { cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { Holiday } from "@/bindings/Holiday";
import type { Me } from "@/bindings/Me";
import { SessionContext } from "@/features/auth/session";

const call = vi.fn();
vi.mock("@/lib/ipc", async (orig) => ({
  ...(await orig<typeof import("@/lib/ipc")>()),
  call: (cmd: string, args?: Record<string, unknown>) => call(cmd, args),
}));

import { HolidaysPage } from "./HolidaysPage";
import { checkHoliday } from "./validation";

const hr: Me = {
  userId: 2,
  username: "maria",
  role: "HR",
  employeeId: null,
  mustChangePassword: false,
  theme: "SYSTEM",
};

let holidays: Holiday[];

function renderAs(who: Me = hr) {
  render(
    <SessionContext.Provider value={{ me: who, signOut: async () => {} }}>
      <HolidaysPage initialYear={2026} />
    </SessionContext.Provider>,
  );
}

function type(label: string, value: string) {
  fireEvent.change(screen.getByLabelText(label), { target: { value } });
}

describe("HolidaysPage", () => {
  beforeEach(() => {
    holidays = [
      {
        id: 1,
        date: "2026-08-21",
        name: "Ninoy Aquino Day",
        kind: "SPECIAL_NON_WORKING",
        locked: true,
      },
      { id: 2, date: "2026-11-30", name: "Bonifacio Day", kind: "REGULAR", locked: false },
    ];
    call.mockImplementation(async (cmd: string, args?: { year?: number }) => {
      if (cmd === "holiday_list") return args?.year === 2026 ? holidays : [];
      return {};
    });
  });

  afterEach(() => {
    cleanup();
    call.mockReset();
  });

  it("lists the year's holidays with their day and type", async () => {
    renderAs();
    const row = await screen.findByRole("row", { name: /Bonifacio Day/ });
    expect(within(row).getByText("Mon, Nov 30, 2026")).toBeTruthy();
    expect(within(row).getByText("Regular holiday")).toBeTruthy();
    expect(call).toHaveBeenCalledWith("holiday_list", { year: 2026 });
  });

  it("tells Staff they can't manage holidays", async () => {
    renderAs({ ...hr, role: "STAFF" });
    expect(
      await screen.findByText("Only Admin and HR can manage the holiday calendar."),
    ).toBeTruthy();
    expect(call).not.toHaveBeenCalled();
  });

  it("moves between years", async () => {
    renderAs();
    await screen.findByRole("row", { name: /Bonifacio Day/ });
    fireEvent.click(screen.getByRole("button", { name: "Next year" }));
    expect(await screen.findByText("No holidays for 2027 yet.")).toBeTruthy();
    expect(call).toHaveBeenLastCalledWith("holiday_list", { year: 2027 });
  });

  it("adds a holiday", async () => {
    renderAs();
    await screen.findByRole("row", { name: /Bonifacio Day/ });
    fireEvent.click(screen.getByRole("button", { name: "Add holiday" }));
    type("Date", "2026-12-30");
    type("Name", " Rizal  Day ");
    fireEvent.click(screen.getByRole("button", { name: "Save holiday" }));

    await waitFor(() =>
      expect(call).toHaveBeenCalledWith("holiday_create", {
        input: { date: "2026-12-30", name: "Rizal Day", kind: "REGULAR" },
      }),
    );
    await waitFor(() => expect(screen.queryByRole("form", { name: "Add holiday" })).toBeNull());
  });

  it("shows the server's message beside the field", async () => {
    call.mockImplementation(async (cmd: string) => {
      if (cmd === "holiday_list") return holidays;
      if (cmd === "holiday_update") {
        throw {
          code: "VALIDATION",
          message: "Please check the form",
          fields: [{ field: "date", message: "This date already has two holidays" }],
        };
      }
      return {};
    });
    renderAs();
    fireEvent.click(await screen.findByRole("button", { name: "Edit Bonifacio Day" }));
    expect((screen.getByLabelText("Date") as HTMLInputElement).value).toBe("2026-11-30");
    fireEvent.change(screen.getByLabelText("Type"), { target: { value: "SPECIAL_WORKING" } });
    fireEvent.click(screen.getByRole("button", { name: "Save holiday" }));

    expect(await screen.findByText("This date already has two holidays")).toBeTruthy();
    expect(call).toHaveBeenCalledWith("holiday_update", {
      id: 2,
      input: { date: "2026-11-30", name: "Bonifacio Day", kind: "SPECIAL_WORKING" },
    });
  });

  it("asks before deleting", async () => {
    renderAs();
    fireEvent.click(await screen.findByRole("button", { name: "Delete Bonifacio Day" }));
    expect(call).not.toHaveBeenCalledWith("holiday_delete", expect.anything());
    fireEvent.click(screen.getByRole("button", { name: "Yes, delete" }));
    await waitFor(() => expect(call).toHaveBeenCalledWith("holiday_delete", { id: 2 }));
  });

  it("offers no changes on a date in a posted payroll period", async () => {
    renderAs();
    const row = await screen.findByRole("row", { name: /Ninoy Aquino Day/ });
    expect(within(row).getByText("Payroll posted")).toBeTruthy();
    expect(within(row).queryByRole("button")).toBeNull();
  });
});

describe("checkHoliday", () => {
  it("rejects impossible dates and missing fields", () => {
    const r = checkHoliday({ date: "2026-02-30", name: "x", kind: "FIESTA" });
    expect(r.ok).toBe(false);
    if (!r.ok) expect(Object.keys(r.errors)).toEqual(["date", "name", "kind"]);
    expect(checkHoliday({ date: "1999-12-31", name: "Old", kind: "REGULAR" }).ok).toBe(false);
    expect(checkHoliday({ date: "2028-02-29", name: "Leap", kind: "REGULAR" }).ok).toBe(true);
  });
});
