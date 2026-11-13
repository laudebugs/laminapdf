// The "Edit and run" editor's TypeScript (todo Q4t): TypeScript's own language service over the
// reader's code, answering completions, hover types and errors (types-protocol.ts). The editor
// (types.ts) starts it on the reader's first edit, never before, since it weighs about 1 MB
// gzipped. Its files are the ES library declarations, @laminapdf/sdk's and @laminapdf/wasm's
// declarations (scripts/build-runner.mjs bundles them as `virtual:lamina-types`), and NODE below.
// Nothing here reads the network or evaluates code.

import FILES from "virtual:lamina-types";
import ts from "typescript";
import type { TypesCompletion, TypesDetails, TypesDiagnostic, TypesReply, TypesRequest } from "./types-protocol.js";

/** The reader's file, beside node_modules/ so the SDK's package resolves as it would in a project. */
const EXAMPLE = "/example.ts";

/**
 * What the Node quick start (packages/sdk/examples/quick-start/node.ts) uses of Node, declared by
 * hand: @types/node's declarations would weigh more than the rest together.
 */
const NODE = `declare module "node:fs/promises" {
  export function writeFile(path: string, data: Uint8Array | string): Promise<void>;
  export function readFile(path: string): Promise<Uint8Array>;
}
declare module "node:path" {
  export function join(...paths: string[]): string;
}
interface ImportMeta { readonly main: boolean; }
`;

const files = new Map<string, string>([...Object.entries(FILES), ["/node.d.ts", NODE]]);

/** The examples' own settings: strict, ES2022 with `using`, the SDK resolved as a bundler would. */
const OPTIONS: ts.CompilerOptions = {
  target: ts.ScriptTarget.ES2022,
  module: ts.ModuleKind.ESNext,
  moduleResolution: ts.ModuleResolutionKind.Bundler,
  customConditions: ["browser"],
  lib: ["lib.es2022.d.ts", "lib.esnext.disposable.d.ts"],
  types: [],
  strict: true,
  noEmit: true,
  skipLibCheck: true,
  isolatedModules: true,
};

let code = "";
let version = -1;

/** Whether `path` is a directory of the virtual files. */
function isDirectory(path: string): boolean {
  const prefix = path.endsWith("/") ? path : `${path}/`;
  for (const name of files.keys()) if (name.startsWith(prefix)) return true;
  return false;
}

const host: ts.LanguageServiceHost = {
  getCompilationSettings: () => OPTIONS,
  getScriptFileNames: () => [EXAMPLE, "/node.d.ts"],
  getScriptVersion: (name) => (name === EXAMPLE ? String(version) : "0"),
  getScriptSnapshot: (name) => {
    const text = name === EXAMPLE ? code : files.get(name);
    return text === undefined ? undefined : ts.ScriptSnapshot.fromString(text);
  },
  getCurrentDirectory: () => "/",
  // The libraries named in OPTIONS are read from this file's folder.
  getDefaultLibFileName: () => "/lib/lib.d.ts",
  fileExists: (name) => name === EXAMPLE || files.has(name),
  readFile: (name) => (name === EXAMPLE ? code : files.get(name)),
  directoryExists: isDirectory,
  getDirectories: () => [],
};

const service = ts.createLanguageService(host, ts.createDocumentRegistry());

/** A display's parts as one string. */
const text = (parts: readonly ts.SymbolDisplayPart[] | undefined) => ts.displayPartsToString([...(parts ?? [])]);

/** The first paragraph of documentation, which is what a tooltip has room for. */
const firstParagraph = (parts: readonly ts.SymbolDisplayPart[] | undefined) => text(parts).split(/\n\s*\n/)[0] ?? "";

function complete(pos: number): TypesCompletion[] {
  const result = service.getCompletionsAtPosition(EXAMPLE, pos, {
    includeCompletionsWithInsertText: false,
    includeCompletionsForModuleExports: false,
  });
  return (result?.entries ?? []).map((entry) => ({ label: entry.name, kind: entry.kind, sortText: entry.sortText }));
}

function details(pos: number, name: string, source: string | undefined): TypesDetails | null {
  const entry = service.getCompletionEntryDetails(EXAMPLE, pos, name, undefined, source, undefined, undefined);
  if (entry === undefined) return null;
  return { signature: text(entry.displayParts), documentation: firstParagraph(entry.documentation) };
}

function hover(pos: number) {
  const info = service.getQuickInfoAtPosition(EXAMPLE, pos);
  if (info === undefined) return null;
  return {
    from: info.textSpan.start,
    to: info.textSpan.start + info.textSpan.length,
    signature: text(info.displayParts),
    documentation: firstParagraph(info.documentation),
  };
}

const SEVERITY = {
  [ts.DiagnosticCategory.Error]: "error",
  [ts.DiagnosticCategory.Warning]: "warning",
  [ts.DiagnosticCategory.Suggestion]: "info",
  [ts.DiagnosticCategory.Message]: "info",
} as const;

function lint(): TypesDiagnostic[] {
  const all = [...service.getSyntacticDiagnostics(EXAMPLE), ...service.getSemanticDiagnostics(EXAMPLE)];
  return all.map((diagnostic) => {
    const from = diagnostic.start ?? 0;
    return {
      from,
      to: from + (diagnostic.length ?? 0),
      severity: SEVERITY[diagnostic.category],
      message: ts.flattenDiagnosticMessageText(diagnostic.messageText, "\n"),
    };
  });
}

function answer(request: TypesRequest): TypesReply {
  const { id } = request;
  switch (request.kind) {
    case "complete":
      return { id, kind: "complete", items: complete(request.pos) };
    case "details":
      return { id, kind: "details", details: details(request.pos, request.name, request.source) };
    case "hover":
      return { id, kind: "hover", hover: hover(request.pos) };
    case "lint":
      return { id, kind: "lint", diagnostics: lint() };
  }
}

addEventListener("message", (event: MessageEvent<TypesRequest>) => {
  const request = event.data;
  if (request.version !== version) {
    code = request.code;
    version = request.version;
  }
  postMessage(answer(request) satisfies TypesReply);
});
