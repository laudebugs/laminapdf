# The link editor's panel added a second banner to the page

[Resolution](resolution.md).

## What we saw

The gallery's accessibility tests failed on every link editor state (`link-editor-page`, `-web`,
`-refused`, `-script`), in light, dark and high contrast, in Chromium, Firefox and WebKit: 36
failures. Each one reported the same two axe rules against the gallery's header:

- `landmark-no-duplicate-banner`: the document should not have more than one banner landmark.
- `landmark-unique`: landmarks should have a unique role or role and name.

The owner hit them running `cargo xtask gallery --update` for laudebugs/laminapdf#633.

## How to reproduce

Open the link editor's panel on a page that already has a banner, such as the reader with its top
bar, and run axe. The browser test "adds no second banner to a page whose top bar is its banner" in
`packages/ui/test/browser/overlays.browser.test.tsx` does this with a `<header>` and a `LinkPanel`.
Before the fix it reported both rules.

## Cause

`LinkPanel` (`packages/ui/src/edit/link-editor.tsx`, added in laudebugs/laminapdf#542) is a
`<section role="dialog">` portalled to `<body>`, and its title row was a `<header>`. A `<header>`
is a banner unless it sits inside sectioning content. The `<section>` would scope it, but
`role="dialog"` replaces the section's role, so nothing scoped it. While the panel was open, the page
had two banners: its own top bar (the gallery's header in the tests) and the panel's title row.

The other panels with a `<header>` (the History tab, the repair details) sit in a `<section>` or
`<aside>` with no role of their own, which keeps their headers scoped.
