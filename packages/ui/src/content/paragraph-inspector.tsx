// The inspector for text the file already shows (Paper A04, "Paragraph"): what the text is set in,
// the notice when characters need another font or the text can't be edited in place, and the style
// fields (font, size, Bold, Italic, Underline, alignment, colour). Each field is one change the
// app makes. The core styles whole runs of text, a line or a segment of one, so no field here
// styles part of a run; the notes say so.

import type { ReactNode } from "react";
import { Button } from "../components/button.js";
import { Select } from "../components/field.js";
import { Notice, type NoticeTone } from "../components/notice.js";
import { chooseOnPress, PressedGroup } from "../components/pressed-group.js";
import { type Swatch, SwatchGroup } from "../components/swatches.js";
import { ToggleButton } from "../components/toggle.js";
import { laterMessage } from "../later-messages.js";
import { CONTENT_COLOURS, CONTENT_TEXT_SIZES, type ContentColour } from "./commands.js";
import type { ContentFontOption } from "./inspector.js";

/** What a notice at the top of the inspector says. */
export interface ParagraphNotice {
  readonly tone: NoticeTone;
  readonly title: string;
  readonly body: string;
}

/** Props of `ParagraphInspector`. */
export interface ParagraphInspectorProps {
  /** "Paragraph". */
  readonly title: string;
  /** "Page 3 · Part of the original page". */
  readonly context: string;
  /** The text it holds, so a person knows which one is picked: “Hello, world!”, cut short. */
  readonly excerpt: string;
  /** The font in the file and its size: "Helvetica · 12 pt". */
  readonly fileFont: string;
  /** A notice: characters in another font, or no font for them. */
  readonly notice?: ParagraphNotice;
  /** Set when the text can't be edited in place: the reason says why and the fields stay off. */
  readonly refusal?: {
    readonly title: string;
    readonly body: string;
    readonly detail: string;
    /** Switches to Select with the text's block picked, to move or delete it. */
    readonly onSelect?: () => void;
  };
  readonly font: {
    readonly options: readonly ContentFontOption[];
    readonly onChange: (fontId: string) => void;
    readonly onAddFont?: () => void;
  };
  readonly size: { readonly value: number; readonly onChange: (points: number) => void };
  readonly bold: { readonly value: boolean; readonly onChange: (on: boolean) => void };
  readonly italic: { readonly value: boolean; readonly onChange: (on: boolean) => void };
  readonly onUnderline: () => void;
  readonly onAlign: (align: "left" | "center" | "right") => void;
  readonly onColour: (colour: ContentColour) => void;
  /** Whether the text is being typed into. */
  readonly editing: boolean;
  /** Starts typing into the picked line (one line only). */
  readonly onEdit?: () => void;
  /** Ends typing: Done. */
  readonly onDone?: () => void;
  /** Puts the line's original text back into the box: Revert text. */
  readonly onRevert?: () => void;
  readonly deleteLabel: string;
  readonly onDelete: () => void;
  /** Why nothing can change (a file whose signature forbids it). */
  readonly locked?: string;
  /** Lines of help at the end. */
  readonly notes: readonly string[];
}

/** The content colours as swatches, none of them chosen: the file's colour isn't one the inspector reads. */
const SWATCHES: readonly Swatch<ContentColour | "file">[] = CONTENT_COLOURS.map((c) => ({
  id: c.id,
  label: laterMessage(`content.colour.${c.id}`),
  color: c.token,
}));

/** Font, size, Bold, Italic, Underline, alignment and colour. */
function StyleFields({ props, disabled }: { props: ParagraphInspectorProps; disabled: boolean }): ReactNode {
  const sizes = [...new Set<number>([...CONTENT_TEXT_SIZES, Math.round(props.size.value)])].sort((a, b) => a - b);
  return (
    <div className="paper-comment-text">
      <Select
        label={laterMessage("content.font")}
        options={props.font.options.map((f) => ({
          value: f.id,
          label: f.note === undefined ? f.label : `${f.label} — ${f.note}`,
          ...(f.disabled === true ? { isDisabled: true } : {}),
        }))}
        value="file"
        isDisabled={disabled}
        onChange={(id) => {
          if (id !== "file") props.font.onChange(id);
        }}
      />
      {props.font.onAddFont === undefined ? null : (
        <Button size={26} variant="ghost" isDisabled={disabled} onPress={props.font.onAddFont}>
          {laterMessage("content.addFont")}
        </Button>
      )}
      <Select
        label={laterMessage("content.size")}
        options={sizes.map((s) => ({ value: String(s), label: laterMessage("comment.width.value", { width: s }) }))}
        value={String(Math.round(props.size.value))}
        isDisabled={disabled}
        onChange={(value) => props.size.onChange(Number(value))}
      />
      <div className="paper-field">
        <span className="paper-field-label" aria-hidden="true">
          {laterMessage("content.style")}
        </span>
        <PressedGroup label={laterMessage("content.style")} className="paper-comment-text-toggles">
          <ToggleButton isSelected={props.bold.value} isDisabled={disabled} onChange={props.bold.onChange}>
            {laterMessage("content.bold")}
          </ToggleButton>
          <ToggleButton isSelected={props.italic.value} isDisabled={disabled} onChange={props.italic.onChange}>
            {laterMessage("content.italic")}
          </ToggleButton>
          <ToggleButton isSelected={false} isDisabled={disabled} icon="underline" onChange={props.onUnderline}>
            {laterMessage("content.text.underline")}
          </ToggleButton>
        </PressedGroup>
        <p className="paper-comment-text-note">{laterMessage("content.text.underline.help")}</p>
      </div>
      <div className="paper-field">
        <span className="paper-field-label" aria-hidden="true">
          {laterMessage("content.alignment")}
        </span>
        <PressedGroup label={laterMessage("content.alignment")} className="paper-comment-text-toggles">
          {(["left", "center", "right"] as const).map((a) => (
            <ToggleButton
              key={a}
              icon={a === "left" ? "align-left" : a === "center" ? "align-centre" : "align-right"}
              aria-label={laterMessage(`inspector.align.${a}`)}
              isSelected={false}
              isDisabled={disabled}
              onChange={chooseOnPress(a, props.onAlign)}
            />
          ))}
        </PressedGroup>
      </div>
      <SwatchGroup<ContentColour | "file">
        label={laterMessage("content.colour")}
        size={28}
        isDisabled={disabled}
        swatches={SWATCHES}
        value="file"
        onChange={(c) => {
          if (c !== "file") props.onColour(c);
        }}
      />
    </div>
  );
}

/** The inspector's content for text the file already shows. */
export function ParagraphInspector(props: ParagraphInspectorProps): ReactNode {
  const refused = props.refusal !== undefined;
  const disabled = props.locked !== undefined || refused;
  return (
    <div className="paper-comment-inspector paper-content-inspector" data-paragraph="">
      <header className="paper-comment-inspector-head">
        <h2 className="paper-comment-inspector-title">{props.title}</h2>
        <p className="paper-comment-inspector-context">{props.context}</p>
        {props.excerpt === "" ? null : <p className="paper-comment-text-note">{props.excerpt}</p>}
      </header>
      {props.locked === undefined ? null : <p className="paper-comment-inspector-locked">{props.locked}</p>}
      {props.refusal === undefined ? null : (
        <>
          <Notice tone="neutral" title={props.refusal.title}>
            {props.refusal.body}
          </Notice>
          <p className="paper-comment-text-note">{props.refusal.detail}</p>
        </>
      )}
      {props.notice === undefined ? null : (
        <Notice tone={props.notice.tone} title={props.notice.title}>
          {props.notice.body}
        </Notice>
      )}
      <p className="paper-comment-text-note" data-file-font="">
        <strong>{laterMessage("content.text.fileFont")}</strong> {props.fileFont}
      </p>
      {refused ? null : <StyleFields props={props} disabled={disabled} />}
      {props.notes.map((note) => (
        <p key={note} className="paper-comment-text-note">
          {note}
        </p>
      ))}
      <div className="paper-comment-inspector-actions">
        {props.refusal?.onSelect === undefined ? null : (
          <Button size={30} onPress={props.refusal.onSelect}>
            {laterMessage("content.text.selectBlock")}
          </Button>
        )}
        {props.editing && props.onDone !== undefined ? (
          <Button size={30} variant="primary" onPress={props.onDone}>
            {laterMessage("content.done")}
          </Button>
        ) : null}
        {props.editing && props.onRevert !== undefined ? (
          <Button size={30} icon="undo" onPress={props.onRevert}>
            {laterMessage("content.text.revert")}
          </Button>
        ) : null}
        {!props.editing && props.onEdit !== undefined && !refused ? (
          <Button size={30} icon="edit-text" isDisabled={disabled} onPress={props.onEdit}>
            {laterMessage("content.text.edit")}
          </Button>
        ) : null}
        {refused ? null : (
          <Button
            size={30}
            variant="danger"
            icon="delete"
            isDisabled={props.locked !== undefined}
            onPress={props.onDelete}
          >
            {props.deleteLabel}
          </Button>
        )}
      </div>
    </div>
  );
}
