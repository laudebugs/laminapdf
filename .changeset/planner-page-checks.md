---
"@laminapdf/sdk": patch
---

In Rust, the save planner now checks page edits field by field. The save is now a conflict when a page edit deletes a signed signature's widget or changes how it is shown (`Restriction::SignedWidget`), or does the same to a field that a signature's lock covers (`Restriction::FieldMdp`); `Violation::field` names the field. Moving a page breaks neither. `laminapdf_ops::Applied::widgets` and `PendingChanges::widgets()` record the fields an edit touched, and `Op::page_changes()` lists the pages it will change, so `SavePlanner::would_violate` can warn before the edit. The page edits that report them come next.
