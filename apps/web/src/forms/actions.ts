// What a push button does when it is pressed (E8 plan §2.3), with nothing to draw: which of a
// field's actions the reader runs, the ops a Hide makes, the submit dialog's plan and how a failed
// send reads. The controller runs the steps; these are pure, so they run in Node with the app's
// tests. Nothing here runs a script: JavaScript, Launch, ImportData and the actions on other files
// are named and left alone (ADR 0021).

import type { FieldAction, HideTarget, Op, SubmitRequest } from "@laminapdf/sdk";
import type { SubmitFormResult } from "@laminapdf/ui";
import type { SubmitPlan, SubmitProblem } from "@laminapdf/ui/editing";

/** Why an action isn't run: the words the reader says where the person clicked. */
type NotRun = "javaScript" | "launch" | "importData" | "otherFile" | "other";

/** One thing the reader does for a press, in the order the file lists the actions. */
export type ActionStep =
  | { readonly kind: "page"; readonly index: number | undefined }
  | { readonly kind: "move"; readonly to: "next" | "previous" | "first" | "last" }
  | { readonly kind: "link"; readonly url: string }
  | { readonly kind: "reset"; readonly fields: readonly string[] }
  | { readonly kind: "hide"; readonly targets: readonly HideTarget[]; readonly hide: boolean }
  /** `index` is the action's place in the field's list, which `submitRequest` takes. */
  | { readonly kind: "submit"; readonly index: number }
  | { readonly kind: "not-run"; readonly why: NotRun; readonly name?: string };

/**
 * The steps for a press of a field with `actions`: its click actions through `/Next`, in order.
 * The pointer and focus triggers don't run: a press is the one thing the reader acts on, so a
 * file can't make the reader send, open or hide anything by moving the pointer.
 */
export function planActions(actions: readonly FieldAction[]): ActionStep[] {
  const steps: ActionStep[] = [];
  actions.forEach((entry, index) => {
    if (entry.trigger !== "click") return;
    const { action } = entry;
    switch (action.type) {
      case "goTo":
        steps.push({ kind: "page", index: action.pageIndex });
        break;
      case "named":
        steps.push(namedStep(action.action, action.name));
        break;
      case "uri":
        steps.push({ kind: "link", url: action.url });
        break;
      case "resetForm":
        steps.push({ kind: "reset", fields: action.fields });
        break;
      case "hide":
        steps.push({ kind: "hide", targets: action.targets, hide: action.hide });
        break;
      case "submitForm":
        steps.push({ kind: "submit", index });
        break;
      case "javaScript":
        steps.push({ kind: "not-run", why: "javaScript" });
        break;
      case "launch":
        steps.push({ kind: "not-run", why: "launch" });
        break;
      case "importData":
        steps.push({ kind: "not-run", why: "importData" });
        break;
      case "goToR":
      case "goToE":
        steps.push({ kind: "not-run", why: "otherFile" });
        break;
      default:
        steps.push({ kind: "not-run", why: "other", name: action.name });
    }
  });
  return steps;
}

/** The step of a named action: the four page moves, else one that isn't run. */
function namedStep(named: string, name: string | undefined): ActionStep {
  switch (named) {
    case "nextPage":
      return { kind: "move", to: "next" };
    case "prevPage":
      return { kind: "move", to: "previous" };
    case "firstPage":
      return { kind: "move", to: "first" };
    case "lastPage":
      return { kind: "move", to: "last" };
    default:
      return { kind: "not-run", why: "other", name: name ?? "Named" };
  }
}

/** The page a move goes to from `current` in a document of `count` pages. */
export function moveTarget(to: "next" | "previous" | "first" | "last", current: number, count: number): number {
  const last = Math.max(0, count - 1);
  switch (to) {
    case "next":
      return Math.min(last, current + 1);
    case "previous":
      return Math.max(0, current - 1);
    case "first":
      return 0;
    default:
      return last;
  }
}

/** Annotation `/F`'s Hidden bit (ISO 32000-2 §12.5.3, Table 167). */
const HIDDEN = 1 << 1;

/**
 * The ops of a Hide: `setFieldHidden` for each field, and `updateAnnot` with the flags changed for
 * each annotation that isn't a field's widget, whose current flags `flagsOf` gives (an annotation it
 * can't find is left alone). Fields and annotations already as asked make no op.
 */
export function hideOps(
  targets: readonly HideTarget[],
  hide: boolean,
  flagsOf: (annotation: string) => number | undefined,
  modified: string,
): Op[] {
  const ops: Op[] = [];
  for (const target of targets) {
    if ("field" in target) {
      ops.push({ op: "setFieldHidden", args: { field: target.field, hidden: hide } } as Op);
      continue;
    }
    const flags = flagsOf(target.annotation);
    if (flags === undefined) continue;
    const next = hide ? flags | HIDDEN : flags & ~HIDDEN;
    if (next !== flags) ops.push({ op: "updateAnnot", args: { id: target.annotation, flags: next, modified } } as Op);
  }
  return ops;
}

/** `text` as shown to a person: control and format characters (bidi marks, zero-width spaces) left out, except line breaks and tabs. */
export function plain(text: string): string {
  return text.replace(/[\p{Cc}\p{Cf}]/gu, (c) => (c === "\n" || c === "\t" ? c : ""));
}

/** An address as the dialog shows it: scheme, host, then the path and the query, which is part of the address. */
function addressOf(address: string, fallback: string): SubmitPlan["to"] {
  try {
    const url = new URL(address);
    const path = `${url.pathname === "/" ? "" : url.pathname}${url.search}`;
    return { scheme: `${url.protocol}//`, host: plain(url.host), path: plain(path) };
  } catch {
    return { scheme: "", host: plain(fallback), path: "" };
  }
}

/** The submit dialog's plan for `request` sent by the button `button`. */
export function submitPlan(button: string, request: SubmitRequest): SubmitPlan {
  return {
    button: plain(button),
    to: addressOf(request.url, request.host),
    format: request.format,
    fields: request.fields.map((f) => ({ name: plain(f.name), value: plain(f.value) })),
    emptyRequired: request.emptyRequired.map(plain),
  };
}

/** The dialog's plan for a button whose request the library refused: the address as stored, no fields. */
export function refusedPlan(button: string, address: string): SubmitPlan {
  return { button: plain(button), to: addressOf(address, address), format: "fdf", fields: [], emptyRequired: [] };
}

/** The dialog's problem for the library's refusal `code` of a request, if it is one. */
export function refusalOf(error: unknown): SubmitProblem | undefined {
  const code = typeof error === "object" && error !== null ? (error as { code?: unknown }).code : undefined;
  if (code === "submit-pdf-refused") return { kind: "pdf" };
  if (code === "submit-url-refused") return { kind: "insecure" };
  return undefined;
}

/**
 * How a send that didn't go through reads, or undefined for one that went through or that the person
 * cancelled. No answer is "blocked" where a web page makes the request (it can't tell a server that
 * refuses cross-origin requests from one that is down), and "unreachable" where the shell does;
 * `mayHaveSent` says whether the request could have reached the server anyway.
 */
export function problemOf(result: SubmitFormResult, canSubmitWithoutCors: boolean): SubmitProblem | undefined {
  if (result.sent || result.reason === "cancelled") return undefined;
  switch (result.reason) {
    case "refused":
      return { kind: "insecure" };
    case "timeout":
      return { kind: "timeout" };
    case "status":
      return { kind: "status", status: result.status };
    default:
      return { kind: canSubmitWithoutCors ? "unreachable" : "blocked", mayHaveSent: result.mayHaveSent };
  }
}
