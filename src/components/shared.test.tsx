import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { MemoryRouter } from "react-router";
import { afterEach, describe, expect, it, vi } from "vitest";
import { Money } from "./Money";
import { PageHeader } from "./PageHeader";
import { EmptyState, ErrorState, Loading } from "./states";

afterEach(cleanup);

describe("PageHeader", () => {
  it("puts only the title in the page heading", () => {
    render(<PageHeader title="Payroll" description="Each period is one cutoff." />);

    const h1 = screen.getByRole("heading", { level: 1 });
    expect(h1.textContent).toBe("Payroll");
    expect(screen.getByText("Each period is one cutoff.")).toBeTruthy();
  });

  it("links each breadcrumb", () => {
    render(
      <MemoryRouter>
        <PageHeader title="Ana Cruz" crumbs={[{ label: "Employees", to: "/employees" }]} />
      </MemoryRouter>,
    );

    const nav = screen.getByRole("navigation", { name: "Breadcrumb" });
    expect(nav.querySelector("a")?.getAttribute("href")).toBe("/employees");
  });

  it("works outside a router when there are no breadcrumbs", () => {
    render(<PageHeader title="Settings" actions={<button type="button">Save</button>} />);

    expect(screen.getByRole("button", { name: "Save" })).toBeTruthy();
    expect(screen.queryByRole("navigation")).toBeNull();
  });
});

describe("states", () => {
  it("Loading is busy but doesn't announce itself as a status", () => {
    const { container } = render(<Loading label="Loading employees" />);

    expect(screen.queryByRole("status")).toBeNull();
    expect(container.querySelector("[aria-busy='true']")).toBeTruthy();
    expect(screen.getByText("Loading employees")).toBeTruthy();
  });

  it("EmptyState shows its message as one paragraph", () => {
    render(<EmptyState message="No payroll posted yet." />);

    expect(screen.getByText("No payroll posted yet.").tagName).toBe("P");
  });

  it("ErrorState keeps Retry outside the alert", () => {
    const retry = vi.fn();
    render(<ErrorState message="Couldn't load the dashboard." onRetry={retry} />);

    expect(screen.getByRole("alert").textContent).toBe("Couldn't load the dashboard.");
    fireEvent.click(screen.getByRole("button", { name: "Retry" }));
    expect(retry).toHaveBeenCalledTimes(1);
  });
});

describe("Money", () => {
  it("shows positive amounts plainly", () => {
    render(<Money cents={2_500_000} />);

    expect(screen.getByText("₱25,000.00")).toBeTruthy();
  });

  it("shows negative amounts in parentheses and says minus to screen readers", () => {
    const { container } = render(<Money cents={-114_943} />);

    expect(screen.getByText("(₱1,149.43)")).toBeTruthy();
    expect(container.textContent).toBe("minus (₱1,149.43)");
  });
});
