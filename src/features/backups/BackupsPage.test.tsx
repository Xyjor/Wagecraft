import { cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { BackupEntry } from "@/bindings/BackupEntry";
import type { Me } from "@/bindings/Me";
import { SessionContext } from "@/features/auth/session";

const call = vi.fn();
vi.mock("@/lib/ipc", async (orig) => ({
  ...(await orig<typeof import("@/lib/ipc")>()),
  call: (cmd: string, args?: Record<string, unknown>) => call(cmd, args),
}));

import { BackupsPage } from "./BackupsPage";

const admin: Me = {
  userId: 1,
  username: "ana",
  role: "ADMIN",
  employeeId: null,
  mustChangePassword: false,
  theme: "SYSTEM",
};

function backup(id: number, extra: Partial<BackupEntry> = {}): BackupEntry {
  return {
    id,
    path: `C:\\Wagecraft\\backups\\wagecraft-backup-${id}.db`,
    sizeBytes: 1258291,
    kind: "AUTO",
    createdByName: null,
    createdAt: "2026-11-02T00:30:00Z",
    onDisk: true,
    ...extra,
  };
}

let list: BackupEntry[];

function renderAs(who: Me = admin) {
  render(
    <SessionContext.Provider value={{ me: who, signOut: async () => {} }}>
      <BackupsPage />
    </SessionContext.Provider>,
  );
}

beforeEach(() => {
  call.mockReset();
  list = [
    backup(2, { kind: "MANUAL", createdByName: "ana", path: "E:\\wagecraft-backup-2.db" }),
    backup(1, { onDisk: false }),
  ];
  call.mockImplementation(async (cmd: string) => {
    if (cmd === "backup_list") return list;
    throw new Error(`unexpected ${cmd}`);
  });
});

afterEach(cleanup);

describe("BackupsPage", () => {
  it("is only for Admins", () => {
    renderAs({ ...admin, role: "HR" });

    expect(screen.getByText("Only Admins can manage backups.")).toBeTruthy();
    expect(call).not.toHaveBeenCalled();
  });

  it("lists each backup with its kind, size, who made it and where it is", async () => {
    renderAs();

    const manual = await screen.findByRole("row", { name: /Manual/ });
    expect(within(manual).getByText("1.2 MB")).toBeTruthy();
    expect(within(manual).getByText("ana")).toBeTruthy();
    expect(within(manual).getByText("E:\\wagecraft-backup-2.db")).toBeTruthy();
    const daily = screen.getByRole("row", { name: /Daily/ });
    expect(within(daily).getByText("Automatic")).toBeTruthy();
  });

  it("flags a backup whose file is gone", async () => {
    renderAs();

    const daily = await screen.findByRole("row", { name: /Daily/ });
    expect(within(daily).getByText("File missing")).toBeTruthy();
    const manual = screen.getByRole("row", { name: /Manual/ });
    expect(within(manual).queryByText("File missing")).toBeNull();
  });

  it("says so when there are no backups yet", async () => {
    list = [];
    renderAs();

    expect(await screen.findByText("No backups yet.")).toBeTruthy();
    expect(screen.queryByRole("table")).toBeNull();
  });

  it("backs up now, says where the file went and refreshes the list", async () => {
    call.mockImplementation(async (cmd: string) => {
      if (cmd === "backup_list") return list;
      if (cmd === "backup_create") {
        list = [backup(3, { kind: "MANUAL", createdByName: "ana" }), ...list];
        return "E:\\wagecraft-backup-3.db";
      }
      throw new Error(`unexpected ${cmd}`);
    });
    renderAs();
    await screen.findByRole("row", { name: /Daily/ });

    fireEvent.click(screen.getByRole("button", { name: "Back up now" }));

    expect((await screen.findByRole("status")).textContent).toContain(
      "Backup saved to E:\\wagecraft-backup-3.db",
    );
    await waitFor(() => expect(screen.getAllByRole("row", { name: /Manual/ })).toHaveLength(2));
  });

  it("does nothing when Admin cancels the folder picker", async () => {
    call.mockImplementation(async (cmd: string) => (cmd === "backup_list" ? list : null));
    renderAs();
    await screen.findByRole("row", { name: /Daily/ });

    fireEvent.click(screen.getByRole("button", { name: "Back up now" }));

    await waitFor(() => expect(call).toHaveBeenCalledWith("backup_create", undefined));
    expect(screen.queryByRole("status")).toBeNull();
    expect(screen.queryByRole("alert")).toBeNull();
  });

  it("shows why a backup failed", async () => {
    call.mockImplementation(async (cmd: string) => {
      if (cmd === "backup_list") return list;
      throw { code: "INTERNAL", message: "The disk is full", fields: [] };
    });
    renderAs();
    await screen.findByRole("row", { name: /Daily/ });

    fireEvent.click(screen.getByRole("button", { name: "Back up now" }));

    expect((await screen.findByRole("alert")).textContent).toContain("The disk is full");
  });
});
