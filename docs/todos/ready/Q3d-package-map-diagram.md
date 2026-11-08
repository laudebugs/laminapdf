# Q3d — A diagram of the packages and the modules they load

| Field | Value |
| --- | --- |
| ID | `Q3d` |
| Status | `ready` |
| Plan | [Docs diagrams §2](../../plans/docs-diagrams.md#2-where-a-diagram-helps) |
| Round / milestone | Round Q3 (docs diagrams) |
| Depends on | [Q3a](./Q3a-diagram-kit-and-first-diagram.md) |
| ADRs | None |
| Branch | `feat/q3d-package-map` |
| PR title | `feat(docs): a diagram of the published packages and modules` |
| Size | S |
| PR | — |

## Goal

The install page lists a dozen packages, and readers ask which one they need and what it pulls in. A layered stack shows `@laminapdf/sdk` over `@laminapdf/wasm` and its three modules, the viewer over the render packages, and the `laminapdf` crate for Rust, so a reader picks a package from one picture.

## Context

- [Docs diagrams plan](../../plans/docs-diagrams.md) §3: the kit, the style and the checks Q3a built; every diagram here uses them.
- `apps/docs/src/content/docs/install.mdx`: "Every published package".
- `apps/docs/src/content/docs/concepts/how-lamina-thinks.mdx`: "Three modules in JavaScript".
- `.changeset/config.json` and the packages' `package.json` files: the names and what depends on what.

## Scope

**In scope**

- Diagram 5: `@laminapdf/sdk` over `@laminapdf/wasm` with its base, text and extract modules; `@laminapdf/viewer` over `@laminapdf/render`, with `-pdfium` and `-hayro` beside it; the `laminapdf` crate as the Rust entry point. The package a page discusses is the `accent` element (one diagram component, a `focus` prop per page).

**Out of scope** (and which todo covers it)

- The crates under the façade (`laminapdf-cos`, `-save` and the rest): contributor docs, not reader docs.

## Tasks

- [ ] `components/diagrams/PackageMap.astro` with a `focus` prop (`sdk`, `modules`, `viewer`, none).
- [ ] Place it under "Every published package" with no focus, and under "Three modules in JavaScript" focused on the modules.

## Acceptance criteria

- [ ] Each diagram is built from the Q3a kit, renders in both themes (screenshots in the PR) and passes the lint, twin and browser tests.
- [ ] Each page's `.md` twin holds the caption and text description, and no SVG or JSX.
- [ ] Each diagram is under 8 KB gzipped (reported in the PR).
- [ ] `cargo xtask check docs` and the docs build pass.
- [ ] Every package name in the diagram is published under that name (`install.mdx`'s table and the npm registry).

## Post-merge checks

- [ ] The diagrams show on `next.docs.laminapdf.com` in both themes.

## Tests

The Q3a lint, twin and browser tests cover each new diagram once it is under `apps/docs/src/components/diagrams/`; no new test files unless a diagram needs a layout the kit lacks.

## Docs to update

The pages named in the tasks, and the plan's §9 to-do.

## Risks and open questions

- A package added later makes the diagram stale; the plan's §7 rule (search the diagram components when a package changes) applies.

## Definition of done

- [ ] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [ ] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
