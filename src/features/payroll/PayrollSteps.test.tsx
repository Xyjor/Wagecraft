import { cleanup, render, screen, within } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import type { PayrollPeriod } from "@/bindings/PayrollPeriod";
import { PayrollSteps } from "./PayrollSteps";

afterEach(cleanup);

function steps(status: PayrollPeriod["status"]) {
  render(<PayrollSteps status={status} />);
  const list = screen.getByRole("list", { name: "Payroll steps" });
  const items = within(list).getAllByRole("listitem");
  return {
    labels: items.map((li) => li.querySelector("[data-label]")?.textContent),
    current: items.find((li) => li.getAttribute("aria-current") === "step")?.textContent ?? "",
    done: items.filter((li) => li.textContent?.includes("done")).length,
  };
}

describe("PayrollSteps", () => {
  it("always lists the four steps in order", () => {
    expect(steps("DRAFT").labels).toEqual(["Draft", "Computed", "Approved", "Posted"]);
  });

  it.each([
    ["DRAFT", "Draft", 0, "Compute the payroll to see everyone's pay."],
    [
      "COMPUTED",
      "Computed",
      1,
      "Fix attendance, leave or pay rates at the source, then recompute.",
    ],
    ["APPROVED", "Approved", 2, "Post it to make the payslips final"],
    ["POSTED", "Posted", 4, "Posted. These payslips are final"],
  ] as const)("at %s, marks the current step and says what's next", (status, label, done, next) => {
    const s = steps(status);
    expect(s.current).toContain(label);
    expect(s.done).toBe(done);
    expect(screen.getByText(new RegExp(next.replace(/[.?]/g, "\\$&")))).toBeTruthy();
  });

  it("marks every step done once posted", () => {
    render(<PayrollSteps status="POSTED" />);
    const posted = screen.getByText("Posted", { selector: "[data-label]" }).closest("li")!;
    expect(posted.getAttribute("aria-current")).toBe("step");
    expect(posted.textContent).toContain("done");
  });
});
