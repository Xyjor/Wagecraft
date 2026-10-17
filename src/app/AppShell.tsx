import { Outlet } from "react-router";
import { Sidebar } from "@/components/layout/Sidebar";
import { TopBar } from "@/components/layout/TopBar";

export function AppShell() {
  return (
    <div className="flex h-screen bg-canvas text-ink">
      {/* Keyboard users skip the twenty-odd menu links; it shows only when focused. */}
      <a
        href="#content"
        className="sr-only z-50 rounded-md bg-accent px-3 py-2 text-sm font-medium text-accent-ink focus:not-sr-only focus:fixed focus:top-2 focus:left-2"
      >
        Skip to content
      </a>
      <Sidebar />
      <div className="flex min-w-0 flex-1 flex-col">
        <TopBar />
        <main id="content" tabIndex={-1} className="flex-1 overflow-auto p-6 outline-none">
          <Outlet />
        </main>
      </div>
    </div>
  );
}
