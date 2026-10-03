import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { Employee } from "@/bindings/Employee";

const call = vi.fn();
vi.mock("@/lib/ipc", async (orig) => ({
  ...(await orig<typeof import("@/lib/ipc")>()),
  call: (cmd: string, args?: Record<string, unknown>) => call(cmd, args),
}));

import { KioskPinTab } from "./KioskPinTab";

const juan = { id: 7, firstName: "Juan", hasKioskPin: false, archivedAt: null } as Employee;

function type(label: string, value: string) {
  fireEvent.change(screen.getByLabelText(label), { target: { value } });
}

describe("KioskPinTab", () => {
  afterEach(() => {
    cleanup();
    call.mockReset();
  });

  it("sets a PIN typed twice and reloads the profile", async () => {
    call.mockResolvedValue({ ...juan, hasKioskPin: true });
    const onSaved = vi.fn(async () => {});
    render(<KioskPinTab employee={juan} onSaved={onSaved} />);
    type("New PIN", "4821");
    type("Type it again", "4821");
    fireEvent.click(screen.getByRole("button", { name: "Set PIN" }));

    expect((await screen.findByRole("status")).textContent).toContain("Tell Juan the new PIN");
    expect(call).toHaveBeenCalledWith("employee_set_kiosk_pin", { id: 7, pin: "4821" });
    expect(onSaved).toHaveBeenCalled();
    expect((screen.getByLabelText("New PIN") as HTMLInputElement).value).toBe("");
  });

  it("checks the PIN before calling the backend", async () => {
    render(<KioskPinTab employee={juan} onSaved={async () => {}} />);
    type("New PIN", "12a");
    fireEvent.click(screen.getByRole("button", { name: "Set PIN" }));
    expect(await screen.findByText("Use 4 to 6 digits")).toBeTruthy();

    type("New PIN", "123456");
    type("Type it again", "123465");
    fireEvent.click(screen.getByRole("button", { name: "Set PIN" }));
    await waitFor(() => expect(screen.getByText("The two PINs don't match")).toBeTruthy());
    expect(call).not.toHaveBeenCalled();
  });

  it("offers to replace an existing PIN, but not for archived employees", () => {
    render(<KioskPinTab employee={{ ...juan, hasKioskPin: true }} onSaved={async () => {}} />);
    expect(screen.getByRole("button", { name: "Replace PIN" })).toBeTruthy();
    cleanup();
    render(
      <KioskPinTab
        employee={{ ...juan, archivedAt: "2026-10-01T00:00:00Z" }}
        onSaved={async () => {}}
      />,
    );
    expect(screen.getByText("Archived employees can't use the time clock.")).toBeTruthy();
  });
});
