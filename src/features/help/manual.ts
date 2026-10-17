import type { Role } from "@/bindings/Role";
import source from "../../../docs/user-manual.md?raw";

// The user manual in docs/ is the single source for the Help page: the app bundles the same file
// GitHub shows, with its screenshots, so the two never disagree. manual.test.ts fails when a
// heading the app links to, a section's audience or a screenshot goes missing.

export type ManualSection = { heading: string; slug: string; body: string };

/** Lower case, punctuation dropped, spaces to hyphens: the anchors GitHub gives headings. */
export function slugify(text: string): string {
  return text
    .trim()
    .toLowerCase()
    .replace(/[^\p{L}\p{N}\s-]/gu, "")
    .replace(/\s/g, "-");
}

/** The text before the first `## `, and each `## ` section with everything under it. */
export function splitSections(md: string): { intro: string; sections: ManualSection[] } {
  const parts = md.split(/^(?=## )/m);
  const intro = parts[0].startsWith("## ") ? "" : parts.shift()!;
  const sections = parts.map((body) => {
    const heading = body.slice(3, body.indexOf("\n")).trim();
    return { heading, slug: slugify(heading), body };
  });
  return { intro, sections };
}

/** Who each `## ` section is for. Sections not about a role's screens are for everyone. */
export const SECTION_ROLES: Record<string, Role[] | "everyone"> = {
  "1. Getting started": "everyone",
  "2. The time clock": "everyone",
  "3. For staff": "everyone",
  "4. For HR and Admin": ["ADMIN", "HR"],
  "5. For Admin": ["ADMIN"],
  "6. Where the data lives": ["ADMIN", "HR"],
  "7. Limits of this version": "everyone",
};

/** The manual without its `# ` title; the Help page has its own. */
export const MANUAL = { source, ...splitSections(source.replace(/^# .*\n/, "")) };

/** The intro and the sections this role can use. */
export function manualFor(role: Role) {
  return {
    intro: MANUAL.intro,
    sections: MANUAL.sections.filter((s) => {
      const who = SECTION_ROLES[s.heading] ?? "everyone";
      return who === "everyone" || who.includes(role);
    }),
  };
}

/** The screenshots, bundled with the app and keyed by file name. */
export const SHOTS: Record<string, string> = Object.fromEntries(
  Object.entries(
    import.meta.glob<string>("../../../docs/screenshots/*.png", {
      eager: true,
      query: "?url",
      import: "default",
    }),
  ).map(([path, url]) => [path.slice(path.lastIndexOf("/") + 1), url]),
);
