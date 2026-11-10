# C7d — Cold first page: paint before the extract module, fetch the backends at once

| Field | Value |
| --- | --- |
| ID | `C7d` |
| Status | `ongoing` |
| Plan | [web reader §6 C7. Accessibility, offline and launch](../../plans/web-app.md#c7-accessibility-offline-and-launch) |
| Round / milestone | Round C (web reader) — M2 Web reader v0.1 |
| Depends on | [C3c](../done/C3c-viewer-perf-harness.md), [C7b](../done/C7b-offline-pwa.md) |
| ADRs | Follows 0009 (one document worker, typed RPC), 0004 (PDFium by default) |
| Branch | `perf/c7d-cold-first-page` |
| PR title | `perf(viewer): paint the first page sooner on a cold load` |
| Size | S (150–300 lines incl. tests) |
| PR | — |

## Goal

On a first visit over Fast 4G, the first page of a file opened from the device paints within [web-app §4](../../plans/web-app.md#4-requirements)'s 3 s target plus the release gate's 25 % margin. C7c's gate measured 5.15 s, so the launch waits for this.

## Context

- The document worker answers one call at a time. The link layer's first `pageLinks` call loaded the extract module (710 KB) before the first render could run.
- The base module and PDFium downloaded one after the other.
- The harness fetched the whole 1 MB file over the throttled network, so its "cold" figure mixed the reader's start-up with the file's download.

## Scope

**In scope**

- The client compiles the backend's WASM on the main thread while the worker starts, and posts the compiled module.
- Text, reading order, structure and link calls wait for the first successful draw (at most 2 s), and links ask at background priority.
- The worker starts the extract module after the first tile is drawn, not when the backend loads.
- The harness measures two cold cases: a file on the device (gated) and a 1 MB linked file read by ranges (reported). Its server answers `Range` requests.

**Out of scope**

- The linked-file cold load (4.75 s): [#382](https://github.com/laudebugs/laminapdf/issues/382).
- The start-up script size: [#380](https://github.com/laudebugs/laminapdf/issues/380).

## Acceptance criteria

- The device-file cold first page is under 3.75 s in the harness on Chromium.
- Links, text selection, find and reader mode still work (viewer browser tests and the web e2e suite).

## Tests

- `packages/viewer/perf/report.test.ts` covers the new rows and history keys.
- The viewer's browser tests and `apps/web`'s e2e suite run unchanged.
