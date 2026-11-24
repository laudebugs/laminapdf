// The support page's words (/support/). The page gives the Mac App Store listing a support address
// (todo D8d). Each claim about the apps matches the app's own text: the crash-report sentence is
// packages/ui's "settings.privacy.crashReports.help" in other words, which test/support.test.ts
// holds to. A change to a sentence that makes a claim goes back to the owner, as the privacy page's
// words do.

/** The owner's support address. */
export const SUPPORT_EMAIL = "support@laminapdf.com";

/** The public issue tracker; the repository is public with issues on. */
export const ISSUES_URL = "https://github.com/laudebugs/laminapdf/issues";

/** The page's title, description and opening. */
export const SUPPORT = {
  title: "Support · Lamina PDF",
  description: `Email ${SUPPORT_EMAIL} for help with Lamina PDF. What to put in your message, and what to leave out.`,
  heading: "Get help with Lamina PDF",
  lead: "Write to us by email and tell us what went wrong. A few details and one precaution make your message easy to act on.",
  email: {
    heading: "Email us",
    before: "Write to",
    after: ", from the address where you want our reply.",
  },
  include: {
    heading: "What to put in your message",
    intro: "Three facts let us find the problem:",
    items: [
      "The operating system you use",
      "The app, web reader or desktop, and its version (Settings › About shows the desktop app's)",
      "What you did, and what happened instead of what you expected",
    ],
  },
  leaveOut: {
    heading: "What to leave out",
    body: [
      "Don't attach your PDFs, and don't send a document that holds private information. Describe the file in words instead: where it came from and what it contains in general.",
      "You don't need a crash report to get help. The desktop app sends one only if you turn that on in Settings › Privacy, and the report is anonymous: it never holds your files, their names or what's in them. The Mac App Store version sends none of its own.",
    ],
  },
  tracker: {
    heading: "Report it in public instead",
    before: "Anyone can read the",
    link: "issue tracker",
    after: ", so leave out anything private there too.",
  },
  privacy: {
    before: "Read how Lamina PDF treats your data on the",
    link: "privacy page",
  },
} as const;
