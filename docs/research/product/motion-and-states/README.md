# Motion and interaction states

Research behind the [motion and interaction states plan](../../../plans/motion-and-states.md) (Round U7), read on `main` on 2026-11-14. Each note gives a file and line for every claim about the code; the reader's note also measured the built reader in Playwright.

| Note | What it covers |
| --- | --- |
| [reader.md](reader.md) | `@laminapdf/ui`, the viewer and the web reader: cursors (measured on 106 controls), state coverage per component, every animation, delight gaps |
| [sites.md](sites.md) | The docs site, the website and the site kit: cursors, states, motion, what the CSP allows |
| [desktop.md](desktop.md) | The desktop app: its own chrome, platform cursor conventions (Apple, Microsoft, GNOME), reduced motion and CSS support in Tauri 2's webviews, tab drag on Windows |
| [guidance.md](guidance.md) | What Paper's design language already says, and outside guidance (Material 3, Carbon, Fluent 2, Apple, WCAG 2.3.3, React Aria), with the proposed tokens and state model |
