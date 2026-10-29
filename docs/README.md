# lamina documentation

Everything we know, plan and decide lives here, in Markdown, versioned with the code.

| Folder | Purpose |
| --- | --- |
| [`research/`](research/) | Research notes and deep dives (e.g. how PDF works). Exploratory, cited, may be opinionated. |
| [`plans/`](plans/) | Roadmaps, project plans and feature design docs before/while they are built. |
| [`todos/`](todos/README.md) | The work board: one file per pull request, in `ready/`, `ongoing/` or `done/`, with dependencies. Plans link to their todos. |
| [`decisions/`](decisions/) | Architecture Decision Records (ADRs) — one file per significant decision, numbered `NNNN-title.md`. |
| [`guidelines/`](guidelines/) | How we work: contributing, git workflow, code organization, Rust and TypeScript conventions. |
| [`references/`](references/) | Pointers to external specs, standards, tools and corpora we rely on. |

## Spikes

Time-boxed experiments under [`spikes/`](../spikes/) that answer one question each; each has a README with the result. They are outside both workspaces and never imported.

| Spike | Question | Result |
| --- | --- | --- |
| [`pdf-basics`](../spikes/pdf-basics/) | Rust or TypeScript for the core? | Rust core (ADR 0001); see [research ch. 07](research/pdf/07-spike-rust-and-typescript.md) |
| [`krilla-fit`](../spikes/krilla-fit/README.md) | Can krilla generate into lamina's object graph? | No (bytes only). Recommends an own builder over `lamina-cos` reusing `pdf-writer`, `subsetter`, `skrifa`, HarfRust; recorded in ADR 0005 §6 |
| [`hayro-parity`](../spikes/hayro-parity/README.md) | How close is hayro to PDFium on the pdf.js and PDFBox corpora? | 81.8 % pass, 90.8 % pass+minor at 96 DPI (macOS); where hayro is clearly wrong, it is mostly fonts and annotations. Recommends PDFium as the default and hayro as the CI oracle (input to ADR 0004) |

## Conventions

- File names are `kebab-case.md`. Ordered series use a numeric prefix (`01-…`, `0001-…`).
- Every folder has a `README.md` index; add your doc to it when you create one.
- Cite sources with links. Prefer primary sources (ISO 32000, PDF Association, vendor docs) over blog posts.
- Mark opinions and recommendations clearly so they aren't mistaken for spec facts.
