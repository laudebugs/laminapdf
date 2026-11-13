# Docs site research

How the docs site at `docs.laminapdf.com` should show its examples, and what developers want from a docs site.

| Note | Summary |
| --- | --- |
| [live-examples.md](live-examples.md) | Showing what an example makes, and letting readers change it: pictures of the output (Android), live samples (MDN), hosted sandboxes (TanStack, StackBlitz), a hosted bundler (React, Sandpack), a playground of our own (Svelte, Vue). Recommends a Result tab with page pictures now and a runner of our own for a few TypeScript examples later. |
| [developer-docs.md](developer-docs.md) | What developers want from documentation and what drives them away, from surveys, studies, 197 Hacker News threads (21,499 comments) and the docs sites they praise, with an audit of this site and a framework comparison. Recommends staying on Starlight and spending a redesign on tested CLI output, a reading-and-extraction path, results beside the code and first-run tutorials for Rust and the CLI. Notes in [developer-docs/](developer-docs/). |
| [editor-completion.md](editor-completion.md) | Completing the SDK's API in "Edit and run" and whether its editor should colour code with Shiki. Measures a list generated from the SDK's types against TypeScript's language service in a worker (about 1.15 MB gzipped, loaded on the first keystroke), and Shiki's weight and CSP cost in the editor. Recommends the language service, and matching the editor's colours to Shiki's in place of a Shiki option. |
