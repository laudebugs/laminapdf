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
  free: "The web reader is free, and your documents stay on your device.",
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
  // Then a link to llms.txt; the owner approved this line and the footer's on 2026-11-12.
  agents: "Docs for coding agents: how to point one at the docs for your version",
  agentsJoin: ", and ",
  agentsIndex: "every page as Markdown in llms.txt",
  packages: "The packages on npm and crates.io",
  crate: "The Rust crate on crates.io",
  // The milestone and its state follow, from progress.json: "The web reader: milestone M2, under way".
  reader: "The web reader",
  desktop: "The desktop app",
  howWeTest: "How we test it: the latest night's results, the licence and how the code is written",
} as const;

/** The footer's words: the licence line, then where coding agents read the docs. */
export const FOOTER = {
  licence: "Lamina PDF is open source, under",
  licenceLink: "the MIT or Apache-2.0 licence",
  agents: "Coding agents can read every page of the docs as Markdown, listed in",
} as const;

// The How we test page's words. The owner approved them on 2026-11-11, and a change to a sentence
// that makes a claim needs their approval again. The figures come from proof.json and the
// licences from Cargo.toml and deny.toml, never from here.
export const HOW_WE_TEST = {
  title: "How Lamina PDF is tested",
  description:
    "Every night, Lamina PDF reads thousands of difficult PDFs and compares what it finds with other PDF tools. The latest night's results, the licence, and how the code is written.",
  heading: "How we test Lamina PDF",
  lead: "Every night, Lamina PDF reads thousands of difficult PDFs and compares what it finds with other PDF tools. This page shows the latest night's results and what each number counts. It also states the licence and how the code is written.",
  night: {
    // "The latest night ran on 10 November 2026 and tested version 0.2.0-alpha.4, commit 709961c."
    ran: "The latest night ran on",
    passedAll: "Every check passed that night.",
    failed: "These checks failed that night:",
    // A night recorded without its jobs' results.
    noJobs: "That night's record doesn't say which checks passed, so it doesn't count as passed.",
    // "4 of the last 5 nights passed every check, counting from 9 November 2026."
    lastNights: (passed: number, recorded: number) =>
      `${passed} of the last ${recorded} ${recorded === 1 ? "night" : "nights"} passed every check, counting from`,
  },
  files: {
    heading: "Files that break PDF readers",
    intro:
      "The files come from the test suites of four other PDF projects: pdf.js, PDFBox, PDFium and qpdf. Many of their files once broke a reader: they are damaged, unusual or built to attack one. Lamina PDF also reads deliberately damaged copies of its own test files.",
    opened:
      "Opened means Lamina PDF read the file's structure and its list of pages. It doesn't mean every page was drawn correctly. Repaired means it fixed broken structure first, as other readers do. Refused means it stopped with an error; the last column counts the refused files that at least one other tool opened.",
    // Shown when no file made Lamina PDF panic, crash or hang.
    noErrors: "No file made Lamina PDF crash, hang or panic.",
    errors: (n: number) => `${n} ${n === 1 ? "file" : "files"} made Lamina PDF crash, hang or panic.`,
    incomplete: "Lamina PDF's report didn't cover every file that night, so these counts cover only part of the run.",
    columns: ["Files from", "Files", "Opened", "Repaired", "Refused", "Refused, another tool opened it"],
  },
  agreement: {
    heading: "Agreement with other PDF tools",
    intro:
      "For every file, Lamina PDF and four other tools (pdf.js, PDFium, qpdf and pypdf) each report three facts: whether the file opens, how many pages it has, and the titles in its outline. Each percentage counts the files where both tools report the fact and give the same answer.",
    excused:
      "A different answer isn't always Lamina PDF's mistake: tools disagree about damaged files. We examine each kind of difference and list the explained ones in the repository, each with its reason. A difference nobody has explained fails the night.",
    noFailures: "No difference was left unexplained.",
    failures: (n: number) => `${n} ${n === 1 ? "difference was" : "differences were"} left unexplained.`,
    columns: ["Tool", "Opens or not", "Page count", "Outline titles"],
    text: "Lamina PDF also compares each page's text with pdf.js's and pdftotext's, ignoring spaces. A page agrees when Lamina PDF's text matches either tool's.",
    textColumns: ["Files from", "Pages compared", "Pages that agree"],
  },
  fuzzing: {
    heading: "Fuzzing: machine-made inputs that try to crash it",
    intro:
      "A fuzzer feeds the code machine-made inputs, millions a night, and keeps any that make it crash. Each target is one part of Lamina PDF, such as the parser or text extraction.",
    // "11 targets ran for 10 minutes each and tried 16,041,418 inputs."
    night: (targets: number, minutes: number) => `${targets} targets ran for ${minutes} minutes each`,
    noCrash: "None found a crash.",
    crashes: (n: number) => `${n} ${n === 1 ? "target" : "targets"} found a crash.`,
    total: (hours: number) => `Over every recorded night, the fuzzers have run for at least ${hours} hours in all.`,
  },
  saving: {
    heading: "Saved files that other tools read back",
    intro:
      "Lamina PDF edits and saves each of its test files in every way it can, then qpdf, pdf.js and PDFium open every saved file.",
    // "Of 672 cases, 556 were saved. Lamina PDF refused the other 116, as the tests expect: …"
    counts: (cases: number, saved: number, refused: number) =>
      `Of ${cases} cases, ${saved} were saved. Lamina PDF refused the other ${refused}, as the tests expect: it refuses a save that would break a signature or that the file can't take.`,
    passed: "read every saved file without an error.",
    failed: "found an error in a saved file.",
    runtimes: "The same example programs, run in each JavaScript runtime and in Rust, must write the same bytes.",
    runtimesPassed: "Matched byte for byte:",
    runtimesFailed: "Didn't match:",
  },
  standards: {
    heading: "PDF/A and PDF/UA",
    intro:
      "veraPDF, the reference checker for the archive (PDF/A) and accessibility (PDF/UA) standards, checks each example that claims one of them.",
    columns: ["Example", "Standard", "veraPDF's verdict"],
    compliant: "Passes",
    notCompliant: "Fails",
  },
  rendering: {
    heading: "Drawing pages: hayro and PDFium",
    intro:
      "The web reader draws pages with PDFium, the renderer in Chrome. Lamina PDF also has hayro, a renderer written in Rust, which may replace PDFium once it draws pages as well as PDFium does. Each night both draw the first 20 pages of every file, and the comparison counts the pages that look the same or nearly the same.",
    // "92.2% of 5,108 pages look the same or nearly the same."
    share: (pages: string) => `of ${pages} pages look the same or nearly the same.`,
    columns: ["Files from", "Pages", "Same or nearly the same"],
  },
  missing: {
    // "There is no figure for this night: the render-diff job failed."
    lead: "There is no figure for this night:",
    results: {
      failure: "failed",
      cancelled: "was cancelled",
      skipped: "was skipped",
      success: "wrote no report",
    } as Record<string, string>,
    notRun: "didn't run",
    // "The drawing comparison failed in a run that night; this figure is from the night's last run."
    partial: (job: string, result: string) =>
      `${job[0]?.toUpperCase()}${job.slice(1)} ${result} in a run that night; this figure is from the night's last run.`,
  },
  // The tables' names for screen readers (each table scrolls on a phone, so it is a named region).
  tables: {
    files: "How Lamina PDF read each source's files",
    agreement: "Agreement with each tool",
    text: "Page text agreement, by source",
    standards: "veraPDF's verdict for each example",
    rendering: "Pages drawn alike, by source",
  },
  limits: {
    heading: "What these figures leave out",
    items: [
      "The files are chosen because they are difficult. They aren't a sample of the PDFs people use every day, so the share opened is not how often an everyday file opens.",
      "Agreeing with other tools is not the same as being right: when every tool is wrong in the same way, Lamina PDF agrees with them.",
      "Opening a file and reading its pages doesn't prove every page is drawn or extracted correctly.",
      "Only nights that ran their full budgets are recorded. A night cut short, such as one whose fuzzing stopped early, records no figures, so it doesn't count among the nights above.",
      "Speed and size are measured elsewhere, on the docs site's Benchmarks page.",
    ],
  },
  licence: {
    heading: "The licence",
    // "Lamina PDF is open source, under `MIT OR Apache-2.0`: …"; the licence comes from Cargo.toml.
    own: [
      "Lamina PDF is open source, under",
      ": you may use it in open or closed software, and choose either licence.",
    ],
    allowed: "Every dependency must be under one of these licences:",
    exceptions: "A few named dependencies may use another licence:",
    exception: (licence: string) => `${licence}, for`,
    banned: "These are refused whatever licence they claim:",
    checks:
      "Every change runs cargo-deny and an npm licence check before it can merge. Those checks cover dependencies, not copied code. No tool can prove that code wasn't copied, so the next section says how the code is written.",
    verapdf:
      "veraPDF, which is under the GPL and MPL, runs only as a separate program in the tests. It is never part of what Lamina PDF ships.",
    docs: "What Lamina PDF writes and what it borrows",
  },
  provenance: {
    heading: "How the code is written",
    // Every claim here was checked against the repository when the owner approved it.
    body: [
      "Lamina PDF's own Rust code reads, edits and saves PDFs, written from the PDF standard (ISO 32000-2) and the standards it cites. For fonts, text shaping and writing page content, it uses open-source libraries such as skrifa, HarfRust and pdf-writer, each under one of the licences listed above.",
      "AI coding agents write most of the code, Claude Code above all. Each of their commits says so in its message, so anyone can count them. The project's owner decides what gets built and in what order.",
      "Every change starts from a written plan in the repository and arrives as a pull request with its tests. An automated code review reads it, and a person reviews it before it merges. It merges only when its checks pass: the tests, plus shorter runs of the comparisons, fuzzing and saved-file checks on this page.",
      "The full checks on this page run every night. They don't hold up a merge, but they do hold up a release: a version goes out only after a full nightly run passes on its commit or an earlier one.",
    ],
  },
  related: {
    heading: "Related pages",
    benchmarks: "Speed and size: the docs site's Benchmarks page",
    privacy: "What the web reader keeps private",
    performance: "How fast the reader is",
    pricing: "Pricing and promises",
  },
} as const;
