# PaperForge Frontend Design & Architecture Strategy

## 1. Design Philosophy: "Infinite Canvas of Knowledge"

**Core Theme:** _Invisible Precision._
The interface should feel like a sophisticated tool for thought—similar to Figma or Linear. It should recede into the background, allowing the _research content_ to take center stage. Avoid "gen-AI" tropes (sparkles, neon gradients, dark-purple voids). Instead, lean into **Swiss Design principles**: strong grid systems, high readability, and information density tailored for experts.

### 1.1 Visual Identity (Modern SF / "Linear-esque")

Taking inspiration from **Linear**, **Vercel**, and cutting-edge **2026 Developer Tools**. The goal is a "high-density, high-performance" aesthetic that feels alive but disciplined.

#### **Color Palette: "Nebula & Void"**

We move beyond flat "slate" to a rich, layered dark mode and a crisp, technical light mode.

- **Canvas (Backgrounds):**
  - `bg-white` (Light Mode) - Crisp, technical.
  - `bg-[#0a0a0c]` (Dark Mode - _"Obsidian"_) - Deep, blue-tinted black. Not #000.
  - **Texture:** Uses subtle grid patterns and radial gradients (`bg-grid-pattern`) to give depth to the void.
- **Primary Action (Brand):**
  - **Electric Indigo** (`#6366f1` / `oklch(0.6 0.2 270)`) - High energy, digital-native.
  - Used for primary buttons, focus rings, and active states.
- **Surface & Depth:**
  - **Glassmorphism:** Extensive use of `backdrop-blur-xl` and translucent borders (`border-white/10`).
  - **Layers:** Background -> Sidebar (slightly lighter/translucent) -> Cards (Glass/Bordered).
- **Text & Readability:**
  - `text-foreground` (White/Black) - High contrast for headings.
  - `text-muted-foreground` (Slate-400/500) - For metadata.
- **Borders:**
  - Extremely subtle (`border-white/5` or `border-black/5`) but present to define structure without visual weight.

#### **Typography**

- **Font Family:** `Geist Sans` / `Inter` (Variable).
- **Characteristics:** Tight tracking for headings (`tracking-tighter`), tabular nums for data.
- **Scale:** Large, confident headings (4xl/6xl) for marketing/empty states; dense, small (13px) for data tables.

### 1.2 UX Core Values

1.  **Non-Blocking flows:** Uploading a paper happens in the background (Toast notification). You continue reading/searching.
2.  **Context Preservation:** When clicking a search result, it opens in a "Split View" or "Side Panel" rather than navigating away, preserving the search query (similar to VS Code).
3.  **Keyboard First:** `Cmd+K` for global command palette. `/` to focus search.

---

## 2. Frontend Architecture (Microsoft/Enterprise Pattern)

We will use a **Feature-Based Architecture** (similar to Domain Driven Design). This separates code by _business domain_ rather than technical type (not just `hooks/` and `components/`). This scales better for enterprise apps.

### 2.1 Directory Structure

```text
frontend/
├── app/                  # Next.js App Router (Routing layer only)
│   ├── (dashboard)/      # Layout route for logged-in app
│   └── api/              # Proxy/BFF layer (if needed)
├── src/
│   ├── components/       # SHARED atomic UI components (Buttons, Inputs)
│   │   ├── ui/           # shadcn/ui primitives
│   │   └── layout/       # AppShell, Sidebar, Header
│   ├── features/         # DOMAIN-SPECIFIC logic & UI
│   │   ├── search/       # SearchBar, ResultsList, Filters
│   │   ├── library/      # DocumentList, UploadButton, DragDropZone
│   │   ├── reader/       # PDFViewer, AnnotationLayer
│   │   └── chat/         # CopilotChat, MessageBubble (Context Service)
│   ├── lib/              # Core utilities
│   │   ├── api.ts        # Typed Fetch Wrapper (Axios/Ky)
│   │   └── query.ts      # React Query Configuration
│   ├── hooks/            # Global hooks (useKeyboardShortcut, etc.)
│   └── types/            # Global Types (DTOs matching Rust backend)
```

### 2.2 Tech Stack Strategy

- **Framework:** Next.js 16 (App Router)
- **Runtime:** Bun
- **Styling:** Tailwind CSS + `clsx` + `cva` (Class Variance Authority for component variants).
- **State Management:**
  - **Server State:** TanStack Query (React Query). _Critical for syncing with async ingestion and vector search._
  - **UI State:** Nuqs (URL-based state management) for search params (shareable URLs).
  - **Global Client State:** Zustand (for simple things like "Sidebar Open/Close" or "PDF Zoom Level").

---

## 3. UI Component Plan

### 3.1 The "App Shell" (Layout)

- **Sidebar:** Collapsible, thin rail (like VS Code / Linear). Icons for: "Library", "Search", "Assistant".
- **Top Bar:** Minimal. Breadcrumbs. Connection Status (WebSocket/Poll status for ingestion).

### 3.2 Feature: Search Experience (`features/search`)

- **Input:** Large, centralized input (like Perplexity/Google). Focus on "Intent".
- **Toggles:** "Hybrid Search" vs "Keyword Only" (Pill switch).
- **Results View:**
  - **Card Design:** Minimalist cards.
    - Title (Bold, Slate-900).
    - Abstract (Truncated, Slate-600).
    - _Badges:_ "PDF Available", "Score: 0.98", "Year: 2024".
  - **Citations:** When hybrid search is used, highlight _why_ it matched (snippet match).

### 3.3 Feature: Document Library & Ingestion (`features/library`)

- **Data Table:** High-density table (TanStack Table). Sortable columns (Date, Size, Status).
- **Ingestion UX:**
  - **Drag & Drop Zone:** Overlay on the whole screen when dragging a file.
  - **Status Pills:** "Queued" (Gray) -> "Processing" (Blue Pulse) -> "Embedded" (Green).
  - _Implementation Note:_ The Ingestion Service puts tasks in SQS. Frontend must poll an endpoint or receive SSE events to update the UI from "Processing" to "Done".

### 3.4 Feature: PDF Reader & Context (`features/reader`)

- **Split Pane Layout:**
  - **Left (60%):** PDF Viewer (custom canvas or `react-pdf`). Clean, reading focus.
  - **Right (40%):** "Context Copilot".
- **Copilot:** Chat interface hooked to the **Context Service**.
  - "Summarize this abstract"
  - "Find related papers to this paragraph" (Triggers Search Service).

---

## 4. API Integration Strategy (The "BFF" Pattern)

Since we have multiple microservices (`gateway`, `search`, `ingestion`), the Frontend should ideally only talk to the **Gateway**.

- **API Client (`src/lib/api.ts`):** A strictly typed client.
  - `GET /search?q=...` -> Calls **Search Service**.
  - `POST /ingest` -> Calls **Ingestion Service**.
  - `GET /status/{id}` -> Checks ingestion status.

### 4.1 Data Fetching Rules

1.  **Stale-While-Revalidate:** Search results are cached aggressively.
2.  **Optimistic Updates:** When uploading, show the file immediately in the list with a "Pending" opacity state before the server confirms.

---

## 5. Implementation Roadmap (Phased)

1.  **Phase 1: Foundation (The Shell)**
    - Setup Shadcn/UI (Button, Table, Dialog, Sheet, Input).
    - Configure Tailwind Theme (Typography, Colors).
    - Create the App Shell (Sidebar + Main Layout).
2.  **Phase 2: The Library (Ingestion)**
    - Implement "Upload Paper" modal.
    - Connect to `POST /ingest` endpoint.
    - Build the Data Table for "My Papers".
3.  **Phase 3: The Brain (Search & RAG)**
    - Implement Hybrid Search UI.
    - Display Vector Search results.
    - (Future) Chat implementation.
