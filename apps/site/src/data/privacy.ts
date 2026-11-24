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
      "The desktop app keeps a list of the files you opened on this computer (their names, sizes and when you opened them) and adds them to your system's recent files. Clear list on its start window, or turning off Settings › Files › Remember recent files, empties both.",
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
      {
        file: "apps/desktop/e2e/specs/cold-open.e2e.ts",
        test: "should clear the list, and record nothing, while Remember recent files is off",
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

/**
 * The desktop app's promises (todo D7c; issue #640 proposed the crash-report one). The owner has
 * approves these words before the pull request merges, as with the web reader's promises above. The app's own words say the same (apps/desktop/src/update-words.ts, "sendsHelp";
 * packages/ui's crash-report help), and its Settings link here. The recent-files promise is the
 * web reader's "What stays on your device" section, which names the desktop app too.
 */
export const DESKTOP = {
  heading: "The desktop app",
  intro:
    "The desktop app opens your PDFs on your own computer, and your documents never leave it. It contacts one server of ours, for two things: update checks and, only if you agree, crash reports.",
} as const;

/** The desktop app's promises, in the order the page lists them. They state no ADR 0011 guarantee. */
export const DESKTOP_PROMISES: readonly Promise[] = [
  {
    heading: "Update checks from the desktop app",
    body: [
      "The app checks for updates once a day, and Settings turns that off. Each check sends our update server the app's version, your operating system, your processor type, the update channel and an anonymous ID.",
      "The ID is random, isn't linked to you or your documents, and goes only to the update server. It lets a new version reach installs in stages, and Settings › Reset anonymous ID makes a new one.",
      "Our update server sees the address your check comes from, as every server does. It doesn't store your address or ID, and it writes no log of requests; we see only aggregate request counts.",
    ],
    rules: [],
    proofs: [
      {
        file: "apps/updates/test/log-leak.test.ts",
        test: "never writes the install ID, or a hash of it, to the console",
      },
    ],
  },
  {
    heading: "Crash reports from the desktop app",
    body: [
      "The desktop app sends a crash report only if you agree. It asks once, after the first crash, and Settings › Privacy turns sending on or off.",
      "A report says what went wrong in the app: the error, where in Lamina PDF's code it happened, the app's version, your operating system and its version. It never holds your files, their names, their folders or what's in them, and you can read the whole report before you send it. A report carries no ID that links it to you or to your other reports.",
      "Reports go to Lamina PDF's own server, which deletes them after 30 days. Our server sees the address your report comes from, as every server does, uses it only to limit how many reports one address sends, and doesn't keep it.",
    ],
    rules: [],
    proofs: [
      {
        file: "apps/desktop/e2e/specs/crash.e2e.ts",
        test: "should send nothing while off, and ask once in the notice, showing the report",
      },
      {
        file: "apps/desktop/e2e/specs/crash.e2e.ts",
        test: "should send each crash once turned on, with nothing of the file in it",
      },
      {
        file: "apps/desktop/src-tauri/src/crash/mod.rs",
        test: "no_report_should_hold_a_marker_of_the_open_file_or_a_path",
      },
      {
        file: "apps/updates/test/log-leak.test.ts",
        test: "never writes a crash report's body or the client's IP to the console",
      },
    ],
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
