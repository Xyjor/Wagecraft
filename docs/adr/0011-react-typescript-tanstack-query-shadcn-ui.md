# ADR-011: React + TypeScript + TanStack Query + shadcn/ui

- **Status:** Accepted (owner decision, §16 Q5); the library list is superseded by [ADR-012](0012-plain-react-data-loading-and-forms-tailwind-without-shadcn.md)
- **Context:** A data-heavy admin UI with forms, tables, charts, and dark mode.
- **Decision:** React 19 + TypeScript (strict), TanStack Query for backend data, React Hook Form + Zod, Tailwind + shadcn/ui, TanStack Table, Recharts.
- **Alternatives:** _Svelte or Vue_: both excellent and lighter, but with fewer ready-made admin components and learning resources.
- **Consequences:** ➕ huge ecosystem, transferable skills. ➖ more dependencies to keep updated.
