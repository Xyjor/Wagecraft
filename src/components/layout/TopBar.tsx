import { ThemeMenu } from "./ThemeMenu";

export function TopBar() {
  return (
    <header className="flex h-14 items-center justify-end border-b border-zinc-200 px-4 dark:border-zinc-800">
      <ThemeMenu />
    </header>
  );
}
