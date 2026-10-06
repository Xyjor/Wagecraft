import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { RestorePreview } from "@/bindings/RestorePreview";

const call = vi.fn();
vi.mock("@/lib/ipc", async (orig) => ({
  ...(await orig<typeof import("@/lib/ipc")>()),
  call: (cmd: string, args?: Record<string, unknown>) => call(cmd, args),
}));

import { RestorePanel } from "./RestorePanel";

const preview: RestorePreview = {
  fileName: "wagecraft-backup-2026-11-01_0800.db",
  savedAt: "2026-11-01T00:00:00Z",
  employeeCount: 42,
  lastPostedStart: "2026-10-16",
  lastPostedEnd: "2026-10-31",
};

beforeEach(() => {
  call.mockReset();
});

afterEach(cleanup);

function choose(result: RestorePreview | null = preview) {
  call.mockImplementation(async (cmd: string) => {
    if (cmd === "backup_restore_choose") return result;
    if (cmd === "backup_restore") return new Promise(() => {}); // the app restarts
    throw new Error(`unexpected ${cmd}`);
  });
  render(<RestorePanel />);
  fireEvent.click(screen.getByRole("button", { name: "Restore a backup…" }));
}

describe("RestorePanel", () => {
  it("shows what the chosen backup holds before anything is replaced", async () => {
    choose();

    const panel = await screen.findByRole("region", { name: "Restore this backup?" });
    expect(panel.textContent).toContain("wagecraft-backup-2026-11-01_0800.db");
    expect(panel.textContent).toContain("42 employees");
    expect(panel.textContent).toContain("Oct 16, 2026 to Oct 31, 2026");
    expect(panel.textContent).toContain("Everything entered after this backup will be lost");
    expect(call).not.toHaveBeenCalledWith("backup_restore", expect.anything());
  });

  it("says when the backup has no posted payroll", async () => {
    choose({ ...preview, lastPostedStart: null, lastPostedEnd: null, employeeCount: 1 });

    const panel = await screen.findByRole("region", { name: "Restore this backup?" });
    expect(panel.textContent).toContain("1 employee,");
    expect(panel.textContent).toContain("No payroll posted yet");
  });

  it("only restores once RESTORE is typed exactly", async () => {
    choose();
    const button = await screen.findByRole("button", { name: "Restore and restart" });
    const box = screen.getByLabelText("Type RESTORE to confirm");

    fireEvent.change(box, { target: { value: "restore" } });
    expect((button as HTMLButtonElement).disabled).toBe(true);

    fireEvent.change(box, { target: { value: "RESTORE" } });
    expect((button as HTMLButtonElement).disabled).toBe(false);
    fireEvent.click(button);

    await waitFor(() =>
      expect(call).toHaveBeenCalledWith("backup_restore", { confirm: "RESTORE" }),
    );
    expect((await screen.findByRole("status")).textContent).toContain("Wagecraft will restart");
  });

  it("shows why a restore was refused and lets Admin try again", async () => {
    call.mockImplementation(async (cmd: string) => {
      if (cmd === "backup_restore_choose") return preview;
      throw {
        code: "CONFLICT",
        message: "This backup is damaged and can't be restored",
        fields: [],
      };
    });
    render(<RestorePanel />);
    fireEvent.click(screen.getByRole("button", { name: "Restore a backup…" }));
    fireEvent.change(await screen.findByLabelText("Type RESTORE to confirm"), {
      target: { value: "RESTORE" },
    });

    fireEvent.click(screen.getByRole("button", { name: "Restore and restart" }));

    expect((await screen.findByRole("alert")).textContent).toContain("damaged");
    expect(screen.queryByRole("status")).toBeNull();
    expect(screen.getByRole("button", { name: "Restore and restart" })).toBeTruthy();
  });

  it("does nothing when Admin cancels the file picker", async () => {
    choose(null);

    await waitFor(() => expect(call).toHaveBeenCalledWith("backup_restore_choose", undefined));
    expect(screen.queryByRole("region", { name: "Restore this backup?" })).toBeNull();
  });

  it("can be called off before restoring", async () => {
    choose();
    await screen.findByRole("region", { name: "Restore this backup?" });

    fireEvent.click(screen.getByRole("button", { name: "Cancel" }));

    expect(screen.queryByRole("region", { name: "Restore this backup?" })).toBeNull();
    expect(call).not.toHaveBeenCalledWith("backup_restore", expect.anything());
  });

  it("shows why a file can't be restored", async () => {
    call.mockImplementation(async () => {
      throw { code: "CONFLICT", message: "This file isn't a Wagecraft backup", fields: [] };
    });
    render(<RestorePanel />);

    fireEvent.click(screen.getByRole("button", { name: "Restore a backup…" }));

    expect((await screen.findByRole("alert")).textContent).toContain(
      "This file isn't a Wagecraft backup",
    );
  });
});
