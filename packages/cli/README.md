# @laminapdf/cli

The `laminapdf` command-line tool on npm: inspect, check, rotate and extract PDFs from a shell, a build script or an AI agent, with versioned JSON output. The package runs a prebuilt native binary, so it needs no Rust toolchain. **Not yet published:** the packages are built and tested on every release build, but none is on npm yet.

```sh
npm install -g @laminapdf/cli
laminapdf info report.pdf
npx @laminapdf/cli extract invoice.pdf --format md
```

The commands, exit codes and JSON reports are documented in the [`laminapdf-cli` README](../../crates/laminapdf-cli/README.md). Homebrew (`brew install laudebugs/tap/laminapdf`) and release archives install the same binary.

## How it picks the binary

Each platform's binary is its own package, which `@laminapdf/cli` lists as an optional dependency at the same version. The platform packages declare `os`, `cpu` and, on Linux, `libc`, so npm installs only the one that matches your machine and skips the rest. No install script runs and nothing downloads at install time, which is the layout Biome and esbuild use. `bin/laminapdf.js` then finds that package and runs its binary with your arguments and standard streams, and exits with the binary's exit code.

| Package | Platform |
| --- | --- |
| `@laminapdf/cli-darwin-arm64` | macOS, Apple silicon |
| `@laminapdf/cli-darwin-x64` | macOS, Intel |
| `@laminapdf/cli-linux-arm64-gnu` | Linux arm64, glibc |
| `@laminapdf/cli-linux-x64-gnu` | Linux x86_64, glibc |
| `@laminapdf/cli-linux-arm64-musl` | Linux arm64, musl (Alpine) |
| `@laminapdf/cli-linux-x64-musl` | Linux x86_64, musl (Alpine) |
| `@laminapdf/cli-win32-arm64` | Windows arm64 |
| `@laminapdf/cli-win32-x64` | Windows x86_64 |

On another operating system or CPU, `npm install` refuses the package (`EBADPLATFORM`, listing the supported ones). If the platform package is missing, usually because optional dependencies were left out (`--omit=optional`), `laminapdf` names the package it needs and exits with code 1.

## For contributors

`platforms.json` is the one list of platform packages, each with its Rust target. `src/platform.ts` matches the host against it and `src/launcher.ts` runs the binary. The committed `package.json` is private at version `0.0.0` and has no optional dependencies: [`tools/cli-release/generate.ts`](../../tools/cli-release/generate.ts) writes the publishable packages from dist's archives, at the CLI crate's version, into `target/cli-npm/`. The package stays out of Changesets' fixed group: its version is the crate's, and the CLI ships on its own `laminapdf-cli-v<version>` tags through [`cli-release.yml`](../../.github/workflows/cli-release.yml), not on the library's `v<version>` tags.

```sh
pnpm --filter @laminapdf/cli test                                    # platform resolution; generated packages against dist's targets
pnpm --filter @laminapdf/cli build
node tools/cli-release/generate.ts --artifacts target/distrib            # after `dist build`: target/cli-npm/<package>/
```
