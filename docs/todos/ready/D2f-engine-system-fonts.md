# D2f — System fonts for the sandboxed engine, through the main process

| Field | Value |
| --- | --- |
| ID | `D2f` |
| Status | `ready` |
| Plan | [Desktop app §6 D2](../../plans/desktop-app.md#d2-shell-and-native-core) |
| Round / milestone | Round D (desktop) — M3 Desktop reader beta |
| Depends on | [D2c](../ongoing/D2c-engine-sandbox.md) |
| ADRs | Amends ADR 0008 §5 (fonts). Follows [Rust guidelines](../../guidelines/rust.md) (`unsafe` only with `// SAFETY:`), [0019](../../decisions/0019-supply-chain-and-licence-hygiene.md) |
| Branch | `feat/d2f-engine-system-fonts` |
| PR title | `feat(desktop): serve system fonts to the sandboxed engine` |
| Size | M (400–700 lines: PDFium font-info callbacks, two protocol messages, the main process's font list) |
| PR | — |

## Goal

Give the sandboxed engine the system's fonts again without giving it the file system. Since D2c the engine can't open a file, so PDFium's lookup of a font a PDF doesn't embed finds nothing and falls back to its built-in fonts (ADR 0008 §5, fonts). Latin text keeps its shapes and widths: PDFium's Foxit base-14 fonts and its multiple-master substitutes follow the PDF's `/Widths`. But a CJK document that doesn't embed its fonts loses its glyphs, and base-14 text looks a little different from an unsandboxed render on macOS, which maps Helvetica to the system's.

## Context

- D2c measured the difference: `golden/text-std14.pdf` and `golden/build-text-latin.pdf` render differently from an in-process PDFium render on macOS; `golden/text-latin.pdf` (fonts embedded) renders identically.
- PDFium's hook is `FPDF_SetSystemFontInfo` with an `FPDF_SYSFONTINFO` of callbacks: `EnumFonts` (call `FPDF_AddInstalledFont` per face), `MapFont`, `GetFont`, `GetFontData`, `GetFaceName`, `GetFontCharset`, `DeleteFont`.
- The web reader's PDFium (WebAssembly) has no system fonts either, so today both readers render the same.

## Scope

**In scope**

- The main process lists the system's font files (macOS: `/System/Library/Fonts`, `/Library/Fonts`, `~/Library/Fonts`; Windows: `%WINDIR%\Fonts` and the per-user folder; Linux: the fontconfig directories) and their face names, and sends the list after `Hello`.
- `FromEngine::FontRequest { req, font }` and `ToEngine::FontData { req }` (the bytes as the payload): the engine asks by the list's index, never by path, so a compromised engine can read only font files the main process listed.
- `laminapdf-render-pdfium`: an `FPDF_SYSFONTINFO` over that list, with `MapFont` matching like PDFium's own folder font info (face name, charset, weight, italic), and a small cache of font bytes.

**Out of scope**

- Granting the engine read access to font folders: ADR 0008 §5 keeps the engine's file system closed.

## Tasks

- [ ] The main process's font list and the two messages (protocol version 3).
- [ ] The PDFium font-info callbacks in the edge crate, `unsafe` with `// SAFETY:` comments.
- [ ] Measure the first page of a CJK document without embedded fonts, before and after.

## Acceptance criteria

- [ ] A CJK document without embedded fonts renders its glyphs in the sandboxed engine on every OS.
- [ ] The engine never names a path; the main process serves only listed font files.
- [ ] `engine_process.rs` compares `golden/text-std14.pdf` with an in-process render again.

## Post-merge checks

None.

## Tests

- Protocol round trips for the new messages; the fuzz target reads them.
- An engine test that renders a non-embedded CJK font and a request for an index outside the list.

## Docs to update

- ADR 0008 §5 (fonts), `apps/desktop/README.md`.

## Risks and open questions

- Font files are large (CJK fonts reach 20–50 MB); the engine caches a few, and a frame carries at most 64 MiB.
- `MapFont` matching decides what a reader sees: compare with PDFium's own choices on the fixtures.

## Definition of done

- [ ] Claimed: its work PR's first commit moved this file `ready/` → `ongoing/`
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
