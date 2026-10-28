# CLAUDE.md

Guidance for Claude Code (and humans) working in this repository.

## What lamina is

lamina is a platform for anything and everything PDF: creating PDFs from code, parsing and extracting content, rendering, editing, form filling, signing, and transforming documents. The project is in its **research phase** — there is no application code yet. The core language, architecture and scope have not been decided; don't assume them. When they are decided, they'll be recorded as ADRs in `docs/decisions/` and summarized here.

## Repository layout

```
.
├── CLAUDE.md            # this file
├── README.md            # project overview
└── docs/                # all documentation (see docs/README.md)
    ├── research/        # deep dives and research notes, one subfolder per topic
    ├── plans/           # roadmaps and feature design docs
    ├── decisions/       # Architecture Decision Records (NNNN-title.md)
    ├── guidelines/      # how we work (contributing, conventions)
    └── references/      # external specs, tools, corpora
```

## Working agreements

- **Pull requests only.** `main` was initialized directly; every change since goes through a branch + PR. Branch names: `<type>/<short-description>` (e.g. `docs/pdf-research`).
- **Conventional Commits** for commit messages: `type(scope): summary`.
- **Document as you go.** Plans go in `docs/plans/`, decisions in `docs/decisions/`, research in `docs/research/`. Update the folder's `README.md` index whenever you add a file.
- **Cite sources.** PDF behavior claims should point to ISO 32000-2 (PDF 2.0) or another primary source. Clearly label recommendations and opinions.
- **Spec first.** When implementing PDF features, read the relevant research chapter in `docs/research/` and the spec section before writing code. Real-world PDFs are frequently malformed — parsers must be tolerant, writers must be strict.
- **Mind licenses.** Many PDF libraries are AGPL or dual-licensed (e.g. iText, MuPDF/PyMuPDF). Don't add a dependency without checking its license.

## Commands

None yet — add build, test and lint commands here once code exists.
