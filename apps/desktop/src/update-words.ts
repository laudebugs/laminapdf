// Settings › Updates' and the status bar's words (S06), kept in the desktop app since only it
// updates itself: in @laminapdf/ui's shared messages they would ship in the web reader's startup
// bundle too. English for now, in one table for the translations to come.

const WORDS = {
  title: "Updates",
  upToDate: "Lamina PDF {version} is up to date",
  current: "You have Lamina PDF {version}",
  checking: "Checking for updates…",
  downloading: "Downloading Lamina PDF {version}",
  downloadingHelp: "You have {current}. The new version installs when you quit, or when you choose Restart to update.",
  ready: "Lamina PDF {version} is ready",
  readyHelp:
    "It installs the next time you quit. Restart now to install it; your windows and tabs come back where you left them.",
  failed: "Couldn't reach the update server. Lamina PDF will try again tomorrow.",
  off: "Automatic updates are off",
  offHelp: "You have {current}. Check now still looks for a new version.",
  automatic: "Automatic updates",
  "automatic.help":
    "Lamina PDF checks once a day, downloads a new version in the background and installs it when you quit.",
  channel: "Channel",
  "channel.beta": "Beta",
  "channel.stable": "Stable",
  "channel.stableNotYet": "Stable (not released yet)",
  "channel.nightly": "Nightly",
  "channel.nightlyHelp":
    "Nightly builds are a separate install: they update only to newer nightlies. To leave the nightly line, install a beta download.",
  nightlyNote: "Nightly builds, with the newest changes, are a separate install from the beta and stable ones.",
  lastChecked: "Last checked {when}",
  neverChecked: "Not checked yet",
  checkNow: "Check now",
  managed: "Managed by your organisation",
  managedChannel: "Your organisation keeps this install on the {channel} channel.",
  managedOff: "Your organisation has turned updates off for this install.",
  packageManager: "Updates come from your package manager",
  packageManagerHelp: "This copy was installed as a .deb or .rpm package, so your system's package manager updates it.",
  sends: "What each check sends",
  sendsHelp:
    "Each check sends the update server your version, operating system, processor type and channel, and an anonymous ID that lets a new version reach installs in stages. The ID is random, isn't linked to you or your documents, and goes nowhere else. Nothing about a check is kept.",
  privacy: "Privacy policy",
  resetId: "Reset anonymous ID",
  resetDone: "New anonymous ID created",
  "summary.on": "Automatic, {channel} channel",
  "summary.off": "Automatic updates off",
  "summary.managed": "Managed by your organisation",
  "summary.packageManager": "From your package manager",
  appStore: "Updates come from the App Store",
  appStoreHelp: "This copy came from the Mac App Store, which updates it.",
  "summary.appStore": "From the App Store",
  readyItem: "Update ready",
  readyChannel: "{channel} channel · you have {current}",
  whatsNew: "What's new in {version}",
  later: "Later",
  closeDetails: "Close update details",
} as const;

/** `template` with each `{name}` filled from `values`; a name with no value stays as it is. */
export function fill(template: string, values: Readonly<Record<string, string | number>> = {}): string {
  return template.replace(/\{(\w+)\}/g, (whole, name: string) => (name in values ? String(values[name]) : whole));
}

/** The words named `id`, with each `{name}` filled from `values`. */
export function words(id: keyof typeof WORDS, values: Readonly<Record<string, string | number>> = {}): string {
  return fill(WORDS[id], values);
}
