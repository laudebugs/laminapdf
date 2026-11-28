// The inspector for an object Lamina PDF added to a page (Paper A01, A02): the kind and page,
// then the fields its kind has. Position and size, Rotate and the crop are also what the handles
// do, so every drag has a typed way (WCAG 2.5.7). Each change is one edit, made by the app.

import { type ReactNode, useState } from "react";
import { Button } from "../components/button.js";
import { Switch } from "../components/choice.js";
import { Select, TextArea, TextInput } from "../components/field.js";
import { Notice } from "../components/notice.js";
import { chooseOnPress, PressedGroup } from "../components/pressed-group.js";
import { type Swatch, SwatchGroup } from "../components/swatches.js";
import { ToggleButton } from "../components/toggle.js";
import { laterMessage } from "../later-messages.js";
import { CONTENT_COLOURS, CONTENT_DASHES, CONTENT_TEXT_SIZES, type ContentColour } from "./commands.js";

/** A value the inspector shows and the choice that changes it. */
export interface ContentChoice<Value> {
  readonly value: Value;
  /** Sets the value on the selected object, as one edit. */
  readonly onChange: (value: Value) => void;
}

/** A font the select offers. */
export interface ContentFontOption {
  readonly id: string;
  /** The name shown: "Newsreader". */
  readonly label: string;
  /** Why it can't be chosen, or what it costs ("Downloads 660 KB the first time"). */
  readonly note?: string;
  readonly disabled?: boolean;
}

/** A text box's own fields (Paper A01). */
export interface ContentTextFields {
  readonly font: ContentChoice<string> & { readonly options: readonly ContentFontOption[] };
  readonly size: ContentChoice<number>;
  readonly bold: ContentChoice<boolean>;
  readonly italic: ContentChoice<boolean>;
  readonly align: ContentChoice<"left" | "center" | "right">;
  readonly colour: ContentChoice<ContentColour | undefined>;
  /** Help under the font: where it is from, or what a character it lacks falls back to. */
  readonly fontNote: string | undefined;
  /** Opens the symbol picker. */
  readonly onInsertSymbol: () => void;
  /** Adds a font file the user chose. */
  readonly onAddFont?: () => void;
}

/** Position and size, in points from the page's top-left, and the turn in degrees. */
export interface ContentGeometry {
  readonly x: number;
  readonly y: number;
  readonly width: number;
  readonly height: number;
  readonly rotate: number;
  /** Whether Width and Height move together. */
  readonly keepRatio: boolean;
  /** A text box has no Height or Rotate of its own: its height follows the text. */
  readonly textBox: boolean;
  /** A skewed object: lamina moves it but doesn't resize or turn it, so Width, Height and Rotate are off. */
  readonly fixed: boolean;
  /** Sets the fields a person changed (the others keep their values), as one edit. */
  readonly onChange: (change: Partial<Record<"x" | "y" | "width" | "height" | "rotate", number>>) => void;
  /** Turns Keep proportions on or off. */
  readonly onKeepRatio: (keep: boolean) => void;
}

/** A shape's fill and outline. */
export interface ContentShapeFields {
  /** `none` for no fill. */
  readonly fill: ContentChoice<ContentColour | "none" | undefined>;
  /** `none` for no outline. */
  readonly stroke: ContentChoice<ContentColour | "none" | undefined>;
  readonly dash: ContentChoice<(typeof CONTENT_DASHES)[number]>;
  readonly width: ContentChoice<number>;
}

/** An image's file line and alternative text. */
export interface ContentImageFields {
  /** "northwind-logo.png · 640 × 196 px · PNG"; left out for a picture the file already had. */
  readonly file?: string;
  /** Asks for another picture to put in its place. */
  readonly onReplace: () => void;
  /** "Shown smaller than its full size; the full image is saved." */
  readonly note?: string;
  /** How far each edge is cut in, in points; the picture's own size is Position and size's. */
  readonly crop: {
    readonly left: number;
    readonly top: number;
    readonly right: number;
    readonly bottom: number;
    /** Sets one edge's inset, as one edit. */
    readonly onChange: (edge: "left" | "top" | "right" | "bottom", points: number) => void;
  };
}

/** Props of `ContentInspector`. */
export interface ContentInspectorProps {
  /** "Rectangle", "Image", "Text box". */
  readonly title: string;
  /** "Page 3 · Added by you". */
  readonly context: string;
  /** Left out for a text box still being typed, which isn't an object yet. */
  readonly geometry?: ContentGeometry;
  /** 0 to 100. */
  readonly opacity?: ContentChoice<number>;
  readonly text?: ContentTextFields;
  readonly shape?: ContentShapeFields;
  readonly image?: ContentImageFields;
  /** Alternative text, for a picture or a shape, and whether it is decoration. */
  readonly alt?: {
    readonly text: string;
    readonly decorative: boolean;
    /** Sets the alternative text. */
    readonly onText: (text: string) => void;
    /** Marks the object as decoration, or not; left out where lamina doesn't mark decoration. */
    readonly onDecorative?: (decorative: boolean) => void;
    /** What the field needs, shown as help under it. */
    readonly help?: string;
    /** Why the text is needed now (the file claims PDF/UA): the field shows it as an error. */
    readonly error?: string;
  };
  /** Lines of help under the fields, such as what a handle does or what stays in the file. */
  readonly notes?: readonly string[];
  /** Why nothing can change (a file whose signature forbids it). */
  readonly locked?: string;
  readonly arrange?: {
    /** Bring to front. */
    readonly onFront: () => void;
    /** Bring forward. */
    readonly onForward: () => void;
    /** Send backward. */
    readonly onBackward: () => void;
    /** Send to back. */
    readonly onBack: () => void;
  };
  /** Puts a copy beside the object. */
  readonly onDuplicate?: () => void;
  readonly deleteLabel?: string;
  /** Deletes the object. */
  readonly onDelete?: () => void;
  /** Ends typing into the box; present only while it is typed into. */
  readonly onDone?: () => void;
  /** Opens the text the file already shows, in this block, to type into (A04). */
  readonly onEditText?: () => void;
}

/** The content colours as swatches. */
const SWATCHES: readonly Swatch<ContentColour>[] = CONTENT_COLOURS.map((c) => ({
  id: c.id,
  label: laterMessage(`content.colour.${c.id}`),
  color: c.token,
}));

/** A number committed when the field loses focus or Enter is pressed; the shown value follows the selection. */
function NumberField({
  label,
  value,
  min,
  max,
  disabled,
  onCommit,
}: {
  label: string;
  value: number;
  min?: number;
  max?: number;
  disabled: boolean;
  onCommit: (value: number) => void;
}): ReactNode {
  const shown = String(Math.round(value * 100) / 100);
  // The field follows what the object reads (after a change, a new pick, Undo): the new reading
  // is in the same render, not an effect later, so it is on the page as soon as the reading is.
  const [field, setField] = useState({ shown, text: shown, handed: undefined as string | undefined });
  const current = field.shown === shown ? field : { shown, text: shown, handed: undefined };
  if (current !== field) setField(current);
  const { text, handed } = current;
  const setText = (next: string) => setField({ ...current, text: next });
  const commit = () => {
    if (text === handed) {
      // Already handed on, and the reading hasn't moved (a turn reads 0 again): show it.
      setText(shown);
      return;
    }
    const parsed = Number(text.replace(",", "."));
    if (!Number.isFinite(parsed) || text.trim() === "") return;
    const clamped = Math.min(max ?? Number.POSITIVE_INFINITY, Math.max(min ?? Number.NEGATIVE_INFINITY, parsed));
    if (clamped === Number(shown)) return;
    setField({ ...current, handed: text });
    onCommit(clamped);
  };
  return (
    <TextInput
      label={label}
      value={text}
      onChange={setText}
      inputMode="decimal"
      isDisabled={disabled}
      onBlur={commit}
      onKeyDown={(event) => {
        if (event.key === "Enter") commit();
      }}
    />
  );
}

/** X, Y, W, H, Rotate and Keep proportions. */
function GeometryFields({ geometry, disabled }: { geometry: ContentGeometry; disabled: boolean }): ReactNode {
  const g = geometry;
  const ratio = g.height === 0 ? 1 : g.width / g.height;
  return (
    <fieldset className="paper-content-geometry">
      <legend className="paper-field-label">{laterMessage("content.geometry")}</legend>
      <div className="paper-content-geometry-grid">
        <NumberField
          label={laterMessage("content.x")}
          value={g.x}
          disabled={disabled}
          onCommit={(x) => g.onChange({ x })}
        />
        <NumberField
          label={laterMessage("content.y")}
          value={g.y}
          disabled={disabled}
          onCommit={(y) => g.onChange({ y })}
        />
        <NumberField
          label={laterMessage("content.width")}
          value={g.width}
          min={4}
          disabled={disabled || g.fixed}
          onCommit={(width) => g.onChange(g.keepRatio ? { width, height: width / ratio } : { width })}
        />
        <NumberField
          label={laterMessage("content.height")}
          value={g.height}
          min={4}
          disabled={disabled || g.fixed || g.textBox}
          onCommit={(height) => g.onChange(g.keepRatio ? { height, width: height * ratio } : { height })}
        />
        {g.textBox ? null : (
          <NumberField
            label={laterMessage("content.rotate")}
            value={g.rotate}
            min={-180}
            max={180}
            disabled={disabled || g.fixed}
            onCommit={(rotate) => g.onChange({ rotate })}
          />
        )}
      </div>
      {g.textBox ? null : (
        <Switch isSelected={g.keepRatio} isDisabled={disabled || g.fixed} onChange={g.onKeepRatio}>
          {laterMessage("content.keepRatio")}
        </Switch>
      )}
      {g.fixed ? <p className="paper-comment-text-note">{laterMessage("content.fixed")}</p> : null}
    </fieldset>
  );
}

/** The crop's four insets: how far each edge is cut in. */
function CropFields({ crop, disabled }: { crop: ContentImageFields["crop"]; disabled: boolean }): ReactNode {
  return (
    <fieldset className="paper-content-geometry" data-content-crop="">
      <legend className="paper-field-label">{laterMessage("content.crop")}</legend>
      <div className="paper-content-geometry-grid">
        {(["left", "top", "right", "bottom"] as const).map((edge) => (
          <NumberField
            key={edge}
            label={laterMessage(`content.crop.${edge}`)}
            value={crop[edge]}
            min={0}
            disabled={disabled}
            onCommit={(points) => crop.onChange(edge, points)}
          />
        ))}
      </div>
    </fieldset>
  );
}

/** The opacity in percent. */
function OpacityField({ opacity, disabled }: { opacity: ContentChoice<number>; disabled: boolean }): ReactNode {
  const [live, setLive] = useState<number | undefined>(undefined);
  const shown = live ?? opacity.value;
  // Each keystroke or drag only moves the slider; the edit is made when it is let go of.
  const commit = () => {
    if (live !== undefined && live !== opacity.value) opacity.onChange(live);
    setLive(undefined);
  };
  return (
    <div className="paper-field paper-content-opacity">
      <label className="paper-field-label" htmlFor="content-opacity">
        {laterMessage("content.opacity")}
      </label>
      <div className="paper-content-opacity-row">
        <input
          id="content-opacity"
          type="range"
          min={0}
          max={100}
          step={1}
          value={shown}
          disabled={disabled}
          onChange={(event) => setLive(Number(event.target.value))}
          onPointerUp={commit}
          onKeyUp={commit}
          onBlur={commit}
        />
        <output className="paper-content-opacity-value" htmlFor="content-opacity">
          {shown}%
        </output>
      </div>
    </div>
  );
}

/** A colour row with a leading "none" swatch. */
function ColourRow({
  label,
  choice,
  none,
  disabled,
}: {
  label: string;
  choice: ContentChoice<ContentColour | "none" | undefined>;
  none: boolean;
  disabled: boolean;
}): ReactNode {
  const swatches: readonly Swatch<ContentColour | "none">[] = none
    ? [{ id: "none", label: laterMessage("content.colour.none"), color: "surface" }, ...SWATCHES]
    : SWATCHES;
  return (
    <SwatchGroup
      label={label}
      size={28}
      isDisabled={disabled}
      swatches={swatches}
      value={choice.value ?? (none ? "none" : "ink")}
      onChange={(c) => {
        if (!disabled) choice.onChange(c);
      }}
    />
  );
}

/** A text box's fields: font, size, style, alignment, colour and Insert symbol…. */
function TextFields({ text, disabled }: { text: ContentTextFields; disabled: boolean }): ReactNode {
  const sizes = [...new Set<number>([...CONTENT_TEXT_SIZES, text.size.value])].sort((a, b) => a - b);
  return (
    <div className="paper-comment-text">
      <Select
        label={laterMessage("content.font")}
        options={text.font.options.map((f) => ({
          value: f.id,
          label: f.note === undefined ? f.label : `${f.label} — ${f.note}`,
          ...(f.disabled === true ? { isDisabled: true } : {}),
        }))}
        value={text.font.value}
        isDisabled={disabled}
        {...(text.fontNote === undefined ? {} : { description: text.fontNote })}
        onChange={text.font.onChange}
      />
      {text.onAddFont === undefined ? null : (
        <Button size={26} variant="ghost" isDisabled={disabled} onPress={text.onAddFont}>
          {laterMessage("content.addFont")}
        </Button>
      )}
      <Select
        label={laterMessage("content.size")}
        options={sizes.map((s) => ({ value: String(s), label: laterMessage("comment.width.value", { width: s }) }))}
        value={String(text.size.value)}
        isDisabled={disabled}
        onChange={(value) => text.size.onChange(Number(value))}
      />
      <div className="paper-field">
        <span className="paper-field-label" aria-hidden="true">
          {laterMessage("content.style")}
        </span>
        <PressedGroup label={laterMessage("content.style")} className="paper-comment-text-toggles">
          <ToggleButton isSelected={text.bold.value} isDisabled={disabled} onChange={text.bold.onChange}>
            {laterMessage("content.bold")}
          </ToggleButton>
          <ToggleButton isSelected={text.italic.value} isDisabled={disabled} onChange={text.italic.onChange}>
            {laterMessage("content.italic")}
          </ToggleButton>
        </PressedGroup>
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
              isSelected={text.align.value === a}
              isDisabled={disabled}
              onChange={chooseOnPress(a, text.align.onChange)}
            />
          ))}
        </PressedGroup>
      </div>
      <SwatchGroup
        label={laterMessage("content.colour")}
        size={28}
        isDisabled={disabled}
        swatches={SWATCHES}
        value={text.colour.value ?? "ink"}
        onChange={(c) => {
          if (!disabled) text.colour.onChange(c);
        }}
      />
      <Button size={30} icon="add-text" isDisabled={disabled} onPress={text.onInsertSymbol}>
        {laterMessage("content.insertSymbol")}
      </Button>
    </div>
  );
}

/** A shape's fill and outline fields. */
function ShapeFields({ shape, disabled }: { shape: ContentShapeFields; disabled: boolean }): ReactNode {
  const widths = [0.5, 1, 1.5, 2, 3, 4, 6, 8, 12];
  const outlined = shape.stroke.value !== "none";
  return (
    <>
      <ColourRow label={laterMessage("content.fill")} choice={shape.fill} none disabled={disabled} />
      <ColourRow label={laterMessage("content.stroke")} choice={shape.stroke} none disabled={disabled} />
      <Select
        label={laterMessage("content.line")}
        options={CONTENT_DASHES.map((d) => ({ value: d, label: laterMessage(`content.dash.${d}`) }))}
        value={shape.dash.value}
        isDisabled={disabled || !outlined}
        onChange={shape.dash.onChange}
      />
      <Select
        label={laterMessage("content.strokeWidth")}
        options={[...new Set([...widths, shape.width.value])]
          .sort((a, b) => a - b)
          .map((w) => ({ value: String(w), label: laterMessage("comment.width.value", { width: w }) }))}
        value={String(shape.width.value)}
        isDisabled={disabled || !outlined}
        onChange={(value) => shape.width.onChange(Number(value))}
      />
    </>
  );
}

/** The Arrange group: four buttons in two rows. */
function ArrangeFields({
  arrange,
  disabled,
}: {
  arrange: NonNullable<ContentInspectorProps["arrange"]>;
  disabled: boolean;
}): ReactNode {
  return (
    <fieldset className="paper-content-arrange">
      <legend className="paper-field-label">{laterMessage("content.arrange")}</legend>
      <Button size={26} isDisabled={disabled} onPress={arrange.onFront}>
        {laterMessage("content.bringToFront")}
      </Button>
      <Button size={26} isDisabled={disabled} onPress={arrange.onForward}>
        {laterMessage("content.bringForward")}
      </Button>
      <Button size={26} isDisabled={disabled} onPress={arrange.onBackward}>
        {laterMessage("content.sendBackward")}
      </Button>
      <Button size={26} isDisabled={disabled} onPress={arrange.onBack}>
        {laterMessage("content.sendToBack")}
      </Button>
    </fieldset>
  );
}

/** A picture's file line, Replace… and note, then its crop. */
function ImageBlock({ image, disabled }: { image: ContentImageFields; disabled: boolean }): ReactNode {
  return (
    <>
      <div className="paper-content-file">
        {image.file === undefined ? null : <p className="paper-content-file-line">{image.file}</p>}
        <Button size={26} isDisabled={disabled} onPress={image.onReplace}>
          {laterMessage("content.replace")}
        </Button>
        {image.note === undefined ? null : <Notice tone="neutral" title={image.note} />}
      </div>
      <CropFields crop={image.crop} disabled={disabled} />
    </>
  );
}

/** Alternative text, or decoration. */
function AltBlock({ alt, disabled }: { alt: NonNullable<ContentInspectorProps["alt"]>; disabled: boolean }): ReactNode {
  return (
    <div className="paper-content-alt">
      <TextArea
        key={alt.text}
        label={laterMessage("content.alt")}
        defaultValue={alt.text}
        isDisabled={disabled || alt.decorative}
        {...(alt.error === undefined ? {} : { errorMessage: alt.error })}
        {...(alt.help === undefined ? {} : { description: alt.help })}
        onBlur={(event) => {
          const text = (event.target as unknown as HTMLTextAreaElement).value;
          if (text !== alt.text) alt.onText(text);
        }}
      />
      {alt.onDecorative === undefined ? null : (
        <Switch isSelected={alt.decorative} isDisabled={disabled} onChange={alt.onDecorative}>
          {laterMessage("content.decorative")}
        </Switch>
      )}
    </div>
  );
}

/** Done, Edit text, Duplicate and Delete, whichever the inspector has. */
function ActionsBlock({ props, disabled }: { props: ContentInspectorProps; disabled: boolean }): ReactNode {
  return (
    <div className="paper-comment-inspector-actions">
      {props.onDone === undefined ? null : (
        <Button size={30} variant="primary" onPress={props.onDone}>
          {laterMessage("content.done")}
        </Button>
      )}
      {props.onEditText === undefined ? null : (
        <Button size={30} icon="edit-text" isDisabled={disabled} onPress={props.onEditText}>
          {laterMessage("content.text.edit")}
        </Button>
      )}
      {props.onDuplicate === undefined ? null : (
        <Button size={30} icon="copy" isDisabled={disabled} onPress={props.onDuplicate}>
          {laterMessage("content.duplicate")}
        </Button>
      )}
      {props.onDelete === undefined || props.deleteLabel === undefined ? null : (
        <Button size={30} variant="danger" icon="delete" isDisabled={disabled} onPress={props.onDelete}>
          {props.deleteLabel}
        </Button>
      )}
    </div>
  );
}

/** The inspector's content for one added object. */
export function ContentInspector(props: ContentInspectorProps): ReactNode {
  const disabled = props.locked !== undefined;
  return (
    <div className="paper-comment-inspector paper-content-inspector">
      <header className="paper-comment-inspector-head">
        <h2 className="paper-comment-inspector-title">{props.title}</h2>
        <p className="paper-comment-inspector-context">{props.context}</p>
      </header>
      {props.locked === undefined ? null : <p className="paper-comment-inspector-locked">{props.locked}</p>}
      {props.image === undefined ? null : <ImageBlock image={props.image} disabled={disabled} />}
      {props.text === undefined ? null : <TextFields text={props.text} disabled={disabled} />}
      {props.geometry === undefined ? null : <GeometryFields geometry={props.geometry} disabled={disabled} />}
      {props.shape === undefined ? null : <ShapeFields shape={props.shape} disabled={disabled} />}
      {props.opacity === undefined ? null : <OpacityField opacity={props.opacity} disabled={disabled} />}
      {props.arrange === undefined ? null : <ArrangeFields arrange={props.arrange} disabled={disabled} />}
      {props.alt === undefined ? null : <AltBlock alt={props.alt} disabled={disabled} />}
      {(props.notes ?? []).map((note) => (
        <p key={note} className="paper-comment-text-note">
          {note}
        </p>
      ))}
      <ActionsBlock props={props} disabled={disabled} />
    </div>
  );
}
