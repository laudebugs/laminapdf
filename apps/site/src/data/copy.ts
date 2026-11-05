// The home page's words, in one place so the owner can approve them together (W1c). Each entry
// marked [OWNER TO APPROVE] waits for the owner's approval in the pull request; numbers and lists
// come from progress.json and the repository's files, never from here.

/** The page's title and description, for the browser tab, search results and link previews. */
export const META = {
  // [OWNER TO APPROVE]
  title: "Lamina PDF: create, read and edit PDFs from Rust and TypeScript",
  // [OWNER TO APPROVE]
  description:
    "A PDF library with one Rust core, for Rust and TypeScript today; a web reader and a desktop app come next. See the roadmap and how far the work has come.",
} as const;

/** The opening section. */
export const OPENING = {
  // [OWNER TO APPROVE]
  headline: "Create, read and edit PDFs with one Rust core, from Rust and TypeScript.",
  // [OWNER TO APPROVE]
  lead: "Lamina PDF is a PDF library for Rust and TypeScript today, and a web reader and a desktop app next. This page shows the plan and how far the work has come.",
  docs: "Read the docs",
  packages: "Install a package",
  app: "Open the web app",
  // [OWNER TO APPROVE]
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
  // [OWNER TO APPROVE]
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
  // [OWNER TO APPROVE] The install page's note on alpha releases follows it (sources.ts, alphaNote).
  intro: "Two packages cover most projects.",
  typescript: "TypeScript, in every JavaScript runtime",
  rust: "Rust",
  // [OWNER TO APPROVE]
  cli: "The `laminapdf` command-line tool inspects, checks and changes PDFs, and serves the same commands to AI agents over MCP. It isn't released yet.",
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
