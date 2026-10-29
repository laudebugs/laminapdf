# Design

lamina's visual design. The owner chose the **Paper** direction on 2026-10-29 ([Paper design plan](../plans/paper-design.md)); the interaction rules it draws are in the [UI/UX plan](../plans/ui-ux.md).

| Document | What it is |
| --- | --- |
| [paper/design-language.md](paper/design-language.md) | Tokens, type, icons, layout, components with HTML snippets, patterns and copy rules |
| [paper/screens.md](paper/screens.md) | Every screen, by feature area, with platforms, todos and canvases |
| [paper/](paper/) area documents | One per feature area: each screen's purpose, layout, controls, states, behaviour and accessibility |

Each area also has a Design canvas (linked from its document) whose source files are committed under `paper/<area>/` for reference. The `.dc.html` files render inside the Design canvas, not standalone, and are not product code.
