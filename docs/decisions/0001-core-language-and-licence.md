# 0001 — Core language, licence and the bytes-in/bytes-out rule

- Status: Accepted
- Date: 2026-10-28

## Context

lamina has to serve three kinds of product from one engine: importable packages (a Rust crate family and `@lamina/pdf` on npm), a web app on Cloudflare, and a desktop app on macOS, Windows and Linux, with mobile possible later ([architecture plan §1](../plans/architecture.md#1-goal)). Every one of them parses untrusted PDF input, the main source of memory-safety bugs in C and C++ PDF engines ([research ch. 05 §8.2](../research/pdf/05-creating-pdfs-from-code.md#82-candidate-core-languages)).

The licensing landscape matters as much as the technology. The engines that cover the whole PDF lifecycle are AGPL or commercial (MuPDF, iText, Ghostscript, PDFsharp since 6.5), and licences drift: QuestPDF moved to a revenue-gated licence in 2023 and PDFsharp to AGPL in 2026. Permissive projects each cover a slice. "The permissive, full-stack slot is empty" ([ch. 05 §8.1](../research/pdf/05-creating-pdfs-from-code.md#81-what-the-landscape-tells-us)). Meanwhile most modern permissive PDF infrastructure (`pdf-writer`, `krilla`, `subsetter`, `hayro`, `skrifa`, `rustybuzz`/`harfrust`) is written in Rust under MIT/Apache-2.0 and compiles to WASM without emscripten.

The `pdf-basics` spike built the same minimal PDF core in Rust and in TypeScript from one spec and tested both against shared fixtures ([research ch. 07](../research/pdf/07-spike-rust-and-typescript.md)):

- The Rust core compiled to **172 KB of WASM (74 KB gzipped)**, ran from TypeScript, produced byte-identical output and agreed with the TS reader on **4,304 of 4,305** differential-fuzz inputs ([ch. 07 §3](../research/pdf/07-spike-rust-and-typescript.md#3-results)).
- Clippy-enforced panic-free library code, overflow-checked arithmetic and the type system meant the review's Rust findings were mostly about performance; most correctness findings were on the TS side ([ch. 07 §4](../research/pdf/07-spike-rust-and-typescript.md#4-what-we-learned)).
- The library was **bytes in, bytes out**: no file system, network, process, threads or clocks. Because of that the same source type-checked for `wasm32-unknown-unknown`, `aarch64-apple-ios` and `aarch64-linux-android` with no code changes, and `check-targets.sh` enforced the rule by grepping ([spike design notes](../../spikes/pdf-basics/rust/README.md#design-notes)).

The architecture research confirmed the choice and named it as the first ADR ([report: decisions](../research/architecture/app-and-package-architecture.md#each-decision-has-a-credible-alternative-and-a-known-price), [ADRs to record](../research/architecture/app-and-package-architecture.md#adrs-to-record)).

## Decision

1. **The lamina core is written in Rust** (edition 2024, toolchain and MSRV policy in [`rust.md`](../guidelines/rust.md#toolchain-edition-msrv)). It is compiled natively for desktop and servers and to `wasm32-unknown-unknown` for browsers, Cloudflare Workers and Node. TypeScript is the first binding language ([ADR 0002](0002-binding-strategy.md)), not a second implementation.

2. **Licence: `MIT OR Apache-2.0`** for all lamina code (crates, npm packages, apps, tooling). Every crate sets `license = "MIT OR Apache-2.0"` (inherited from `[workspace.package]`), every npm package sets `"license": "MIT OR Apache-2.0"`, and the repository root carries `LICENSE-MIT` and `LICENSE-APACHE`. Contributions are accepted under the same dual licence (inbound = outbound, as Apache-2.0 §5 provides); whether to also require a DCO sign-off is left to the contributing guide.

3. **Bytes in, bytes out.** Core crates (parsing, object model, ops, save, build, text, render adapters' pure parts):
   - take input as `&[u8]` or through a caller-implemented trait (`ByteSource` for range reads, `WriteSink` for streaming output) and return bytes or values;
   - **MUST NOT** use `std::fs`, `std::net`, `std::process`, `std::thread` (no spawning), clocks (`SystemTime`, `Instant`) or ambient randomness. Anything they need from the environment (time for a signature, fonts, a random salt) is passed in as a parameter or a trait;
   - **MUST NOT** depend on `wasm-bindgen`, `tauri`, `tokio`, `napi` or similar runtime or binding crates, or branch on `cfg(target_arch)` for platform behaviour.

   The core does not *create* threads, but it is not single-threaded by design: its types should be `Send` where that costs little, so **shells may run it on several threads or workers** (for example rendering separate pages in parallel). Which types are `Send`/`Sync` is settled per type in the document-model ADR (0005). Platform code (bindings, Tauri commands, Worker handlers, file pickers, async runtimes) lives only in edge crates and apps ([code organization §2](../guidelines/code-organization.md#2-boundaries-and-dependency-direction)). Product CI keeps the spike's portability gate as an xtask: a forbidden-API check plus `cargo check` for the wasm32, iOS and Android targets.

4. **No AGPL or GPL dependencies in core** or in anything we ship. LGPL code is not linked into shipped artefacts either. Copyleft tools (veraPDF, Poppler, pdftk, Ghostscript) may run as external processes in CI only ([ch. 05 §8.4](../research/pdf/05-creating-pdfs-from-code.md#84-licensing-traps-to-avoid)). MuPDF in any form is excluded. Enforcement is [ADR 0019](0019-supply-chain-and-licence-hygiene.md).

## Consequences

- One implementation of each of the ~180 operations in the [feature inventory](../research/pdf/06-feature-inventory.md) serves the web app, desktop app and packages. A fix lands everywhere at once.
- Memory safety and panic-free library code are enforced by the compiler and lint table ([`rust.md`](../guidelines/rust.md#lints)) rather than by review alone, which matters for a parser of hostile input.
- The permissive licence is a commitment: users can embed lamina in closed products and network services. It also constrains us: features that only exist in copyleft engines must be built, not borrowed.
- The contributor barrier is higher than for a TypeScript codebase, and compile times are longer. Every new platform needs a binding crate (ADR 0002) instead of "just running".
- Because the core cannot reach for I/O, APIs must be designed around injected sources and sinks from the start. Large files work through range reads (mmap on desktop, `Blob.slice`/OPFS on the web, HTTP range in a Worker) rather than whole-file buffers.
- WASM builds inherit the 4 GB memory ceiling on browsers without Memory64 (Safari); the desktop app runs the core natively instead ([architecture §6](../plans/architecture.md#6-risks)).

## Alternatives considered

- **TypeScript core.** Largest contributor pool and native on every JS runtime; the spike's TS core was ~40% shorter and 16 KiB gzipped. Rejected as the core: weak for CPU-heavy work (rendering, codecs, subsetting), hard to serve non-JS users, and every safety property the Rust compiler gave would have to be re-verified by review. It stays the right language for the SDK layer and UI.
- **Wrap C/C++ engines** (PDFium, qpdf, HarfBuzz). Fastest route to coverage and rendering fidelity, but no stable PDFium API, multi-MB emscripten WASM, memory-safety risk on untrusted input and little room to differentiate. PDFium is used instead as an optional render backend and test oracle (ADR 0004).
- **Go, JVM/.NET, Zig/C cores.** Weak WASM or browser story, heavy runtimes, or no memory safety ([ch. 05 §8.2](../research/pdf/05-creating-pdfs-from-code.md#82-candidate-core-languages)).
- **Apache-2.0 only, or MPL-2.0.** Apache-2.0 alone is incompatible with GPLv2-only downstream projects; MIT alone lacks the patent grant. The dual licence is the Rust ecosystem convention and is compatible with every dependency we plan to use. MPL-2.0 adds file-level copyleft obligations for users, which works against the "empty permissive slot" we want to fill.
- **Allowing the core to own threads or async I/O** (e.g. `rayon` or `tokio` inside the core). Simpler for native callers, but it breaks `wasm32-unknown-unknown` without cross-origin isolation, Cloudflare Workers and mobile hosts. Shells own concurrency instead.
