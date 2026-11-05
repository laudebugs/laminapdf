// The stack of open layers, innermost last, so Escape closes the innermost one and nothing under
// it. React Aria's overlays already close themselves on Escape while focus is inside them; this
// stack covers the rest: Escape pressed while focus is elsewhere, and layers React Aria doesn't
// draw. It holds no DOM, so its rules are tested in Node.

/** One open layer: how to close it, whether Escape may, and the layer it was opened from. */
export interface Layer {
  /** Closes the layer. */
  readonly dismiss: () => void;
  /** False for a dialog that asks a required question: Escape leaves it, and what lies beneath, open. */
  readonly dismissable: boolean;
  /** The layer this one opened from, such as the dialog a menu belongs to. */
  readonly parent?: Layer | undefined;
}

/** Whether `layer` was opened, directly or not, from `ancestor`. */
function within(layer: Layer, ancestor: Layer): boolean {
  for (let p = layer.parent; p !== undefined; p = p.parent) if (p === ancestor) return true;
  return false;
}

/** The open layers. Each `open` returns the function that removes its layer, wherever it sits. */
export class LayerStack {
  readonly #layers: Layer[] = [];

  /**
   * Adds `layer` and returns the function that removes it. A layer goes on top, except beneath
   * any open layer opened from it: React registers a child before its parent when both open in
   * one render, and the child must still close first.
   */
  open(layer: Layer): () => void {
    const firstChild = this.#layers.findIndex((l) => within(l, layer));
    if (firstChild === -1) this.#layers.push(layer);
    else this.#layers.splice(firstChild, 0, layer);
    return () => {
      const at = this.#layers.lastIndexOf(layer);
      if (at !== -1) this.#layers.splice(at, 1);
    };
  }

  /** How many layers are open. */
  get size(): number {
    return this.#layers.length;
  }

  /** Whether `layer` is the innermost open layer. */
  isTopmost(layer: Layer): boolean {
    return this.#layers.at(-1) === layer;
  }

  /**
   * Escape: dismisses the innermost layer if it may be dismissed. Returns whether a layer was
   * open, so the caller knows the key was used even when a required question kept it open.
   */
  escape(): boolean {
    const top = this.#layers.at(-1);
    if (top === undefined) return false;
    if (top.dismissable) top.dismiss();
    return true;
  }
}
