---
"@laminapdf/sdk": patch
---

In Rust, the save planner now checks page edits field by field. A page edit that deletes or changes how a signed signature's widget is shown makes the save a conflict (`Restriction::SignedWidget`), and so does one that touches a field that a signature's lock covers (`Restriction::FieldMdp`); `Violation::field` names the field. Moving a page breaks neither. `laminapdf_ops::Applied::widgets` and `PendingChanges::widgets()` record the fields an edit touched, and `Op::page_changes()` lists the pages it will change, so `SavePlanner::would_violate` can warn before the edit. The page edits that report them come next.
