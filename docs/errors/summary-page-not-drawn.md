# `summary-page-not-drawn`

- Type: warning
- Kind: `unsupported` (valid input that Lamina PDF can't handle yet)

## What happened

`commentSummary` (in Rust, `Document::comment_summary`) listed a page's comments without drawing the page beside them. The rest of the summary is as asked: every comment on that page is still listed, with its number.

## Why

A comment summary is a PDF/UA-1 document, and the copy of a page has to keep it one. Two things about a page stop that:

- The page uses a font the file only names, such as Helvetica. PDF/UA-1 needs every font a page uses to be embedded, so a copy of the page would break the summary's conformance.
- Lamina PDF can't read the page's content to its end within the document's limits. The summary takes the page's tags out of its copy, since the drawn page is one figure of the summary, and content it can't read could still hold some.

Lamina PDF leaves the drawing out instead.

## How to fix

Nothing, if the list of comments is enough. To see a page set in fonts the file doesn't embed beside its comments, embed its fonts first, for example by printing the document to PDF again, then make the summary again. The list layout (`layout: "list"`, in Rust `SummaryLayout::List`) never draws pages and never raises this warning.

## Example

```ts
const summary = doc.commentSummary({ font });
for (const warning of summary.warnings) {
  if (warning.code === "summary-page-not-drawn") console.warn(warning.data);
}
```

## Data fields

- `page`: the page of the summarised document whose drawing was left out, from 0. The warning has no location of its own, since the page belongs to the summarised document, not to the summary.
- `reason`: `fonts-not-embedded` or `content-unreadable`.
