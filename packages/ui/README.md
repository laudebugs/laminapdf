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

`add` returns a handle whose `dismiss()` removes the toast early.

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

`ReaderShell` lays out the reader: the top bar across the top, a mode's toolbar under it, then the rail, the canvas and the docked panel side by side, and the status bar along the bottom. Each part is a slot the shell fills, and the canvas is the page's `main` landmark:

```ts
<CommandsProvider registry={registry} context={context} keys="other">
  <ReaderShell
    topBar={<ReaderTopBar reader={controller} viewer={viewer} keys="other" sidebarOpen={sidebar} more={{ items, onAction }} />}
    rail={<PagesPanel />}
    status={<NetworkIndicator count="0 network requests" />}
  >
    <div ref={mountViewer} />
  </ReaderShell>
  <PasswordDialog reader={controller} />
  <RegistryPalette isOpen={palette} onOpenChange={setPalette} />
</CommandsProvider>
```

`ReaderTopBar` shows the file name, with "Loading pages…" or "Locked" while those apply. Once a document is open, the bar adds the page input and the zoom. The page input takes a page number or a page label, and announces one that doesn't exist. The zoom steps in and out, and its percentage opens a menu of the fit modes and presets. Every button runs a command from the registry (next section), so the bar must sit inside a `CommandsProvider`. The sidebar toggle and Commands appear once the shell handles `view.toggle-sidebar` and `palette.open`.

The viewer comes in as any object with its store and three calls (`goToPage`, `setZoom`, `rotate`), so `@laminapdf/viewer`'s viewer fits without this package importing it. `PasswordDialog` asks for a password whenever the controller needs one and passes each try to it.

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
  run: (ctx) => toggleThumbnails(),
});
```

`registerReaderCommands` adds the reader's 39 commands. Those the reader and the viewer can do run here. The rest, such as find, print and the sidebar, are the shell's: it passes a handler for each in the context's `actions`. Until it does, the command is left out of every view, as a command whose capability the platform never has is.

The shell gives `CommandsProvider` the registry and a `CommandContext`: the platform, the reader, the viewer, the page labels and its handlers. The provider also listens for shortcuts. Keys with Ctrl, Alt or ⌘ work anywhere; single keys such as Page Down or `+` work only while focus is in the canvas, and never in a text field. Views read the registry through hooks:

- `useCommand(id)` gives a button or menu item its title, shortcut, availability and `run`, and re-renders when they change.
- `useShortcutMap()` lists every bound shortcut by group, for the shortcut help and native menu accelerators.
- `RegistryPalette` is the palette: ranked rows with their shortcuts, recent commands first, disabled ones with their reason, and a page that asks for missing arguments.

A Node test checks every shortcut against the keys the browser, the OS and assistive tech keep (`NEVER_BIND`), and against the browser keys a command may take only with a reason (`OVERRIDES`). It also fails when two commands share keys where their scopes meet. A new browser or OS key a command needs goes in `OVERRIDES`, with the reason.

## The home screen, repairs and failures

`OpenDropZone` is the home screen's way in. While the zone shows, the whole window takes a dropped or pasted PDF, and Choose a file asks the platform's picker. Whichever way a file arrives, it opens when its name ends in `.pdf` or "%PDF-" appears in its first kilobyte; anything else is refused with a message. The shell passes the recent list and the privacy line as its children.

When the reader had to repair a damaged file to open it, `RepairDetails` goes in the inspector slot: one plain sentence for each kind of repair, over its code. When nothing could be recovered, `OpenFailure` takes the canvas and focuses Try another file. Every warning and error in `fixtures/codes.json` has a sentence, or shares its family's, and a test keeps it that way.

Two more parts cover special cases: a whole-file download and debugging. `WholeFileDownload` fills the canvas while a host without range support sends the whole file. `DebugMenu` switches the render backend; the shell puts it in the top bar's debug slot for `#debug` only.

## Trying the chrome

`pnpm --filter @laminapdf/ui demo` serves `examples/ui-demo.html`: the whole chrome over a stand-in viewer, built with the React Compiler as an app would build it. Open any PDF. A name with "locked" asks for the password "lamina", "damaged" opens with repairs, and "broken" can't be opened. Ctrl+K (⌘K on a Mac) opens the command palette. A browser test runs axe on the page in each of these states. A Node test runs the compiler over every source file and fails on any function the compiler gives up on, unless that function opts out with `"use no memo"` and says why.

## Adding a component

1. Write its props type first, with TSDoc, and export it from `src/index.ts`.
2. Build it on the React Aria Components primitive that fits, and give its root a `paper-` class.
3. Style it in its own file in `src/styles/`, imported from `index.css`, inside `@layer paper.components`. Take every colour and length from a token. A test fails on a literal colour, a `px` value or a token that doesn't exist. A new size goes into `@laminapdf/tokens` as a component token.
4. Add its states to `test/states.tsx`, and its section to the table in `test/sources.test.ts`. The browser tests then check every state with axe in the light, dark and high-contrast themes, left to right and right to left, and check its targets at both densities. Add keyboard tests for anything interactive. Test a control that needs a capability against `fakePlatform` in `test/fakes.ts`, with the flags each case needs.

Run the browser tests with `cargo xtask pdf-runtimes chromium` (or `firefox`, `webkit`); `pnpm --filter @laminapdf/ui test` runs the Node checks.
