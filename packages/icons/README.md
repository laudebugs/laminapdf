# @laminapdf/icons

Every icon in Lamina PDF's Paper design language lives in this package as one SVG file, drawn on a 24-unit grid with a 1.5-unit stroke in `currentColor` and no fills. The package turns those files into two things the apps use. The root entry is a framework-free module of their markup, for the viewer, native menus and plain pages. A second entry holds a React `<Icon>` component. A lint test checks every file against the design language's rules, so a new icon follows the grid by rule rather than by eye. The package is private to this repository for now.

## Using the icons

Without a framework, `iconSvg` returns a complete `<svg>` string:

```ts
import { iconSvg } from "@laminapdf/icons";

button.innerHTML = `${iconSvg("find", { size: 18 })}<span>Find</span>`;
```

In React, `<Icon>` renders the same markup:

```ts
import { Icon } from "@laminapdf/icons/react";

<button aria-label="Find in document"><Icon name="find" size={18} /></button>
```

An icon is decorative by default: it gets `aria-hidden="true"` and `focusable="false"`, because the button around it carries the name. Name an icon-only button with a verb phrase, such as "Find in document"; `iconButtonProps(label)` returns the `aria-label` and throws on an empty one. Only an icon that stands alone, outside any labelled control, takes a `label` (`title` in `iconSvg`), which makes it `role="img"` with that accessible name.

Sizes are the ones the design language renders: 14 and 15 px in the status bar, 16 in buttons with a text label, 18 in the top bar, 20 in sheets, 22 on the phone, and 32 for empty states. The icon takes its colour from the text around it, so set `color` on the button: `text-3` in desktop chrome, `accent` when selected. Every icon draws at a 1.5-unit stroke except Check inside a checkbox, which takes `strokeWidth: 2`.

The React component is a separate entry so the root one never loads React. React 19 is an optional peer dependency.

## Adding an icon

1. Draw it on the 24-unit grid with the existing icons beside you, and save it as `svg/<name>.svg` inside the standard root: `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">`. Name the file in lowercase words joined by hyphens.
2. Run `pnpm --filter @laminapdf/icons lint:icons`. It fails on anything the rules forbid and warns when the shapes come closer than 3.5 units to the edge.
3. Run `pnpm --filter @laminapdf/icons generate` to rewrite the module, then add a row to a table in the design language's iconography section. A test fails until the row's markup matches the file.
4. For a second name of an existing icon, such as `upload` for `share`, add it to `aliases.json` instead of copying the file.

The lint allows only `path`, `circle`, `ellipse`, `rect`, `line`, `polyline` and `polygon` elements, with their geometry and nothing else, so any fill, style, class, transform, ID, or line cap and join setting fails. Shapes within 2 units of the edge fail too. Two exceptions are written into the rules by icon name: the icons drawn with dots (More, Drag grip, Tag) may use circles smaller than 1.5 units, and Mark area may use a dashed rectangle. The bounding box follows each path's drawn curve, arcs included, not its control points.

To see every icon at every size, in both themes, run `pnpm --filter @laminapdf/icons gallery`.

## Why a package of its own

The icons could have been a folder of the React component library. They are a package of their own because the viewer has no framework, and native menus and the docs site need the same glyphs without React. If no consumer outside React appears, folding the package into the component library takes one re-export.

## Icons each area needs

The set holds the design language's icons: the reader's, those the area boards drew, and six drawn with this package (fit width, fit page, error, positive, settings, clear). Each area's first screen todo draws the icons it still lacks, by the rules above, and adds them here and to the design language.

| Area (rounds) | Icons in the set | Still to draw |
| --- | --- | --- |
| Foundations and reader (C3–C7, U1–U4) | sidebar, pages, find, zoom-in, zoom-out, plus, check, fit-width, fit-page, single-page, two-pages, focus-mode, view-settings, rotate-left, rotate-right, back, forward, chevron-left, chevron-up, chevron-down, chevron-right, more, close, copy, share, download, printer, folder, file, info, error, positive, warning, help, settings, keyboard, lock, eye, eye-off, clear, external-link, bookmark, paperclip, reader, network-shield, repaired, signed-file | none |
| Desktop shell (D3–D7) | file, folder, printer, reload, download, bell, sync | none known |
| Edit session, organise and navigation (E1–E3) | undo, redo, clock, insert-page, extract, delete, crop, resize, move-to-page, rotate-left, rotate-right, list, filter, arrow-up, arrow-down, drag-grip, rename | duplicate |
| Comment and review (E4–E5) | highlight, underline, strike, caret, comment, comments-list, text-box, callout, stamp, ink, pen, eraser, lasso, shapes, rectangle, ellipse, line, arrow, polygon, polyline | reply |
| Add and edit content (E6–E7) | add-text, edit-text, text-not-editable, image, link, page-marks, tools, align-left, align-centre, align-right, move, resize, select | none known |
| Forms (E8–E9) | text-field, check-box, radio-group, dropdown, list-box, push-button, date, calculated, tab-order, detect-fields, snap-to-grid, reset | required field |
| Sign, protect and redact (E10–E12) | signed-file, signature-image, certificate, shield-cross, lock, key, smart-card, keyboard, redact, mark-text, mark-area | sanitise |
| Convert, compare and produce (E13–E17) | ruler, distance, perimeter, area, layers, flatten, structure, accessibility, tag, action, play, cube, replace | OCR, compare, batch |
| Account, cloud and product (H, E18, U5) | cloud, cloud-off, upload, offline, sync, server, bell, laptop, phone, add-to-home-screen, link | person (account) |
