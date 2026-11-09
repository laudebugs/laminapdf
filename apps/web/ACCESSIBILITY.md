# Accessibility of the web reader

Lamina PDF's web reader aims to meet WCAG 2.2 AA in its own interface and to work fully without a mouse. This page records what the reader does, what checks it on every pull request, and where it still falls short. The manual pass with screen readers has not run yet; its results go in the last section.

## What the reader does

- **Keyboard.** Every command runs from the keyboard, and the command palette (Ctrl K, ⌘K on macOS) reaches all of them. **Keyboard shortcuts** lists every shortcut; open it with `?` while the page has focus, or from More. It reads the same list the reader dispatches from, so the help always matches the keys. Single-key shortcuts such as Page Down and `+` act only while the page has focus, and Settings can turn them off (WCAG 2.1.4).
- **Keys taken from the browser.** While a document is open, the reader handles Ctrl F, Ctrl P and the few other keys listed at the foot of Keyboard shortcuts, because its own find and print work on the whole document. The browser keeps every other key.
- **Focus.** The first Tab stop is **Skip to document**, which moves focus to the pages. Dialogs keep focus inside them and hand it back on close. Following a link, an outline entry, a thumbnail or Go to page moves focus to the page it lands on; a find match leaves focus in the find field.
- **Structure.** The top bar, the sidebar (`nav`), the pages (`main`) and the status bar are landmarks. Each page is a region named by its label and position, such as "Page iv, 4 of 120".
- **Announcements.** A screen reader hears the document that opened and its page count, the page in view once scrolling settles, and a zoom the reader chose. Find says its match count, and failures show as alerts.
- **Themes and motion.** View settings offers Light, Dark, High contrast and Match system, and four accent colours. High contrast follows the system's colours (`forced-colors`). The stored theme and accent apply before the first paint, so the reader never flashes the default look. Settings › Appearance and accessibility can reduce motion whatever the system says. Page colours are separate from the theme, so a dark theme never inverts the page.

## What checks it on every pull request

- **axe**, in Chromium, Firefox and WebKit (`e2e/a11y.e2e.test.ts`). It runs on the home screen, an open document with its sidebar, find, View settings, Settings, Keyboard shortcuts and the palette, each in the light, dark and high-contrast themes. Any serious or critical violation fails the build.
- **A keyboard-only journey** in the same three browsers (`e2e/keyboard.e2e.test.ts`). It opens a file, skips to the document, pages through it, reads the help, finds a word, jumps to a page and back, and opens properties and print. It also checks the spoken page.
- **Reduced motion**: with motion reduced, nothing in the chrome animates and the pages never scroll smoothly.
- **Contrast**: every token pair of every theme meets its WCAG ratio (`@laminapdf/tokens`' tests), and `@laminapdf/ui`'s browser tests run axe on each component in each theme.

## Known limits

- **Only pages near the view exist in the page.** A screen reader's "read all" stops at the last mounted page. Use the outline, the page regions, Go to page or Page Down to move on.
- **F6 region cycling is the desktop app's.** Browsers keep F6 for their address bar, so the web reader offers the skip link and landmarks instead.
- **Page regions carry no `aria-setsize` or `aria-posinset`.** Those attributes are not allowed on a region; each page's name gives its position instead.
- **Palette rows.** axe's experimental rule `label-content-name-mismatch` is off for the palette. A row's group and shortcut describe the row rather than name it, as in a menu, and `@laminapdf/ui`'s tests check that description.
- **On a phone**, View settings opens Settings, since the phone's top bar has no View button to anchor its popover.

## Manual pass with assistive technology

Not run yet. The pass needs a Windows machine with NVDA and a Mac with VoiceOver. For each, record the date, the versions and every finding with its severity and issue link:

| Date | Assistive technology and browser | Versions | Findings |
| --- | --- | --- | --- |
| — | NVDA + Firefox, NVDA + Chrome (Windows) | — | — |
| — | VoiceOver + Safari (macOS) | — | — |
| — | Keyboard only | — | — |
