import { Monitor, Moon, Sun } from "lucide-react";
import { useTheme } from "@/lib/useTheme";
import type { Theme } from "@/lib/theme";

const OPTIONS = [
  { value: "LIGHT", label: "Light", icon: Sun },
  { value: "DARK", label: "Dark", icon: Moon },
  { value: "SYSTEM", label: "System", icon: Monitor },
] as const satisfies readonly { value: Theme; label: string; icon: unknown }[];

export function ThemeMenu() {
  const [theme, setTheme] = useTheme();

  return (
    <fieldset className="grid grid-cols-3 gap-0.5 rounded-md border border-rule bg-sunken p-0.5 text-sm">
      <legend className="sr-only">Theme</legend>
      {OPTIONS.map((o) => (
        <label
          key={o.value}
          className="flex cursor-pointer items-center justify-center gap-1.5 rounded px-2 py-1 text-ink-muted has-checked:bg-surface has-checked:font-medium has-checked:text-ink has-checked:shadow-[0_1px_2px_rgb(0_0_0/0.12)] has-focus-visible:ring-2 has-focus-visible:ring-focus"
        >
          <input
            type="radio"
            name="theme"
            value={o.value}
            checked={theme === o.value}
            onChange={() => setTheme(o.value)}
            className="sr-only"
          />
          <o.icon aria-hidden="true" className="size-3.5" strokeWidth={1.75} />
          {o.label}
        </label>
      ))}
    </fieldset>
  );
}
