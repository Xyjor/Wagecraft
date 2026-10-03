import { useTheme } from "@/lib/useTheme";
import type { Theme } from "@/lib/theme";

const OPTIONS: { value: Theme; label: string }[] = [
  { value: "LIGHT", label: "Light" },
  { value: "DARK", label: "Dark" },
  { value: "SYSTEM", label: "System" },
];

export function ThemeMenu() {
  const [theme, setTheme] = useTheme();

  return (
    <fieldset className="flex rounded-md border border-zinc-300 p-0.5 text-sm dark:border-zinc-700">
      <legend className="sr-only">Theme</legend>
      {OPTIONS.map((o) => (
        <label
          key={o.value}
          className="cursor-pointer rounded px-2.5 py-1 text-zinc-600 has-checked:bg-zinc-900 has-checked:text-white has-focus-visible:ring-2 has-focus-visible:ring-sky-500 dark:text-zinc-300 dark:has-checked:bg-zinc-100 dark:has-checked:text-zinc-900"
        >
          <input
            type="radio"
            name="theme"
            value={o.value}
            checked={theme === o.value}
            onChange={() => setTheme(o.value)}
            className="sr-only"
          />
          {o.label}
        </label>
      ))}
    </fieldset>
  );
}
