# TypeScript conventions

These apply to all product TypeScript in `packages/` and `apps/`. They are based on the [TypeScript handbook and tsconfig reference](https://www.typescriptlang.org/tsconfig/) and on what the [`pdf-basics` TS spike](../../spikes/pdf-basics/ts/README.md) enforced (strict flags, ESM-only, no Node APIs, Biome, Vitest). Layout and package boundaries are covered in [`code-organization.md`](code-organization.md).

**MUST**/**SHOULD** are rules. *Recommendation* marks advice.

## Compiler settings

- **MUST:** every package extends the root `tsconfig.base.json`. Its strictness flags are the spike's; the `lib`/`types` portability setup and `erasableSyntaxOnly` are new (the spike type-checked with `lib: ["ES2022", "DOM"]` + `types: ["node"]`, and built with `lib: ["ES2022"]` + `types: []` because its `src/` used no web APIs). Verified with TypeScript 7.0.2:

```jsonc
{
  "compilerOptions": {
    "target": "ES2022",
    "module": "NodeNext",
    "moduleResolution": "NodeNext",
    "lib": ["ES2022", "DOM"],             // types TextEncoder/TextDecoder, crypto.subtle, fetch; DOM-only globals are banned by Biome
    "types": [],                          // no ambient @types: Buffer, process, require, node:* fail to compile
    "strict": true,
    "noUncheckedIndexedAccess": true,
    "exactOptionalPropertyTypes": true,
    "noImplicitOverride": true,
    "noFallthroughCasesInSwitch": true,
    "noUnusedLocals": true,
    "noUnusedParameters": true,
    "isolatedModules": true,
    "verbatimModuleSyntax": true,
    "erasableSyntaxOnly": true,           // no enums/namespaces/parameter properties: any per-file transpiler (esbuild, Vitest) just erases types
    "declaration": true,
    "declarationMap": true,
    "sourceMap": true,
    "skipLibCheck": true
  }
}
```

- **MUST:** libraries use `module`/`moduleResolution` `NodeNext`. The handbook calls the node16–nodenext family the only correct `module` options for code that runs in Node.js ([modules reference](https://www.typescriptlang.org/docs/handbook/modules/reference.html)), and output that resolves under Node also works in bundlers and Deno. Apps that are always bundled (`apps/web`) may use `"module": "preserve"` with `"moduleResolution": "bundler"` and `noEmit`.
- **MUST NOT:** use `paths` aliases in published packages (the handbook warns against it). Depend on workspace packages by name instead.
- **MUST:** `tsc --noEmit` (`pnpm typecheck`) passes for `src/`, tests and scripts. Test and script configs may add `"types": ["node", "vitest/globals"]`.
- The spike ran on TypeScript 7 (the native compiler). Pin the version in the pnpm catalog and upgrade it deliberately.

## Modules and packaging

- **MUST:** packages are **ESM-only**: `"type": "module"`, with relative imports written with their `.js` extension (`import { open } from "./document.js"`). No CommonJS output. Packages run from compiled `dist/`, not from `src/` under Node's type stripping (which would need `.ts` specifiers plus `allowImportingTsExtensions`/`rewriteRelativeImportExtensions`; we don't use them).
- **MUST:** `package.json` declares `"exports"` (with `"types"` first in each condition), `"files": ["dist"]`, `"sideEffects": false` and `"engines"`. Internal packages set `"private": true`. Workspace dependencies use `"workspace:*"`, and shared third-party versions use `"catalog:"` ([pnpm](https://pnpm.io/catalogs)).
- **MUST:** use `import type { … }` for type-only imports (`verbatimModuleSyntax` requires it).

## Portability

Shared packages run in browsers, Cloudflare Workers, Deno, Bun, Node, React Native/Hermes and the Tauri webview.

- **MUST NOT:** shared packages import `node:*` modules, use `Buffer`, `process`, `__dirname`, `require`, or touch the file system or network. Bytes are `Uint8Array`. Text goes through `TextEncoder`/`TextDecoder`, and randomness or crypto through `globalThis.crypto`. `types: []` makes the Node APIs fail to compile (unless a dependency's `.d.ts` pulls in `@types/node`, which the portability test catches). Keep a portability test, as in the spike's `portability.test.ts`.
- **Why `"DOM"` in `lib`:** the web-standard APIs shared packages need (`TextEncoder`, `crypto.subtle`, `fetch`, `URL`) are only declared in TypeScript's `DOM` lib; `ES2022` alone has none of them. Hand-writing ambient declarations for `SubtleCrypto` and `fetch` would be large and drift from the platform, so we take `DOM` and ban the DOM-only globals instead.
- **MUST NOT:** shared packages depend on DOM-only globals (`window`, `document`, `navigator`, `localStorage`, `HTMLElement` …) unless the package is explicitly a browser UI package (for example `@laminapdf/viewer`). The root `biome.json` enforces this with `style/noRestrictedGlobals` (`"options": { "deniedGlobals": { "window": "…", "document": "…" } }`), which browser UI packages switch off in their nested config, and the portability test scans `src/` for them too.
- **SHOULD:** environment access (fonts, file pickers, workers) is passed in by the caller. Node-specific convenience lives in a separate package (e.g. `@laminapdf/node`).
- Note that `BigInt` needs Hermes ≥ 0.70 on React Native (from the spike).

## Lint, format, test

- **MUST:** [Biome](https://biomejs.dev/) does linting **and** formatting. There's one root `biome.json` with the `recommended` preset and the formatter enabled. Nested configs use `"extends": "//"` and say why they override anything ([Biome: big projects](https://biomejs.dev/guides/big-projects/)). Don't add ESLint or Prettier alongside it.
- **MUST:** `biome ci` passes in CI. Turn off a rule inline with `// biome-ignore lint/<group>/<rule>: <reason>`. The reason is mandatory.
- **MUST:** the root `biome.json` enforces the [maintainability guideline](maintainability.md)'s size and error rules as errors:
  - `complexity/noExcessiveCognitiveComplexity` caps cognitive complexity at 15;
  - `complexity/noExcessiveLinesPerFunction` caps a function at 60 lines;
  - `style/useThrowOnlyError` allows throwing only `Error` subclasses.

  Test files are exempt from the length rule (an override on `**/test/**` and `**/*.test.ts`): a `describe` callback holds a whole suite, and each `it` is the unit. The rules apply in full everywhere else.
- **MUST:** today's offenders are listed by file in the `overrides` of their own package's `biome.json`, because Biome matches a root override's paths against each nested config's folder. The lists only shrink: Q2p splits the functions over the complexity and length limits. A new file never joins a list. JSON has no comments, so this section and the todos are where the lists are explained.
- **MUST:** every export of a package entry has a doc comment. `cargo xtask check ts` runs `apps/docs/scripts/check-api-docs.mjs`, which is TypeDoc's `notDocumented` validation, with warnings as errors, over each package's entries in `exports`. `@laminapdf/wasm` is left out: its code and types are generated by wasm-bindgen. It checks declarations (classes, interfaces, type aliases, functions, variables, enums) and public methods and accessors, not every property.
- **MUST:** `cargo xtask check ts` passes dependency-cruiser, jscpd and knip, which find import cycles, relative imports into another package, duplicated blocks and unused code ([maintainability guideline](maintainability.md#how-its-checked)). Fix a finding where it lives. A tool's baseline or ignore list takes an entry only for what another todo will fix, or for a repetition the guideline explains.
- **MUST:** tests use [Vitest](https://vitest.dev/). Unit tests sit next to the source (`foo.test.ts`). Integration, WASM-interop and corpus tests live in `test/`. Shared fixtures come from root `fixtures/`.
- **MUST:** `describe` names the unit and `it` states the behavior (`describe("open")`, `it("repairs a file whose xref offset is wrong")`). One behavior per test.
- **SHOULD:** run shared packages' tests in a real browser too (Vitest browser mode with Playwright) once they're product code. The spike only ran them in Node.

## Naming and style

| Thing | Convention |
| --- | --- |
| Files | `kebab-case.ts`; tests `kebab-case.test.ts` |
| Types, interfaces, classes | `PascalCase` (`PdfDocument`, `OpenOptions`) |
| Functions, variables, methods | `camelCase` (`open`, `pageCount`) |
| Module-level constants | `UPPER_SNAKE_CASE` for true constants (`MAX_DEPTH`) |
| String-literal unions | `PascalCase` or `kebab-case` members, consistent per type |
| Acronyms | Treat as words: `PdfDocument`, `XrefEntry`, `parseHtml` |

- **MUST:** named exports only. No `export default`. Named exports keep names consistent at every import site and work well with tree-shaking and refactors.
- **MUST NOT:** use `enum`, `namespace` or parameter properties (`erasableSyntaxOnly`). Use string-literal unions and `as const` objects.
- **MUST NOT:** use `any`. Use `unknown` and narrow it. Each non-null assertion (`!`) needs a comment explaining why the value can't be absent. Prefer `noUncheckedIndexedAccess`-friendly code (`.at()`, explicit checks).

## Errors

- **MUST:** each package defines one error class, extending `Error`, with `name` set, a `readonly code` typed as a string-literal union, and structured fields. It also exports an `isXError(e: unknown): e is XError` guard. In `@laminapdf/sdk` this is **`LaminaError<C extends ErrorCode>`** with `isLaminaError(e, code?)` and `toJSON()`, carrying the [ADR 0026](../decisions/0026-error-and-diagnostic-contract.md) fields (`code`, `kind`, `hint`, `didYouMean`, `location`, `data`, `docsUrl`). Codes shared with the Rust core **MUST** match `fixtures/codes.json`: the unions and tables are generated into `src/generated/` by `cargo xtask gen codes`, never written by hand, and generated files are excluded from Biome.
- **MUST NOT:** put document text in `message` or `hint`; it goes only in `data` (ADR 0026 §4).
- **MUST:** throw only instances of that class (or other `Error` subclasses). Never throw strings or plain objects. When wrapping, pass `{ cause }`.
- **MUST:** structured clone keeps neither a custom error class nor its own properties, so an error crosses a worker only as a plain object. An error class that can cross a worker therefore has a `toDiagnostic()` method returning the ADR 0026 §1 object; `LaminaError`'s `toJSON()` does the same job. `@laminapdf/sdk`'s worker posts that object, and the page rebuilds a `LaminaError` from it, so byte ranges travel as `[offset, length]` pairs in `data.ranges`.
- **MUST NOT:** swallow errors silently. Recoverable irregularities in input are returned as `warnings` on the result, as in Rust.
- **SHOULD:** document thrown errors with `@throws {@link LaminaError}` and the codes they can carry.

## API design (types first)

- **SHOULD:** design the public API as types before writing the implementation: write `index.ts`'s exported types, review them in the PR, then implement. The `.d.ts` *is* the contract.
- **MUST:** `src/index.ts` exports public symbols **by name**. `export *` from internal modules is not allowed, and sub-folders don't get barrel files. Expose bigger areas as subpath exports (`@laminapdf/sdk/forms`). A package with one entry per export condition (`@laminapdf/sdk`: `src/index.ts`, `src/entry/node.ts`, `src/entry/workerd.ts`) lists its public names once, in `src/api.ts`, and each entry adds its own `init` and re-exports that module with `export *`, so the entries can't drift; a test compares their exports.
- **SHOULD:** public inputs and outputs use `readonly` properties and `ReadonlyArray`/`Readonly<…>`. Options are a single object parameter (`open(bytes, { maxDecoded })`), and an optional property means "omitted", not `undefined` (`exactOptionalPropertyTypes`).
- **SHOULD:** accept `Uint8Array` (and `ArrayBuffer` where cheap) and return `Uint8Array`. Async only where work is truly async (WASM instantiation, workers).
- **MUST:** every exported symbol has a TSDoc comment (`@param`, `@returns`, `@throws`, `@example`; [tsdoc.org](https://tsdoc.org/)).
- *Recommendation:* once packages are published, add an API report check (for example [API Extractor](https://api-extractor.com/) or `attw` / [Are the Types Wrong?](https://arethetypeswrong.github.io/)) to catch accidental API and packaging changes.

## References

- TypeScript: [tsconfig reference](https://www.typescriptlang.org/tsconfig/), [Modules reference](https://www.typescriptlang.org/docs/handbook/modules/reference.html) (`exports`, `nodenext`, `bundler`, `paths` warning)
- [Biome](https://biomejs.dev/) and [Biome: big projects](https://biomejs.dev/guides/big-projects/)
- [Vitest](https://vitest.dev/)
- pnpm: [Workspaces](https://pnpm.io/workspaces), [Catalogs](https://pnpm.io/catalogs)
- [TSDoc](https://tsdoc.org/)
- [Vite: avoid barrel files](https://vite.dev/guide/performance#avoid-barrel-files)
- lamina spike: [`spikes/pdf-basics/ts/`](../../spikes/pdf-basics/ts/README.md) (`tsconfig.json`, `biome.json`, `src/errors.ts`, `test/portability.test.ts`)
