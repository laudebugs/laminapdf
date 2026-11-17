---
"@laminapdf/sdk": minor
---

A file with usage rights (UR3) now holds each annotation edit to the right it needs: adding an annotation needs `/Create`, changing one `/Modify`, and deleting one `/Delete`. `wouldViolate` and the save planner used to allow every annotation edit when the file granted any one of them, so a file that granted only `/Create` let a delete through without saying it would invalidate the usage rights.

In Rust: `AnnotRight`, `Op::annot_right` and `PendingChanges::annot_rights`.
