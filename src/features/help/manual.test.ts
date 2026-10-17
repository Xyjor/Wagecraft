import { describe, expect, it } from "vitest";
import type { Role } from "@/bindings/Role";
import { HELP_ROUTES, helpHref } from "./helpTopics";
import { MANUAL, manualFor, SECTION_ROLES, SHOTS, slugify, splitSections } from "./manual";

const ROLES: Role[] = ["ADMIN", "HR", "STAFF"];

describe("slugify", () => {
  it.each([
    ["1. Getting started", "1-getting-started"],
    ["4.8 Payroll", "48-payroll"],
    ["Light and dark", "light-and-dark"],
    ["What each role sees", "what-each-role-sees"],
  ])("%s → #%s, like GitHub", (text, slug) => {
    expect(slugify(text)).toBe(slug);
  });
});

describe("splitSections", () => {
  it("splits at each ## heading and keeps the text before the first one as the intro", () => {
    const { intro, sections } = splitSections(
      "Intro.\n\n## One\n\nA\n\n### Sub\n\nB\n\n## Two\n\nC\n",
    );
    expect(intro.trim()).toBe("Intro.");
    expect(sections.map((s) => s.heading)).toEqual(["One", "Two"]);
    expect(sections[0].body).toContain("### Sub");
  });
});

describe("the bundled manual", () => {
  it("drops the document title, since the page has its own", () => {
    expect(MANUAL.intro).not.toMatch(/^# /m);
  });

  it("says who sees every ## section", () => {
    for (const s of MANUAL.sections) expect(SECTION_ROLES, s.heading).toHaveProperty(s.heading);
  });

  it("has every screenshot it shows", () => {
    const refs = [...MANUAL.source.matchAll(/\]\(screenshots\/([^)]+)\)/g)].map((m) => m[1]);
    expect(refs.length).toBeGreaterThan(10);
    for (const name of refs) expect(SHOTS, name).toHaveProperty(name);
  });

  it("shows Staff no HR or Admin sections", () => {
    const headings = manualFor("STAFF").sections.map((s) => s.heading);
    expect(headings).toContain("3. For staff");
    expect(headings).not.toContain("4. For HR and Admin");
    expect(headings).not.toContain("5. For Admin");
  });

  it("shows Admin everything", () => {
    expect(manualFor("ADMIN").sections).toHaveLength(MANUAL.sections.length);
  });
});

describe("help for each page", () => {
  it("points every route at a heading the reader can see", () => {
    for (const role of ROLES) {
      const visible = manualFor(role).sections.flatMap((s) =>
        [...s.body.matchAll(/^#{2,3} (.+)$/gm)].map((m) => slugify(m[1])),
      );
      for (const route of HELP_ROUTES) {
        if (route.roles && !route.roles.includes(role)) continue;
        const heading = typeof route.heading === "function" ? route.heading(role) : route.heading;
        expect(visible, `${route.pattern} for ${role}`).toContain(slugify(heading));
      }
    }
  });

  it.each([
    ["/", "STAFF", "/help#home"],
    ["/", "HR", "/help#41-home"],
    ["/payroll/4", "HR", "/help#48-payroll"],
    ["/employees/12/edit", "ADMIN", "/help#44-employees"],
    ["/settings", "ADMIN", "/help#53-settings"],
    ["/my-payslips", "STAFF", "/help#my-payslips"],
    ["/help", "STAFF", "/help"],
  ] as const)("%s as %s opens %s", (path, role, href) => {
    expect(helpHref(path, role)).toBe(href);
  });
});
