// The Organise mode's dialogs (Paper O06): Move to page…, the single-pointer way to do what a drag
// does (WCAG 2.2 SC 2.5.7), with a preview of the new order; Blank pages…, which inserts blank pages
// of a size beside the selection or at the end; and the question before an edit that would break a
// signature whose box is on the page (O06-delete-signed), which names the signer.

import { type ReactNode, useEffect, useState } from "react";
import { Button } from "../components/button.js";
import { Radio, RadioGroup } from "../components/choice.js";
import { TextInput } from "../components/field.js";
import { laterCountMessage, laterMessage } from "../later-messages.js";
import { Dialog } from "../overlays/dialog.js";
import { QuestionDialog } from "../overlays/question-dialog.js";
import { gapOf, type MovePosition, moveToGap, pageRanges } from "./moves.js";

/** The page whose number or label is `text`, by label first, then by number (1-based); undefined when none. */
export function pageNamed(text: string, labels: readonly string[]): number | undefined {
  const value = text.trim();
  if (value === "") return undefined;
  const byLabel = labels.indexOf(value);
  if (byLabel >= 0) return byLabel;
  const number = /^\d+$/.test(value) ? Number(value) : Number.NaN;
  return Number.isInteger(number) && number >= 1 && number <= labels.length ? number - 1 : undefined;
}

/** Props of `MoveToPageDialog`. */
export interface MoveToPageDialogProps {
  readonly isOpen: boolean;
  readonly onClose: () => void;
  /** Every page's ID and label, in order. */
  readonly pages: readonly { readonly id: string; readonly label: string }[];
  /** The selected pages' indices. */
  readonly selected: readonly number[];
  /** Moves the selection to gap `gap` (moves.ts). */
  readonly onMove: (gap: number) => void;
}

/** Move to page…: where the selection goes, named by a position and a page, and the order it makes. */
export function MoveToPageDialog(props: MoveToPageDialogProps): ReactNode {
  const { pages, selected } = props;
  const [position, setPosition] = useState<MovePosition>("after");
  const [page, setPage] = useState("");
  useEffect(() => {
    if (props.isOpen) setPage("");
  }, [props.isOpen]);
  const labels = pages.map((p) => p.label);
  const needsPage = position === "after" || position === "before";
  const index = pageNamed(page, labels);
  const error =
    needsPage && page.trim() !== "" && index === undefined
      ? laterMessage("organise.move.page.error", { page })
      : undefined;
  const gap = needsPage
    ? index === undefined
      ? undefined
      : gapOf(position, index, pages.length)
    : gapOf(position, 0, pages.length);
  const move =
    gap === undefined
      ? undefined
      : moveToGap(
          pages.map((p) => p.id),
          selected,
          gap,
        );
  const labelOf = new Map(pages.map((p) => [p.id, p.label]));
  return (
    <Dialog
      isOpen={props.isOpen}
      onOpenChange={(open) => {
        if (!open) props.onClose();
      }}
      title={laterMessage("organise.move.title")}
      closeLabel={laterMessage("organise.close")}
      size="form"
      actions={
        <>
          <Button onPress={props.onClose}>{laterMessage("organise.move.cancel")}</Button>
          <Button
            variant="primary"
            isDisabled={gap === undefined || move === undefined}
            onPress={() => {
              if (gap === undefined) return;
              props.onMove(gap);
              props.onClose();
            }}
          >
            {laterMessage("organise.move.go")}
          </Button>
        </>
      }
    >
      <RadioGroup
        label={laterMessage("organise.move.position")}
        value={position}
        onChange={(value) => setPosition(value as MovePosition)}
      >
        <Radio value="after">{laterMessage("organise.move.after")}</Radio>
        <Radio value="before">{laterMessage("organise.move.before")}</Radio>
        <Radio value="start">{laterMessage("organise.move.start")}</Radio>
        <Radio value="end">{laterMessage("organise.move.end")}</Radio>
      </RadioGroup>
      {needsPage ? (
        <TextInput
          label={laterMessage("organise.move.page")}
          description={laterMessage("organise.move.page.hint", { count: pages.length })}
          value={page}
          onChange={setPage}
          {...(error === undefined ? {} : { errorMessage: error })}
          autoFocus
        />
      ) : null}
      {move === undefined ? null : (
        <p className="paper-organise-order">
          {laterMessage("organise.move.order", {
            order: move.order.map((id) => labelOf.get(id) ?? "?").join(", "),
          })}
        </p>
      )}
    </Dialog>
  );
}

/** The sizes Blank pages… offers: the selected page's, US Letter or A4 (points). */
export type BlankSize = "match" | "letter" | "a4";

/** Where Blank pages… puts its pages. */
export type BlankWhere = "before" | "after" | "end";

/** Props of `BlankPagesDialog`. */
export interface BlankPagesDialogProps {
  readonly isOpen: boolean;
  readonly onClose: () => void;
  /** Whether pages are selected: without, the pages go at the end. */
  readonly hasSelection: boolean;
  readonly onInsert: (count: number, size: BlankSize, where: BlankWhere) => void;
}

/** Blank pages…: how many, of what size, and where. */
export function BlankPagesDialog(props: BlankPagesDialogProps): ReactNode {
  const [count, setCount] = useState("1");
  const [size, setSize] = useState<BlankSize>("match");
  const [where, setWhere] = useState<BlankWhere>("after");
  useEffect(() => {
    if (props.isOpen) setCount("1");
  }, [props.isOpen]);
  const n = /^\d+$/.test(count.trim()) ? Number(count.trim()) : Number.NaN;
  const valid = Number.isInteger(n) && n >= 1 && n <= 100;
  const place = props.hasSelection ? where : "end";
  return (
    <Dialog
      isOpen={props.isOpen}
      onOpenChange={(open) => {
        if (!open) props.onClose();
      }}
      title={laterMessage("organise.blank.title")}
      closeLabel={laterMessage("organise.close")}
      size="form"
      actions={
        <>
          <Button onPress={props.onClose}>{laterMessage("organise.blank.cancel")}</Button>
          <Button
            variant="primary"
            isDisabled={!valid}
            onPress={() => {
              props.onInsert(n, props.hasSelection ? size : size === "match" ? "letter" : size, place);
              props.onClose();
            }}
          >
            {laterMessage("organise.blank.go")}
          </Button>
        </>
      }
    >
      <TextInput
        label={laterMessage("organise.blank.count")}
        value={count}
        onChange={setCount}
        inputMode="numeric"
        autoFocus
        {...(valid || count.trim() === "" ? {} : { errorMessage: laterMessage("organise.blank.count.error") })}
      />
      <RadioGroup label={laterMessage("organise.blank.size")} value={size} onChange={(v) => setSize(v as BlankSize)}>
        {props.hasSelection ? <Radio value="match">{laterMessage("organise.blank.size.match")}</Radio> : null}
        <Radio value="letter">{laterMessage("organise.blank.size.letter")}</Radio>
        <Radio value="a4">{laterMessage("organise.blank.size.a4")}</Radio>
      </RadioGroup>
      {props.hasSelection ? (
        <RadioGroup
          label={laterMessage("organise.blank.where")}
          value={where}
          onChange={(v) => setWhere(v as BlankWhere)}
        >
          <Radio value="before">{laterMessage("organise.blank.before")}</Radio>
          <Radio value="after">{laterMessage("organise.blank.after")}</Radio>
          <Radio value="end">{laterMessage("organise.blank.end")}</Radio>
        </RadioGroup>
      ) : null}
    </Dialog>
  );
}

/** Props of `SignedPageDialog`. */
export interface SignedPageDialogProps {
  /** The question, or undefined when none is asked. */
  readonly ask:
    | {
        readonly action: "delete" | "rotate";
        /** The page's label. */
        readonly page: string;
      }
    | undefined;
  /**
   * The person's answer: Remove signature and delete (or rotate), where the change applies and
   * the next save asks once before it removes the signature, or Cancel.
   */
  readonly onAnswer: (proceed: boolean) => void;
}

/**
 * The question before deleting or turning a page that holds a signature's box (O06-delete-signed).
 * It names the page; it can't name the signer, whom the save planner's signatures don't carry.
 * Remove signature and delete (or rotate) is the danger ghost button; Cancel, the safe answer, has the focus.
 */
export function SignedPageDialog({ ask, onAnswer }: SignedPageDialogProps): ReactNode {
  const page = ask?.page ?? "";
  const remove = laterMessage(ask?.action === "rotate" ? "organise.signed.rotate" : "organise.signed.delete");
  return (
    <QuestionDialog
      open={ask !== undefined}
      title={laterMessage("organise.signed.title", { page })}
      description={laterMessage("organise.signed.body", { page, action: remove })}
      no={laterMessage("organise.signed.cancel")}
      yes={remove}
      yesVariant="danger"
      onAnswer={onAnswer}
    />
  );
}

/** "1 page selected", "24 pages": the toolbar's note on the selection. */
export function selectionNote(selected: number, total: number): string {
  return selected === 0
    ? laterCountMessage(total, { one: "organise.note.none.one", many: "organise.note.none" })
    : laterCountMessage(selected, { one: "organise.note.selected.one", many: "organise.note.selected" });
}

/** "Moved pages 5–6 after page 9.": what a move said, for the announcer. */
export function movedMessage(
  movedIndices: readonly number[],
  order: readonly string[],
  labelOf: (id: string) => string,
): string {
  const pages = pageRanges(movedIndices, (i) => labelOf(order[i] ?? ""));
  const first = movedIndices[0] ?? 0;
  const one = movedIndices.length === 1;
  if (first === 0) return laterMessage(one ? "organise.moved.start.one" : "organise.moved.start", { pages });
  return laterMessage(one ? "organise.moved.one" : "organise.moved", { pages, after: labelOf(order[first - 1] ?? "") });
}
