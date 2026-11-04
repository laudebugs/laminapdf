# @laminapdf/tokens

Every colour, size and font of Lamina PDF's Paper design language lives in one file, `tokens.json`, and this package turns it into the two forms the apps use: CSS custom properties named `--paper-<name>` and typed TypeScript constants. Components read these tokens and never write a colour or a size of their own, so changing a value in `tokens.json` changes it everywhere. The package is private to this repository for now.

## Using the tokens

Import the stylesheet once, from your app's entry module or main stylesheet:

```ts
import "@laminapdf/tokens/tokens.css";
```

Then set up to three attributes. The theme and the accent go on the same element, usually `<html>`; the page appearance goes on the element that holds the pages:

```html
<html data-theme="dark" data-accent="plum">
  <main data-page-appearance="sepia">…</main>
</html>
```

| Attribute | Values | Without it |
| --- | --- | --- |
| `data-theme` | `light`, `dark`, `high-contrast` | Light, or dark when the system prefers dark |
| `data-accent` | `teal`, `blue`, `plum`, `green` | Teal. High contrast ignores the accent |
| `data-page-appearance` | `original`, `sepia`, `dark`, `high-contrast` | Original |

Forced colours (Windows contrast themes) switch to the high-contrast tokens whatever `data-theme` says. Every selector in the stylesheet sits inside `:where()`, so the blocks win by their order alone and any rule of yours overrides them.

In TypeScript, the same values come as constants. `THEMES` and `PAGE_APPEARANCES` hold one record per accent, so a renderer can read a page's paper and selection tint without a stylesheet:

```ts
import { cssVar, PAGE_APPEARANCES, SPACE, THEMES } from "@laminapdf/tokens";

THEMES.dark.blue.accent; // "#91B3DC"
PAGE_APPEARANCES.sepia.teal["selection-tint"]; // "#C5CCBE"
SPACE[16]; // 16 (px)
cssVar("surface"); // "var(--paper-surface)"
```

Sizes are numbers in px and durations numbers in ms. Colours are CSS colour strings. In the high-contrast theme they are system colour names such as `Canvas`, which only a browser can resolve.

## How the values are organised

The tokens come in three layers, and each layer reads only the one below it. Primitives are the raw palettes and scales, such as `primitive.color.warm.968`, where the number is the colour's OKLCH lightness times 1000. Primitives never become custom properties, so no component can reach one. Semantic tokens carry the design language's names (`ground`, `surface`, `text`, `muted`, `accent`, `focus`), with one value per theme and accent. Component tokens hold the sizes the design language fixes for one component, such as `top-bar-height` and `focus-ring-width`.

| Group in `tokens.json` | Becomes | Example |
| --- | --- | --- |
| `theme.<theme>` and `accent.<accent>.<light or dark>` | The semantic colours and shadows | `--paper-surface` |
| `accent.<accent>.page` | Each page appearance's tint of the accent | `--paper-page-sepia-selection-tint` |
| `appearance.<appearance>` | The page colours, under `data-page-appearance` | `--paper-page-ink` |
| `annotation`, `content` | Fixed colours, the same in every theme | `--paper-annotation-yellow` |
| `semantic`, `font`, `type` | Space, radius, font stacks, type roles | `--paper-space-16`, `--paper-type-body` |
| `component`, `breakpoint`, `motion` | Component sizes, breakpoints, durations | `--paper-rail-width` |

A type role is two properties: `--paper-type-body` is a `font` shorthand, and `--paper-type-body-letter-spacing` holds the letter spacing. CSS can't read a custom property inside a media query, so use `BREAKPOINTS` from TypeScript for those.

## Changing a token

Edit `tokens.json`, then regenerate the CSS, the TypeScript and the contrast table:

```sh
pnpm --filter @laminapdf/tokens generate
```

The package's tests run the same generator with `--check` first, so a stale output fails them. The file follows the Design Tokens Format Module 2025.10 of the W3C Design Tokens Community Group: `$value`, `$type` (inherited from the closest group), `$description`, and aliases written as `{group.name}`. Lamina PDF's own fields sit under `$extensions["dev.lamina"]`:

| Field | Meaning |
| --- | --- |
| `status` | `from-files` (copied from the reviewed design files), `proposed` (new, waiting for the owner) or `approved`. Every token above the primitives has one |
| `role` | What a colour does: `background`, `text`, `boundary`, `focus` or `decorative` |
| `contrast` | The pairs the colour must pass, described below |
| `system` | In the high-contrast theme, the CSS system colour the token maps to |
| `followsAccent` | The alias names the Teal accent, and the generator swaps in each accent |
| `reducedMotion` | The duration under `prefers-reduced-motion` |

A colour's role decides what it owes the contrast test. A `text`, `boundary` or `focus` colour declares at least one pair. At least one pair must sit on each `background` colour. A `decorative` colour, such as a hairline, needs no pair. So a new colour can't skip the test: it fails until it has a role and the pairs that role needs.

## Declaring a contrast pair

Each pair names the background the colour sits on and the ratio it needs:

```json
"contrast": [
  { "on": "{surface}", "min": 4.5 },
  { "on": "{ground}", "min": 3, "kind": "boundary" },
  { "on": "{hover}", "over": "{ground}", "min": 4.5 },
  { "on": "{ground}", "min": 3, "kind": "boundary", "decorative": true }
]
```

A short name such as `{surface}` means that token in the same theme and accent (or the same page appearance), and a full path such as `{appearance.original.paper}` means that token anywhere. `kind` is `text` (the default, at least 4.5:1), `boundary` or `focus` (at least 3:1), following WCAG 2.2's success criteria 1.4.3 and 1.4.11. A translucent background, such as `hover`, names what it sits `over`, and the test composites it first. A pair marked `"decorative": true` or `"exempt": "<reason>"` is measured and reported but never fails. `fg` replaces the token as the foreground, for pairs such as page ink on a highlight colour.

The tests measure every pair in the light and dark themes with each of the four accents and in every page appearance, and fail with a list of the pairs below their minimum. [`CONTRAST.md`](CONTRAST.md) lists every pair and its ratio, one column per accent. Pairs of system colours read `browser` there: the browser tests measure them in Chromium, Firefox and WebKit with each engine's own system colours.
