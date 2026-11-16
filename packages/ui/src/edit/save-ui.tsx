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
import { type LaterMessageId, laterMessage } from "../later-messages.js";
import { message } from "../messages.js";
import { Dialog } from "../overlays/dialog.js";
import { Menu, type MenuEntry, MenuTrigger, Popover, PopoverTrigger } from "../overlays/popover.js";

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

/** "Saving this file would invalidate its certification. …": the close confirmation's line on a save that would. */
export function conflictLine(signatures: readonly SaveSignature[]): string {
  return laterMessage("edit.close.conflict", { kind: conflictKind(signatures) });
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
export function SaveStrip({
  plan,
  copy,
}: {
  plan: SavePlanSlice | undefined;
  /**
   * Save a copy, offered while a save would invalidate a signature (O02's banner after Apply
   * anyway): its name here ("Save a copy…", "Download a copy") and what it does.
   */
  copy?: { readonly label: string; readonly onPress: () => void };
}): ReactNode {
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
      {conflict && copy !== undefined ? (
        <Button size={26} onPress={copy.onPress}>
          {copy.label}
        </Button>
      ) : null}
      {listed.length === 0 ? null : <StripSignatures listed={listed} />}
    </section>
  );
}

/** The strip's Details: the signatures it names, shown and hidden in place. */
function StripSignatures({ listed }: { listed: readonly SaveSignature[] }): ReactNode {
  const [details, setDetails] = useState(false);
  return (
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
  );
}

/** How a copy is written: saved as a file where the platform can, else downloaded. */
export type CopyKind = "save" | "download";

/** The remove-signature confirmation's words for a copy, saved or downloaded (O02's "Save a copy without the certification?"). */
const COPY_WORDS: Record<
  CopyKind,
  { readonly title: LaterMessageId; readonly body: LaterMessageId; readonly confirm: LaterMessageId }
> = {
  save: { title: "save.copy.title", body: "save.copy.body", confirm: "save.copy.confirm" },
  download: {
    title: "save.copy.title.download",
    body: "save.copy.body.download",
    confirm: "save.copy.confirm.download",
  },
};

/**
 * The confirmation before a save that invalidates signatures (O02's "Remove the certification and
 * save?"): it names each one; Cancel has the focus, and the confirm button is the danger one.
 */
export function RemoveSignaturesDialog({
  signatures,
  copy,
  onAnswer,
}: {
  /** The signatures the save would invalidate; undefined while nothing is asked. */
  signatures: readonly SaveSignature[] | undefined;
  /**
   * Set for a copy, which leaves the open file signed: `save` where the copy is saved as a file,
   * `download` where the browser downloads it. Left out for a save of the file itself.
   */
  copy?: CopyKind | undefined;
  onAnswer: (remove: boolean) => void;
}): ReactNode {
  const kind = conflictKind(signatures ?? []);
  const words = copy === undefined ? undefined : COPY_WORDS[copy];
  return (
    <Dialog
      isOpen={signatures !== undefined}
      isRequired
      role="alertdialog"
      onOpenChange={(open) => {
        if (!open) onAnswer(false);
      }}
      onEscape={() => onAnswer(false)}
      title={words === undefined ? message("save.remove.title", { kind }) : laterMessage(words.title, { kind })}
      description={words === undefined ? message("save.remove.body") : laterMessage(words.body)}
      actions={
        <>
          <Button autoFocus onPress={() => onAnswer(false)}>
            {message("edit.close.cancel")}
          </Button>
          <Button variant="danger-filled" onPress={() => onAnswer(true)}>
            {words === undefined ? message("save.remove.confirm") : laterMessage(words.confirm)}
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

/** The restrictions a signature puts on changes, as the save planner names them (`Violation.restriction`). */
export type RestrictionKind = "docMdp" | "fieldMdp" | "usageRights" | "signedBytes" | "signedWidget";

/** A signature restriction a change breaks, as Details names it: the signature, and for a certification its level. */
export interface SignatureRestriction {
  readonly restriction: RestrictionKind;
  /** The certification's DocMDP level (`/P`: 1, 2 or 3). */
  readonly permission?: number;
  /** The 1-based version of the file the signature closes, when known. */
  readonly revision?: number;
}

/** What Details says of a restriction: its name, and what it allows (a certification's by its DocMDP level). */
interface RestrictionWords {
  readonly title: LaterMessageId;
  readonly allows: (permission?: number) => LaterMessageId;
}

/** A signature's own restriction: the signed bytes, its field lock, or its box on the page. */
const SIGNATURE: RestrictionWords = {
  title: "organise.details.signature",
  allows: () => "organise.details.signature.allows",
};

/** Details' words for each restriction. */
const RESTRICTION_WORDS: Record<RestrictionKind, RestrictionWords> = {
  docMdp: {
    title: "organise.details.certification",
    allows: (permission) =>
      permission === 1 ? "organise.details.p1" : permission === 3 ? "organise.details.p3" : "organise.details.p2",
  },
  usageRights: { title: "organise.details.usageRights", allows: () => "organise.details.usageRights.allows" },
  fieldMdp: SIGNATURE,
  signedBytes: SIGNATURE,
  signedWidget: SIGNATURE,
};

/**
 * Details (O02, O06): a button whose popover names each signature a change breaks and what it
 * allows. By kind and level: the save plan carries no signer's name or date, and names the field
 * only by Lamina PDF's own ID for it, not the name the file gives it.
 */
export function SignatureDetails({ restrictions }: { restrictions: readonly SignatureRestriction[] }): ReactNode {
  return (
    <PopoverTrigger>
      <Button size={30}>{message("save.strip.details")}</Button>
      <Popover aria-label={laterMessage("organise.details")} placement="bottom end">
        <dl className="paper-signature-details">
          {restrictions.map((restriction, i) => {
            const words = RESTRICTION_WORDS[restriction.restriction];
            return (
              // biome-ignore lint/suspicious/noArrayIndexKey: a fixed list, read once per open.
              <div key={i}>
                <dt>{laterMessage(words.title)}</dt>
                <dd>{laterMessage(words.allows(restriction.permission))}</dd>
                {restriction.revision === undefined ? null : (
                  <dd>{laterMessage("organise.details.revision", { revision: restriction.revision })}</dd>
                )}
              </div>
            );
          })}
        </dl>
      </Popover>
    </PopoverTrigger>
  );
}
