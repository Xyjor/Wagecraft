import { act, cleanup, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { useTheme } from "./useTheme";
import { THEME_KEY } from "./theme";

function Probe({ id }: { id: string }) {
  const [theme, setTheme] = useTheme();
  return (
    <button type="button" data-id={id} onClick={() => setTheme("DARK")}>
      {`${id}:${theme}`}
    </button>
  );
}

describe("useTheme", () => {
  beforeEach(() => {
    localStorage.clear();
    document.documentElement.classList.remove("dark");
    vi.stubGlobal("matchMedia", (query: string) => ({
      matches: false,
      media: query,
      addEventListener: () => {},
      removeEventListener: () => {},
    }));
  });
  afterEach(() => {
    cleanup();
    vi.unstubAllGlobals();
  });

  it("keeps every user of the hook in step", () => {
    render(
      <>
        <Probe id="a" />
        <Probe id="b" />
      </>,
    );

    act(() => screen.getByText("a:SYSTEM").click());

    expect(screen.getByText("a:DARK")).toBeTruthy();
    expect(screen.getByText("b:DARK")).toBeTruthy();
    expect(localStorage.getItem(THEME_KEY)).toBe("DARK");
    expect(document.documentElement.classList.contains("dark")).toBe(true);
  });
});
