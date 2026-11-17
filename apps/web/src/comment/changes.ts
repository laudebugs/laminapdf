// The inspector's changes to the selected annotations (Paper C02): each change one edit, with one
// `updateAnnot` (or `deleteAnnot`) per annotation its kind takes it for. An annotation's record
// carries its colour and opacity but not its outline, fill, line endings or smoothing, so what this
// session wrote of those is kept here, by annotation ID, for the inspector to show.

import type { Op, PageAnnotation } from "@laminapdf/sdk";
import { type BorderStyleName, type CommentColour, type LineEndingName, laterMessage } from "@laminapdf/ui/editing";
import type { Viewer } from "@laminapdf/viewer";
import { colourValue } from "./marks.ts";
import {
  type AnnotChange,
  borderPatch,
  borderStyle,
  deleteAnnotOp,
  endsPatch,
  fillPatch,
  kindName,
  smoothingPatch,
  updateAnnotOp,
} from "./ops.ts";

/** What this session wrote of an annotation's drawing, which its record doesn't carry. */
export interface KnownStyle {
  readonly width?: number;
  readonly fill?: CommentColour | "none";
  readonly border?: BorderStyleName;
  readonly ends?: readonly [LineEndingName, LineEndingName];
  readonly smoothing?: boolean;
}

/** The inspector's changes over the selection `selection()` gives, applied through `edits`. */
export class AnnotationChanges {
  readonly #edits: Viewer["edits"];
  readonly #selection: () => readonly PageAnnotation[];
  readonly #changed: () => void;
  readonly #known = new Map<string, KnownStyle>();

  /** `changed` runs when what the session knows of an annotation changes. */
  constructor(edits: Viewer["edits"], selection: () => readonly PageAnnotation[], changed: () => void) {
    this.#edits = edits;
    this.#selection = selection;
    this.#changed = changed;
  }

  /** What this session wrote of annotation `id`'s drawing. */
  known(id: string): KnownStyle {
    return this.#known.get(id) ?? {};
  }

  /** Records what a new annotation `id` was drawn with. */
  remember(id: string, style: KnownStyle): void {
    this.#known.set(id, { ...this.known(id), ...style });
  }

  /** The line colour, or a markup's or note's colour. */
  setColour(colour: CommentColour, family: "fill" | "line"): void {
    const [r, g, b] = colourValue(colour, family);
    this.#change(() => ({ color: [r, g, b] }));
  }

  /** A closed shape's fill: none, or a fill colour. */
  setFill(fill: CommentColour | "none"): void {
    const interior = fill === "none" ? null : ([...colourValue(fill, "fill")] as [number, number, number]);
    this.#change(
      (a) => spec(fillPatch(a.subtype, interior)),
      () => ({ fill }),
    );
  }

  /** A line's width: an ink pen's, or an outline's (keeping its style). */
  setWidth(width: number): void {
    this.#change(
      (a) => spec(borderPatch(a.subtype, { width, style: borderStyle(this.known(a.id).border ?? "solid") })),
      () => ({ width }),
    );
  }

  /** The opacity, 0 to 100. */
  setOpacity(percent: number): void {
    this.#change(() => ({ opacity: Math.max(0, Math.min(1, percent / 100)) }));
  }

  /** An outline's style: solid, dashed or cloudy (keeping its width). */
  setBorder(style: BorderStyleName): void {
    this.#change(
      (a) =>
        a.subtype === "Ink"
          ? undefined
          : spec(borderPatch(a.subtype, { width: this.known(a.id).width ?? 1, style: borderStyle(style) })),
      () => ({ border: style }),
    );
  }

  /** An open line's endings. */
  setEnds(ends: readonly [LineEndingName, LineEndingName]): void {
    this.#change(
      (a) => spec(endsPatch(a.subtype, ends)),
      () => ({ ends }),
    );
  }

  /** Whether ink is smoothed through its points. */
  setSmoothing(smoothing: boolean): void {
    this.#change(
      (a) => spec(smoothingPatch(a.subtype, smoothing)),
      () => ({ smoothing }),
    );
  }

  /** The note (`/Contents`); empty removes it. */
  setNote(text: string): void {
    this.#change(() => ({ contents: text === "" ? null : text }));
  }

  /** Deletes the selection, as one step. */
  delete(): void {
    const selection = this.#selection();
    const [first] = selection;
    if (first === undefined) return;
    const label = laterMessage("comment.step.delete", { kind: kindName(first.subtype).toLocaleLowerCase() });
    void this.#edits
      .apply(
        selection.map((a) => deleteAnnotOp(a.id)),
        label,
      )
      .catch(() => undefined);
  }

  /** Applies one `updateAnnot` per selected annotation that `change` gives one for, as one step; then `remember`s. */
  #change(
    change: (a: PageAnnotation) => Omit<AnnotChange, "id" | "modified"> | undefined,
    remember?: (a: PageAnnotation) => KnownStyle,
  ): void {
    const selection = this.#selection();
    const ops: Op[] = selection.flatMap((a) => {
      const made = change(a);
      return made === undefined ? [] : [updateAnnotOp(a.id, made)];
    });
    const [first] = selection;
    if (ops.length === 0 || first === undefined) return;
    const label = laterMessage("comment.step.change", { kind: kindName(first.subtype).toLocaleLowerCase() });
    void this.#edits.apply(ops, label).then(
      () => {
        if (remember === undefined) return;
        for (const a of selection) this.remember(a.id, remember(a));
        this.#changed();
      },
      // A failed edit is said by the session's failure toast.
      () => undefined,
    );
  }
}

/** A spec patch as `updateAnnot`'s change, or none. */
function spec(patch: AnnotChange["spec"] | undefined): Omit<AnnotChange, "id" | "modified"> | undefined {
  return patch === undefined ? undefined : { spec: patch };
}
