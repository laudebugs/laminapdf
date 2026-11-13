// The gallery's routes and the settings a URL can choose: plain data, shared with the Playwright
// specs (e2e/), which enumerate every combination they screenshot. No React here.

/** One page of the gallery. */
export interface Route {
  /** The `route` URL parameter. */
  id: string;
  /** The page's heading. */
  title: string;
  /** The `@laminapdf/ui` components whose states from `@laminapdf/ui/testing` it shows. */
  components: readonly string[];
  /** Components it renders itself (overlays and the reader parts that need a controller). */
  renders?: readonly string[];
  /** Whether it shows page content, so it is drawn in every page appearance too. */
  pageRelated?: boolean;
  /**
   * The overlay states, one open at a time (the `state` URL parameter): modal overlays cover the
   * page, so each state gets its own screenshot.
   */
  states?: readonly string[];
}

/** Every route, in the design language's §7 order. */
export const ROUTES: readonly Route[] = [
  { id: "tokens", title: "Tokens", components: [] },
  { id: "icons", title: "Icons", components: [] },
  { id: "buttons", title: "Buttons", components: ["Button", "IconButton"] },
  { id: "toggles", title: "Toggles", components: ["ToggleButton", "ToolToggle"] },
  { id: "modes-and-tabs", title: "Mode switcher and tabs", components: ["ModeSwitcher", "SegmentedTabs"] },
  {
    id: "fields",
    title: "Form controls",
    components: ["TextInput", "TextArea", "Select", "Checkbox", "RadioGroup", "Switch"],
  },
  {
    id: "pickers",
    title: "Swatches and appearance chips",
    components: ["SwatchGroup", "PageAppearanceChips", "PageAppearanceFields"],
    pageRelated: true,
  },
  {
    id: "feedback",
    title: "Badges, notices and toasts",
    components: ["Badge", "Notice", "Toast", "ProgressBar", "Skeleton", "EmptyState"],
  },
  {
    id: "status-bar",
    title: "Status bar items",
    components: ["StatusBarItem", "PageLabel", "NetworkIndicator", "RepairedNotice", "ShortcutChip"],
  },
  {
    id: "rail",
    title: "Thumbnails, outline and history",
    components: ["Thumbnail", "OutlineRow", "OutlineTree", "HistoryPanel", "ThreadsPanel", "BookmarkEditor"],
    pageRelated: true,
  },
  {
    id: "reader",
    title: "Reader chrome",
    components: [
      "ReaderTopBar",
      "EditArea",
      "SaveStrip",
      "FindBar",
      "FindResults",
      "PageInput",
      "ZoomControl",
      "WholeFileDownload",
      "RepairDetails",
      "OpenFailure",
      "RemoteFailure",
      "OfflineNotice",
      "RecentList",
    ],
  },
  { id: "network", title: "Network activity", components: ["NetworkActivityPanel"] },
  { id: "reader-mode", title: "Reader mode", components: ["ReaderView", "ReaderSettingsPanel"] },
  {
    id: "reader-shell",
    title: "Reader shell",
    components: [],
    renders: ["ReaderShell", "OpenDropZone", "DropOverlay", "DebugMenu"],
  },
  {
    id: "shell-regions",
    title: "Shell regions",
    components: [],
    renders: ["ReaderShell", "LeftRail", "Inspector", "FocusModeItem", "PhoneBottomBar", "SelectionActionBar"],
  },
  {
    id: "dialogs",
    title: "Dialogs",
    components: [],
    renders: [
      "Dialog",
      "PasswordDialog",
      "ExternalLinkDialog",
      "PrintDialog",
      "PrintProgressDialog",
      "PropertiesDialog",
      "InitialViewDialog",
      "InstallDialog",
      "SettingsDialog",
      "ShortcutsDialog",
      "ViewSettingsButton",
      "RemoveSignaturesDialog",
      "ConformanceDialog",
      "NamedDestinationsDialog",
    ],
    states: [
      "default",
      "form",
      "wide",
      "alert",
      "password",
      "external-link",
      "print",
      "print-restricted",
      "print-progress",
      "properties",
      "initial-view",
      "initial-view-action",
      "install",
      "settings",
      "shortcuts",
      "view-settings",
      "remove-signatures",
      "conformance",
      "named-destinations",
    ],
  },
  {
    id: "sheets",
    title: "Sheets",
    components: [],
    renders: ["BottomSheet", "MoreSheet"],
    states: ["medium", "large", "non-modal", "more"],
  },
  {
    id: "menus",
    title: "Menus, popovers and tooltips",
    components: [],
    renders: ["Menu", "MenuTrigger", "Popover", "PopoverTrigger", "Tooltip"],
    states: ["menu", "popover", "tooltip"],
  },
  {
    id: "link-preview",
    title: "Link preview",
    components: [],
    renders: ["LinkPreview"],
    states: ["popover", "sheet"],
  },
  {
    id: "link-editor",
    title: "Link editor",
    components: [],
    renders: ["LinkPanel", "LinkEditor"],
    states: ["page", "web", "refused", "script"],
  },
  {
    id: "selection-toolbar",
    title: "Selection toolbar",
    components: [],
    renders: ["SelectionToolbar", "SelectionToolbarDivider"],
  },
  {
    id: "palette",
    title: "Command palette",
    components: [],
    renders: ["CommandPalette"],
    states: ["results", "empty"],
  },
  { id: "toasts", title: "Toast region", components: [], renders: ["ToastRegion"] },
];

/** The UI themes the gallery draws (`data-theme`). */
export const THEMES = ["light", "dark", "high-contrast"] as const;
/** The page appearances (`data-page-appearance`), for page-related routes. */
export const APPEARANCES = ["original", "sepia", "dark", "high-contrast"] as const;
/** The densities (`DensityProvider`). */
export const DENSITIES = ["desktop", "touch"] as const;
/** The text directions (`dir` on the root). */
export const DIRECTIONS = ["ltr", "rtl"] as const;
/** Full motion, or none (transitions and animations off). */
export const MOTIONS = ["full", "reduce"] as const;

/** What the URL chooses. */
export interface Settings {
  route: string;
  theme: (typeof THEMES)[number];
  appearance: (typeof APPEARANCES)[number];
  density: (typeof DENSITIES)[number];
  dir: (typeof DIRECTIONS)[number];
  motion: (typeof MOTIONS)[number];
  state: string;
}

/** The settings when the URL leaves them out. */
const DEFAULTS: Settings = {
  route: "",
  theme: "light",
  appearance: "original",
  density: "desktop",
  dir: "ltr",
  motion: "full",
  state: "",
};

/** `value` when it is one of `allowed`, else `fallback`. */
function oneOf<T extends string>(allowed: readonly T[], value: string | null, fallback: T): T {
  return allowed.find((a) => a === value) ?? fallback;
}

/** The settings a query string chooses: unknown values fall back to the defaults. */
export function parseSettings(search: string): Settings {
  const params = new URLSearchParams(search);
  return {
    route: params.get("route") ?? DEFAULTS.route,
    theme: oneOf(THEMES, params.get("theme"), DEFAULTS.theme),
    appearance: oneOf(APPEARANCES, params.get("appearance"), DEFAULTS.appearance),
    density: oneOf(DENSITIES, params.get("density"), DEFAULTS.density),
    dir: oneOf(DIRECTIONS, params.get("dir"), DEFAULTS.dir),
    motion: oneOf(MOTIONS, params.get("motion"), DEFAULTS.motion),
    state: params.get("state") ?? DEFAULTS.state,
  };
}

/** The query string for `settings`, leaving out what equals the defaults. */
export function settingsQuery(settings: Partial<Settings>): string {
  const params = new URLSearchParams();
  for (const [key, value] of Object.entries(settings)) {
    if (value !== undefined && value !== DEFAULTS[key as keyof Settings]) params.set(key, value);
  }
  const query = params.toString();
  return query === "" ? "" : `?${query}`;
}
