// Saving in the editor (Paper O02): the top bar's Save button with its options, the strip under
// the top bar that says how the file saves when that isn't plain, and the confirmations before a
// save that breaks a signature or a standard the file claims. The shell plans the save and writes
// the file; these draw what the plan says. They are in `@laminapdf/ui/editing`, loaded once a
// document is open (#380).

import { type ReactNode, useState } from "react";
import { useCommand } from "../commands/react.js";
import type { CommandId } from "../commands/types.js";
import { Button, IconButton } from "../components/button.js";
import { Notice } from "../components/notice.js";
import type { KeyPlatform } from "../components/shortcut-chip.js";
import { message } from "../messages.js";
import { Dialog } from "../overlays/dialog.js";
import { Menu, type MenuEntry, MenuTrigger } from "../overlays/popover.js";

/** A signature a save is about: its kind and the field that holds it (the save plan's). */
export interface SaveSignature {
  /** `approval`, `certification`, `docTimeStamp` or `usageRights`. */
  readonly kind: string;
  readonly field?: string;
}

/** What the strip and the confirmations read of a save plan. */
export interface SavePlanSlice {
  readonly constraint: "free" | "appendOnly" | "requiresRewrite" | "conflict";
  /** Every signature in the file. */
  readonly signatures: readonly SaveSignature[];
  /** The signatures this save would invalidate. */
  readonly wouldInvalidate: readonly SaveSignature[];
}

/** A signature's kind in a sentence: "certification", "signature". */
export function signatureKind(kind: string): string {
  switch (kind) {
    case "certification":
      return message("save.kind.certification");
    case "docTimeStamp":
      return message("save.kind.docTimeStamp");
    case "usageRights":
      return message("save.kind.usageRights");
    default:
      return message("save.kind.approval");
  }
}

/**
 * The lines naming `signatures`: "The certification", or "The signature (1 of 2)" for several.
 * By kind, not field: the plan's field is an internal ID, not the name a person gave it.
 */
function signatureLines(signatures: readonly SaveSignature[]): string[] {
  const count = signatures.length;
  return signatures.map((signature, i) => {
    const kind = signatureKind(signature.kind);
    return count === 1 ? message("save.signature", { kind }) : message("save.signature.nth", { kind, n: i + 1, count });
  });
}

/** The kind a conflict names: the certification when one is among them, else the first's. */
function conflictKind(signatures: readonly SaveSignature[]): string {
  const certified = signatures.some((s) => s.kind === "certification");
  return signatureKind(certified ? "certification" : (signatures[0]?.kind ?? "approval"));
}

/** The save commands the options menu lists, in order. */
const OPTIONS: readonly CommandId[] = ["file.save-as", "file.save-optimised", "file.save-copy"];

/**
 * Save and its options (O02): a primary button while there are changes, and a menu of Save as…,
 * Save as optimised and Download a copy, each disabled with its reason when it can't run.
 */
export function SaveButton({ keys }: { keys: KeyPlatform }): ReactNode {
  const save = useCommand("file.save");
  const views = [
    useCommand(OPTIONS[0] as CommandId),
    useCommand(OPTIONS[1] as CommandId),
    useCommand(OPTIONS[2] as CommandId),
  ];
  if (save.availability.state !== "available") return null;
  const entries: MenuEntry<CommandId>[] = views
    .filter((view) => view.availability.state !== "hidden")
    .map((view) => ({
      id: view.id,
      label: view.title,
      ...(view.availability.state === "disabled" ? { isDisabled: true, disabledReason: view.availability.reason } : {}),
    }));
  return (
    // biome-ignore lint/a11y/useSemanticElements: a split button's two parts; <fieldset> holds form controls.
    <div className="paper-top-bar-save" role="group" aria-label={save.title}>
      <Button variant="primary" size={26} onPress={() => save.run()}>
        {save.title}
      </Button>
      {entries.length === 0 ? null : (
        <MenuTrigger>
          <IconButton icon="chevron-down" label={message("save.options")} size={26} />
          <Menu
            aria-label={message("save.options")}
            items={entries}
            onAction={(id) => views.find((view) => view.id === id)?.run()}
            platform={keys}
            placement="bottom end"
          />
        </MenuTrigger>
      )}
    </div>
  );
}

/**
 * The strip under the top bar (O02) when the file doesn't save plainly: signed (the changes are
 * added after the signature), rewritten whole, or a save that would invalidate a signature, with
 * the signatures it names. A labelled region, announced politely; nothing for a plain file.
 */
export function SaveStrip({ plan }: { plan: SavePlanSlice | undefined }): ReactNode {
  const [details, setDetails] = useState(false);
  if (plan === undefined || plan.constraint === "free") return null;
  const conflict = plan.constraint === "conflict";
  const listed = conflict ? plan.wouldInvalidate : plan.signatures;
  const tone = conflict ? "danger" : plan.constraint === "appendOnly" ? "neutral" : "warning";
  const title = conflict
    ? message("save.strip.conflict", { kind: conflictKind(plan.wouldInvalidate) })
    : message(plan.constraint === "appendOnly" ? "save.strip.appendOnly" : "save.strip.requiresRewrite");
  return (
    <section className="paper-save-strip" aria-label={message("save.strip")} aria-live="polite">
      <Notice tone={tone} title={title}>
        {conflict ? message("save.strip.conflictHint") : null}
      </Notice>
      {listed.length === 0 ? null : (
        <>
          <Button variant="ghost" size={26} aria-expanded={details} onPress={() => setDetails(!details)}>
            {message(details ? "save.strip.hideDetails" : "save.strip.details")}
          </Button>
          {details ? (
            <ul className="paper-save-strip-signatures">
              {signatureLines(listed).map((line) => (
                <li key={line}>{line}</li>
              ))}
            </ul>
          ) : null}
        </>
      )}
    </section>
  );
}

/**
 * The confirmation before a save that invalidates signatures (O02's "Remove the certification and
 * save?"): it names each one; Cancel has the focus, and the confirm button is the danger one.
 */
export function RemoveSignaturesDialog({
  signatures,
  onAnswer,
}: {
  /** The signatures the save would invalidate; undefined while nothing is asked. */
  signatures: readonly SaveSignature[] | undefined;
  onAnswer: (remove: boolean) => void;
}): ReactNode {
  return (
    <Dialog
      isOpen={signatures !== undefined}
      isRequired
      role="alertdialog"
      onOpenChange={(open) => {
        if (!open) onAnswer(false);
      }}
      onEscape={() => onAnswer(false)}
      title={message("save.remove.title", { kind: conflictKind(signatures ?? []) })}
      description={message("save.remove.body")}
      actions={
        <>
          <Button autoFocus onPress={() => onAnswer(false)}>
            {message("edit.close.cancel")}
          </Button>
          <Button variant="danger-filled" onPress={() => onAnswer(true)}>
            {message("save.remove.confirm")}
          </Button>
        </>
      }
    >
      <ul className="paper-save-signatures">
        {signatureLines(signatures ?? []).map((line) => (
          <li key={line}>{line}</li>
        ))}
      </ul>
    </Dialog>
  );
}

/** The confirmation before a save that breaks a standard the file claims (PDF/A, PDF/UA): each warning, then Save anyway. */
export function ConformanceDialog({
  warnings,
  onAnswer,
}: {
  /** The warnings' sentences; undefined while nothing is asked. */
  warnings: readonly string[] | undefined;
  onAnswer: (save: boolean) => void;
}): ReactNode {
  return (
    <Dialog
      isOpen={warnings !== undefined}
      isRequired
      role="alertdialog"
      onOpenChange={(open) => {
        if (!open) onAnswer(false);
      }}
      onEscape={() => onAnswer(false)}
      title={message("save.conformance.title")}
      description={message("save.conformance.body")}
      actions={
        <>
          <Button autoFocus onPress={() => onAnswer(false)}>
            {message("edit.close.cancel")}
          </Button>
          <Button variant="primary" onPress={() => onAnswer(true)}>
            {message("save.conformance.confirm")}
          </Button>
        </>
      }
    >
      <ul className="paper-save-signatures">
        {(warnings ?? []).map((warning) => (
          <li key={warning}>{warning}</li>
        ))}
      </ul>
    </Dialog>
  );
}
