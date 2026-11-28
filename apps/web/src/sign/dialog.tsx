// The "Add a signature image" dialog (Paper G01): draw, type or upload a signature, or pick one
// kept on this device; how it goes on the page; whether to keep it. It only collects the choice;
// the controller makes the picture and writes it. Everything it says calls the result a picture
// of a signature, never a digital signature.

import type { SavedSignature } from "@laminapdf/ui";
import {
  Button,
  Checkbox,
  Dialog,
  EmptyState,
  Radio,
  RadioGroup,
  SegmentedTabs,
  TabPanel,
  TextInput,
} from "@laminapdf/ui";
import { laterMessage } from "@laminapdf/ui/editing";
import { type PointerEvent, type ReactNode, useEffect, useRef, useState } from "react";
import {
  cropDrawing,
  INKS,
  type InkName,
  MAX_STROKES,
  type Stroke,
  type TYPED_STYLES,
  uploadProblem,
} from "./capture.ts";
import { placementChoices, type SignaturePlacement } from "./placement.ts";
import { drawStrokes, PEN, typedPreviews } from "./render.ts";

/** Where the signature in the dialog comes from. */
export type SignatureSource =
  | { readonly kind: "draw"; readonly strokes: readonly Stroke[]; readonly ink: InkName }
  | { readonly kind: "type"; readonly name: string; readonly style: (typeof TYPED_STYLES)[number]["id"] }
  | { readonly kind: "upload"; readonly bytes: Uint8Array }
  | { readonly kind: "saved"; readonly id: string };

/** What Place signature hands back. */
export interface SignatureRequest {
  readonly source: SignatureSource;
  readonly placement: SignaturePlacement;
  /** Keep it in the library on this device. */
  readonly keep: boolean;
}

type Tab = "draw" | "type" | "upload" | "saved";

/** Props of `SignatureDialog`. */
export interface SignatureDialogProps {
  readonly open: boolean;
  /** The signatures kept on this device. */
  readonly saved: readonly (SavedSignature & { readonly url: string })[];
  /** The file has a certification signature: placing a picture as page content would break it. */
  readonly certified: boolean;
  /** Where "stored on this device" says it is: the browser's storage or the app's data folder. */
  readonly storedIn: "browser" | "app";
  readonly onCancel: () => void;
  readonly onPlace: (request: SignatureRequest) => void;
  readonly onRemoveSaved: (id: string) => void;
}

const TABS = ["draw", "type", "upload", "saved"] as const;

/** The pad: pointer strokes drawn on a canvas, with the strokes kept in the caller's state. */
function Pad({
  strokes,
  ink,
  onChange,
}: {
  strokes: readonly Stroke[];
  ink: InkName;
  onChange: (strokes: readonly Stroke[]) => void;
}): ReactNode {
  const canvas = useRef<HTMLCanvasElement>(null);
  const drawing = useRef<[number, number][] | undefined>(undefined);
  const draw = (live: readonly Stroke[]) => {
    const element = canvas.current;
    const context = element?.getContext("2d");
    if (element === null || context === null || context === undefined) return;
    const scale = window.devicePixelRatio || 1;
    const { width, height } = element.getBoundingClientRect();
    element.width = Math.round(width * scale);
    element.height = Math.round(height * scale);
    context.scale(scale, scale);
    context.clearRect(0, 0, width, height);
    // The baseline the signature sits above.
    context.strokeStyle = "rgba(0, 0, 0, 0.18)";
    context.lineWidth = 1;
    context.beginPath();
    context.moveTo(16, height * 0.72);
    context.lineTo(width - 16, height * 0.72);
    context.stroke();
    drawStrokes(context, live, INKS[ink]);
  };
  useEffect(() => draw(strokes), [strokes, ink]);
  const at = (event: PointerEvent<HTMLCanvasElement>): [number, number] => {
    const box = event.currentTarget.getBoundingClientRect();
    return [event.clientX - box.left, event.clientY - box.top];
  };
  const label =
    strokes.length === 0
      ? laterMessage("sign.draw.pad.none")
      : strokes.length === 1
        ? laterMessage("sign.draw.pad.one")
        : laterMessage("sign.draw.pad", { count: strokes.length });
  return (
    <canvas
      ref={canvas}
      className="paper-signature-pad"
      role="img"
      aria-label={label}
      onPointerDown={(event) => {
        if (strokes.length >= MAX_STROKES) return;
        event.currentTarget.setPointerCapture(event.pointerId);
        drawing.current = [at(event)];
        draw([...strokes, drawing.current]);
      }}
      onPointerMove={(event) => {
        const stroke = drawing.current;
        if (stroke === undefined) return;
        stroke.push(at(event));
        draw([...strokes, stroke]);
      }}
      onPointerUp={() => {
        const stroke = drawing.current;
        drawing.current = undefined;
        if (stroke !== undefined) onChange([...strokes, stroke]);
      }}
      onPointerCancel={() => {
        drawing.current = undefined;
        draw(strokes);
      }}
    />
  );
}

type StyleId = (typeof TYPED_STYLES)[number]["id"];

/** The Draw tab: the pad, the ink and the two ways back. */
function DrawTab(props: {
  strokes: readonly Stroke[];
  ink: InkName;
  setStrokes: (strokes: readonly Stroke[]) => void;
  setInk: (ink: InkName) => void;
}): ReactNode {
  const { strokes, ink, setStrokes, setInk } = props;
  return (
    <>
      <Pad strokes={strokes} ink={ink} onChange={setStrokes} />
      <p className="paper-signature-hint">{laterMessage("sign.draw.hint")}</p>
      <div className="paper-signature-row">
        <RadioGroup
          label={laterMessage("sign.draw.ink")}
          orientation="horizontal"
          value={ink}
          onChange={(value) => setInk(value as InkName)}
        >
          <Radio value="black">{laterMessage("sign.draw.black")}</Radio>
          <Radio value="blue">{laterMessage("sign.draw.blue")}</Radio>
        </RadioGroup>
        <Button isDisabled={strokes.length === 0} onPress={() => setStrokes(strokes.slice(0, -1))}>
          {laterMessage("sign.draw.undo")}
        </Button>
        <Button isDisabled={strokes.length === 0} onPress={() => setStrokes([])}>
          {laterMessage("sign.draw.clear")}
        </Button>
      </div>
    </>
  );
}

/** The Type tab: the name and three styles, each previewed in its own face. */
function TypeTab(props: {
  name: string;
  style: StyleId;
  setName: (name: string) => void;
  setStyle: (style: StyleId) => void;
}): ReactNode {
  const { name, style, setName, setStyle } = props;
  return (
    <>
      <TextInput label={laterMessage("sign.type.name")} value={name} onChange={setName} />
      <RadioGroup
        label={laterMessage("sign.type.style")}
        value={style}
        onChange={(value) => setStyle(value as StyleId)}
      >
        {typedPreviews(name).map(({ style: s, text }) => (
          <Radio key={s.id} value={s.id}>
            <span
              className="paper-signature-preview"
              style={{
                fontFamily: s.family,
                fontStyle: s.italic ? "italic" : "normal",
                letterSpacing: `${s.spacing / 2}px`,
              }}
            >
              {text === "" ? laterMessage(`sign.type.${s.id}`) : text}
            </span>
          </Radio>
        ))}
      </RadioGroup>
    </>
  );
}

/** The Upload tab: a PNG or JPEG chosen from a file, with the reason when it can't be used. */
function UploadTab(props: {
  upload: { bytes: Uint8Array; url: string } | undefined;
  failed: boolean;
  onChoose: (file: File | undefined) => void;
}): ReactNode {
  const { upload, failed, onChoose } = props;
  const picker = useRef<HTMLInputElement>(null);
  return (
    <>
      <div className="paper-signature-drop">
        <span>{laterMessage("sign.upload.drop")}</span>
        <input
          ref={picker}
          type="file"
          accept="image/png,image/jpeg"
          hidden
          onChange={(event) => onChoose(event.currentTarget.files?.[0])}
        />
        <Button onPress={() => picker.current?.click()}>{laterMessage("sign.upload.choose")}</Button>
      </div>
      {failed ? (
        <p className="paper-signature-error" role="alert">
          {laterMessage("sign.upload.error")}
        </p>
      ) : null}
      {upload === undefined ? null : (
        <img className="paper-signature-image" src={upload.url} alt={laterMessage("sign.upload.preview")} />
      )}
    </>
  );
}

/** The Saved tab: the library as tiles, each to use or remove. */
function SavedTab(props: {
  saved: SignatureDialogProps["saved"];
  chosen: string | undefined;
  onChoose: (id: string) => void;
  onRemove: (id: string) => void;
}): ReactNode {
  const { saved, chosen, onChoose, onRemove } = props;
  if (saved.length === 0) return <EmptyState title={laterMessage("sign.saved.empty")} headingLevel={3} />;
  return (
    <ul className="paper-signature-tiles">
      {saved.map((item) => (
        <li key={item.id} className="paper-signature-tile" data-selected={item.id === chosen}>
          <img src={item.url} alt="" />
          <Button onPress={() => onChoose(item.id)}>{laterMessage("sign.saved.use", { name: item.name })}</Button>
          <Button onPress={() => onRemove(item.id)}>{laterMessage("sign.saved.remove", { name: item.name })}</Button>
        </li>
      ))}
    </ul>
  );
}

/** How it goes on the page, whether to keep it, and where "kept" means. */
function PlacementSection(props: {
  placement: SignaturePlacement;
  drawn: boolean;
  certified: boolean;
  keepable: boolean;
  keep: boolean;
  storedIn: "browser" | "app";
  onPlacement: (placement: SignaturePlacement) => void;
  onKeep: (keep: boolean) => void;
}): ReactNode {
  const { placement, drawn, certified, keepable, keep, storedIn, onPlacement, onKeep } = props;
  return (
    <div className="paper-signature-section">
      <RadioGroup
        label={laterMessage("sign.placeAs")}
        value={placement}
        onChange={(value) => onPlacement(value as SignaturePlacement)}
        {...(certified ? { description: laterMessage("sign.placeAs.contentOff") } : {})}
      >
        <Radio value="stamp">{laterMessage("sign.placeAs.movable")}</Radio>
        {drawn ? <Radio value="ink">{laterMessage("sign.placeAs.ink")}</Radio> : null}
        <Radio value="content" isDisabled={certified}>
          {laterMessage("sign.placeAs.content")}
        </Radio>
      </RadioGroup>
      {placement === "content" ? (
        <p className="paper-signature-hint">{laterMessage("sign.placeAs.contentHelp")}</p>
      ) : null}
      {keepable ? (
        <>
          <Checkbox isSelected={keep} onChange={onKeep}>
            {laterMessage("sign.keep")}
          </Checkbox>
          <p className="paper-signature-hint">
            {laterMessage(storedIn === "app" ? "sign.keep.desktop" : "sign.keep.web")}
          </p>
        </>
      ) : null}
    </div>
  );
}

/** What the dialog has collected so far. */
interface Collected {
  tab: Tab;
  strokes: readonly Stroke[];
  ink: InkName;
  name: string;
  style: StyleId;
  upload: { bytes: Uint8Array; url: string } | undefined;
  failed: boolean;
  chosen: string | undefined;
  placement: SignaturePlacement;
  keep: boolean;
}

const FRESH: Collected = {
  tab: "draw",
  strokes: [],
  ink: "black",
  name: "",
  style: "script",
  upload: undefined,
  failed: false,
  chosen: undefined,
  placement: "stamp",
  keep: true,
};

/** The collected choices, reset at each opening, and the setter of one field of them. */
function useCollected(open: boolean): [Collected, <K extends keyof Collected>(key: K, value: Collected[K]) => void] {
  const [state, setState] = useState<Collected>(FRESH);
  useEffect(() => {
    if (open) setState(FRESH);
  }, [open]);
  // A preview URL made for an upload is let go when it is replaced or the dialog goes.
  useEffect(() => {
    const url = state.upload?.url;
    return url === undefined ? undefined : () => URL.revokeObjectURL(url);
  }, [state.upload]);
  return [state, (key, value) => setState((was) => ({ ...was, [key]: value }))];
}

/** The dialog. Each opening starts from the Draw tab with nothing drawn. */
export function SignatureDialog(props: SignatureDialogProps): ReactNode {
  const { open, saved, certified, storedIn, onCancel, onPlace, onRemoveSaved } = props;
  const [c, set] = useCollected(open);
  const source = sourceOf(c.tab, { ...c, upload: c.upload?.bytes });
  const need = needOf(c.tab, c);
  // Ink needs a drawn signature, drawn now or saved with its strokes; the choice falls back to a stamp without one.
  const drawn = c.tab === "draw" || (c.tab === "saved" && saved.find((s) => s.id === c.chosen)?.strokes !== undefined);
  const placement = placementChoices(drawn, certified).includes(c.placement) ? c.placement : "stamp";
  const choose = async (file: File | undefined) => {
    if (file === undefined) return;
    const bytes = new Uint8Array(await file.arrayBuffer());
    const bad = uploadProblem(bytes) !== undefined;
    set("failed", bad);
    set("upload", bad ? undefined : { bytes, url: URL.createObjectURL(file) });
  };
  return (
    <Dialog
      isOpen={open}
      onOpenChange={(isOpen) => {
        if (!isOpen) onCancel();
      }}
      title={laterMessage("sign.title")}
      description={laterMessage("sign.subtitle")}
      closeLabel={laterMessage("sign.close")}
      size="form"
      actions={
        <>
          <Button onPress={onCancel}>{laterMessage("sign.cancel")}</Button>
          <Button
            variant="primary"
            isDisabled={source === undefined}
            onPress={() => source !== undefined && onPlace({ source, placement, keep: c.keep && c.tab !== "saved" })}
          >
            {laterMessage("sign.place")}
          </Button>
        </>
      }
    >
      <SegmentedTabs
        aria-label={laterMessage("sign.tabs")}
        selectedKey={c.tab}
        onSelectionChange={(tab) => set("tab", tab)}
        tabs={TABS.map((id) => ({
          id,
          label: laterMessage(id === "saved" ? "sign.tab.saved" : `sign.tab.${id}`, { count: saved.length }),
        }))}
      >
        <TabPanel id="draw">
          <DrawTab
            strokes={c.strokes}
            ink={c.ink}
            setStrokes={(v) => set("strokes", v)}
            setInk={(v) => set("ink", v)}
          />
        </TabPanel>
        <TabPanel id="type">
          <TypeTab name={c.name} style={c.style} setName={(v) => set("name", v)} setStyle={(v) => set("style", v)} />
        </TabPanel>
        <TabPanel id="upload">
          <UploadTab upload={c.upload} failed={c.failed} onChoose={(file) => void choose(file)} />
        </TabPanel>
        <TabPanel id="saved">
          <SavedTab saved={saved} chosen={c.chosen} onChoose={(id) => set("chosen", id)} onRemove={onRemoveSaved} />
        </TabPanel>
      </SegmentedTabs>
      {need === undefined ? null : <p className="paper-signature-hint">{laterMessage(need)}</p>}
      <PlacementSection
        placement={placement}
        drawn={drawn}
        certified={certified}
        keepable={c.tab !== "saved"}
        keep={c.keep}
        storedIn={storedIn}
        onPlacement={(v) => set("placement", v)}
        onKeep={(v) => set("keep", v)}
      />
      <p className="paper-signature-note">{laterMessage("sign.note")}</p>
    </Dialog>
  );
}

/** The source the tab's state makes, or undefined while it isn't ready. */
function sourceOf(
  tab: Tab,
  state: {
    strokes: readonly Stroke[];
    ink: InkName;
    name: string;
    style: StyleId;
    upload: Uint8Array | undefined;
    chosen: string | undefined;
  },
): SignatureSource | undefined {
  switch (tab) {
    case "draw":
      return cropDrawing(state.strokes, PEN) === undefined
        ? undefined
        : { kind: "draw", strokes: state.strokes, ink: state.ink };
    case "type":
      return state.name.trim() === "" ? undefined : { kind: "type", name: state.name, style: state.style };
    case "upload":
      return state.upload === undefined ? undefined : { kind: "upload", bytes: state.upload };
    case "saved":
      return state.chosen === undefined ? undefined : { kind: "saved", id: state.chosen };
  }
}

/** What the person still has to do before Place signature works: the help text beside it. */
function needOf(
  tab: Tab,
  state: { strokes: readonly Stroke[]; name: string; upload: unknown; chosen: string | undefined },
): "sign.place.needDrawing" | "sign.place.needName" | "sign.place.needImage" | undefined {
  if (tab === "draw") return state.strokes.length === 0 ? "sign.place.needDrawing" : undefined;
  if (tab === "type") return state.name.trim() === "" ? "sign.place.needName" : undefined;
  if (tab === "upload") return state.upload === undefined ? "sign.place.needImage" : undefined;
  return state.chosen === undefined ? "sign.place.needImage" : undefined;
}
