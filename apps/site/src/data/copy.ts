// The home page's words, in one place. The owner approved them on 2026-11-05, and a change to a
// claim, the headline or the description goes back to the owner. Numbers and lists come from
// progress.json and the repository's files, never from here.

/** The page's title and description, for the browser tab, search results and link previews. */
export const META = {
  title: "Lamina PDF: create, read and edit PDFs from Rust and TypeScript",
  description:
    "A PDF library with one Rust core, for Rust and TypeScript, and a web reader that opens PDFs in your browser; a desktop app comes next. See the roadmap and how far the work has come.",
} as const;

/** The opening section. */
export const OPENING = {
  headline: "Create, read and edit PDFs with one Rust core, from Rust and TypeScript.",
  lead: "Lamina PDF is a PDF library for Rust and TypeScript, and a web reader that opens PDFs in your browser; a desktop app comes next. This page shows the plan and how far the work has come.",
  docs: "Read the docs",
  packages: "Install a package",
  app: "Open the web app",
  // Shown beside "Open the web app" (pricing status at launch, owner decision 2026-11-10).
  free: "The web reader is free and will stay free. Paid features will come later, on top of it.",
  caption:
    "TypeScript and Rust make the same first PDF: one A4 page with a line of text. The Command line tab shows examples from the tool's README.",
} as const;

/** The milestones section; the summary sentence is built from the counts. */
export const ROADMAP = {
  heading: "The roadmap",
  states: { done: "Done", "under-way": "Under way", "not-started": "Not started" },
} as const;

/** The dashboard. */
export const DASHBOARD = {
  heading: "Where the work stands",
  explainer: "Each todo on the board is one pull request of work.",
  rounds: "By area",
  inProgress: "In progress",
  nextUp: "Next up",
  merged: "Merged most recently",
  // Shown in place of an empty list.
  noneInProgress: "Nothing is in progress right now.",
  noneNext: "Nothing is ready to start: every open todo waits on another.",
  noneMerged: "Nothing has been merged yet.",
} as const;

/** The packages section. */
export const PACKAGES = {
  heading: "The packages",
  // The install page's note on alpha releases follows it (sources.ts, alphaNote).
  intro: "Two packages cover most projects.",
  typescript: "TypeScript, in every JavaScript runtime",
  rust: "Rust",
  cli: "The `laminapdf` command-line tool inspects, checks and changes PDFs, and serves the same commands to AI agents over MCP. It isn't released yet.",
} as const;

/** The privacy promises, each linking to its part of /privacy/ (the words are src/data/privacy.ts's). */
export const PRIVACY_PROMISES = {
  heading: "Private by design",
  intro:
    "The web reader opens your PDFs inside your browser. A test checks each of these promises before every release.",
  more: "Read the privacy page",
} as const;

/** Where each part lives. */
export const PARTS = {
  heading: "Where each part lives",
  docs: "The docs: guides, recipes and the API reference",
  packages: "The packages on npm and crates.io",
  crate: "The Rust crate on crates.io",
  // The milestone and its state follow, from progress.json: "The web reader: milestone M2, under way".
  reader: "The web reader",
  desktop: "The desktop app",
} as const;
