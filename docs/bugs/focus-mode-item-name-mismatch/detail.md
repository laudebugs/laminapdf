# Focus mode's status-bar item fails axe: its name leaves out its shortcut

[Resolution](resolution.md).

## What was seen

Since the inverted-L shell merged (laudebugs/laminapdf#322, todo U1c), `cargo xtask gallery --test` fails 15 tests on `main`, all on the `shell-regions` route:

- **Nine axe tests**, `e2e/a11y.spec.ts`, in Chromium, Firefox and WebKit, each in the light, dark and high-contrast themes. axe reports `label-content-name-mismatch` on one button: "Elements must have their visible text as part of their accessible name."
- **Six screenshot tests**, `e2e/visual.spec.ts`, in Chromium. They had no baselines: the route was new, and its screenshots were never written.

No CI workflow runs the gallery's tests, so `main` stayed green. The failures showed only in a local run.

## How to reproduce it

```sh
cargo xtask gallery --test
```

In the reader, turn focus mode on (Ctrl+\ or ⌘\) and run axe on the page: `label-content-name-mismatch` flags the status bar's "Focus mode" item. axe-core 4.14.0 runs that rule by default (tag `wcag21a`, impact serious). `@laminapdf/ui`'s own shell axe test passed because it never turns focus mode on, so the item was never on the page it checked. The gallery's `shell-regions` route shows the shell in focus mode.

## Diagnosis

`FocusModeItem` (`packages/ui/src/shell/focus-mode.tsx`) showed "Focus mode" and a shortcut chip ("Ctrl \" or "⌘\"), but named the button with `aria-label="Focus mode"`. The visible text is "Focus mode Ctrl \", so the name held only part of it. WCAG 2.5.3 (Label in Name) asks for all of it, so that a person using speech input can say what they see.

The report suggested hiding the chip from the name with `aria-hidden` and giving the keys in `aria-keyshortcuts`. Two measurements ruled that out:

1. **axe ignores `aria-hidden` here.** The rule compares the name with the text a sighted person sees (`visibleVirtual` with its screen-reader flag off), so a chip hidden from assistive technology still counts. With `aria-hidden` on the chip, the rule still failed in Chromium.
2. **`aria-keyshortcuts` never reached the page.** The component already passed it to React Aria's `ToggleButton`, but the rendered button had no such attribute. `filterDOMProps` in `react-aria` 1.22.1 lets through `id`, the four labelling attributes (`aria-label`, `aria-labelledby`, `aria-describedby`, `aria-details`), `data-*` and a few global ones. `aria-keyshortcuts` isn't among them, so the keys were never announced. Hiding the chip would have left them nowhere at all.

The repository already has a button with a shortcut chip that passes the rule: the top bar's Commands button (`packages/ui/src/reader/top-bar.tsx`). It sets no `aria-label`, so its name comes from its content, chip included.
