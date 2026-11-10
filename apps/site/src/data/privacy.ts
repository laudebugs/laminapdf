// The privacy page's words (/privacy/): ADR 0011's guarantees in plain language. Each promise
// names the guarantees it states (ADR 0011 §3, §6) and the test that proves it, which
// test/privacy.test.ts holds to the ADR and to the test files; a promise no test can check says so.
// A change to a promise goes back to the owner, as the home page's words do.

/** Where a promise is checked: a test file of the repository and the test's own name. */
interface Proof {
  readonly file: string;
  readonly test: string;
}

/** One promise: a heading, its explanation, the ADR 0011 rules it states and its proof. */
export interface Promise {
  readonly heading: string;
  readonly body: readonly string[];
  /** ADR 0011 §3's numbered guarantees (1–10), or §6's rules as "6.1"… */
  readonly rules: readonly string[];
  /** The tests that check it; empty only with `untested` saying why. */
  readonly proofs: readonly Proof[];
  readonly untested?: string;
}

const PRIVACY_SUITE = "apps/web/e2e/privacy.e2e.test.ts";

/** The page's title, description and opening. */
export const PRIVACY = {
  title: "Privacy · Lamina PDF",
  description:
    "Lamina PDF's web reader opens your PDFs inside your browser. Nothing you open is uploaded, and nothing about you is collected.",
  heading: "Privacy",
  lead: "The web reader opens your PDFs inside your browser tab. Nothing you open is uploaded to us, and nothing about you is collected. A test checks each promise below before every release.",
} as const;

/** The reader's promises, in the order the page lists them. */
export const PROMISES: readonly Promise[] = [
  {
    heading: "Your files stay on your device",
    body: [
      "The reader opens a file you choose, drop or paste inside this browser tab. No request it makes carries any part of the file.",
    ],
    rules: ["1"],
    proofs: [{ file: PRIVACY_SUITE, test: "a local file stays in the browser" }],
  },
  {
    heading: "A linked file comes only from its own site",
    body: [
      "When you open a PDF from a link, the reader fetches it from that site and the sites it redirects to, with no cookies and no referrer.",
      "Our server never learns the file's address: the reader keeps it after the # in its own address, a part browsers never send.",
    ],
    rules: ["2", "3"],
    proofs: [
      { file: PRIVACY_SUITE, test: "a remote file is fetched from its host alone, without a referrer or cookies" },
    ],
  },
  {
    heading: "No cookies",
    body: ["The reader sets no cookies, and neither does this website."],
    rules: ["4"],
    proofs: [{ file: PRIVACY_SUITE, test: "a local file stays in the browser" }],
  },
  {
    heading: "Nothing from other companies",
    body: [
      "Every script, style sheet, font and image comes from Lamina PDF's own site. The only other site the reader contacts is the host of a PDF you asked it to open.",
    ],
    rules: ["5"],
    proofs: [
      { file: PRIVACY_SUITE, test: "a remote file is fetched from its host alone, without a referrer or cookies" },
    ],
  },
  {
    heading: "No tracking",
    body: [
      "The reader sends us nothing about you or your files. Each request it makes to our server fetches a file of the reader itself: there are no analytics, error reports or counters.",
      "It reads nothing about your device that could tell it apart from another, and your browser's settings stop it from using your camera, microphone or location.",
    ],
    rules: ["6", "7"],
    proofs: [{ file: PRIVACY_SUITE, test: "a local file stays in the browser" }],
  },
  {
    heading: "Passwords stay in the tab",
    body: [
      "A password you type to open a protected PDF goes only to the part of the reader that opens the file, in this tab. It is never stored or sent.",
    ],
    rules: ["8"],
    proofs: [
      { file: PRIVACY_SUITE, test: "a local encrypted file's password goes nowhere" },
      { file: PRIVACY_SUITE, test: "a remote encrypted file's password goes nowhere, its host included" },
    ],
  },
  {
    heading: "What stays on your device, and how to clear it",
    body: [
      "The reader keeps copies of the files you opened, so they open again without a connection. It also keeps a list of recent files: their names, sizes, page counts and when you opened them, never their contents or addresses. It also keeps your settings and its own files, so it works offline.",
      "Settings › Storage and privacy › Clear recent files and cached documents removes the copies and the list. Your browser's settings for this site remove everything.",
    ],
    rules: ["9"],
    proofs: [
      {
        file: "apps/web/e2e/recent-files.e2e.test.ts",
        test: "keeps nothing about a linked document: neither its name nor its address (ADR 0011)",
      },
      {
        file: "apps/web/e2e/recent-files.e2e.test.ts",
        test: "empties the list and the cached documents from Settings",
      },
    ],
  },
  {
    heading: "Strict browser rules",
    body: [
      "Every response from the reader's site tells your browser to run only the reader's own code, to keep other sites from framing it, and to keep it from your camera, microphone and location.",
    ],
    rules: ["10"],
    proofs: [{ file: "apps/web/e2e/smoke.test.ts", test: "%s is the app, with ADR 0011's headers" }],
  },
];

/** How to check the promises yourself. */
export const CHECK = {
  heading: "Check it yourself",
  body: [
    "The reader's status bar counts every network request the open document caused; select it to see each one and where it went. Your browser's developer tools show every request the page makes.",
  ],
} as const;

/** What later, optional features will and won't do (ADR 0011 §6). */
export const LATER: Promise = {
  heading: "If Lamina PDF adds accounts or cloud features",
  body: [
    "They will be optional, and off until you turn them on. The reader will keep working without an account.",
    "Before any part of a document leaves your device, the reader will say what it sends, where it is kept and for how long, and ask you for that document.",
    "Reading without an account will stay cookie-free.",
  ],
  rules: ["6.1", "6.2", "6.5"],
  proofs: [],
  untested: "These features don't exist yet; their own work adds the tests.",
};

/** The id of a promise's section on the page, from its heading. */
export const sectionId = (heading: string): string =>
  heading
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/(^-|-$)/g, "");
