# 0021 — PDF JavaScript: never run it; read the form helpers as data

- Status: Proposed (E8a; accepted when the owner signs off on its PR)
- Date: 2026-11-16

## Context

Forms are the next round (E8), and most real forms carry JavaScript. Their format, keystroke, validate and calculate actions (`/AA` `F`, `K`, `V`, `C`) are scripts, and almost all of them only call Acrobat's built-in helpers: `AFNumber_Format(2, 0, 0, 0, "$", true)`, `AFDate_FormatEx("mm/dd/yyyy")`, `AFSimple_Calculate("SUM", ["a", "b"])` ([04 §3.3](../research/pdf/04-interactive-security.md#33-calculation-order-and-scripts)). A filler that ignores them writes `/V` = `1234.5` where Acrobat shows `$1,234.50`, and a total field that never adds up.

JavaScript is also the PDF format's largest attack surface. Acrobat's script engine and its object bindings have been Reader's leading exploit primitive since `util.printf` (CVE-2008-2992) and `Collab.getIcon` (CVE-2009-0927), and still are: a use-after-free in `resetForm` (CVE-2023-21608) was exploited in the wild in 2023 ([CISA](https://www.cve.org/CVERecord?id=CVE-2023-21608)). Scripts also power phone-home tracking, form-data exfiltration and `app.alert` phishing ([04 §4](../research/pdf/04-interactive-security.md#4-javascript-in-pdf)). Even pdf.js shipped arbitrary code execution in 2024 (CVE-2024-4367). The bug was outside its scripting sandbox: pdf.js compiled glyph paths into `new Function` bodies with a font's `FontMatrix` in them, and fixed it by removing that path ([GHSA-wgrm-67xf-hhpq](https://github.com/mozilla/pdf.js/security/advisories/GHSA-wgrm-67xf-hhpq)). The lesson is wider than JavaScript: no code may be generated from document data.

The [full-editor plan](../plans/full-editor.md#9-open-questions) leaves one question open before forms ship (§9 Q1): never execute scripts, or run a restricted sandbox with only the Acrobat form-helper API. This ADR answers it for form behaviour (E8) and for authoring (E17), and states what detection, reporting and stripping do. In short: Lamina PDF never runs JavaScript from a document. It recognises the form-helper calls as data and does their work natively, reports every script it doesn't handle, and writes scripts only as text the user typed.

## Options

| | 1. Never execute; native helpers | 2. Sandboxed engine, opt-in | 3. Both: native first, sandbox for the rest |
| --- | --- | --- | --- |
| Covers | the AF* helper calls, `/CO` order, declarative actions | any script within the API we bind | both |
| New attack surface | a pattern matcher over script text | a JS engine, its bindings and their memory | the engine's |
| WebAssembly added | none | about 230–250 KB gzip (QuickJS or QuickJS-ng), plus a form-API layer of about 20 KB | the engine's |
| Same answer on web, desktop, CLI | yes | only if the engine ships everywhere | only if the engine ships everywhere |
| Deterministic output | yes | needs a frozen clock, no randomness, budgets | as 2 |
| Work before forms ship | the recogniser (E8f) | an engine crate, bindings, fuzzing, an external review | both |

The engine sizes were measured from the published packages on 2026-11-16. `quickjs-emscripten` 0.32.0's synchronous build is 503,134 B (231,517 B gzip), its QuickJS-ng build 528,551 B (247,167 B gzip), and its asynchronous build 1,027,523 B (360,492 B gzip). pdf.js ships the same engine as `quickjs-eval.wasm`, 469,105 B (216,718 B gzip), with a 62,403 B form-API bundle (20,396 B gzip). Boa (`boa_engine`) would be larger, by an estimate not measured here.

The candidate engines are permissively licensed, so licence doesn't decide it: QuickJS and QuickJS-ng (MIT), `rquickjs` (MIT), `quickjs-emscripten` (MIT) and Boa (MIT or Unlicense). None is GPL, so [ADR 0001](0001-core-language-and-licence.md) and [ADR 0019](0019-supply-chain-and-licence-hygiene.md) allow each. MuJS, MuPDF's engine, is AGPL and excluded.

Lamina PDF's renderers don't run scripts either. `cargo xtask fetch-pdfium` takes pdfium-binaries' plain builds, not their `pdfium-v8-*` ones, and without V8 PDFium's script runtime is a stub whose `ExecuteScript` does nothing ([`cjs_runtimestub.cpp`](https://pdfium.googlesource.com/pdfium/+/refs/heads/main/fxjs/cjs_runtimestub.cpp)). PDFium's own AF* functions live in that runtime, so they don't run either. `@embedpdf/pdfium` is built without them too. A V8 build would cost about 10 MB more (macOS arm64: 3.53 MB without, 13.36 MB with). So the page we draw never reflects a script's change, and any sandbox would be Lamina PDF's own.

Others have gone both ways. pdf.js runs scripts in a QuickJS sandbox with all 17 AF* helpers, `event`, `getField`, `util` and `app`, stubs out mail, submit and launch, but forwards timers to the page; its library leaves scripting off by default, and its viewer turns it on ([Mozilla Hacks](https://hacks.mozilla.org/2021/10/implementing-form-filling-and-accessibility-in-the-firefox-pdf-viewer/), [`src/scripting_api`](https://github.com/mozilla/pdf.js/tree/master/src/scripting_api)). HexaPDF runs no engine: it matches `AFNumber_Format`, `AFPercent_Format` and `AFSimple_Calculate` calls in the script text and does their work natively ([`java_script_actions.rb`](https://github.com/gettalong/hexapdf/blob/master/lib/hexapdf/type/acro_form/java_script_actions.rb); AGPL, so a design reference only). No published study says what share of real forms' scripts are only helper calls.

## Decision

### 1. Lamina PDF never runs JavaScript from a document

No surface runs a document's scripts: not the library, the web reader, the desktop app, the CLI or the MCP server. This covers every place a script can live: `Catalog /Names /JavaScript`, `/OpenAction`, `/AA` on the catalog, pages, annotations and fields, `JavaScript` actions on links, outline items and in `/Next` chains, and XFA's `<script>` elements. A link or button whose action is JavaScript does nothing, and the reader says so where the user clicked.

Option 1 is chosen. The sandbox (option 2, and option 3's second half) is deferred, not rejected: §5 says what would reopen it.

### 2. The form helpers are recognised as data, and run natively

E8f's recogniser reads a field's `F`, `K`, `V` and `C` scripts as text and matches each against a closed list of call shapes: one call to a known `AF*` function, with literal arguments only (numbers, strings, booleans, and for `AFSimple_Calculate` an array of field-name strings), optionally ending in `;`. The list starts with:

- `AFNumber_Format` / `AFNumber_Keystroke`, `AFPercent_Format` / `AFPercent_Keystroke`;
- `AFDate_FormatEx` / `AFDate_KeystrokeEx`, `AFTime_Format` / `AFTime_Keystroke` (and the older `AFDate_Format` index form);
- `AFSpecial_Format` / `AFSpecial_Keystroke` (zip codes, phone numbers, social security numbers);
- `AFRange_Validate`;
- `AFSimple_Calculate` with `SUM`, `AVG`, `PRD`, `MIN`, `MAX`.

A matched call becomes a typed behaviour (`NumberFormat { decimals, separator, negative_style, currency, prepend }` and so on), which the field model stores and the native code applies: keystroke filtering while typing, the formatted appearance (E8d), range checks, and calculation in `/CO` order. Adobe's [JavaScript for Acrobat API Reference](https://opensource.adobe.com/dc-acrobat-sdk-docs/library/jsapiref/) doesn't document these helpers: Acrobat defines them in its bundled `AForm.js`. So E8f's tests take Acrobat's own output as the reference, and pdf.js's implementation (Apache-2.0) as a second reading.

Matching is parsing, never evaluation. The recogniser tokenises the script and compares tokens. It doesn't call an engine, doesn't generate code, and never builds a string that anything executes. Anything that doesn't match a shape exactly is not handled: a second statement, a variable, an expression in an argument, `event.value = …`, `this.getField`, a function the list doesn't name. Lamina PDF leaves that field's value as the user typed it and lists the script (§3).

### 3. Every script is detected and reported

`doc.disclosure()` (T2c) already lists every action a trigger fires, through `/Next`, with JavaScript sizes. E8 and E11 build on it:

- **Inventory.** E8f's report is also the evidence §5 asks for, since no published study measures it. Each script appears with its location (the trigger and the object that holds it), its size, and one of three states: `handled` (a recognised helper call), `not handled` (anything else) or `xfa` (an XFA script, never handled).
- **Reader.** A document with scripts shows it, in the document's properties and, when a form field's behaviour depends on a script that isn't handled, beside that field: Lamina PDF fills the field but doesn't format or calculate it.
- **Library and CLI.** The same inventory, as JSON (`laminapdf reveal`'s `scripts` category) and through the façade.
- **Stripping.** E11d's `StripActiveContent` removes every script location listed above, and XFA. Stripping a form's helper scripts also removes its formatting in other viewers, so the report names the fields that lose it.

### 4. Authoring writes scripts as text, and never runs them

E17b's action editor lets a user write a `JavaScript` action on a link, button, field or page, or as the document's open action. Lamina PDF stores what the user typed, under a warning that it won't run here and will run in other viewers. It doesn't run, test or lint the script. For the common case, the editor offers the recognised helpers as a form: choose "Number, 2 decimals, $", and it writes the `AFNumber_Format` call the recogniser would match, so the field behaves the same in Lamina PDF and in Acrobat.

### 5. What would reopen a sandbox

A sandbox (option 2) comes back as an amendment to this ADR, with its own spike, when E8f's reports show forms that matter to users fail without one: scripts that aren't handled, on fields users fill. Before it could ship, it would need all of these:

- an engine compiled to WebAssembly in its own worker, the same on every surface, with no host functions but the form API: no network, file, clock or timer access, and no `eval` of strings built from the document outside the engine;
- budgets on instructions and memory per script, deterministic output (a fixed clock, no randomness), and a script that exceeds a budget treated as not handled;
- fuzzing of the bindings, an external security review, and the owner's sign-off;
- size within the budget of the module that would load it (ADR 0016 §6), loaded only when a form needs it.

### 6. Web, desktop and the save constraint

The decision is the same on the web, the desktop, the CLI and the MCP server: none runs scripts, and all report them the same way. No server feature is involved; everything stays on the device. Recognising helpers and filling fields add no save behaviour beyond E8's: an incremental save, allowed under DocMDP P2 and P3 but not P1, and refused for fields that FieldMDP or `/Lock` locks.

## Consequences

- Forms format, validate and calculate in Lamina PDF as they do in Acrobat for the helper calls, which E8f's tests check against Acrobat's output. Forms whose behaviour is custom JavaScript fill but don't format or calculate, and the reader says which fields.
- No JavaScript engine is added to any package: no WebAssembly size, no new crate, nothing to fuzz but the recogniser, which E8f fuzzes with the other parsers.
- The recogniser's list is the contract: a new helper shape is a code change with tests, never a broader match.
- E11d's sanitiser and `laminapdf reveal` treat scripts as active content, and the report names what stripping changes.
- The full-editor plan's §9 Q1 is answered: never execute, with native helpers, and a sandbox only by amendment (§5).

## Sources

Checked 2026-11-16; sizes measured from the npm packages named.

- pdf.js: [Mozilla Hacks, form filling](https://hacks.mozilla.org/2021/10/implementing-form-filling-and-accessibility-in-the-firefox-pdf-viewer/), [`src/scripting_api`](https://github.com/mozilla/pdf.js/tree/master/src/scripting_api), [`web/app_options.js`](https://github.com/mozilla/pdf.js/blob/master/web/app_options.js) (`enableScripting`), `pdfjs-dist` 6.5.438 (`wasm/quickjs-eval.wasm`, `build/pdf.sandbox.min.mjs`); CVE-2024-4367: [GHSA-wgrm-67xf-hhpq](https://github.com/mozilla/pdf.js/security/advisories/GHSA-wgrm-67xf-hhpq), [PR #18015](https://github.com/mozilla/pdf.js/pull/18015).
- PDFium: [`build_overrides/pdfium.gni`](https://pdfium.googlesource.com/pdfium/+/refs/heads/main/build_overrides/pdfium.gni), [`fxjs/ijs_runtime.cpp`](https://pdfium.googlesource.com/pdfium/+/refs/heads/main/fxjs/ijs_runtime.cpp), [`fxjs/cjs_runtimestub.cpp`](https://pdfium.googlesource.com/pdfium/+/refs/heads/main/fxjs/cjs_runtimestub.cpp), [`fxjs/cjs_publicmethods.cpp`](https://pdfium.googlesource.com/pdfium/+/refs/heads/main/fxjs/cjs_publicmethods.cpp); [pdfium-binaries chromium/7881](https://github.com/bblanchon/pdfium-binaries/releases/tag/chromium%2F7881).
- Engines: [QuickJS](https://github.com/bellard/quickjs), [QuickJS-ng](https://github.com/quickjs-ng/quickjs), [`quickjs-emscripten`](https://www.npmjs.com/package/quickjs-emscripten) 0.32.0, [`rquickjs`](https://crates.io/crates/rquickjs), [`boa_engine`](https://crates.io/crates/boa_engine).
- Native helpers without an engine: [HexaPDF `java_script_actions.rb`](https://github.com/gettalong/hexapdf/blob/master/lib/hexapdf/type/acro_form/java_script_actions.rb) (AGPL; design only).
- Acrobat: [JavaScript for Acrobat API Reference](https://opensource.adobe.com/dc-acrobat-sdk-docs/library/jsapiref/index.html); CVE-2023-21608: [Adobe APSB23-01](https://helpx.adobe.com/security/products/acrobat/apsb23-01.html), [CVE.org](https://www.cve.org/CVERecord?id=CVE-2023-21608).
