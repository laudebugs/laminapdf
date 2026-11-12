# Resolution: the save calls are part of `ViewerEngine`

## Fix

`ViewerEngine` (`packages/viewer/src/client.ts`) now names `planSave`, `copyEdits` and `adoptEdits`, the calls the edit session needs to save. The desktop app's `DesktopEngineClient` answers them as it answers the other edit calls: each rejects with a `not-yet-supported` `DesktopEngineError`, since its engine doesn't edit or save yet. Desktop saving is E1i's.

## Proof

`cargo xtask desktop-frontend`, the desktop app's typecheck (`typecheck:wasm`) and tests, and the viewer's edit-session tests all pass. The type checker is the test here: had `ViewerEngine` missed a call the viewer makes, or the desktop client a call `ViewerEngine` names, the build would fail as it did on `main`.

## Left for later

Nothing in the code. When the desktop engine learns to save (E1i), these three stubs are where it plugs in.

A call the viewer starts making must still be added in two places, `ViewerEngine`'s list and `DesktopEngineClient`, and the type checker names both. What let this break reach `main` was the order of merges, not a missing check: each PR was tested against a `main` without the other. Merging `main` into a PR and letting CI run again before merging it, which the review process asks for, catches it; a merge queue or GitHub's "require branches to be up to date" rule would enforce it, and that choice is the owner's.
