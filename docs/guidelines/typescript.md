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
- **MUST NOT:** shared packages depend on DOM-only globals (`window`, `document`, `navigator`, `localStorage`, `HTMLElement` …) unless the package is explicitly a browser UI package (for example `@lamina/viewer`). The root `biome.json` enforces this with `style/noRestrictedGlobals` (`"options": { "deniedGlobals": { "window": "…", "document": "…" } }`), which browser UI packages switch off in their nested config, and the portability test scans `src/` for them too.
- **SHOULD:** environment access (fonts, file pickers, workers) is passed in by the caller. Node-specific convenience lives in a separate package (e.g. `@lamina/node`).
- Note that `BigInt` needs Hermes ≥ 0.70 on React Native (from the spike).

## Lint, format, test

- **MUST:** [Biome](https://biomejs.dev/) does linting **and** formatting. There's one root `biome.json` with the `recommended` preset and the formatter enabled. Nested configs use `"extends": "//"` and say why they override anything ([Biome: big projects](https://biomejs.dev/guides/big-projects/)). Don't add ESLint or Prettier alongside it.
- **MUST:** `biome ci` passes in CI. Turn off a rule inline with `// biome-ignore lint/<group>/<rule>: <reason>`. The reason is mandatory.
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
| Acronyms | Treat as words: `PdfError`, `XrefEntry`, `parseHtml` |

- **MUST:** named exports only. No `export default`. Named exports keep names consistent at every import site and work well with tree-shaking and refactors.
- **MUST NOT:** use `enum`, `namespace` or parameter properties (`erasableSyntaxOnly`). Use string-literal unions and `as const` objects.
- **MUST NOT:** use `any`. Use `unknown` and narrow it. Each non-null assertion (`!`) needs a comment explaining why the value can't be absent. Prefer `noUncheckedIndexedAccess`-friendly code (`.at()`, explicit checks).

## Errors

- **MUST:** each package defines one error class, extending `Error`, with `name` set, a `readonly code` typed as a string-literal union, and structured fields (`offset`). It also exports an `isXError(e: unknown): e is XError` guard. This is the spike's `PdfError` pattern. Codes shared with the Rust core **MUST** match its error variants.
- **MUST:** throw only instances of that class (or other `Error` subclasses). Never throw strings or plain objects. When wrapping, pass `{ cause }`.
- **MUST NOT:** swallow errors silently. Recoverable irregularities in input are returned as `warnings` on the result, as in Rust.
- **SHOULD:** document thrown errors with `@throws {@link PdfError}` and the codes they can carry.

## API design (types first)

- **SHOULD:** design the public API as types before writing the implementation: write `index.ts`'s exported types, review them in the PR, then implement. The `.d.ts` *is* the contract.
- **MUST:** `src/index.ts` exports public symbols **by name**. `export *` from internal modules is not allowed, and sub-folders don't get barrel files. Expose bigger areas as subpath exports (`@lamina/pdf/forms`).
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
