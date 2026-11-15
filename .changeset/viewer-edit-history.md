---
"@laminapdf/viewer": minor
---

A new entry point, `@laminapdf/viewer/edit-history`, exports the edit session's history: its steps, where the document stands among them, and how many steps aren't saved. The document worker keeps its history with it. An app whose document model runs somewhere else, such as Lamina PDF's desktop app in its engine process, can keep its history the same way.
