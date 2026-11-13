// TypeScript in the "Edit and run" editor (todo Q4t): completion of the SDK's API, a value's type
// on hover or by key, and type errors, all answered by TypeScript's language service in a worker
// (types-worker.ts, served at /run/types.js). The worker weighs about 1 MB gzipped, so it starts
// on the reader's first edit, a completion they ask for (Ctrl-Space) or the type key, never when
// "Edit and run" opens: a reader who only runs an example loads none of it. Until it answers, the
// editor completes as before: the file's own names and TypeScript's snippets. Type errors are
// marks only: a run never waits for them, and the SDK's own error still reports what they miss.

import {
  autocompletion,
  type Completion,
  type CompletionContext,
  type CompletionResult,
  type CompletionSource,
  completeFromList,
} from "@codemirror/autocomplete";
import { localCompletionSource, typescriptSnippets } from "@codemirror/lang-javascript";
import { type Diagnostic, linter } from "@codemirror/lint";
import { type EditorState, type Extension, StateEffect, StateField, type Text } from "@codemirror/state";
import { EditorView, hoverTooltip, keymap, showTooltip, type Tooltip, ViewPlugin } from "@codemirror/view";
import type { TypesDetails, TypesHover, TypesQuery, TypesReply, TypesRequest } from "./types-protocol.js";

/** The worker's address, beside the runner's, under the same headers (public/_headers' `/run/*`). */
const WORKER = "/run/types.js";

/**
 * The key that shows the type at the cursor: Ctrl-Shift-Space, beside completion's Ctrl-Space.
 * Not VS Code's Ctrl-K then I: Ctrl-K (⌘K) opens the docs' search.
 */
const TYPE_KEY = "Ctrl-Shift-Space";

/** The keys for TypeScript's help without a pointer, which the editor names under itself. */
export const TYPES_KEYS_HINT = `Ctrl+Space lists what can go here; ${TYPE_KEY.replaceAll("-", "+")} shows the type at the cursor.`;

/**
 * Where TypeScript is: not started, running, or failed (its worker didn't load or stopped), when
 * the editor goes back to completing the file's own names, for good.
 */
let typescriptState: "idle" | "running" | "failed" = "idle";
let worker: Worker | undefined;
let nextId = 0;
const waiting = new Map<number, (reply: TypesReply) => void>();

/** A version per document text: CodeMirror's texts are immutable, so a new text is a new version. */
const versions = new WeakMap<Text, number>();
let nextVersion = 0;
/** The version whose code the worker was sent last. */
let sentVersion = -1;

function versionOf(doc: Text): number {
  let version = versions.get(doc);
  if (version === undefined) {
    version = nextVersion++;
    versions.set(doc, version);
  }
  return version;
}

/** What TypeScript says when it can't say anything: no completions, no type, no errors. */
function nothing(id: number, kind: TypesQuery["kind"]): TypesReply {
  switch (kind) {
    case "complete":
      return { id, kind, items: [] };
    case "details":
      return { id, kind, details: null };
    case "hover":
      return { id, kind, hover: null };
    case "lint":
      return { id, kind, diagnostics: [] };
  }
}

/** Gives up on TypeScript: every waiting request gets nothing, and none is asked again. */
function fail(): void {
  typescriptState = "failed";
  worker?.terminate();
  worker = undefined;
  for (const [id, done] of waiting) done(nothing(id, "lint"));
  waiting.clear();
}

/** Starts the worker, once per page; every editor on the page shares it. */
function start(): Worker | undefined {
  if (typescriptState === "idle") {
    typescriptState = "running";
    worker = new Worker(WORKER, { type: "module" });
    worker.addEventListener("message", (event: MessageEvent<TypesReply>) => {
      waiting.get(event.data.id)?.(event.data);
      waiting.delete(event.data.id);
    });
    worker.addEventListener("error", fail);
    worker.addEventListener("messageerror", fail);
  }
  return worker;
}

/** Whether TypeScript is answering: before it starts, and after it fails, nothing waits for it. */
const running = () => typescriptState === "running";

/** Asks the worker `query` about `doc`; nothing if TypeScript has failed. */
function ask<K extends TypesQuery["kind"]>(
  doc: Text,
  query: Extract<TypesQuery, { kind: K }>,
): Promise<Extract<TypesReply, { kind: K }>> {
  const id = nextId++;
  const version = versionOf(doc);
  const target = start();
  if (target === undefined) return Promise.resolve(nothing(id, query.kind) as Extract<TypesReply, { kind: K }>);
  const request = { ...query, id, version, ...(version === sentVersion ? {} : { code: doc.toString() }) };
  sentVersion = version;
  return new Promise((resolve) => {
    waiting.set(id, (reply) =>
      resolve((reply.kind === query.kind ? reply : nothing(id, query.kind)) as Extract<TypesReply, { kind: K }>),
    );
    target.postMessage(request as TypesRequest);
  });
}

/** CodeMirror's icon for each TypeScript `ScriptElementKind` it has one for. */
const ICONS: Readonly<Record<string, string>> = {
  method: "method",
  function: "function",
  "local function": "function",
  constructor: "class",
  property: "property",
  getter: "property",
  setter: "property",
  class: "class",
  interface: "interface",
  type: "type",
  enum: "enum",
  "enum member": "enum",
  module: "namespace",
  keyword: "keyword",
  var: "variable",
  let: "variable",
  const: "constant",
  "local var": "variable",
  parameter: "variable",
};

/** A signature and its documentation, as a tooltip's or a completion's information. */
function describe(details: TypesDetails): HTMLElement {
  const box = document.createElement("div");
  box.className = "cm-types-info";
  const signature = document.createElement("code");
  signature.className = "cm-types-signature";
  signature.textContent = details.signature;
  box.append(signature);
  if (details.documentation !== "") {
    const doc = document.createElement("p");
    doc.className = "cm-types-doc";
    doc.textContent = details.documentation;
    box.append(doc);
  }
  return box;
}

/** A name being typed, or nothing typed yet after a dot. */
const WORD = /[\w$]*$/;

/** TypeScript's completions at the cursor, once TypeScript has started or when asked for. */
async function completeTypes(context: CompletionContext): Promise<CompletionResult | null> {
  if (typescriptState === "failed" || (typescriptState === "idle" && !context.explicit)) return null;
  const word = context.matchBefore(WORD);
  const from = word?.from ?? context.pos;
  const afterDot = context.state.sliceDoc(Math.max(0, from - 1), from) === ".";
  if (!context.explicit && from === context.pos && !afterDot) return null;
  const { doc } = context.state;
  const { items } = await ask(doc, { kind: "complete", pos: context.pos });
  if (context.aborted || items.length === 0) return null;
  const pos = context.pos;
  const options: Completion[] = items.map((item) => ({
    label: item.label,
    type: ICONS[item.kind] ?? "text",
    sortText: item.sortText,
    info: async () => {
      const { details } = await ask(doc, { kind: "details", pos, name: item.label });
      return details === null ? null : describe(details);
    },
  }));
  return { from, options, validFor: /^[\w$]*$/ };
}

/** The editor's own names, until TypeScript answers for them, and again if it fails. */
const localUntilStarted: CompletionSource = (context) => (running() ? null : localCompletionSource(context));

/** The type at `pos`, as a tooltip, or nothing. */
async function typeTooltip(state: EditorState, pos: number): Promise<Tooltip | null> {
  const { hover } = await ask(state.doc, { kind: "hover", pos });
  return hover === null ? null : tooltipOf(hover);
}

function tooltipOf(hover: TypesHover): Tooltip {
  return {
    pos: hover.from,
    end: hover.to,
    above: true,
    create: () => {
      const dom = describe(hover);
      dom.setAttribute("role", "tooltip");
      return { dom };
    },
  };
}

/** Shows or clears the type key's tooltip. */
const setTypeTip = StateEffect.define<Tooltip | null>();

/** The type key's tooltip: gone on the next edit or cursor move. */
const typeTip = StateField.define<Tooltip | null>({
  create: () => null,
  update(tip, tr) {
    for (const effect of tr.effects) if (effect.is(setTypeTip)) return effect.value;
    return tr.docChanged || tr.selection !== undefined ? null : tip;
  },
  provide: (field) => showTooltip.from(field),
});

/**
 * The type key: the type at the cursor, for readers without a pointer, shown as a tooltip and read
 * out by screen readers (CodeMirror's announcement region), since a tooltip alone isn't.
 */
function showTypeAtCursor(view: EditorView): boolean {
  const { doc, selection } = view.state;
  const pos = selection.main.head;
  void ask(doc, { kind: "hover", pos }).then(({ hover }) => {
    // Only if nothing moved while TypeScript answered.
    if (view.state.doc !== doc || view.state.selection.main.head !== pos) return;
    const said = hover === null ? "No type here." : [hover.signature, hover.documentation].join(" ").trim();
    view.dispatch({
      effects: [setTypeTip.of(hover === null ? null : tooltipOf(hover)), EditorView.announce.of(said)],
    });
  });
  return true;
}

/**
 * Starts TypeScript on the reader's first edit: typing, deleting, pasting or dropping, and Format.
 * Reset, which puts back the example, isn't an edit.
 */
const startOnEdit = ViewPlugin.define(() => ({
  update(update) {
    const edited = update.transactions.some(
      (tr) => tr.docChanged && (tr.isUserEvent("input") || tr.isUserEvent("delete") || tr.isUserEvent("move")),
    );
    if (edited) start();
  },
}));

/** Type errors as marks, once TypeScript has started. */
const typeErrors = linter(
  async (view): Promise<Diagnostic[]> => {
    if (!running()) return [];
    const { doc } = view.state;
    const { diagnostics } = await ask(doc, { kind: "lint" });
    // A later edit gets its own check.
    if (view.state.doc !== doc) return [];
    return diagnostics.map((d) => ({ ...d, source: "TypeScript" }));
  },
  { delay: 500 },
);

/** The look of the information TypeScript gives, in the Paper tokens (theme.ts). */
const typesTheme = EditorView.theme({
  ".cm-types-info": { maxWidth: "36rem", padding: "0.25rem 0.5rem" },
  ".cm-types-signature": { display: "block", whiteSpace: "pre-wrap", fontFamily: "var(--paper-font-mono)" },
  ".cm-types-doc": { margin: "0.25rem 0 0", color: "var(--paper-muted)" },
});

/** Everything above, as the editor's extensions. */
export function typescriptHelp(): Extension {
  return [
    autocompletion({
      override: [completeTypes, localUntilStarted, completeFromList(typescriptSnippets)],
    }),
    hoverTooltip((view, pos) => (running() ? typeTooltip(view.state, pos) : null)),
    typeTip,
    keymap.of([
      { key: TYPE_KEY, run: showTypeAtCursor },
      {
        key: "Escape",
        run: (view) => {
          if (view.state.field(typeTip) === null) return false;
          view.dispatch({ effects: setTypeTip.of(null) });
          return true;
        },
      },
    ]),
    startOnEdit,
    typeErrors,
    typesTheme,
  ];
}
