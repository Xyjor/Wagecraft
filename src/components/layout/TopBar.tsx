import { UserMenu } from "./UserMenu";

export function TopBar() {
  return (
    <header className="flex h-14 shrink-0 items-center justify-end gap-2 border-b border-rule bg-surface px-4">
      <UserMenu />
    </header>
  );
}
