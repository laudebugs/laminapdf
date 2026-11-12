# Resolution: the save calls are part of `ViewerEngine`

## Fix

`ViewerEngine` (`packages/viewer/src/client.ts`) now names `planSave`, `copyEdits` and `adoptEdits`, the calls the edit session needs to save. The desktop app's `DesktopEngineClient` answers them as it answers the other edit calls: each rejects with a `not-yet-supported` `DesktopEngineError`, since its engine doesn't edit or save yet. Desktop saving is E1i's.

## Proof

`cargo xtask desktop-frontend`, the desktop app's typecheck (`typecheck:wasm`) and tests, and the viewer's edit-session tests all pass. The type checker is the test here: had `ViewerEngine` missed a call the viewer makes, or the desktop client a call `ViewerEngine` names, the build would fail as it did on `main`.

## Left for later

Nothing. When the desktop engine learns to save (E1i), these three stubs are where it plugs in.
