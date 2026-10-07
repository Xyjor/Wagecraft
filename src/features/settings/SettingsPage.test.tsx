import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { Me } from "@/bindings/Me";
import type { Settings } from "@/bindings/Settings";
import { SessionContext } from "@/features/auth/session";

const call = vi.fn();
vi.mock("@/lib/ipc", async (orig) => ({
  ...(await orig<typeof import("@/lib/ipc")>()),
  call: (cmd: string, args?: Record<string, unknown>) => call(cmd, args),
}));

import { SettingsPage } from "./SettingsPage";

const admin: Me = {
  userId: 1,
  username: "ana",
  role: "ADMIN",
  employeeId: null,
  mustChangePassword: false,
  theme: "SYSTEM",
};

const saved: Settings = {
  companyName: "Acme Trading",
  companyAddress: "12 Rizal St, Makati",
  companyTin: "123456789000",
  companyLogo: null,
  idleTimeoutMinutes: 15,
  backupFolder: "",
  defaultBackupFolder: "C:\\Users\\ana\\AppData\\Wagecraft\\backups",
  backupKeep: 14,
};

const LOGO = "data:image/png;base64,iVBORw0KGgo=";

function renderAs(who: Me = admin) {
  render(
    <SessionContext.Provider value={{ me: who, signOut: async () => {} }}>
      <SettingsPage />
    </SessionContext.Provider>,
  );
}

function field(label: string) {
  return screen.getByLabelText(label) as HTMLInputElement;
}

beforeEach(() => {
  call.mockReset();
  call.mockImplementation(async (cmd: string, args?: Record<string, unknown>) => {
    if (cmd === "settings_get") return saved;
    if (cmd === "settings_update") return { ...saved, ...(args?.input as object) };
    if (cmd === "settings_pick_backup_folder") return "E:\\Backups";
    if (cmd === "settings_pick_logo") return { ...saved, companyLogo: LOGO };
    if (cmd === "settings_clear_logo") return { ...saved, companyLogo: null };
    throw new Error(`unexpected ${cmd}`);
  });
});

afterEach(cleanup);

describe("SettingsPage", () => {
  it("is only for Admins", () => {
    renderAs({ ...admin, role: "HR" });

    expect(screen.getByText("Only Admins can change settings.")).toBeTruthy();
    expect(call).not.toHaveBeenCalled();
  });

  it("fills the form with the saved settings, TIN grouped", async () => {
    renderAs();

    await waitFor(() => expect(field("Company name").value).toBe("Acme Trading"));
    expect(field("Address").value).toBe("12 Rizal St, Makati");
    expect(field("TIN").value).toBe("123-456-789-000");
    expect(field("Sign out after this many idle minutes").value).toBe("15");
    expect(field("Daily backups to keep").value).toBe("14");
    expect(screen.getByText(saved.defaultBackupFolder)).toBeTruthy();
  });

  it("saves the form and says so", async () => {
    renderAs();
    await waitFor(() => expect(field("Company name").value).toBe("Acme Trading"));

    fireEvent.change(field("Sign out after this many idle minutes"), { target: { value: "30" } });
    fireEvent.click(screen.getByRole("button", { name: "Save settings" }));

    await waitFor(() =>
      expect(call).toHaveBeenCalledWith("settings_update", {
        input: {
          companyName: "Acme Trading",
          companyAddress: "12 Rizal St, Makati",
          companyTin: "123-456-789-000",
          idleTimeoutMinutes: 30,
          backupFolder: "",
          backupKeep: 14,
        },
      }),
    );
    expect((await screen.findByRole("status")).textContent).toContain("Settings saved");
  });

  it("checks the numbers before sending anything", async () => {
    renderAs();
    await waitFor(() => expect(field("Company name").value).toBe("Acme Trading"));

    fireEvent.change(field("Sign out after this many idle minutes"), { target: { value: "2" } });
    fireEvent.change(field("Company name"), { target: { value: " " } });
    fireEvent.click(screen.getByRole("button", { name: "Save settings" }));

    expect(await screen.findByText("Choose between 5 and 120 minutes")).toBeTruthy();
    expect(screen.getByText("Enter the company name")).toBeTruthy();
    expect(call).not.toHaveBeenCalledWith("settings_update", expect.anything());
  });

  it("shows the server's reason next to the field", async () => {
    call.mockImplementation(async (cmd: string) => {
      if (cmd === "settings_get") return saved;
      throw {
        code: "VALIDATION",
        message: "Some fields are invalid",
        fields: [{ field: "backupFolder", message: "Choose a folder that exists on this PC" }],
      };
    });
    renderAs();
    await waitFor(() => expect(field("Company name").value).toBe("Acme Trading"));

    fireEvent.click(screen.getByRole("button", { name: "Save settings" }));

    expect(await screen.findByText("Choose a folder that exists on this PC")).toBeTruthy();
  });

  it("picks another backup folder, and can go back to the default", async () => {
    renderAs();
    await waitFor(() => expect(field("Company name").value).toBe("Acme Trading"));

    fireEvent.click(screen.getByRole("button", { name: "Choose folder…" }));
    expect(await screen.findByText("E:\\Backups")).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "Save settings" }));
    await waitFor(() =>
      expect(call).toHaveBeenCalledWith(
        "settings_update",
        expect.objectContaining({
          input: expect.objectContaining({ backupFolder: "E:\\Backups" }),
        }),
      ),
    );

    fireEvent.click(screen.getByRole("button", { name: "Use the default folder" }));
    expect(screen.getByText(saved.defaultBackupFolder)).toBeTruthy();
  });

  it("has no logo at first and adds one straight away", async () => {
    renderAs();
    await waitFor(() => expect(screen.getByText("No logo yet.")).toBeTruthy());
    expect(screen.queryByRole("button", { name: "Remove logo" })).toBeNull();

    fireEvent.click(screen.getByRole("button", { name: "Choose logo…" }));

    await waitFor(() => expect(screen.getByRole("status").textContent).toBe("Logo saved."));
    expect(call).toHaveBeenCalledWith("settings_pick_logo", undefined);
    expect(screen.getByRole("img", { name: "Company logo" }).getAttribute("src")).toBe(LOGO);
    expect(screen.queryByText("No logo yet.")).toBeNull();
  });

  it("removes the logo", async () => {
    call.mockImplementation(async (cmd: string) => {
      if (cmd === "settings_get") return { ...saved, companyLogo: LOGO };
      if (cmd === "settings_clear_logo") return { ...saved, companyLogo: null };
      throw new Error(`unexpected ${cmd}`);
    });
    renderAs();
    await waitFor(() => expect(screen.getByRole("img", { name: "Company logo" })).toBeTruthy());

    fireEvent.click(screen.getByRole("button", { name: "Remove logo" }));

    await waitFor(() => expect(screen.getByRole("status").textContent).toBe("Logo removed."));
    expect(screen.getByText("No logo yet.")).toBeTruthy();
  });

  it("keeps unsaved form edits when the logo changes", async () => {
    renderAs();
    await waitFor(() => expect(field("Company name").value).toBe("Acme Trading"));
    fireEvent.change(field("Company name"), { target: { value: "Acme Trading Corp." } });

    fireEvent.click(screen.getByRole("button", { name: "Choose logo…" }));

    await waitFor(() => expect(screen.getByRole("status").textContent).toBe("Logo saved."));
    expect(field("Company name").value).toBe("Acme Trading Corp.");
  });

  it("shows why a logo was refused and keeps the old state", async () => {
    call.mockImplementation(async (cmd: string) => {
      if (cmd === "settings_get") return saved;
      if (cmd === "settings_pick_logo")
        throw { code: "CONFLICT", message: "Choose a PNG or JPEG image for the logo", fields: [] };
      throw new Error(`unexpected ${cmd}`);
    });
    renderAs();
    await waitFor(() => expect(screen.getByText("No logo yet.")).toBeTruthy());

    fireEvent.click(screen.getByRole("button", { name: "Choose logo…" }));

    await waitFor(() =>
      expect(screen.getByText("Choose a PNG or JPEG image for the logo")).toBeTruthy(),
    );
    expect(screen.getByText("No logo yet.")).toBeTruthy();
  });

  it("says nothing when the logo picker is cancelled", async () => {
    call.mockImplementation(async (cmd: string) => {
      if (cmd === "settings_get" || cmd === "settings_pick_logo") return saved;
      throw new Error(`unexpected ${cmd}`);
    });
    renderAs();
    await waitFor(() => expect(screen.getByText("No logo yet.")).toBeTruthy());

    fireEvent.click(screen.getByRole("button", { name: "Choose logo…" }));

    await waitFor(() => expect(call).toHaveBeenCalledWith("settings_pick_logo", undefined));
    await waitFor(() =>
      expect(
        (screen.getByRole("button", { name: "Choose logo…" }) as HTMLButtonElement).disabled,
      ).toBe(false),
    );
    expect(screen.queryByRole("status")).toBeNull();
  });
});
