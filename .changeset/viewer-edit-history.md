---
"@laminapdf/viewer": minor
---

An app whose document model runs outside the document worker, such as Lamina PDF's desktop app, can now keep its edit history the way the worker does. The new entry point `@laminapdf/viewer/edit-history` exports the code that keeps an edit session's history: its steps, where the document stands among them, and how many steps aren't saved.
