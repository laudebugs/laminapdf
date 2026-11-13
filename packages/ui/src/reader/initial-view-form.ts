// The initial view dialog's form (O13), apart from its controls: the values read from the file,
// and the change Apply makes as one patch of `setInitialView` (ISO 32000-2 §12.2, §7.7.2). A
// field the user left as the file had it is left out of the patch, so the edit keeps it.

/** The zoom a file opens at, as the dialog offers it. */
export type InitialZoom = "default" | "fitPage" | "fitWidth" | number;

/** How the pages are laid out (`/PageLayout`); `"unset"` leaves it to the reader. */
export type InitialLayout =
  | "unset"
  | "singlePage"
  | "oneColumn"
  | "twoColumnLeft"
  | "twoColumnRight"
  | "twoPageLeft"
  | "twoPageRight";

/** What shows beside the page (`/PageMode`); `"unset"` leaves it to the reader. */
export type InitialMode =
  | "unset"
  | "none"
  | "outlines"
  | "thumbnails"
  | "fullScreen"
  | "optionalContent"
  | "attachments";

/** The window entries the dialog edits. */
interface InitialWindow {
  readonly displayDocTitle: boolean;
  readonly fitWindow: boolean;
  /** `/HideToolbar`, `/HideMenubar` and `/HideWindowUI` together: the design's one choice. */
  readonly hideControls: boolean;
}

/**
 * How a file asks to open, as the dialog reads it: `@laminapdf/sdk`'s `InitialView` with the
 * page as an index. `open` is `"action"` when the file runs an action instead, which the dialog
 * shows and keeps.
 */
export interface InitialViewData {
  readonly open:
    | { readonly kind: "default" }
    | { readonly kind: "page"; readonly page: number; readonly zoom: InitialZoom }
    | { readonly kind: "action"; readonly action: string };
  readonly pageMode?: Exclude<InitialMode, "unset"> | undefined;
  readonly pageLayout?: Exclude<InitialLayout, "unset"> | undefined;
  readonly preferences: {
    readonly hideToolbar: boolean;
    readonly hideMenubar: boolean;
    readonly hideWindowUi: boolean;
    readonly fitWindow: boolean;
    readonly displayDocTitle: boolean;
  };
}

/** The form's values. */
export interface InitialViewForm {
  /** The 0-based page to open at; `undefined` for the reader's first page. */
  readonly page: number | undefined;
  readonly zoom: InitialZoom;
  readonly layout: InitialLayout;
  readonly mode: InitialMode;
  readonly window: InitialWindow;
}

/** The change Apply makes: `@laminapdf/sdk`'s `InitialViewPatch` with the page as an index. */
export interface InitialViewChange {
  readonly open?: { readonly page: number; readonly zoom: InitialZoom } | null;
  readonly pageMode?: Exclude<InitialMode, "unset"> | null;
  readonly pageLayout?: Exclude<InitialLayout, "unset"> | null;
  readonly preferences?: {
    readonly displayDocTitle?: boolean;
    readonly fitWindow?: boolean;
    readonly hideToolbar?: boolean;
    readonly hideMenubar?: boolean;
    readonly hideWindowUi?: boolean;
  };
}

/** The form as the file has it. */
export function formFrom(view: InitialViewData): InitialViewForm {
  const prefs = view.preferences;
  return {
    page: view.open.kind === "page" ? view.open.page : undefined,
    zoom: view.open.kind === "page" ? view.open.zoom : "default",
    layout: view.pageLayout ?? "unset",
    mode: view.pageMode ?? "unset",
    window: {
      displayDocTitle: prefs.displayDocTitle,
      fitWindow: prefs.fitWindow,
      hideControls: prefs.hideToolbar && prefs.hideMenubar && prefs.hideWindowUi,
    },
  };
}

/**
 * What changed from `before` to `after`, as a patch; `undefined` when nothing did. A file whose
 * open action runs an action (`keepOpen`) keeps it: the patch never names the page.
 */
export function changeOf(
  before: InitialViewForm,
  after: InitialViewForm,
  keepOpen: boolean,
): InitialViewChange | undefined {
  const change: { -readonly [K in keyof InitialViewChange]: InitialViewChange[K] } = {};
  if (!keepOpen && (before.page !== after.page || before.zoom !== after.zoom)) {
    // The reader's first page at the reader's zoom is no open action at all.
    change.open =
      after.page === undefined && after.zoom === "default" ? null : { page: after.page ?? 0, zoom: after.zoom };
  }
  if (before.mode !== after.mode) change.pageMode = after.mode === "unset" ? null : after.mode;
  if (before.layout !== after.layout) change.pageLayout = after.layout === "unset" ? null : after.layout;
  const preferences = windowChange(before.window, after.window);
  if (preferences !== undefined) change.preferences = preferences;
  return Object.keys(change).length === 0 ? undefined : change;
}

/** The window entries that changed, `undefined` when none did. */
function windowChange(before: InitialWindow, after: InitialWindow): InitialViewChange["preferences"] | undefined {
  const preferences: Record<string, boolean> = {};
  if (before.displayDocTitle !== after.displayDocTitle) preferences.displayDocTitle = after.displayDocTitle;
  if (before.fitWindow !== after.fitWindow) preferences.fitWindow = after.fitWindow;
  if (before.hideControls !== after.hideControls) {
    const hide = after.hideControls;
    Object.assign(preferences, { hideToolbar: hide, hideMenubar: hide, hideWindowUi: hide });
  }
  return Object.keys(preferences).length === 0 ? undefined : preferences;
}
