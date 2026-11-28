// The signature image's controller (Paper G01): opens the dialog with the signatures kept on
// this device, makes the picture of what was chosen, adds it to the document and places it on the
// page in view as one undo step. The form controller applies the op, so a file whose signatures
// would break asks first, as every placement in Fill & Sign does.

import type { Platform, SavedSignature, ToastQueue } from "@laminapdf/ui";
import { laterMessage } from "@laminapdf/ui/editing";
import type { Viewer } from "@laminapdf/viewer";
import { storedAuthor } from "../author.ts";
import type { FormController } from "../forms/controller.ts";
import { pointOn } from "../page-points.ts";
import type { Engine } from "../session.ts";
import { cropDrawing, INKS, type InkName, nameFor, type Stroke, TYPED_STYLES } from "./capture.ts";
import type { SignatureRequest } from "./dialog.tsx";
import {
  placeSignatureOp,
  type ReadySignature,
  SIGNATURE_SIDE,
  type SignatureContext,
  signatureBox,
} from "./placement.ts";
import { drawingToPng, imageToPng, PEN, type Picture, typedToPng } from "./render.ts";

/** A saved signature with a URL its picture shows from. */
export type ShownSignature = SavedSignature & { readonly url: string };

/** What the dialog shows. */
export interface SignatureState {
  readonly open: boolean;
  readonly saved: readonly ShownSignature[];
  /** A certification signature forbids a picture as part of the page. */
  readonly certified: boolean;
}

const BLACK = INKS.black;

/** A one-pixel PNG: the picture a probe of "would this break a signature" places, so the op is valid. */
const PROBE_PNG = Uint8Array.from(
  atob("iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNkYPhfDwAChwGA60e6kgAAAABJRU5ErkJggg=="),
  (c) => c.charCodeAt(0),
);

/** A picture and what is known of how it was made. */
interface Made {
  readonly picture: Picture;
  readonly kind: SavedSignature["kind"];
  readonly strokes?: SavedSignature["strokes"];
  readonly ink?: SavedSignature["ink"];
  readonly suggested: string;
}

/** The signature image's controller for one viewer. */
export class SignatureController {
  readonly #engine: Engine;
  readonly #viewer: Viewer;
  readonly #platform: Platform;
  readonly #toasts: ToastQueue;
  readonly #forms: FormController;
  #state: SignatureState = { open: false, saved: [], certified: false };
  readonly #listeners = new Set<() => void>();
  /** The object URLs the saved list shows pictures from. */
  #urls: string[] = [];

  constructor(engine: Engine, viewer: Viewer, platform: Platform, toasts: ToastQueue, forms: FormController) {
    this.#engine = engine;
    this.#viewer = viewer;
    this.#platform = platform;
    this.#toasts = toasts;
    this.#forms = forms;
  }

  getState(): SignatureState {
    return this.#state;
  }

  subscribe(listener: () => void): () => void {
    this.#listeners.add(listener);
    return () => this.#listeners.delete(listener);
  }

  #set(next: Partial<SignatureState>): void {
    this.#state = { ...this.#state, ...next };
    for (const listener of [...this.#listeners]) listener();
  }

  /** Where the library is: the app's data folder on the desktop, this browser otherwise. */
  get storedIn(): "browser" | "app" {
    return this.#platform.capabilities.hasNativeMenus ? "app" : "browser";
  }

  /** Whether the platform keeps a library: without one, the dialog still draws, types and uploads. */
  get keeps(): boolean {
    return this.#platform.capabilities.signatureLibrary;
  }

  /** Opens the dialog over the signatures kept on this device. */
  async open(): Promise<void> {
    const [saved, certified] = await Promise.all([this.#library(), this.#certified()]);
    this.#set({ open: true, saved, certified });
  }

  close(): void {
    this.#set({ open: false });
    this.#release();
  }

  /** Removes one signature from this device. */
  async remove(id: string): Promise<void> {
    await this.#platform.signatures.remove(id);
    this.#set({ saved: await this.#library() });
  }

  /** Makes the picture of `request`, keeps it if asked, and places it on the page in view. */
  async place(request: SignatureRequest): Promise<void> {
    const made = await this.#make(request);
    if (made === undefined) return;
    const where = this.#inView();
    const context = await this.#forms.placementFor(where.page);
    if (context === undefined) return;
    const blob = await this.#engine.addBlob(made.picture.bytes);
    const ready: ReadySignature = {
      blob,
      width: made.picture.width,
      height: made.picture.height,
      ...(made.strokes === undefined ? {} : { strokes: made.strokes }),
      ...(made.ink === undefined ? {} : { ink: made.ink }),
    };
    const created = new Date();
    const full: SignatureContext = {
      ...context,
      author: storedAuthor(),
      created,
      contentId: `signature-${created.getTime().toString(36)}`,
    };
    const box = signatureBox(ready, { at: where.at });
    const op = placeSignatureOp(ready, request.placement, box, full);
    const applied = await this.#forms.placeOps([op], laterMessage("sign.step"));
    if (!applied) return;
    const kept = request.keep && request.source.kind !== "saved" && this.keeps ? await this.#keep(made) : true;
    this.close();
    if (!kept) this.#toasts.add({ message: laterMessage("sign.keep.failed") });
    this.#toasts.add({
      message: laterMessage("sign.placed.toast"),
      action: { label: laterMessage("sign.undo"), onPress: () => void this.#viewer.edits.undo() },
    });
  }

  /**
   * Where a signature placed from the toolbar goes, with no click to say: the middle of what the
   * window shows, so it lands where the person is looking, else the middle of the current page.
   */
  #inView(): { readonly page: number; readonly at: readonly [number, number] } {
    const box = this.#viewer.element.getBoundingClientRect();
    const middle = pointOn(this.#viewer, this.#engine, {
      clientX: box.left + box.width / 2,
      clientY: box.top + box.height / 2,
    });
    if (middle !== undefined) return { page: middle.page, at: [middle.x, middle.y] };
    const page = this.#viewer.getState().currentPage;
    const size = this.#engine.getState().pages[page];
    return { page, at: [(size?.width ?? 0) / 2, (size?.height ?? 0) / 2] };
  }

  /** The signatures kept on this device, each with a URL for its picture. */
  async #library(): Promise<ShownSignature[]> {
    this.#release();
    if (!this.keeps) return [];
    const all = await this.#platform.signatures.list();
    return all.map((signature) => {
      const url = URL.createObjectURL(new Blob([new Uint8Array(signature.bytes)], { type: signature.type }));
      this.#urls.push(url);
      return { ...signature, url };
    });
  }

  #release(): void {
    for (const url of this.#urls.splice(0)) URL.revokeObjectURL(url);
  }

  /** Whether a picture added to the page's content would break a certification signature: a certification covers the whole file, so any page answers for all. */
  async #certified(): Promise<boolean> {
    const page = this.#viewer.getState().currentPage;
    try {
      const context = await this.#forms.placementFor(page);
      if (context === undefined) return false;
      const probe = placeSignatureOp(
        { blob: await this.#engine.addBlob(PROBE_PNG), width: 1, height: 1 },
        "content",
        [0, 0, SIGNATURE_SIDE, SIGNATURE_SIDE],
        { ...context, author: undefined, contentId: "probe" },
      );
      return (await this.#forms.wouldViolate([probe])).length > 0;
    } catch {
      // The probe didn't run: keep the picture out of the page's content, which is the safe side.
      return true;
    }
  }

  /** The picture `request` asks for. */
  async #make({ source }: SignatureRequest): Promise<Made | undefined> {
    if (source.kind === "draw") return this.#drawn(source.strokes, source.ink);
    if (source.kind === "type") return this.#typed(source.name, source.style);
    if (source.kind === "upload") {
      const picture = await imageToPng(source.bytes);
      return picture === undefined ? undefined : { picture, kind: "image", suggested: "" };
    }
    return this.#kept(source.id);
  }

  async #drawn(strokes: readonly Stroke[], inkName: InkName): Promise<Made | undefined> {
    const drawing = cropDrawing(strokes, PEN);
    if (drawing === undefined) return undefined;
    const ink = INKS[inkName];
    return {
      picture: await drawingToPng(drawing, ink),
      kind: "drawn",
      strokes: drawing.strokes.map((s) => s.map(([x, y]) => [x, y] as [number, number])),
      ink,
      suggested: "",
    };
  }

  async #typed(name: string, styleId: string): Promise<Made | undefined> {
    const style = TYPED_STYLES.find((s) => s.id === styleId) ?? TYPED_STYLES[0];
    if (style === undefined) return undefined;
    const picture = await typedToPng(name, style, BLACK);
    return picture === undefined ? undefined : { picture, kind: "typed", ink: BLACK, suggested: name };
  }

  #kept(id: string): Made | undefined {
    const saved = this.#state.saved.find((s) => s.id === id);
    if (saved === undefined) return undefined;
    return {
      picture: { bytes: saved.bytes, width: saved.width, height: saved.height },
      kind: saved.kind,
      ...(saved.strokes === undefined ? {} : { strokes: saved.strokes }),
      ...(saved.ink === undefined ? {} : { ink: saved.ink }),
      suggested: saved.name,
    };
  }

  /** Adds `made` to the library under a name no other signature has; false when the device refuses it. */
  async #keep(made: Made): Promise<boolean> {
    const taken = this.#state.saved.map((s) => s.name);
    try {
      await this.#platform.signatures.put({
        id: crypto.randomUUID(),
        name: nameFor(made.suggested, taken, laterMessage("sign.name.default")),
        kind: made.kind,
        bytes: made.picture.bytes,
        type: "image/png",
        width: made.picture.width,
        height: made.picture.height,
        ...(made.strokes === undefined ? {} : { strokes: made.strokes }),
        ...(made.ink === undefined ? {} : { ink: made.ink }),
        added: Date.now(),
      });
      return true;
    } catch {
      // The signature is on the page already; only keeping it failed (storage full or blocked).
      return false;
    }
  }
}
