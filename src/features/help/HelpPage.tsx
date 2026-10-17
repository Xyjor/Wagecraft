import { Children, isValidElement, useEffect, useMemo, type ReactNode } from "react";
import { Link, useLocation } from "react-router";
import Markdown, { type Components } from "react-markdown";
import remarkGfm from "remark-gfm";
import { PageHeader } from "@/components/PageHeader";
import { useSession } from "@/features/auth/session";
import { manualFor, SHOTS, slugify } from "./manual";

/** The plain text of a heading's children, for its anchor. */
function textOf(node: ReactNode): string {
  return Children.toArray(node)
    .map((child) => {
      if (typeof child === "string" || typeof child === "number") return String(child);
      if (isValidElement<{ children?: ReactNode }>(child)) return textOf(child.props.children);
      return "";
    })
    .join("");
}

function heading(Tag: "h2" | "h3" | "h4") {
  return function Heading({ children }: { children?: ReactNode }) {
    return (
      <Tag id={slugify(textOf(children))} className="scroll-mt-4">
        {children}
      </Tag>
    );
  };
}

const components: Components = {
  h2: heading("h2"),
  h3: heading("h3"),
  h4: heading("h4"),
  img: ({ src, alt }) => {
    const name = typeof src === "string" ? src.slice(src.lastIndexOf("/") + 1) : "";
    const url = SHOTS[name];
    if (!url) return null;
    return <img src={url} alt={alt ?? ""} loading="lazy" />;
  },
  // Links inside the manual stay inside the app: anchors scroll, app paths route, and anything
  // else is shown as text because the app works offline.
  a: ({ href, children }) => {
    if (href?.startsWith("#")) return <a href={href}>{children}</a>;
    if (href?.startsWith("/")) return <Link to={href}>{children}</Link>;
    return <span>{children}</span>;
  },
  table: ({ children }) => (
    <div className="overflow-x-auto">
      <table>{children}</table>
    </div>
  ),
};

/** The user manual, bundled with the app and limited to the reader's role. */
export function HelpPage() {
  const { me } = useSession();
  const { hash } = useLocation();
  const manual = useMemo(() => manualFor(me.role), [me.role]);

  const contents = useMemo(
    () =>
      manual.sections.map((s) => ({
        heading: s.heading,
        slug: s.slug,
        subs: [...s.body.matchAll(/^### (.+)$/gm)].map((m) => ({
          heading: m[1].trim(),
          slug: slugify(m[1]),
        })),
      })),
    [manual],
  );

  useEffect(() => {
    if (!hash) return;
    document.getElementById(decodeURIComponent(hash.slice(1)))?.scrollIntoView?.();
  }, [hash]);

  return (
    <div>
      <PageHeader
        title="Help"
        description="The Wagecraft user manual, showing the parts for your role. The ? at the top of each screen opens the part about that screen."
      />
      <div className="flex items-start gap-10">
        <article className="manual min-w-0 max-w-3xl flex-1">
          <Markdown remarkPlugins={[remarkGfm]} components={components}>
            {manual.intro}
          </Markdown>
          {manual.sections.map((s) => (
            <Markdown key={s.slug} remarkPlugins={[remarkGfm]} components={components}>
              {s.body}
            </Markdown>
          ))}
        </article>
        <nav aria-label="Contents" className="sticky top-0 hidden w-56 shrink-0 text-sm lg:block">
          <p className="mb-2 font-semibold">Contents</p>
          <ol className="space-y-2 border-l border-rule">
            {contents.map((c) => (
              <li key={c.slug}>
                <a
                  href={`#${c.slug}`}
                  className="-ml-px block border-l border-transparent pl-3 font-medium text-ink hover:border-accent hover:text-accent"
                >
                  {c.heading}
                </a>
                {c.subs.length > 0 && (
                  <ol className="mt-1 space-y-0.5">
                    {c.subs.map((sub) => (
                      <li key={sub.slug}>
                        <a
                          href={`#${sub.slug}`}
                          className="-ml-px block border-l border-transparent py-0.5 pl-5 text-ink-muted hover:border-accent hover:text-accent"
                        >
                          {sub.heading}
                        </a>
                      </li>
                    ))}
                  </ol>
                )}
              </li>
            ))}
          </ol>
        </nav>
      </div>
    </div>
  );
}
