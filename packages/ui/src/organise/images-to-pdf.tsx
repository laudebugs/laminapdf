// A new PDF from images (Paper O07, From images): the chosen images in order, each moved with
// buttons, the page size (fit each image, Letter or A4) and the margin. One page per image; the
// result is a new document, so the primary says so: "Make a PDF of 3 images".

import { type ReactNode, type RefObject, useId, useLayoutEffect, useRef, useState } from "react";
import { Button, IconButton } from "../components/button.js";
import { Select } from "../components/field.js";
import { laterCountMessage, laterMessage } from "../later-messages.js";
import { Dialog } from "../overlays/dialog.js";

/** What the user chose: the images' order and how each page is laid out. */
export interface ImagesToPdfChoice {
  /** Indices into `files`, in the pages' order. */
  readonly order: readonly number[];
  /** Each page the image's own size, or Letter or A4 with the image fitted inside. */
  readonly pageSize: "image" | "letter" | "a4";
  /** Points of white space around each image. */
  readonly margin: number;
}

/** Props of `ImagesToPdfDialog`. */
export interface ImagesToPdfDialogProps {
  isOpen: boolean;
  /** The chosen images, in the order chosen. */
  files: readonly { name: string }[];
  /** Choose images…: the shell opens the file picker and passes the new files. */
  onChooseFiles: () => void;
  /** The primary: make the PDF, a new file. The shell closes the dialog. */
  onMake: (choice: ImagesToPdfChoice) => void;
  /** Cancel, Close, Escape or the scrim. */
  onClose: () => void;
}

/** The margins offered, in points: none, a quarter, half and a whole inch. */
const MARGINS = { none: 0, narrow: 18, medium: 36, wide: 72 } as const;
type Margin = keyof typeof MARGINS;

const identity = (count: number) => Array.from({ length: count }, (_, i) => i);

/** `order` with the entry at `at` swapped with its neighbour `delta` away. */
function moved(order: readonly number[], at: number, delta: -1 | 1): number[] {
  const next = [...order];
  const to = at + delta;
  const a = next[at];
  const b = next[to];
  if (a === undefined || b === undefined) return next;
  next[at] = b;
  next[to] = a;
  return next;
}

/** The last move: which image (an index into `files`) went which way. */
interface LastMove {
  readonly image: number;
  readonly delta: -1 | 1;
}

/** The order, kept for these files and starting again when the shell passes new ones, and the last move. */
function useOrder(files: ImagesToPdfDialogProps["files"]) {
  const [state, setState] = useState<{ files: typeof files; order: number[]; last?: LastMove }>({
    files,
    order: identity(files.length),
  });
  if (state.files !== files) setState({ files, order: identity(files.length) });
  const current = state.files === files ? state : { order: identity(files.length) };
  const { order } = current;
  return {
    order,
    last: "last" in current ? current.last : undefined,
    move: (at: number, delta: -1 | 1) => {
      const image = order[at];
      if (image !== undefined) setState({ files, order: moved(order, at, delta), last: { image, delta } });
    },
  };
}

/**
 * A move that takes an image to the list's end disables the button just pressed, and a disabled
 * button drops focus to the page; so focus goes to the other button of the same row.
 */
function useFocusAfterMove(list: RefObject<HTMLOListElement | null>, order: readonly number[], last?: LastMove) {
  useLayoutEffect(() => {
    if (last === undefined) return;
    const at = order.indexOf(last.image);
    const atEdge = last.delta === -1 ? at === 0 : at === order.length - 1;
    if (!atEdge) return;
    const row = list.current?.querySelector(`[data-image="${last.image}"]`);
    row?.querySelector<HTMLButtonElement>(last.delta === -1 ? "[data-move=down]" : "[data-move=up]")?.focus();
  }, [list, order, last]);
}

/** "receipt-back.jpg, 1 of 3" after a move, for the live region; empty before the first. */
function moveAnnouncement(files: ImagesToPdfDialogProps["files"], order: readonly number[], last?: LastMove): string {
  if (last === undefined) return "";
  return laterMessage("imagesToPdf.moved", {
    name: files[last.image]?.name ?? "",
    position: order.indexOf(last.image) + 1,
    count: order.length,
  });
}

/** The images in order, each with Move up and Move down. */
function ImageOrder({
  files,
  order,
  last,
  labelledBy,
  onMove,
}: {
  files: ImagesToPdfDialogProps["files"];
  order: readonly number[];
  last: LastMove | undefined;
  labelledBy: string;
  onMove: (at: number, delta: -1 | 1) => void;
}): ReactNode {
  const list = useRef<HTMLOListElement>(null);
  useFocusAfterMove(list, order, last);
  if (order.length === 0) return <p className="paper-documents-file-facts">{laterMessage("imagesToPdf.none")}</p>;
  return (
    <ol className="paper-documents-order" aria-labelledby={labelledBy} ref={list}>
      {order.map((index, at) => {
        const name = files[index]?.name ?? "";
        return (
          <li key={index} className="paper-documents-order-item" data-image={index}>
            <span className="paper-documents-file-name">{name}</span>
            {/* biome-ignore lint/a11y/useSemanticElements: a group of buttons for one list item; <fieldset> holds form controls. */}
            <span className="paper-documents-order-moves" role="group" aria-label={name}>
              <IconButton
                icon="chevron-up"
                size={26}
                label={laterMessage("imagesToPdf.up", { name })}
                data-move="up"
                isDisabled={at === 0}
                onPress={() => onMove(at, -1)}
              />
              <IconButton
                icon="chevron-down"
                size={26}
                label={laterMessage("imagesToPdf.down", { name })}
                data-move="down"
                isDisabled={at === order.length - 1}
                onPress={() => onMove(at, 1)}
              />
            </span>
          </li>
        );
      })}
    </ol>
  );
}

/** Makes a new PDF of the chosen images, one page each. */
export function ImagesToPdfDialog(props: ImagesToPdfDialogProps): ReactNode {
  const { order, last, move } = useOrder(props.files);
  const heading = useId();
  const [pageSize, setPageSize] = useState<ImagesToPdfChoice["pageSize"]>("image");
  const [margin, setMargin] = useState<Margin>("none");
  const count = order.length;
  return (
    <Dialog
      isOpen={props.isOpen}
      onOpenChange={(open) => {
        if (!open) props.onClose();
      }}
      size="form"
      title={laterMessage("imagesToPdf.title")}
      description={laterMessage("imagesToPdf.description")}
      closeLabel={laterMessage("imagesToPdf.close")}
      actions={
        <>
          <Button onPress={props.onClose}>{laterMessage("imagesToPdf.cancel")}</Button>
          <Button
            variant="primary"
            isDisabled={count === 0}
            onPress={() => props.onMake({ order, pageSize, margin: MARGINS[margin] })}
          >
            {laterCountMessage(count, { one: "imagesToPdf.go.one", many: "imagesToPdf.go" })}
          </Button>
        </>
      }
    >
      <h3 className="paper-documents-heading" id={heading}>
        {laterMessage("imagesToPdf.images")}
      </h3>
      <ImageOrder files={props.files} order={order} last={last} labelledBy={heading} onMove={move} />
      <p className="paper-visually-hidden" aria-live="polite">
        {moveAnnouncement(props.files, order, last)}
      </p>
      <Button size={30} icon="image" onPress={props.onChooseFiles}>
        {laterMessage("imagesToPdf.choose")}
      </Button>
      <Select<ImagesToPdfChoice["pageSize"]>
        label={laterMessage("imagesToPdf.size")}
        value={pageSize}
        onChange={setPageSize}
        options={(["image", "letter", "a4"] as const).map((value) => ({
          value,
          label: laterMessage(`imagesToPdf.size.${value}`),
        }))}
      />
      <Select<Margin>
        label={laterMessage("imagesToPdf.margin")}
        value={margin}
        onChange={setMargin}
        options={(Object.keys(MARGINS) as Margin[]).map((value) => ({
          value,
          label: laterMessage(`imagesToPdf.margin.${value}`),
        }))}
      />
    </Dialog>
  );
}
