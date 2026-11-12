# The viewer stopped typechecking after the desktop client and saving merged

## What happened

After #421 merged on 2026-11-12, `main` stopped building. The reader's deploy failed on `345184be` (Web reader run 43137391157), and `cargo xtask desktop-frontend` failed in a fresh checkout, with:

```
src/view/viewer.ts(461,43): error TS2339: Property 'planSave' does not exist on type 'ViewerEngine'.
src/view/viewer.ts(462,44): error TS2339: Property 'copyEdits' does not exist on type 'ViewerEngine'.
src/view/viewer.ts(463,43): error TS2339: Property 'adoptEdits' does not exist on type 'ViewerEngine'.
```

Every PR built on that `main` fails the same way.

## Reproduce

On `345184be`: `cargo xtask desktop-frontend`, or `pnpm --filter @laminapdf/viewer run build:wasm` after the WebAssembly is built.

## Cause

Two PRs changed the same contract from either side, and neither saw the other's change:

- #444 (E1e, Save and Save as) added three calls to the viewer's edit session: `planSave`, `copyEdits` and `adoptEdits`. The viewer passed them to the session from its client, which was then a `ViewerClient`.
- #421 (D2d, the desktop engine client) narrowed the viewer's client to `ViewerEngine`, a `Pick` of the `ViewerClient` calls the viewer uses, so the desktop app's own client could stand in for the document worker. Its list was written before #444 merged and doesn't name the three save calls.

Each PR passed CI on its own; git merged them without a conflict, since they touched different lines. Together, the viewer calls three methods its client type no longer has.
