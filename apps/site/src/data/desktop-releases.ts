// The desktop app's release notes (/desktop/releases/<version>/): the page the updater's `notes`
// address names (ADR 0014; apps/updates). One const per release, so an old page keeps its words.
//
// Todo D7c wrote these words; the owner approves them before the pull request merges.
// Each claim about the app matches a source the repository checks: the download page's words
// (src/data/downloads.ts), the privacy page, and the desktop column of docs/capabilities/README.md,
// which test/desktop-releases.test.ts reads. The known issues wait for the clean-machine checks and
// the measurements on the reference machines (docs/todos/ongoing/D7c-desktop-beta-release.md).
//
// The page is the address a tagged release's `notes` names (the publish step, .github/actions/
// desktop-publish) and the app's "What's new" opens (update/commands.rs).
import { WORDS } from "./downloads.js";
import { ISSUES_URL, SUPPORT_EMAIL } from "./support.js";

/** One release's page. */
export interface DesktopRelease {
  readonly version: string;
  readonly title: string;
  readonly description: string;
  readonly heading: string;
  readonly lead: string;
  readonly pricing: string;
  readonly does: { readonly heading: string; readonly items: readonly string[] };
  readonly systems: { readonly heading: string; readonly body: string };
  readonly install: { readonly heading: string; readonly windows: readonly string[]; readonly linux: string };
  readonly known: { readonly heading: string; readonly items: readonly string[] };
  readonly verify: {
    readonly heading: string;
    readonly intro: string;
    readonly checks: readonly { readonly label: string; readonly code: string }[];
    readonly note: string;
  };
  readonly report: { readonly heading: string; readonly body: string };
}

/** The first beta, for Windows and Linux. */
export const RELEASE_0_1_0: DesktopRelease = {
  version: "0.1.0",
  title: "Lamina PDF 0.1.0 for Windows and Linux · Lamina PDF",
  description:
    "The first beta of Lamina PDF's desktop app, for Windows and Linux: what it does, its known issues, how to check your download and how to report a bug.",
  heading: "Lamina PDF 0.1.0, the first beta for Windows and Linux",
  lead: "Lamina PDF 0.1.0 reads and organises PDFs on your own computer, with no upload and no connection needed. This beta runs on Windows and Linux; macOS follows.",
  // The same sentence as the download page's, so the two can't differ (src/data/downloads.ts).
  pricing: WORDS.pricing,
  does: {
    heading: "What it does",
    items: [
      "Reads PDFs in tabs and windows, on your own computer and offline.",
      "Opens a PDF when you double-click it or choose Open with, and lists your recent files. The start window's Clear list, or Settings › Files › Clear recent files, empties that list and your system's own.",
      "Organises pages in a grid: moves, rotates, duplicates and deletes them. It saves the file in place.",
      "Edits a document's title, author, subject and keywords.",
      "Checks for updates once a day, and you can turn that off in Settings. The app updates from the beta channel.",
      "Sends crash reports only if you turn them on in Settings › Privacy. Your documents never leave your computer.",
    ],
  },
  systems: {
    heading: "Where it runs",
    body: "Windows 11 on x64 or Arm, and Ubuntu 22.04 or later or a supported Fedora, on x64 or Arm. Windows 10 22H2 is untested, and we may not fix problems found only there. macOS follows in a later release. The Mac App Store build is a separate install and doesn't update from this beta.",
  },
  install: {
    heading: "Installing it",
    windows: [
      "Run the setup .exe. Windows may show “Windows protected your PC”, because Microsoft SmartScreen hasn't seen many downloads of Lamina PDF yet.",
      "Select More info, check that the publisher is Laurence Ininda, then select Run anyway. The warning fades as more people install the app. The .msi is for managed installs.",
    ],
    linux:
      "Make the AppImage executable (chmod +x on it) and run it; it updates itself. The .deb and .rpm update with your system's packages.",
  },
  known: {
    heading: "Known issues",
    items: [
      "Removing a value in Document properties hides it from the current version of the file, but the old value can still be found in earlier parts of the file, so don't rely on it to hide private details.",
      "Some features aren't in the desktop app yet. It has no Find in a document, no sidebar or outline, no Reader mode and no Back and Forward, and it can't change the colour theme: it follows your computer's light or dark appearance. These menu items are visibly turned off, and nothing you open or save is affected. We know about these.",
      "A blank window on Linux. Some NVIDIA and Wayland setups draw a blank window. Choose Settings › Graphics › Compatibility, or start the app with --safe-graphics. After two starts that end at once, the app switches to Compatibility on its own and says so.",
      "A SmartScreen warning on Windows. The first installs show “Windows protected your PC”; the steps are under Installing it.",
    ],
  },
  verify: {
    heading: "Checking your download",
    intro:
      "The download page lists each file with its SHA-256. GitHub Actions built each file from the public source and attested it. Check a file before you run it.",
    checks: [
      {
        label: "Windows (PowerShell): the status is Valid and SignerCertificate names the publisher.",
        code: "Get-AuthenticodeSignature .\\<file> | Format-List",
      },
      {
        label:
          "Linux: import the project's release key from laminapdf.com/keys/lamina-release.asc, then check a package, or SHA256SUMS and the files it lists.",
        code: "gpg --import lamina-release.asc\ngpg --verify <file>.asc <file>\ngpg --verify SHA256SUMS.asc SHA256SUMS && sha256sum --check --ignore-missing SHA256SUMS",
      },
      {
        label: "Either system: check which workflow built the file.",
        code: "gh attestation verify <file> --repo laudebugs/laminapdf",
      },
    ],
    note: "A file that fails a check isn't ours: delete it and tell us.",
  },
  report: {
    heading: "Reporting a bug",
    body: `Open an issue on the project's issue tracker (${ISSUES_URL}) or write to ${SUPPORT_EMAIL}. The support page lists what to put in a message: your system, the app's version (Settings › About) and what happened. Leave your PDFs out.`,
  },
};

/** Every release page, newest first. */
export const DESKTOP_RELEASES: readonly DesktopRelease[] = [RELEASE_0_1_0];

/** The path of `version`'s notes page, or `undefined` when there is none (a nightly has no notes). */
export function notesPath(version: string): string | undefined {
  return DESKTOP_RELEASES.some((release) => release.version === version) ? `/desktop/releases/${version}/` : undefined;
}
