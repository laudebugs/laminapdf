# @laminapdf/ui

The Paper components that Lamina PDF's reader and editor are built from: buttons, toggles, the mode switcher, tabs, form controls, colour swatches and page-appearance chips. Beside them sit the items that show state: thumbnails, outline rows, status-bar items, shortcut chips, notices, toasts, progress, empty states and badges.

Each component shows every state the design language defines, works from the keyboard, lays out right to left, and grows to touch size on a coarse pointer. The components are presentational: they take props and report events, and never reach a viewer, a store or the command registry. The package is private to this repository for now.

## Using the components

Import the stylesheet once, then the components:

```ts
import "@laminapdf/ui/styles.css";
import { Button, IconButton, ModeSwitcher } from "@laminapdf/ui";

<Button variant="primary" onPress={save}>Save</Button>
<IconButton icon="find" label="Find in document" onPress={openFind} />
```

The stylesheet brings in the tokens, so themes, accents and page appearances work as `@laminapdf/tokens` describes. Every rule sits in a CSS layer, so an app's unlayered rules win without `!important`. The components are built on React Aria Components, which handle press, focus, keyboard and right-to-left behaviour.

Text and labels always come in as props, so the app translates them. An icon-only control requires its accessible name: `IconButton` won't type-check without `label`.

## Density

On desktop every target is at least 24 × 24 px. On touch every target is at least 44 × 44 px, and fields use 16 px text so phones don't zoom into them. A coarse pointer switches to touch sizes on its own. To set the density for part of the page, whatever the pointer, wrap it:

```ts
<DensityProvider density="touch">…</DensityProvider>
```

`useDensity()` returns the density a provider set, for components that need it in code.

## Components

| Design language | Component | Notes |
| --- | --- | --- |
| §7.2 Mode switcher | `ModeSwitcher` | A group of pressed buttons, each its own Tab stop |
| §7.3 Icon button | `IconButton` | 26, 28 or 32 px; 44 px with a 22 px icon on touch |
| §7.4 Text buttons | `Button` | `primary`, `secondary`, `ghost`, `danger`, `danger-filled`; 32, 30 or 26 px; `isPending` keeps the width |
| §7.5 Segmented tabs | `SegmentedTabs`, `TabPanel` | Arrow keys select as they move; `keyboardActivation="manual"` waits for Enter |
| §7.6 Thumbnail | `Thumbnail` | `aria-current="page"` on the current page; the viewer's image goes in as `children` |
| §7.7 Outline row | `OutlineRow` | `aria-current="location"`; a chevron button with `aria-expanded` for a row with children |
| §7.7 Outline row | `OutlineTree` | The outline as a WAI-ARIA tree of outline rows; `currentOutlineItem` finds the section in view |
| §7.8 Status bar items | `StatusBarItem`, `NetworkIndicator`, `RepairedNotice`, `PageLabel` | The app supplies the counts and wording |
| §7.9 Selection mini toolbar | `SelectionToolbar`, `SelectionToolbarDivider` | Above a rectangle the app gives, flipping below; Shift+F10 reaches it |
| §7.10 Command palette | `CommandPalette`, `ShortcutChip` | The palette's shell over rows the app ranks; the chip writes `⌥⌘E` on macOS and `Ctrl Shift E` elsewhere |
| §7.12 Colour swatches | `SwatchGroup` | Colours are token names, so only design-language colours appear |
| §7.19 Form controls | `TextInput`, `TextArea`, `Select`, `Checkbox`, `RadioGroup`, `Radio`, `Switch` | Label above, help or error below, wired with `aria-describedby` and `aria-invalid` |
| §7.16 Bottom sheet | `BottomSheet` | Medium and large detents; the grabber is a button; modal or not |
| §7.18 Dialog | `Dialog` | Modal; `isRequired` for a question that must be answered; a bottom sheet on a phone |
| §7.20 Toast and notice | `Notice`, `Toast`, `ToastRegion`, `ToastQueue` | A notice is a status message; the region announces each toast |
| §7.21 Empty state | `EmptyState` | A title, one sentence and at most two actions |
| §7.22 Progress | `ProgressBar`, `Skeleton` | Indeterminate without a `value`; it stands still under reduced motion |
| §7.23 Badge and tag | `Badge` | Always a word, never colour alone |
| §7.24 Secondary-toolbar tool toggle | `ToolToggle`, `ToggleButton` | `aria-pressed` |
| §7.25 Page-appearance chips | `PageAppearanceChips` | Each preview draws "Aa" in its appearance's paper and ink |
| §7.26 Menu | `Menu`, `MenuTrigger`, `Popover`, `PopoverTrigger` | The menu button pattern; a popover is a named dialog |
| §7.27 Tooltip | `Tooltip` | After 500 ms of hover, at once on keyboard focus |

## Overlays

Pick the layer by what it asks of people:

- A **notice** for a state that lasts, inside the panel it concerns.
- A **toast** to confirm something that already happened. It goes away, so it never carries an error, and its action (such as Undo) must also be in a menu or a shortcut.
- A **popover** or **menu** for a short choice next to the button that opened it, and the **selection toolbar** for actions on selected text.
- A **dialog** when the work must stop for an answer. On a phone it rises as a bottom sheet.
- A **bottom sheet** for a panel on the phone: modal when it asks something, non-modal when the page stays in use beside it.

Render one `ToastRegion` near the root before any toast arrives, so screen readers announce each one, and show toasts through its queue:

```ts
const toasts = new ToastQueue();

<ToastRegion queue={toasts} aria-label="Notifications" />

const toast = toasts.add({ message: "Highlight deleted", action: { label: "Undo", onPress: undo } });
```

`add` returns a handle whose `dismiss()` removes the toast early. A toast stays 4 s unless its `timeout` says otherwise; one that waits on the user (`timeout: Infinity`) can carry `dismissLabel` ("Later"), a second button that only closes it.

Overlays stack in the design language's z-order, which the tokens hold: docked chrome, then sheets, then dialogs over the scrim, then menus, popovers and tooltips, then toasts. A menu opened from a dialog draws above it. Escape closes the innermost open layer only, and focus goes back to the control that opened it. A dialog that asks a required question stays open, and so does everything beneath it.

## The platform and the reader's state

Components never ask which shell they run in, and `cargo xtask check ts` fails on any package that imports Tauri, calls `isTauri()` or reads the user agent. Instead, each shell creates one `Platform` at startup, as [the platform decision](../../docs/decisions/0010-platform-adapter.md) describes, and hands it down. Components ask it what it can do. The web app passes `createWebPlatform()`; the desktop app will pass its own implementation of the same interface:

```ts
const platform = createWebPlatform();

<PlatformProvider platform={platform}>
  <App />
</PlatformProvider>
```

`createWebPlatform()` detects each capability once, from the browser's own APIs: the file pickers where the browser has them, a file input and downloads where it doesn't, and storage in IndexedDB and the origin-private file system. A control that needs a capability reads `useCapability(flag)`, which answers one of three states: `available`; `disabled`, with a reason that says how to get the capability (another browser, or installing the app); or `hidden`, when this platform never has it.

`ReaderController` holds the reader's lifecycle: empty, opening, asking for a password, open, or failed. It asks the platform for files and hands each one to the opener the shell gives it, such as the viewer client's `open`. A password goes to the opener and is never kept. The controller calls the `close` the opener returned with a document once nothing will show it: when it is closed, replaced or overtaken by a later open.

Toolbars and menus call the controller's methods, so commands can call the same ones. Components read the controller and the viewer through `useStoreState(store, select)`, which re-renders only when the selected value changes. The selector must return a primitive, or an object that stays the same while its contents do; a selector that builds a new object on every call re-renders the component on every update:

```ts
const page = useStoreState(viewer, (state) => state.currentPage);
const reader = useReaderState(controller);
```

Every message the reader shows comes from one catalogue. `message(id, values)` fills each `{name}` placeholder from `values`, and a translation is a second catalogue of the same shape.

## The reader's frame

`ReaderShell` lays out the reader: the top bar across the top, a mode's toolbar under it, then the rail, the canvas and the inspector side by side, and the status bar along the bottom. Each part is a slot, and each slot is its own landmark: the top bar is the `banner`, the rail the `navigation`, the canvas the `main`, the inspector the `complementary` and the status bar the `contentinfo`. A `ShellLayout` holds the state the regions share, so the shell, the commands and the status bar read the same one:

```ts
// In a component: the layout (the rail's open state, width and tab kept in storage), and the modes with Read.
const { layout, modes } = useShell(platform.storage);

const context = { platform, reader, viewer, actions: { ...shellActions(layout, modes, { rail: true }), "palette.open": openPalette } };

<CommandsProvider registry={registry} context={context} keys="other">
  <ReaderShell
    layout={layout}
    modes={modes}
    topBar={<ReaderTopBar reader={controller} viewer={viewer} keys="other" sidebarOpen={railOpen} more={{ items, onAction }} />}
    rail={<LeftRail layout={layout} tabs={[{ id: "pages", label: "Pages", panel: <PagesPanel /> }]} />}
    inspector={<Inspector layout={layout} label="Highlight"><HighlightPanel /></Inspector>}
    status={<><NetworkIndicator count="0 network requests" /><FocusModeItem layout={layout} /></>}
  >
    <div ref={mountViewer} />
  </ReaderShell>
  <PasswordDialog reader={controller} />
  <RegistryPalette isOpen={palette} onOpenChange={setPalette} />
</CommandsProvider>
```

- **The outline** (`OutlineTree`) is the Outline tab's tree. Up and Down move between rows, Right and Left open and close one, Home and End go to the ends, and typing finds a row by its title. Enter or a click goes to the row's section. Each row shows its page label, and long outlines draw only the rows in view. `current` marks the item that holds the reading position and opens its parents. `currentOutlineItem(items, top)` finds that item: the latest one at or before the top of the view.
- **Find's results** (`FindResults`) is the rail's Results tab while find is open. Its `find` prop takes what `FindBar` takes plus `results()` and `goTo(ordinal)`, so you can pass `viewer.find`. `heading(page)` names each page's group, such as "Page 3 · Repairs backlog". Each match is a link: the match in bold, with the text around it. The current one is tinted and ringed, carries `aria-current="true"`, and stays in view as Next and Previous move it. A click or Enter goes to it, then calls `onNavigate`. With no matches, it offers to turn off whichever of Whole words or Match case is on.
- **Leaving for the web** (`ExternalLinkDialog`) asks before a link in the document opens a website. Pass the pending link as `link`: its `href`, and the `parts` from `@laminapdf/viewer`'s `externalLink`, which split the address around its host. The address shows in full with its host in bold, so link text that claims another address can't hide this one. Cancel has focus. Open website calls `onOpen` and closes, and Copy link calls `onCopy` and stays.
- **The offline notice** (`OfflineNotice`) goes in `ReaderShell`'s `notice` slot, under the top bar and above any secondary toolbar, while the browser is offline. It is a polite status, so a screen reader hears it once, and Dismiss calls `onDismiss`.
- **The Recent list** (`RecentList`) shows the home screen's recent documents from `platform.recent`. Each row gives the name over "This device" and its page count, with the date it was last opened. A local file without a kept handle adds "Choose the file again to reopen". Rows call `onOpen` with their entry, and Clear list calls `onClear`. The list draws nothing while it is empty. `RecentFiles.add(file, { pages, fingerprint })` keeps the page count and the document's fingerprint. Entries never hold a URL, so the web reader lists local files only.
- **Where the reader stopped** shows in the Recent list too. Given `stoppedAt` (the page each entry's reader stopped at), a row adds "stopped at page 12". Given `continueReading`, the document open when the app last closed comes first, under Continue reading, with "page 12 of 40".
- **Reading positions** (`platform.positions`, `ReadingPositions`) keep where the reader stopped in each document under its fingerprint: `get`, `set`, `remove` and `clear`. An entry is `{ view, pageCount, readAt }`, and this package never looks inside `view`: the app checks it when it reads it. The web store keeps the 500 entries read most recently, in IndexedDB, or in memory where IndexedDB doesn't open. `recent.reopen(id, { ask: false })` reads a file only where the browser kept the permission, without asking, as an app does at start, when nothing has been clicked.
- **Install** (`InstallDialog`) says what installing the reader gives and that the browser asks to confirm, with Not now and Install. Show it only where the browser offers an install.
- **Print** and **Document properties** come from their own entry, `@laminapdf/ui/dialogs`, so an app can load them after its first paint. The web reader loads them once a document opens. Reader mode's column and its settings panel (`ReaderView`, `ReaderSettingsPanel`) are `@laminapdf/ui/reader-mode` for the same reason, and the editor's parts are `@laminapdf/ui/editing` (below).
- **Print** (`PrintDialog`) asks which pages (all, the current one, or a range of numbers or labels such as `1–5, 11, A-1`), the scale, the quality and whether comments print, then calls `onPrint` with a `PrintChoice`. A range that names a page the file lacks shows the error under the field and focuses it. `PrintProgressDialog` counts the pages drawn, with Cancel. On the web, `Platform.print` prints them from a hidden frame, one page at a time.
- **A restricted file's print** follows the author's `permissions`, when given. If printing is restricted, `PrintDialog` says so and asks once more in an alertdialog, with Cancel focused, before `onPrint`; if only high quality is, Quality stays at Standard.
- **Document properties** (`PropertiesDialog`) shows a `DocumentPropertiesData`, the shape `@laminapdf/sdk`'s `doc.properties()` returns, in four sections. It merges the Info and XMP fields, and XMP wins where they differ. It gives the pages with their label runs, the sizes by paper name, the fonts in a table, and the encryption with the author's permissions; it counts signatures and never verifies them. Each value has a copy button and Copy all copies every section as text, both through `onCopy`.
- **Back and Forward** sit before the page input when `ReaderTopBar` gets `history`: the page label each button goes to, or `undefined` to disable that button. They are named by where they go ("Back to page 2") and run `nav.back` and `nav.forward`.
- **The rail** (`LeftRail`, labelled "Sidebar" on screen) is closed until the user opens it, then reopens on the tab and at the width the user last left it. Its handle resizes it by pointer, or by the arrow keys, Home and End once focused. Below 1024 px wide it is a drawer over the canvas, which a press on the scrim or Escape closes.
- **The inspector** (`Inspector`) shows either a selection's details or a panel the user opened, such as `RepairDetails`. It opens with `layout.openInspector({ id, selection })` and never covers the canvas: the canvas narrows to make room. A selection's inspector closes when the selection is cleared (`layout.deselect()`) or on Escape. A panel the user opened closes with its own button, or with Escape from inside it. Give the inspector a `label` unless its content is a landmark itself.
- **Focus mode** (`view.focus-mode`, Ctrl+\ or ⌘\) hides the rail, the mode's toolbar and any panel the user opened, and keeps the status bar. Opening a panel, such as the repair details from the status bar, leaves focus mode. In the status bar, `FocusModeItem` shows that focus mode is on and leaves it when pressed. A selection still opens the inspector, and Escape clears the selection before it leaves focus mode. A find bar or tool toolbar passed as `secondaryToolbar` stays until it is closed. Focus mode lasts the window only.
- **Modes** register with `ModeRegistry.register({ id, title, icon, toolbar, enter, leave })`. The current mode's `toolbar` fills the slot under the top bar unless `secondaryToolbar` replaces it. Switching calls the old mode's `leave`, then the new mode's `enter`; `leave` clears what only the old mode could select. `shellActions` gives each mode its `mode.<id>` command. `useShell` registers Read alone, so the reader shows no mode switcher until a second mode exists.

**On a phone**, a window 599 px wide or less, the shell changes:

- `bottomBar` (a `PhoneBottomBar`) takes the status bar's place, within reach of a thumb: Search once find has a handler, Pages when the rail has tabs, and More.
- Pages opens the rail as the Contents sheet, and the inspector is a sheet too.
- More, in either bar, opens `MoreSheet`: the top bar's More items as rows sized for touch.
- `ReaderTopBar` keeps the file name and More, and slides away while the document scrolls down.
- Both bars stay clear of the notch and the home indicator when the page's viewport meta sets `viewport-fit=cover`.

`ReaderTopBar` shows the file name, with "Loading pages…" or "Locked" while those apply. Once a document is open, the bar adds the page input and the zoom. The page input takes a page number or a page label, and announces one that doesn't exist. The zoom steps in and out, and its percentage opens a menu of the fit modes and presets. Every button runs a command from the registry (next section), so the bar must sit inside a `CommandsProvider`.

The sidebar toggle and Commands appear once the shell handles `view.toggle-sidebar` and `palette.open`. `shellActions` handles the sidebar only when the shell has rail tabs to show.

The viewer comes in as any object with its store and four calls (`goToPage`, `jumpToPage`, `setZoom`, `rotate`), so `@laminapdf/viewer`'s viewer fits without this package importing it. `PasswordDialog` asks for a password whenever the controller needs one and passes each try to it.

The shell builds the More menu, and marks each item that needs a capability with `needs`. `useGatedEntries` then keeps the item, disables it with how to get the capability, or leaves it out on a platform that never has it.

## Commands

Every action the reader can take is a command in one `CommandRegistry`. The top bar, the menus, the palette, the shortcut help and the desktop app's native menus all read the same registry. So a command's title, its shortcut and whether it can run now are the same everywhere.

A command has an id (`namespace.verb-object`, such as `view.zoom-in`), a title, a group, its shortcuts and where it lives besides the palette. `enabled` says whether it can run now or why not, and `run` does it. Typed arguments come with a parser, which the palette also uses for text typed after the title (`go to 12`, `zoom 150`):

```ts
const registry = new CommandRegistry();
registerReaderCommands(registry);
registry.register({
  id: "view.toggle-thumbnails",
  title: "Show or hide thumbnails",
  group: "view",
  home: ["view-menu"],
  shortcuts: [{ mac: "T", other: "T", scope: "page" }],
  enabled: (ctx) => (ctx.viewer === undefined ? "Open a document first." : true),
  run: () => toggleThumbnails(),
});
```

`registerReaderCommands` adds the reader's 46 commands. Those the reader and the viewer can do run in this package. The rest, such as find, print and the sidebar, are the shell's: it passes a handler for each in the context's `actions`. Until it does, every view leaves the command out, as it leaves out a command whose capability the platform never has.

The shell gives `CommandsProvider` the registry and a `CommandContext`: the platform, the reader, the viewer, the page labels and its handlers. The provider also listens for shortcuts. Keys with Ctrl, Alt or ⌘ work anywhere; single keys such as Page Down or `+` work only while focus is in the canvas, and never in a text field. Views read the registry through two hooks and a component:

- `useCommand(id)` gives a button or menu item its title, shortcut, availability and `run`, and re-renders when they change.
- `useShortcutMap()` lists every bound shortcut by group, for the shortcut help and native menu accelerators.
- `RegistryPalette` is the palette: ranked rows with their shortcuts, recent commands first, disabled ones with their reason, and a page that asks for missing arguments.
- `ShortcutsDialog` is Keyboard shortcuts, built from `useShortcutMap()`: searchable, with the match count announced, the single-key switch, and the browser keys the reader takes while a document is open. Its rows mark a single key "Off" while single keys are off.

A Node test checks every shortcut against the keys the browser, the OS and assistive tech keep (`NEVER_BIND`), and against the browser keys a command may take only with a reason (`OVERRIDES`). It also fails when two commands share keys where their scopes meet. A new browser or OS key a command needs goes in `OVERRIDES`, with the reason.

## Editing

An open document's edit session comes in as any object with its store and three calls (`undo`, `redo`, `goTo`), so `@laminapdf/viewer`'s `viewer.edits` fits. Put it in the commands' context as `edits`. The editor's parts are in their own entry, `@laminapdf/ui/editing`, so an app can load them once a document opens: `EditArea`, `HistoryPanel`, the edit commands and the tools.

Give `ReaderTopBar` an `EditArea` as `editArea` while a document is open. Once the first edit is made, it shows how many changes are unsaved after the file name ("4 unsaved changes", announced politely), then Undo and Redo after a divider. Their names say which step they take back or make again ("Undo rotate page 5").

`registerEditCommands` adds the edit commands. `edit.undo` is ⌘Z or Ctrl+Z; `edit.redo` is ⇧⌘Z, or Ctrl+Y and Ctrl+Shift+Z on Windows and Linux. Both leave a text field its own undo. `edit.go-to-step` takes a step number. `edit.history`, `pages.rotate-right` and `pages.rotate-left` are the shell's: it knows the rail and the pages, so it gives their handlers in `actions`. Each command is disabled with the reason while the document can't be edited, such as an encrypted file.

`HistoryPanel` is the rail's History tab. It lists the file as opened, then one row per step with its pages and time; the current row is tinted and carries `aria-current="step"`, and undone steps are struck through. Choosing a row goes to that step in one call.

The list is a single Tab stop: Up and Down move between rows, Home and End go to the ends, and Enter goes to the step. It draws only the rows in view, so the 500 steps a session keeps cost nothing. When an undo restores only part of a step, a notice at the top says so until the next edit.

`ReaderController.setCloseGuard(guard)` asks before the user's open or close lets the document go: `guard` resolves true to let it go. The reader's own opens pass `{ discard: true }` to skip it, and `requestClose()` is the user's close.

**Interaction modes** say how pointer input on the page is read: `select`, `hand` (drag to scroll) and `textSelect`. Each is a tool module in a `ToolRegistry`: a name, a cursor, the viewer layer it draws into, whether it selects text, and its pointer handlers. `attachTools(registry, host)` passes pointer input on the viewer's element to the tool that is on. A later tool adds its name to `InteractionModes` by declaration merging, then registers:

```ts
declare module "@laminapdf/ui" {
  interface InteractionModes {
    measure: true;
  }
}

tools.register({ name: "measure", cursor: "crosshair", layer: "annot", selectsText: false, pointerDown: start });
tools.use("measure");
```

Shell modes (`ModeRegistry`, above) are different: a shell mode owns a toolbar, and the tools on that toolbar are interaction modes.

## Settings, appearance and announcements

The app's look is an `Appearance`: a theme (`light`, `dark`, `high-contrast`, or `system` to follow the OS), an accent (`teal`, `blue`, `plum`, `green`), motion (`system` or `reduce`) and whether single-key shortcuts are on. `applyAppearance(document.documentElement, appearance)` sets it as `data-theme`, `data-accent` and `data-motion`, which the tokens and styles read, so a switch re-renders nothing. `parseAppearance` reads a stored one, with a default for anything missing. Storing it is the app's job, and so is setting it before the first paint, before React runs.

`ViewSettingsButton` is the top bar's View button with its popover: theme and accent, the `page` section, then More settings…. `PageAppearanceFields` is that section for the page appearance: the chips, "This document" or "All documents" while a document is open, and a line saying photos and annotation colours stay as they are. `SettingsDialog` is the app's one Settings dialog. It shows the sections of a `SettingsRegistry` in the order they were added, so later work adds a section rather than a dialog:

```tsx
const [settings] = useState(() => new SettingsRegistry());
useSettingsSection(settings, "appearance", message("settings.appearance"), () => (
  <AppearanceSettings appearance={appearance} onChange={setAppearance} />
));
```

`Announcer` speaks what changes without focus moving. `polite(text, kind)` waits until nothing newer of the same kind, such as a page or a zoom, has come for a moment. So scrolling past twenty pages says only the last, and two kinds due together are said together. `assertive(text)` speaks at once, for errors. Render its `LiveRegions` once, for the app's life.

## The home screen, repairs and failures

`OpenDropZone` is the home screen's way in. While the zone shows, the whole window takes a dropped or pasted PDF, and Choose a file asks the platform's picker. Whichever way a file arrives, it opens when its name ends in `.pdf` or "%PDF-" appears in its first kilobyte; anything else is refused with a message. The shell passes the recent list and the privacy line as its children.

When the reader had to repair a damaged file to open it, `RepairDetails` goes in the inspector slot: one plain sentence for each kind of repair, over its code. When nothing could be recovered, `OpenFailure` takes the canvas and focuses Try another file. Every warning and error in `fixtures/codes.json` has a sentence, or shares its family's, and a test keeps it that way.

Two more parts cover special cases: a whole-file download and debugging. `WholeFileDownload` fills the canvas while a host without range support sends the whole file. `DebugMenu` switches the render backend; the shell puts it in the top bar's debug slot for `#debug` only.

## Trying the chrome

`pnpm --filter @laminapdf/ui demo` serves `examples/ui-demo.html`: the whole chrome over a stand-in viewer, built with the React Compiler as an app would build it. Open any PDF. A name with "locked" asks for the password "lamina", "damaged" opens with repairs, and "broken" can't be opened. Ctrl+K opens the command palette; the demo writes keys as on Windows and Linux, so a Mac uses Ctrl+K too. A browser test runs axe on the page in each of these states.

A Node test runs the compiler over every source file and fails on any function the compiler gives up on, unless that function opts out with `"use no memo"` and says why.

## Adding a component

1. Write its props type first, with TSDoc, and export it from `src/index.ts`.
2. Build it on the React Aria Components primitive that fits, and give its root a `paper-` class.
3. Style it in its own file in `src/styles/`, imported from `index.css`, inside `@layer paper.components`. Take every colour and length from a token. A test fails on a literal colour, a `px` value or a token that doesn't exist. A new size goes into `@laminapdf/tokens` as a component token.
4. Add its states to `test/states.tsx`, and its section to the table in `test/sources.test.ts`. The browser tests then check every state with axe in the light, dark and high-contrast themes, left to right and right to left, and check its targets at both densities. Add keyboard tests for anything interactive. Test a control that needs a capability against `fakePlatform` in `test/fakes.ts`, with the flags each case needs.

Run the browser tests with `cargo xtask pdf-runtimes chromium` (or `firefox`, `webkit`); `pnpm --filter @laminapdf/ui test` runs the Node checks.
