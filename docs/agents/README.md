# Agent reference

The detail behind the root [`CLAUDE.md`](../../CLAUDE.md), one file per topic, so a session loads only what its work needs. `CLAUDE.md` holds what every session needs (the working agreements, the everyday commands, a short layout) and a table of which file here to read before working on what.

| File | What it holds |
| --- | --- |
| [`layout.md`](layout.md) | The repository tree, every crate, package, app and tool, file by file |
| [`commands.md`](commands.md) | Every `cargo xtask` task and the other commands, with their flags |
| [`ci-and-release.md`](ci-and-release.md) | The PR lanes, fuzzing, the nightly lane, the library, CLI, desktop and daily releases |
| [`decisions.md`](decisions.md) | Every accepted ADR in one paragraph, and the plans that follow them |
| [`api-reading-and-editing.md`](api-reading-and-editing.md) | Reading, content streams, editing (ops, undo, page ops) and saving |
| [`api-text-and-extraction.md`](api-text-and-extraction.md) | Fonts, shaping and text extraction; extraction for retrieval and reader mode |
| [`api-building.md`](api-building.md) | The builder: pages, text, tagging, conformance, flowing layout, Markdown and JSON documents |
| [`api-facade-and-cli.md`](api-facade-and-cli.md) | The `laminapdf` façade, the CLI and its MCP server |
| [`api-bindings.md`](api-bindings.md) | The WebAssembly modules, `@laminapdf/sdk`, its worker client and the viewer |
| [`api-rendering.md`](api-rendering.md) | `RenderBackend`, the PDFium and hayro adapters, and their web packages |
| [`spikes.md`](spikes.md) | How to run each research spike |

When to update these files: [git workflow §5](../guidelines/git-workflow.md#5-pull-requests).
