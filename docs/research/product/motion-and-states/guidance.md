# Motion and interaction states for Paper: research

Date: 2026-11-14. Read on `main` at `cd888aa2d` (read only).
Labels: **[Paper]** = what the repo already says, **[Source]** = outside guidance with a URL, **[Rec]** = my recommendation (an opinion, for the owner to decide).

---

## Part A. What Paper already says

### A.1 Rules that exist today, quoted

**Personality and principles: the frame that motion must serve**

- `docs/plans/ui-ux.md:11`: "Make lamina feel like **the fastest, calmest PDF app on every platform**".
- `docs/plans/paper-design.md:22`: Paper is "Calm, editorial, book-like".
- `docs/design/paper/design-language.md:14`: "The chrome recedes like the margins of a well-set book, so the page is the only thing that speaks … when a screen feels busy, remove chrome rather than adding colour."
- `docs/plans/ui-ux.md:55`: "**Accessible by construction.** WCAG 2.2 AA: 24 px minimum targets (44 px on touch) …"

**State colours (§2.1 light theme; dark theme at lines 93, 100–102; high contrast at 112, 115)**

- `design-language.md:19`: "Thresholds: text ≥ 4.5:1; UI component boundaries, focus rings and state indicators ≥ 3:1."
- `:37`: `accent-strong` `#16454B`: "Link hover; primary hover".
- `:39`: `accent-tint`: "Selected rows and pressed toggles".
- `:41`: `focus` `#1F5C63`: "Focus ring: same as `accent`".
- `:47`: `hover` `rgba(31,29,26,0.06)`: "Hover fill of ghost and icon buttons".
- `:48`: `pressed` `rgba(31,29,26,0.10)`: "Pressed fill … ⚠ muted 4.38: a pressed control's label switches to `text-3`".
- `:49`: `disabled`: "Disabled controls (exempt from contrast; also set `disabled`/`aria-disabled`)".
- `:58`: "The *selected* state never relies on the fill alone: a selected annotation gets a 2 px `focus` outline …".
- `:100–101` (dark): `hover` `rgba(236,231,222,0.08)`, `pressed` `rgba(236,231,222,0.14)`.
- `packages/tokens/tokens.json:2354–2380`: `hover` and `pressed` are `approved`, role `background`. A `primary-pressed` token (`#123A3F`) exists per accent (`tokens.json:4243`, `4608`, …).

**§7.0 Shared states: the only state table** (`design-language.md:544–557`)

| Line | State | Rule |
| --- | --- | --- |
| 548 | Default | ghost: transparent, `text-3`; secondary: white, `control-border`; primary: `accent`; toggle: transparent with a transparent 1 px border |
| 549 | Hover **(proposed)** | `hover` fill; secondary "`hover` fill over white"; primary `accent-strong`; toggle `hover` fill |
| 550 | Pressed **(proposed)** | "`pressed` fill, label `text-3`"; primary `#123A3F`; toggle "—" |
| 551 | Selected | toggle only: "`accent` border, `accent-tint` fill, `accent` 600" |
| 552 | Focus (keyboard) | "`outline: 2px solid #1F5C63; outline-offset: 2px` (1 px offset where space is tight … `-2px` inset inside edge-to-edge phone bars)" |
| 553 | Disabled **(proposed)** | "`disabled` text, no hover"; primary "`hairline` fill and border, `disabled` text" |

- `:555`: "Use `:focus-visible`, never remove the ring, and every board shows the ring on one control."
- `:557`: on dark media the ring is "2 px white with offset 2".
- `:559`: "Blocked primary … `aria-disabled="true"`, stays focusable, and has `aria-describedby` pointing at the strip that says why."
- ADR 0024 `:168`: component tokens include "focus-ring width and offset"; generated as `--paper-focus-ring-width: 2px`, `-offset: 2px`, `-offset-tight: 1px`, `-offset-inset: -2px`.

**Per-component state notes**

- `:606` mode switcher: "Hover **(proposed)**: label turns `text`." Inactive keeps "transparent underline (keeps height stable)".
- `:671` thumbnails: "Keyboard focus (proposed) is distinct from the current page: a rounded (radius 6) 2 px `focus` ring …".
- `:695` outline row: "Current: `accent` text 600 on `accent-tint`, `aria-current="location"`. Hover **(proposed)**: `hover` fill."
- `:718` status bar: "Pressed item | While its panel or popover is open: `accent-tint` fill, 1 px `accent` border, `aria-expanded="true"`".
- `:1090` menu: "The opening button shows the `pressed` fill and `aria-expanded="true"`."
- `:1092` menu: "Active item: the palette's active-row style (`accent-tint`, inset 1 px `accent` ring)."
- `:1174`: a "Target" outline "is not the focus ring, so a board still shows one 2 px ring".
- `:1178` document tabs: "hover `hover` fill".
- `:959` fields: "error text below (12 `danger`, with `aria-describedby`)".
- `:1410` (canvas skeleton): `a{color:#1F5C63}a:hover{color:#16454B}`.

**Timing and motion**

- `:530` page pill: "fades out 1.5 s after scrolling stops" (token `duration-pill-fade` 1500 ms, approved, `tokens.json:1713`).
- `:1006` toast: "4 s, paused while hovered or focused" (`duration-toast` 4000 ms, `from-files`, `tokens.json:1701`).
- `:1034` progress: "Indeterminate: the same bar with a 30% segment sliding (1.2 s, linear); with `prefers-reduced-motion`, a static 30% segment plus the label." (`duration-progress-cycle` 1200 ms with `reducedMotion: 0`, `tokens.json:1725–1739`).
- `:1035`: "**Spinner:** avoid; pages use placeholders, sidebars skeletons (`#ECE7DE` blocks, no shimmer under reduced motion)."
- `:1105` tooltip: "shown on hover after 500 ms and on keyboard focus; hoverable; Escape dismisses." (`duration-tooltip-delay` 500 ms, `tokens.json:1741`).
- `:1106` link preview: "Shown after 300 ms on hover or focus". The viewer implements 300 ms hover/focus and 500 ms long-press (CLAUDE.md, T5a).
- `:746` mini toolbar: "Shown on `pointerup`". ADR 0024 `:125`: "It appears on `pointerup`, never during a drag".
- `:899` bottom sheet: "The sheet moves without a slide, so reduced motion changes nothing."
- `:233` accent switch: "swaps CSS custom properties under `[data-accent]`, with no re-render and no transition under reduced motion."
- `:235–236`: the cached theme and accent are applied "before first paint, so there is no flash of teal"; an account value that differs "applies it without animation".
- `reader-b.md:59`: "Switching theme only swaps CSS custom properties: no re-render, no layout change. With reduced motion there is no transition."
- `reader-a.md:78`, `:84`: rail skeletons with "no shimmer under reduced motion"; "Reduced motion: no fades between low-res and sharp."
- `:1371` checklist: "`prefers-reduced-motion`: no sliding sheets or shimmer, instant cross-fades".
- `:1364`: "Visible `:focus-visible` ring on every focusable element … focus never hidden under sticky bars or sheets (2.4.11)".
- `:1373`: "`forced-colors` mode keeps borders and focus rings visible (use real borders/outlines, not only shadows)".
- `packages/tokens/README.md:74`: the `reducedMotion` extension is "The duration under `prefers-reduced-motion`".

**Pointer and hover context**

- `:522`: "A coarse pointer (`pointer: coarse`) uses the touch rules (§8.7) at any width."
- `:1173`: pen hover preview "only while a pen hovers".
- `add-edit-content.md:397`: "hovering an object shows a 1 px `accent` outline."

**Cursors**: the design language never mentions `cursor`. The only cursor rules are in code (A.3).

### A.2 Gaps: what a component author can't decide today

1. **No transition durations or easings at all.** The `motion` group holds four delays and lifetimes (toast, pill, progress, tooltip delay). It has no duration for hover, press, enter, exit or panel movement, and no easing tokens (`tokens.json:1699–1752` has `$type: duration` only). So `shell.css` hard-codes `160ms ease-out` for the inspector and `phone.css` hard-codes `160ms ease` for the phone top bar.
2. **Whether hover and press animate at all.** Today every hover in `@laminapdf/ui` snaps; no component sets a colour transition. Nothing says whether that is intended.
3. **Hover-in vs hover-out, press-in vs release.** Not addressed.
4. **What may animate.** There is no rule that only `opacity`, `transform` and colour may transition. Two existing animations move layout: the inspector animates `inline-size` (`shell.css:60–66`) and the phone top bar animates `margin-block-start` (`phone.css:26–35`).
5. **Enter and exit of overlays** (menu, popover, tooltip, dialog, palette, toast, link preview, drawer rail): nothing says whether they fade, scale, slide or appear instantly. The checklist's "instant cross-fades" (`:1371`) is the only hint, and it contradicts itself: a cross-fade is not instant.
6. **What reduced motion means.** Three readings exist: "no transition" (`:233`, `reader-b.md:59`), "instant cross-fades" (`:1371`), and the code's Settings switch, which kills every animation and transition with `!important` (`settings.css:217–228`). WCAG 2.3.3 only targets motion, not colour or opacity (B.3). An author can't tell whether a 120 ms fade is allowed under reduced motion.
7. **Theme, accent and appearance switches** are animation-free only "under reduced motion" (`:233`). Nothing says whether they animate otherwise; I recommend they never do (B.2.5).
8. **Hover on touch.** There is no rule to use React Aria's `data-hovered` rather than `:hover`. The code mixes them: 22 `[data-hovered]` rules and 9 raw `:hover` rules (document tabs, threads, history, rail handle, properties rows, reader blocks). On touch, the raw ones stick after a tap.
9. **The states are incomplete.** §7.0 covers five buttons and toggles. It doesn't cover:
   - selected + hover, selected + pressed;
   - open/expanded (status bar and menu triggers each have their own rule);
   - pending/busy (code has `[data-pending]` → `cursor: progress`, no spec);
   - invalid;
   - read-only;
   - drag-over and drop target (only the full-window drop zone);
   - current (`aria-current`), indeterminate checkboxes.
   It also gives no **precedence order**: what wins when a control is selected, hovered and focused at once.
10. **Hover, pressed and disabled are still (proposed)** in §7.0 (`:549–553`), though the tokens are approved.
11. **Cursors are unspecified.** Code uses `cursor: default` on 19 button-like controls and `pointer` on 4: recent rows, thread parts, phone bottom bar. The docs site and website use `pointer` on their buttons (`site.css:240`, `site-kit/header.css:109`, `select.css:48`, `CopyMarkdown.astro:92`, `Example.astro:644`). PDF links in the viewer use `pointer` (`link-layer.ts:534`). So the same kind of button gets a different cursor in each app.
12. **Disabled cursor** (`default` vs `not-allowed`) and **pending cursor** (`progress` vs `wait`) are not decided.
13. **First paint.** The no-flash rule covers colours (`:235`), but nothing stops transitions from running when styles or the theme arrive late.
14. **Delays.** Tooltip (500) and link preview (300) are set. A second tooltip after the first ("warm-up"), menu submenu open delay, and hover-intent delays for row-revealed buttons (`reader-b.md:182`, `pages-navigation-extraction.md:118`) are not.
15. **The sites.** The docs site and website have no motion or state rules beyond the shared tokens.

---

## Part B. Outside guidance

### B.1 Motion systems: durations and easings

**Material 3** (values from the Android Material Components docs, which mirror the M3 tokens: <https://github.com/material-components/material-components-android/blob/master/docs/theming/Motion.md>; spec page <https://m3.material.io/styles/motion/easing-and-duration/tokens-specs>)

- Durations: short1–4 = 50/100/150/200 ms; medium1–4 = 250/300/350/400; long1–4 = 450–600; extra-long1–4 = 700–1000.
- Easing: standard `cubic-bezier(0.2, 0, 0, 1)` ("start and end on screen"), standard-decelerate `(0, 0, 0, 1)` ("enter the screen"), standard-accelerate `(0.3, 0, 1, 1)` ("exit the screen").
- Emphasized easing (a two-segment path), emphasized-decelerate `(0.05, 0.7, 0.1, 1)`, emphasized-accelerate `(0.3, 0, 0.8, 0.15)`.
- Rule: entering → decelerate, exiting → accelerate, staying on screen → standard. Standard is for utility motion; emphasized for expressive moments.

**IBM Carbon** (<https://carbondesignsystem.com/elements/motion/overview/>)

- **Productive** motion is "efficient, responsive, and subtle … button states, dropdowns, and data tables". **Expressive** is for "significant moments".
- Durations: `fast-01` 70 ms (button and toggle micro-interactions), `fast-02` 110 ms (fades), `moderate-01` 150 ms (small expansions), `moderate-02` 240 ms (expansion, toasts), `slow-01` 400 ms (large expansions), `slow-02` 700 ms (background dimming).
- Productive curves: standard `(0.2, 0, 0.38, 0.9)`, entrance `(0, 0, 0.38, 0.9)`, exit `(0.2, 0, 1, 0.9)`.
- Use standard instead of exit for something that leaves but stays nearby to be recalled, "such as a side panel".
- "Micro-interactions should use ease-out on user input and fall within 90–120ms"; "duration should scale with the distance".
- Carbon's own button transitions background, box-shadow, border-color and outline at `fast-01` with the productive entrance curve (`carbon/packages/styles/scss/components/button/_mixins.scss`, <https://github.com/carbon-design-system/carbon>).

**Fluent 2** (tokens: <https://github.com/microsoft/fluentui/blob/master/packages/tokens/src/global/durations.ts>, `curves.ts`; principles: <https://fluent2.microsoft.design/motion>)

- Durations: ultraFast 50, faster 100, fast 150, normal 200, gentle 250, slow 300, slower 400, ultraSlow 500 ms.
- Curves: decelerateMid `(0, 0, 0, 1)`, decelerateMax `(0.1, 0.9, 0.2, 1)`, accelerateMid `(1, 0, 1, 1)`, easyEase `(0.33, 0, 0.67, 1)`, linear.
- Principles: functional, natural, consistent; larger elements get more time.

**Atlassian** (<https://atlassian.design/foundations/motion/applying-motion>): the most explicit about states.

- Interactions: `instant` 0 ms for focus, 50 ms for list-item hover, 100 ms for pressed, 150 ms for emphasised hover and press. Hover and press use ease-out "practical" `cubic-bezier(0.4, 1, 0.6, 1)`.
- Transitions: popup exit 100 ms, popup enter 150 ms; modal exit 200 ms, enter 250 ms; page-level 400 ms.
- "Exits should be 50ms or 100ms shorter than the entrance." Entrances use ease-out, exits ease-in.

**GitHub Primer** (source: `primer/react/packages/react/src/Button/ButtonBase.module.css`, <https://github.com/primer/react>)

- `transition: 80ms cubic-bezier(0.65, 0, 0.35, 1); transition-property: color, fill, background-color, border-color;`
- `&:active { transition: none; }`: the press shows at once, and the release eases back.
- `cursor: pointer` on buttons.

**Adobe Spectrum 2** (`react-spectrum/packages/@react-spectrum/s2/style/spectrum-theme.ts`)

- Default transition 150 ms, `cubic-bezier(0.45, 0, 0.4, 1)`.
- Its default property list holds colours, opacity, shadow, transform and filter, never layout.

**Apple HIG Motion** (<https://developer.apple.com/design/human-interface-guidelines/motion>)

- "Add motion purposefully, supporting the experience without overshadowing it. Don't add motion for the sake of adding motion."
- "Make motion optional … avoid using it as the only way to communicate important information."
- "Aim for brevity and precision in feedback animations."
- "In apps, generally avoid adding motion to UI interactions that occur frequently."
- "Let people cancel motion. As much as possible, don't make people wait for an animation to complete before they can do anything."

**Summary of typical values across systems**

| Use | Range | Systems |
| --- | --- | --- |
| Hover and press colour | 50–150 ms; Primer 80, Carbon 70, Atlassian 50–150, Spectrum 150 | all |
| Press-in | 0 ms (Primer), or ≤ 100 ms | Primer, Atlassian |
| Small enter (tooltip, menu, popover) | 100–200 ms | M3 short3–4, Carbon moderate-01, Atlassian 150, Fluent fast/normal |
| Small exit | 75–150 ms (shorter than enter) | Atlassian, M3 accelerate |
| Panels, dialogs, sheets | 200–300 ms enter, 150–250 ms exit | M3 medium, Carbon moderate-02, Atlassian 200/250 |
| Large or page-level | 300–500 ms | M3 medium–long, Carbon slow-01, Atlassian 400 |

Easing is the same everywhere: decelerate (ease-out) for entering, accelerate (ease-in) for exiting, a standard ease-in-out for movement on screen, linear only for continuous progress.

### B.2 Hover and press transitions done right

1. **Animate only paint and compositor properties.** Use colour, background, border-colour, opacity and transform, never layout (width, height, margin, padding, inset). Josh Comeau: "Two properties — transform and opacity — are very cheap to animate" (<https://www.joshwcomeau.com/animation/css-transitions/>). List properties explicitly; never write `transition: all`. Primer and Spectrum both list properties (B.1).
2. **Press: instant in, eased out.** Primer sets `:active { transition: none }`, so the pressed fill lands on the frame the pointer goes down; the release eases back. Nielsen's 0.1 s limit is where a reaction feels instantaneous (<https://www.nngroup.com/articles/response-times-3-important-limits/>), so a press that fades in over 150 ms spends most of that budget.
3. **Hover: quick in, a little slower out.** Comeau: "make the enter animation quick and snappy, while the exit animation can be a bit more relaxed". The slower exit stops flicker when the pointer sweeps across a toolbar. Atlassian adds the hover-reveal versus hover-exit fades. Comeau's "doom flicker": never move the hover target itself; move an inner element.
4. **Frequency decides.** Apple: avoid motion on frequent interactions. Emil Kowalski: "How often users will see an animation is a key factor"; Raycast has no open animation, "the optimal experience"; repeated keyboard actions should not animate; "UI animations should generally stay under 300ms" (<https://emilkowal.ski/ui/you-dont-need-animations>). For Paper, that means:
   - the palette (⌘K), keyboard focus moves and active-row changes in lists and menus never animate;
   - the focus ring is instant (Atlassian: focus = `instant`).
5. **No transitions on first paint or theme switches.**
   - CSS Transitions start only when an element already had a before-change style, so first render doesn't transition (<https://www.w3.org/TR/css-transitions-1/#starting>). They do fire when a stylesheet, font or theme class arrives after first render.
   - next-themes' `disableTransitionOnChange` disables "all CSS transitions when switching themes" so that "your UI with different transition durations won't feel inconsistent" (<https://github.com/pacocoursey/next-themes>). It works by injecting `* { transition: none !important }` for one frame.
   - Paper already applies the cached theme before first paint; the switch itself also needs that suppression.
6. **Touch.**
   - React Aria's `data-hovered` is "like `:hover`, but not applied on touch devices, preventing sticky hover states".
   - `data-pressed` is "like the `:active` pseudo class, but removed when the pointer is dragged off".
   - `data-focus-visible` is like `:focus-visible` but not applied on input click or programmatic focus (<https://react-aria.adobe.com/quality>).
   - React Aria's button article explains why: on touch "the active state persists even when your finger is not over it", and the pseudo-classes "are also affected by mouse event emulation". Presses are also cancelled by scrolling or an overlay (<https://react-aria.adobe.com/blog/building-a-button-part-1>).
   - Outside React Aria (the Astro sites, the viewer's own DOM), gate hover styles with `@media (hover: hover) and (pointer: fine)`, Baseline widely available since 2018 (web-features `interaction`). Use `any-hover` only for hover-only extras on hybrid devices.
7. **Interruptible.** Use transitions, not keyframes, for state changes: transitions retarget mid-flight, keyframes restart (Kowalski). The inspector's keyframe `paper-inspector-in` can't reverse if the selection ends mid-animation.

### B.3 Reduced motion

- **WCAG 2.2 SC 2.3.3 Animation from Interactions (AAA):** "Motion animation triggered by interaction can be disabled, unless the animation is essential …" (<https://www.w3.org/WAI/WCAG22/Understanding/animation-from-interactions.html>). Changes of colour or opacity that don't change perceived size, shape or position are not "motion animation" by its definition. The Understanding page's errata touches blur, so check the current errata before relying on the blur case. Technique C39 is `prefers-reduced-motion`.
- **SC 2.2.2 Pause, Stop, Hide (A):** moving, blinking or scrolling content that starts by itself, runs beside other content and lasts more than 5 s needs a pause, stop or hide. Auto-updating content needs one whatever its length (<https://www.w3.org/WAI/WCAG22/Understanding/pause-stop-hide.html>). For Paper: the indeterminate progress bar loops while a job runs, so the reduced-motion static segment (`:1034`) is right. A long job's bar should also stop sliding when the job can't move, and the label carries the state.
- **What triggers vestibular symptoms.** Val Head on A List Apart: "Animations that move an object across a large amount of space are most apt to trigger a negative response"; parallax, scrolljacking and zoom are the worst; "Animation that involves only non-moving properties, like opacity, color, and blurs, are unlikely to be problematic" (<https://alistapart.com/article/designing-safer-web-animation-for-motion-sensitivity/>). web.dev: reduce means removing "all non-essential movement" (<https://web.dev/articles/prefers-reduced-motion>). Apple: "Make motion optional" (B.1).
- **So reduced motion means replace movement, not remove feedback.**
  - Keep the hover, press, selected and focus colour changes: they are information, not motion.
  - Replace slides, scales and zooms with a short opacity fade or an instant change.
  - Stop shimmer and looping movement.
  - Paper's own Settings switch (`settings.css:217`) goes further and removes fades too, which is allowed (more than AA asks) but leaves overlays popping in. Owner decision: D2 below.

### B.4 Modern CSS for enter and exit (support from web-features 3.46.0 on unpkg, read 2026-11-14)

| Feature | Baseline | Chrome / Edge | Firefox | Safari | Use in Paper |
| --- | --- | --- | --- | --- | --- |
| `@starting-style` | Newly available 2024-08 | 117 | 129 | 17.5 | CSS-only enter transitions for elements that appear (`display` from none, new DOM). Must follow the rule it overrides with the same specificity (MDN <https://developer.mozilla.org/en-US/docs/Web/CSS/@starting-style>) |
| `transition-behavior: allow-discrete` | Newly available 2024-08 | 117 | 129 | 17.4 | Exit transitions of `display: none` |
| `overlay` (top layer kept during exit) | **Not Baseline** | 117 | — | — | Exit of native `popover`/`<dialog>` is cut short in Firefox and Safari. React Aria's overlays are portals, not top layer, so this matters only if Paper adopts native popovers |
| View Transitions, same document | Newly available **2025-10-14** | 111 | 144 | 18 | The web reader is a single-page app: mode switches, the Reader toggle, opening Organise. MDN <https://developer.mozilla.org/en-US/docs/Web/API/View_Transition_API> |
| `view-transition-class` | Newly available 2025-10 | 125 | 144 | 18.2 | Style groups of named transitions |
| `:active-view-transition` | Newly available 2026-01 | 125 | 147 | 18.2 | Types and conditional styles |
| View Transitions, cross document | **Not Baseline** | 126 | — | 18.2 | Docs site and website (Astro, multi-page): progressive only; a browser without it just navigates |
| Element-scoped view transitions | Not Baseline | 147 | — | — | Not yet |
| `linear()` easing | Widely available (2023-12) | 113 | 112 | 17.2 | Springs as CSS: an approximated spring for a sheet snap, with no JS |
| `interpolate-size` / `calc-size()` | Not Baseline | 129 | — | — | Animating to `auto` height; avoid (layout anyway) |
| Scroll-driven animations | Newly available 2026-10-27 | 115 | 159 | 26 | Not needed |
| `prefers-reduced-motion` | Widely available | 74 | 63 | 10.1 | Everywhere |

- **The desktop app's webviews matter.**
  - WebView2 on Windows is evergreen Chromium.
  - WKWebView on macOS follows the installed Safari version.
  - WebKitGTK on Linux depends on the distribution's package and can trail Safari by a lot.
  - Anything above must degrade to "appears instantly" when unsupported, never to "doesn't appear".
- **View transitions and reduced motion.** Wrap `::view-transition-*` animations in `@media (prefers-reduced-motion: no-preference)`, or replace them with the root's default cross-fade, which is opacity only.
- **React Aria Components** exposes `data-entering`/`data-exiting` on Popover, Modal and Tooltip (<https://react-aria.adobe.com/Popover>), with a `shouldSkipAnimation` prop. Paper uses neither today (no match in `packages/ui/src`). Those hooks are the natural place for enter and exit styles. Check the installed RAC version's docs for whether exit waits on transitions as well as keyframe animations before relying on a transition-only exit.

### B.5 Cursor conventions

- **The spec.** CSS Basic User Interface 4 defines `default` as "The platform-dependent default cursor. Often rendered as an arrow" and `pointer` as "The cursor is a pointer that indicates a link" (<https://www.w3.org/TR/css-ui-4/#cursor>). Browsers give `<button>` the arrow.
- **Against a hand on buttons.**
  - Hidde de Vries quotes Microsoft: "The hand pointer must mean 'this target is a link' and nothing else". Apple's macOS guidelines tie the hand to "the content is a URL link". OS buttons and browser chrome show the arrow (<https://hidde.blog/some-pointers-on-default-cursors/>).
  - Adam Silver, "Buttons shouldn't have a hand cursor" (<https://medium.com/simple-human/b11e99ca374b>).
  - Tailwind v4 removed `cursor: pointer` from buttons in its preflight (PR #8962, <https://github.com/tailwindlabs/tailwindcss/pull/8962>); shadcn/ui documents adding it back.
- **For a hand on buttons.** The web has trained people to expect a hand on anything clickable. De Vries concedes "there are now two standards" and "either one is okay to use", and asks for consistency within a product.
- **What systems do**
  - Default (arrow) on buttons:
    - Radix Themes ships cursor tokens `--cursor-button: default`, `--cursor-menu-item: default`, `--cursor-checkbox/radio/switch/slider-thumb: default`, `--cursor-link: pointer`, `--cursor-disabled: not-allowed`. "By default, interactive elements that don't link to another page use the regular arrow cursor", and teams can switch to pointer by overriding one token (<https://www.radix-ui.com/themes/docs/theme/cursors>; source `radix-ui/themes/.../tokens/cursor.css`).
    - Adobe Spectrum 2 sets no cursor on Button, ActionButton or Link (S2 source), so buttons keep the browser's arrow.
    - React Aria is unstyled and sets none.
  - Pointer on buttons: Primer (`cursor: pointer`, disabled `not-allowed`), Carbon (`cursor: pointer`, disabled `not-allowed`), Material Web (`cursor: pointer`, disabled `default`).
- **Rule of thumb, both camps.** `text` over editable and selectable text, `grab`/`grabbing` for draggable things, `col-resize`/`row-resize` for splitters, `progress` while the app stays usable during work, `wait` only when it doesn't, `crosshair` for drawing and creation tools.

### B.6 "Delight" in a productivity tool

- **Responsiveness over decoration.** Nielsen: 0.1 s feels instantaneous; at 1 s flow holds but the delay is noticed, so show work in progress; at 10 s attention is lost, so show percent-done and a way to cancel (<https://www.nngroup.com/articles/response-times-3-important-limits/>). Paper's placeholders, low-res pass and sharp tiles (`reader-a.md:70–84`) already follow the 1 s rule.
- **Perceived performance.**
  - Skeletons and placeholders at the content's true size; Paper has these, with no shimmer.
  - Optimistic UI: apply the edit at once, reconcile after. Linear's speed is widely attributed to a local object store and optimistic updates; the sources are secondary write-ups, for example <https://performance.dev/how-is-linear-so-fast-a-technical-breakdown>. Lamina's editor is local by design, so an op's visible result can land on the next frame; the save is the only slow step.
  - Fast animations read as fast software (Kowalski).
- **Restraint.**
  - Linear's 2026 refresh is explicitly "a calmer interface": not every element carries equal weight, fewer icons and borders, a dimmer sidebar (<https://linear.app/now/behind-the-latest-design-refresh>, <https://linear.app/changelog/2026-03-12-ui-refresh>).
  - Apple: "Gratuitous or excessive animation can distract people and may make them feel disconnected or physically uncomfortable."
  - Kowalski: delight belongs where users rarely interact, "so the delight doesn't wear off".
- **Micro-interactions with purpose.**
  - Dan Saffer, *Microinteractions* (O'Reilly, 2013; <https://www.oreilly.com/library/view/microinteractions/9781449342760/>), splits each one into **trigger → rules → feedback → loops and modes**. Feedback should be proportional to importance (his "foghorn" test). Too many modes confuse.
  - For Paper, the candidates that earn feedback are the moments the product is about:
    - a save that kept signatures valid;
    - a redaction proved removed;
    - the network shield staying at 0;
    - an undo;
    - a copy;
    - a page reorder settling.
  - Each gets a small, quick, purposeful confirmation (a toast, a check, a settle), not an ornament.
- **Things, Figma, Arc** are common examples: Things' Magic Plus button, Figma's instant canvas, Arc's playful but rare flourishes. I didn't fetch primary sources for them, so treat these as uncited impressions. The pattern they share is lavish care in rare moments and none in the hot path.

---

## Proposal for Paper (recommendations, for the owner)

### P1. Motion tokens (`tokens.json` › `motion`; DTCG `duration` and `cubicBezier` types)

Durations (`--paper-duration-*`):

| Token | Value | Reduced motion | Use |
| --- | --- | --- | --- |
| `duration-instant` | 0 ms | 0 | Focus ring, active row in lists, menus and palette, selection, anything keyboard-driven, the palette's open and close, press-in |
| `duration-state-in` | 80 ms | 80 (colour only) | Hover-in, release back to hover; colour, background, border only (Primer 80, Carbon 70) |
| `duration-state-out` | 150 ms | 150 (colour only) | Hover-out (Comeau, Atlassian 150) |
| `duration-enter-small` | 120 ms | 120 opacity only | Tooltip, link preview, menu, popover, mini toolbar: fade with ≤ 4 px translate or scale from 0.97 at the anchor |
| `duration-exit-small` | 80 ms | 80 opacity only | Their exit (Atlassian: exit 50–100 ms shorter) |
| `duration-enter-panel` | 200 ms | 120 opacity only, or 0 | Dialog, inspector, rail drawer below 1024 px, toast, sheet detent snap |
| `duration-exit-panel` | 150 ms | 80 opacity only, or 0 | Their exit |
| `duration-view` | 250 ms | 0 or the root cross-fade | Same-document view transitions (Reader toggle, Organise); the ceiling for any UI motion |

Keep `duration-toast` 4000, `duration-pill-fade` 1500, `duration-progress-cycle` 1200 (reduced: 0) and `duration-tooltip-delay` 500. Add `duration-link-preview-delay` 300, which is already the viewer's figure (`:1106`), so it has a token.

Easings (`--paper-ease-*`):

| Token | Value | Use |
| --- | --- | --- |
| `ease-standard` | `cubic-bezier(0.2, 0, 0, 1)` (M3 standard) | Anything that stays on screen: colour states, detent snaps, reorder settle |
| `ease-enter` | `cubic-bezier(0, 0, 0.2, 1)` (decelerate; between M3 and Carbon productive entrance) | Things appearing |
| `ease-exit` | `cubic-bezier(0.3, 0, 1, 1)` (M3 standard-accelerate) | Things leaving for good; use `ease-standard` for a panel that will come back (Carbon) |
| `ease-linear` | `linear` | Progress loops and the pill fade only |

Rules that go with them:

- Transition properties are listed, never `all`. They are colour, background-color, border-color, outline-color, opacity, transform, box-shadow, and nothing that moves layout.
- So the inspector's open becomes a `transform`/`opacity` enter on a docked panel whose width is set at once. The phone top bar slides with `transform: translateY(-100%)`, which makes room with no margin animation.
- Press is instant: `[data-pressed] { transition-duration: 0ms }`. Release uses `duration-state-in`.
- No transition on first paint, theme, accent, page appearance, density or direction changes. One `[data-motion-suppressed]` on `:root` for a frame around each switch, like next-themes. It also covers the account-sync re-apply (`:236`).
- Nothing used on most keystrokes animates (Kowalski's frequency rule): the palette, find next and previous, page input, zoom steps, mode shortcuts.
- Under reduced motion, transforms and sizes go to zero movement; colour and opacity stay unless Settings › Reduce motion is on (D2).
- Every enter has its exit (no pop-out after a fade-in), and an exit never blocks input: the element is `inert` and `pointer-events: none` from the moment it starts leaving (Apple: let people cancel motion).

### P2. State model every interactive element must style

Five independent axes. A component styles each one it has. The **precedence** below is the order in which rules apply, so later wins on the properties it sets. In CSS, write them in this order inside `@layer paper.components`, with React Aria's data attributes (`data-hovered`, `data-pressed`, `data-focus-visible`, `data-selected`, `data-disabled`, `data-pending`, `data-invalid`, `data-open`/`aria-expanded`, `data-drop-target`, `data-current`).

1. **Rest**: the default look.
2. **Value**: selected / checked / indeterminate / current (`aria-current`) / open (expanded, its popover showing). These set the base fill: `accent-tint`, `accent` text or border, the underline. Open uses the `pressed` fill on a ghost trigger (`:1090`) or `accent-tint` with an `accent` border on a status item (`:718`). One rule for both is a decision (D4).
3. **Interaction**: hover, then pressed (pressed wins over hover).
   - `hover` and `pressed` are translucent, so they layer over the value fill: `background-image: linear-gradient(var(--paper-hover) 0 0)` over the base `background-color`. Selected + hover and selected + pressed then need no extra tokens.
   - The primary button swaps solids instead: `accent` → `accent-strong` → `primary-pressed`.
   - A label on `pressed` switches `muted` → `text-3` (`:48`).
   - Hover only from `data-hovered`, or behind `@media (hover: hover) and (pointer: fine)` outside React Aria.
4. **Validity**: invalid sets the border and help text to `danger` and adds `aria-invalid` and `aria-describedby`. It never changes the focus ring's colour.
5. **Focus**: `data-focus-visible` draws the 2 px `focus` outline at its offset. The outline is independent of fills, so it shows in every state above, and it is never transitioned.
6. **Availability**, last, overriding 3:
   - **Disabled**: `disabled` text and `hairline` border, no hover or press styles, no focus (native `disabled`).
   - **Blocked / `aria-disabled`**: keeps focus and its ring, no hover fill, a readable label (`:559`).
   - **Pending / busy**: the hover and press fills are suppressed, `aria-busy` is set, the label shows the progress, and the cursor is `progress`.
   - **Read-only**: no hover fill, text still selectable.

Plus, where the element can receive drops: **drag-over / drop target** = `accent-tint` fill with a 2 px dashed `accent` inset (as the window drop zone, `:1025`). It applies at level 3 and wins over hover.

A component's tests (the gallery's `states.tsx`) should render each combination it supports: rest, hover, pressed, focus-visible, selected + hover, selected + focus, disabled, pending, invalid + focus. U6f's visual tests then catch drift.

### P3. Cursors (tokens like Radix's, so one switch flips them)

| Token | Recommended value | Applies to |
| --- | --- | --- |
| `--paper-cursor-action` | `default` | Buttons, icon buttons, toggles, tabs, mode switcher, menu items, check boxes, radios, switches, swatches, rows that act (open, select) |
| `--paper-cursor-link` | `pointer` | `<a href>` in chrome and on the sites, and PDF links in the viewer (already `pointer`) |
| `--paper-cursor-disabled` | `default` | Disabled and blocked controls. `not-allowed` reads as an error and is not used by macOS or Windows controls |
| `--paper-cursor-pending` | `progress` | Pending buttons (already in code) |
| (fixed) | `text`, `grab`/`grabbing`, `col-resize`, `crosshair` | Text fields and the text layer; sheet grabber and drag grips; rail handle; creation tools (the ToolRegistry already declares its cursors) |

Why `default` for actions:

- The CSS spec, Apple and Microsoft agree.
- The desktop app sits beside native apps that never show a hand on a button.
- Paper's calm personality argues for the quieter cursor; Radix and Spectrum do the same.
- The hover fill is the affordance.

The four `pointer` buttons in `@laminapdf/ui` and the sites' `pointer` buttons would move to the tokens. The owner's "right cursor" may mean the hand, though, so this is decision D1.

### Decisions for the owner

- **D1. Cursor on buttons.** `default` everywhere (recommended), `pointer` everywhere (the web habit; Primer, Carbon, Material), or `default` in the apps and `pointer` on the docs site and website. One token switches it.
- **D2. Reduced motion.** Keep short colour and opacity fades under the system preference and remove only movement (recommended, per WCAG 2.3.3's definition). Settings › Reduce motion then either does the same or stays stricter, with no transitions at all as today. The checklist's "instant cross-fades" (`:1371`) should be reworded either way.
- **D3. Do hover and press animate at all?** Today they snap. The recommendation (80 in, 150 out, press instant) is a quiet improvement; snapping everywhere is also defensible for a calm tool (Apple's frequency rule). Either way it should be written down.
- **D4. Open/expanded trigger style.** One look for menu triggers and status items (the `pressed` fill, or `accent-tint` with an `accent` border).
- **D5. Overlay enter and exit.** Fade with a 0.97 scale at the anchor (recommended) vs fade only vs instant. The palette stays instant whichever is chosen.
