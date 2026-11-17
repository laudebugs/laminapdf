// One editing gesture on a page, such as the annotation layer's moves: what keeps the page from
// showing an annotation twice, or flickering, while an editor draws a change before making it. Starting
// one leaves the annotations it changes out of the page's renders, and its preview shows only once
// the page is drawn without them. Committing applies the edit and keeps the preview until the
// page's new pixels are on screen, then removes it in the same task, so the frame that shows the
// new bitmap is the first without the preview. Cancelling shows the annotations again (drawn from
// the bitmap cache, so at once) and removes the preview with them.

import type { Op } from "@laminapdf/sdk";
import type { EditResult } from "../protocol.js";
import type { PreviewShape } from "./annotation-geometry.js";

/** Where a gesture is: drawing its preview, applying its edit, or over. */
export type GestureState = "starting" | "active" | "committing" | "committed" | "cancelled";

/** What a gesture needs of the viewer and of the layer that draws its preview. */
export interface GestureHost {
  /** Leaves the annotations at these places out of the page's renders; resolves once drawn so. */
  hideAnnotations(page: number, places: readonly number[]): Promise<void>;
  /** Resolves once the page's pixels in view show it as it is now. */
  whenDrawn(page: number): Promise<void>;
  /** Applies `ops` as one edit and redraws the pages it touched (which shows hidden annotations again). */
  applyEdit(ops: readonly Op[], options?: { readonly label?: string }): Promise<EditResult>;
  /** Draws `shapes` over the page (page points), or removes the preview with `undefined`. */
  preview(page: number, shapes: readonly PreviewShape[] | undefined): void;
}

/** An editing gesture on one page: start it with `AnnotationLayer.startGesture`. */
export class AnnotationGesture {
  readonly page: number;
  readonly #host: GestureHost;
  readonly #hidden: readonly number[];
  #state: GestureState = "starting";
  #shapes: readonly PreviewShape[] = [];
  /** Resolves once the page is drawn without the hidden annotations. */
  readonly #started: Promise<void>;

  constructor(host: GestureHost, page: number, hidden: readonly number[]) {
    this.#host = host;
    this.page = page;
    this.#hidden = hidden;
    this.#started = (hidden.length > 0 ? host.hideAnnotations(page, hidden) : Promise.resolve()).then(() => {
      if (this.#state !== "starting") return;
      this.#state = "active";
      this.#draw();
    });
  }

  /** Where the gesture is. */
  get state(): GestureState {
    return this.#state;
  }

  /** The places in the page's `/Annots` this gesture leaves out of its renders. */
  get hidden(): readonly number[] {
    return this.#hidden;
  }

  /** Resolves once the gesture's preview can show: the page is drawn without what it hides. */
  ready(): Promise<void> {
    return this.#started;
  }

  /** Replaces the preview: what the gesture would make, in page points. */
  update(shapes: readonly PreviewShape[]): void {
    if (this.#state !== "starting" && this.#state !== "active") return;
    this.#shapes = shapes;
    // Until the page is drawn without the hidden annotations, the preview would show them twice.
    if (this.#state === "active") this.#draw();
  }

  /**
   * Applies `ops` as one edit. The preview stays until the page's new pixels are drawn and goes in
   * that task. A failed edit shows the annotations again, removes the preview and rejects.
   */
  async commit(ops: readonly Op[], options: { readonly label?: string } = {}): Promise<EditResult> {
    if (this.#state !== "starting" && this.#state !== "active") throw new Error(`the gesture is ${this.#state}`);
    // The preview shows from now on, whatever the hiding: the edit's redraw replaces both.
    const wasStarting = this.#state === "starting";
    this.#state = "committing";
    if (wasStarting) await this.#started;
    this.#draw();
    try {
      const result = await this.#host.applyEdit(ops, options);
      await this.#host.whenDrawn(this.page);
      return result;
    } catch (e) {
      await this.#host.hideAnnotations(this.page, []);
      throw e;
    } finally {
      this.#state = "committed";
      this.#host.preview(this.page, undefined);
    }
  }

  /** Ends the gesture with no edit: the annotations show again and the preview goes with them. */
  async cancel(): Promise<void> {
    if (this.#state !== "starting" && this.#state !== "active") return;
    this.#state = "cancelled";
    await this.#started;
    if (this.#hidden.length > 0) await this.#host.hideAnnotations(this.page, []);
    this.#host.preview(this.page, undefined);
  }

  #draw(): void {
    this.#host.preview(this.page, this.#shapes);
  }
}
