# PaperForge Frontend

This project implements the [Frontend Design Strategy](../docs/FRONTEND_DESIGN.md) using Next.js 16 (App Router), Bun, and Tailwind CSS.

## Architecture: Feature-Sliced Design (Modified)

We use a domain-driven structure under `src/` to separate concerns:

```text
frontend/
├── src/
│   ├── app/                  # Routing Layer (Next.js App Router)
│   │   ├── (dashboard)/      # Authenticated Layout Group
│   │   │   ├── library/      # /library Page
│   │   │   ├── search/       # /search Page
│   │   │   └── layout.tsx    # Dashboard Internal Layout (Sidebar)
│   │   ├── layout.tsx        # Root Layout (Providers, Fonts)
│   │   └── globals.css       # Global Styles & Theme
│   ├── components/
│   │   ├── ui/               # Shared UI Primitives (Shadcn/ui)
│   │   └── layout/           # Shared Layout Components (Sidebar, Header)
│   ├── features/             # Domain Logic
│   │   ├── library/          # Library specific components, types, hooks
│   │   └── reader/           # PDF Reader specific logic
│   ├── lib/                  # Utilities (API, Utils)
│   ├── hooks/                # Global Hooks
│   └── types/                # Global Types
```

## Developing

1.  **Install dependencies**:

    ```bash
    bun install
    ```

2.  **Run development server**:

    ```bash
    bun dev
    ```

3.  **Add new UI component**:
    Use `shadcn` via `bunx`:
    ```bash
    bunx --bun shadcn@latest add <component-name>
    ```
    Ensure `components.json` is configured to output to `src/components/ui`.

## Theme

The theme is defined in `src/app/globals.css` using `oklch` color spaces for dynamic theming (Light/Dark mode). The core palette is "Slate & Trust" (Zinc + Royal Blue).
