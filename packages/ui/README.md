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

## Adding a component

1. Write its props type first, with TSDoc, and export it from `src/index.ts`.
2. Build it on the React Aria Components primitive that fits, and give its root a `paper-` class.
3. Style it in its own file in `src/styles/`, imported from `index.css`, inside `@layer paper.components`. Take every colour and length from a token. A test fails on a literal colour, a `px` value or a token that doesn't exist. A new size goes into `@laminapdf/tokens` as a component token.
4. Add its states to `test/states.tsx`, and its section to the table in `test/sources.test.ts`. The browser tests then check every state with axe in the light, dark and high-contrast themes, left to right and right to left, and check its targets at both densities. Add keyboard tests for anything interactive.

Run the browser tests with `cargo xtask pdf-runtimes chromium` (or `firefox`, `webkit`); `pnpm --filter @laminapdf/ui test` runs the Node checks.
