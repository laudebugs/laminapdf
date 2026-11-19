// The annotation inspector (Paper C02), docked while Comment mode has a selection: the kind and
// page, then the properties the selection's kind has (line colour or colour, fill, width,
// opacity, border, line endings, smoothing; a text box's own fields first, text-inspector.tsx),
// the note, the author and time, and Delete. Each
// change is one edit, made by the app; a value the file holds but the reader can't read (an
// outline's width in a file from elsewhere) shows as "As in the file" until it is set. Several
// selected annotations show what they share: colour, opacity and Delete.

import { type ReactNode, type Ref, useRef } from "react";
import { Button } from "../components/button.js";
import { Switch } from "../components/choice.js";
import { Select, TextArea, TextInput } from "../components/field.js";
import { chooseOnPress, PressedGroup } from "../components/pressed-group.js";
import { type Swatch, SwatchGroup } from "../components/swatches.js";
import { ToggleButton } from "../components/toggle.js";
import { type LaterMessageId, laterMessage } from "../later-messages.js";
import {
  AttachmentFields,
  type AttachmentFieldsProps,
  SoundFields,
  type SoundFieldsProps,
} from "./attachment-inspector.js";
import {
  BORDER_STYLES,
  type BorderStyleName,
  type CommentColour,
  LINE_ENDINGS,
  LINE_WIDTHS,
  type LineEndingName,
} from "./commands.js";
import { TextFields, type TextFieldsProps } from "./text-inspector.js";

/** A choice the inspector offers: its value now (undefined: as in the file) and what sets it. */
export interface InspectorChoice<Value> {
  readonly value: Value | undefined;
  /** Sets the value on the selected annotations. */
  readonly onChange: (value: Value) => void;
}

/** Props of `AnnotationInspector`. */
export interface AnnotationInspectorProps {
  /** "Rectangle", or "3 items". */
  readonly title: string;
  /** Whether the title and context head the fields; false where the container shows them, as the phone's sheet does (C08). */
  readonly headed?: boolean;
  /** A text box's own fields (C03), first. */
  readonly text?: Omit<TextFieldsProps, "disabled">;
  /** An attached file's own fields (C05), first. */
  readonly attachment?: Omit<AttachmentFieldsProps, "disabled">;
  /** A sound comment's Play (C05). */
  readonly sound?: SoundFieldsProps;
  /** Under the title: "Page 3". */
  readonly context?: string;
  /** The colour's label: "Line colour" for a shape or a line, "Colour" for the rest. */
  readonly colourLabel: string;
  readonly colours: readonly Swatch<CommentColour>[];
  readonly colour: InspectorChoice<CommentColour>;
  /** A shape's fill: none, or one of the fill colours. */
  readonly fill?: InspectorChoice<CommentColour | "none"> & { readonly colours: readonly Swatch<CommentColour>[] };
  /** A line tool's width, in points. */
  readonly width?: InspectorChoice<number>;
  /** 0 to 100; absent for a box not yet made, which has no opacity to change. */
  readonly opacity?: InspectorChoice<number>;
  readonly border?: InspectorChoice<BorderStyleName>;
  /** Cloudy is only for closed shapes. */
  readonly cloudy?: boolean;
  /** A line's or polyline's start and end. */
  readonly ends?: InspectorChoice<readonly [LineEndingName, LineEndingName]>;
  readonly smoothing?: InspectorChoice<boolean>;
  /** The comment (`/Contents`); absent for several. Committed when the field loses focus. */
  readonly note?: {
    /** The comment as it is now. */
    readonly value: string;
    /** Writes the edited comment to the annotation. */
    readonly onCommit: (text: string) => void;
  };
  /** The note field, for focusing it when a note is placed. */
  readonly noteRef?: Ref<HTMLTextAreaElement>;
  /** "R. Osei · 29 Oct 2026, 09:12". */
  readonly author?: string;
  /** Asks for the name the reader's comments carry (`comment.set-author`). */
  readonly onChangeAuthor?: () => void;
  /** Why nothing can change (a locked annotation). */
  readonly locked?: string;
  readonly deleteLabel: string;
  /** Deletes the selected annotations. */
  readonly onDelete: () => void;
}

/** A width select; "As in the file" while the file's width is unknown. */
function WidthField({ width, disabled }: { width: InspectorChoice<number>; disabled: boolean }): ReactNode {
  const known = width.value !== undefined;
  const options = [
    ...(known ? [] : [{ value: "", label: laterMessage("inspector.width.unknown") }]),
    ...LINE_WIDTHS.map((w) => ({ value: String(w), label: laterMessage("comment.width.value", { width: w }) })),
  ];
  return (
    <Select
      label={laterMessage("inspector.width")}
      options={options}
      value={known ? String(width.value) : ""}
      isDisabled={disabled}
      onChange={(value) => {
        if (value !== "") width.onChange(Number(value));
      }}
    />
  );
}

/** The Line start and Line end selects. */
function EndsFields({
  ends,
  disabled,
}: {
  ends: InspectorChoice<readonly [LineEndingName, LineEndingName]>;
  disabled: boolean;
}): ReactNode {
  const known = ends.value !== undefined;
  const [start, end] = ends.value ?? ["none", "none"];
  const options = [
    ...(known ? [] : [{ value: "unknown", label: laterMessage("inspector.ending.unknown") }]),
    ...LINE_ENDINGS.map((e) => ({ value: e, label: laterMessage(`inspector.ending.${e}` as LaterMessageId) })),
  ];
  const set = (at: 0 | 1) => (value: string) => {
    if (value === "unknown") return;
    const next: [LineEndingName, LineEndingName] = [start, end];
    next[at] = value as LineEndingName;
    ends.onChange(next);
  };
  return (
    <>
      <Select
        label={laterMessage("inspector.lineStart")}
        options={options}
        value={known ? start : "unknown"}
        isDisabled={disabled}
        onChange={set(0)}
      />
      <Select
        label={laterMessage("inspector.lineEnd")}
        options={options}
        value={known ? end : "unknown"}
        isDisabled={disabled}
        onChange={set(1)}
      />
    </>
  );
}

/** The Border toggle group: real toggle buttons, one pressed. */
function BorderField({
  border,
  cloudy,
  disabled,
}: {
  border: InspectorChoice<BorderStyleName>;
  cloudy: boolean;
  disabled: boolean;
}): ReactNode {
  const styles = BORDER_STYLES.filter((s) => cloudy || s !== "cloudy");
  const label = laterMessage("inspector.border");
  return (
    <div className="paper-field">
      <span className="paper-field-label" aria-hidden="true">
        {label}
      </span>
      <PressedGroup label={label} className="paper-comment-border">
        {styles.map((style) => (
          <ToggleButton
            key={style}
            isSelected={border.value === style}
            isDisabled={disabled}
            onChange={chooseOnPress(style, border.onChange)}
          >
            {laterMessage(`inspector.border.${style}`)}
          </ToggleButton>
        ))}
      </PressedGroup>
    </div>
  );
}

/** The opacity, as a percentage typed in and set when the field loses focus or on Enter. */
function OpacityField({ opacity, disabled }: { opacity: InspectorChoice<number>; disabled: boolean }): ReactNode {
  const shown = opacity.value === undefined ? "" : String(Math.round(opacity.value));
  // Enter and then leaving the field would set the same value twice before the record shows it.
  const sent = useRef<number | undefined>(undefined);
  const commit = (text: string) => {
    const value = Math.round(Number.parseFloat(text));
    if (!Number.isFinite(value) || value < 0 || value > 100 || value === opacity.value || value === sent.current)
      return;
    sent.current = value;
    opacity.onChange(value);
  };
  return (
    <div className="paper-comment-opacity">
      <TextInput
        key={shown}
        label={laterMessage("inspector.opacity")}
        defaultValue={shown}
        inputMode="numeric"
        isDisabled={disabled}
        onBlur={(event) => commit((event.target as HTMLInputElement).value)}
        onKeyDown={(event) => {
          if (event.key === "Enter") commit((event.target as HTMLInputElement).value);
        }}
      />
      <span className="paper-comment-unit" aria-hidden="true">
        %
      </span>
    </div>
  );
}

/** The inspector's content: its title, the properties, the note, the author and Delete. */
export function AnnotationInspector(props: AnnotationInspectorProps): ReactNode {
  const disabled = props.locked !== undefined;
  const { fill, width, border, ends, smoothing, note } = props;
  return (
    <div className="paper-comment-inspector">
      {props.headed === false ? null : (
        <header className="paper-comment-inspector-head">
          <h2 className="paper-comment-inspector-title">{props.title}</h2>
          {props.context === undefined ? null : <p className="paper-comment-inspector-context">{props.context}</p>}
        </header>
      )}
      {props.locked === undefined ? null : <p className="paper-comment-inspector-locked">{props.locked}</p>}
      <KindFields {...props} disabled={disabled} />
      {props.colours.length === 0 ? null : (
        <SwatchGroup
          label={props.colourLabel}
          size={28}
          swatches={props.colours}
          value={props.colour.value as CommentColour}
          onChange={(c) => {
            if (!disabled) props.colour.onChange(c);
          }}
        />
      )}
      {fill === undefined ? null : (
        <SwatchGroup
          label={laterMessage("inspector.fill")}
          size={28}
          swatches={[{ id: "none", label: laterMessage("inspector.fill.none"), color: "surface" }, ...fill.colours]}
          value={fill.value as CommentColour | "none"}
          onChange={(c) => {
            if (!disabled) fill.onChange(c);
          }}
        />
      )}
      {width === undefined ? null : <WidthField width={width} disabled={disabled} />}
      {props.opacity === undefined ? null : <OpacityField opacity={props.opacity} disabled={disabled} />}
      {border === undefined ? null : <BorderField border={border} cloudy={props.cloudy === true} disabled={disabled} />}
      {ends === undefined ? null : <EndsFields ends={ends} disabled={disabled} />}
      {smoothing === undefined ? null : (
        <Switch isSelected={smoothing.value ?? true} isDisabled={disabled} onChange={smoothing.onChange}>
          {laterMessage("inspector.smoothing")}
        </Switch>
      )}
      {note === undefined ? null : (
        <NoteField
          note={note}
          disabled={disabled}
          {...(props.noteRef === undefined ? {} : { noteRef: props.noteRef })}
        />
      )}
      <AuthorFields {...props} />
      <div className="paper-comment-inspector-actions">
        <Button size={30} variant="danger" icon="delete" isDisabled={disabled} onPress={props.onDelete}>
          {props.deleteLabel}
        </Button>
      </div>
    </div>
  );
}

/** A kind's own fields, first: a text box's, an attached file's or a sound comment's Play. */
function KindFields({
  text,
  attachment,
  sound,
  disabled,
}: Pick<AnnotationInspectorProps, "text" | "attachment" | "sound"> & { disabled: boolean }): ReactNode {
  return (
    <>
      {text === undefined ? null : <TextFields {...text} disabled={disabled} />}
      {attachment === undefined ? null : <AttachmentFields {...attachment} disabled={disabled} />}
      {sound === undefined ? null : <SoundFields {...sound} />}
    </>
  );
}

/** The author and time, and Change your name…. */
function AuthorFields({
  author,
  onChangeAuthor,
}: Pick<AnnotationInspectorProps, "author" | "onChangeAuthor">): ReactNode {
  return (
    <>
      {author === undefined ? null : (
        <dl className="paper-comment-inspector-author">
          <dt>{laterMessage("inspector.author")}</dt>
          <dd>{author}</dd>
        </dl>
      )}
      {onChangeAuthor === undefined ? null : (
        <Button size={26} variant="ghost" onPress={onChangeAuthor}>
          {laterMessage("inspector.author.change")}
        </Button>
      )}
    </>
  );
}

/** The note, committed when the field loses focus; a new annotation's value starts it afresh. */
function NoteField({
  note,
  disabled,
  noteRef,
}: {
  note: NonNullable<AnnotationInspectorProps["note"]>;
  disabled: boolean;
  noteRef?: Ref<HTMLTextAreaElement>;
}): ReactNode {
  return (
    <div className="paper-comment-note" ref={(div) => attachNoteRef(div, noteRef)}>
      <TextArea
        key={note.value}
        label={laterMessage("inspector.note")}
        defaultValue={note.value}
        isDisabled={disabled}
        onBlur={(event) => {
          const text = (event.target as unknown as HTMLTextAreaElement).value;
          if (text !== note.value) note.onCommit(text);
        }}
      />
    </div>
  );
}

/** Points `ref` at the note's textarea inside `div` (TextArea takes no ref of its own). */
function attachNoteRef(div: HTMLDivElement | null, ref: Ref<HTMLTextAreaElement> | undefined): void {
  if (ref === undefined || ref === null) return;
  const area = div?.querySelector("textarea") ?? null;
  if (typeof ref === "function") ref(area);
  else (ref as { current: HTMLTextAreaElement | null }).current = area;
}
