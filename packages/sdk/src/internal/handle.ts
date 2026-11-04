// The base of every handle that owns memory in a WebAssembly instance: `free()` and
// `Symbol.dispose` (the same thing; a second call does nothing), `disposed` for any call after it,
// `instance-reset` for a handle whose instance was replaced, and a parent that frees it too (a
// page with its document or builder). A handle the garbage collector reclaims is still freed, by
// wasm-bindgen's own finalizer; `onLeak` reports those.
import { packageError } from "../errors.js";
import type { Glue, Instance } from "./instance.js";

/**
 * `Symbol.dispose`, or, where a runtime lacks it (Safari), `Symbol.for("Symbol.dispose")`: the key
 * the `using` helpers of esbuild and Babel fall back to.
 */
const DISPOSE: symbol = typeof Symbol.dispose === "symbol" ? Symbol.dispose : Symbol.for("Symbol.dispose");

/** A handle the garbage collector reclaimed without a `free()`. */
export interface Leak {
  /** The handle's class: `Document`, `Builder` or `Image`. */
  readonly handle: string;
}

let leakHook: ((leak: Leak) => void) | undefined;
const leaks =
  typeof FinalizationRegistry === "function"
    ? new FinalizationRegistry<string>((handle) => leakHook?.({ handle }))
    : undefined;

/**
 * Reports every document, builder and image created from now on that the garbage collector
 * reclaims without a `free()` or `using`: a debugging aid for code that should free what it
 * makes. Reclaimed handles are freed anyway, only later. `onLeak()` turns the reports off.
 *
 * @param hook - called once per leaked handle, some time after the collection.
 */
export function onLeak(hook?: (leak: Leak) => void): void {
  leakHook = hook;
}

/** Watches `handle` for a leak while a hook is set; {@link forget} once it is freed. */
export function watch(handle: object, name: string): void {
  if (leakHook !== undefined) leaks?.register(handle, name, handle);
}

/** Stops watching `handle`. */
export function forget(handle: object): void {
  leaks?.unregister(handle);
}

/** A handle on memory in one WebAssembly instance. */
export abstract class Handle<G extends Glue = Glue> {
  /** The class, for messages (minifiers rename classes). */
  readonly #name: string;
  readonly #instance: Instance<G>;
  readonly #generation: number;
  readonly #parent: Handle | undefined;
  #freed = false;

  /**
   * @param name - the class, as messages name it.
   * @param instance - the instance the handle's memory lives in.
   * @param parent - the handle it is freed with, if any.
   */
  protected constructor(name: string, instance: Instance<G>, parent?: Handle) {
    this.#name = name;
    this.#instance = instance;
    this.#generation = instance.generation;
    this.#parent = parent;
  }

  /** The instance this handle lives in (for the handles it makes). */
  protected get instance(): Instance<G> {
    return this.#instance;
  }

  /** Whether the handle's raw objects can be freed: its instance is current and hasn't trapped. */
  protected get owned(): boolean {
    return this.#instance.owns(this.#generation);
  }

  /** Whether `free()` ran on this handle or its parent. */
  protected get isFreed(): boolean {
    return this.#freed || this.#parent?.isFreed === true;
  }

  /**
   * @throws {@link LaminaPdfError} `disposed` after `free()` (here or on the parent);
   *   `instance-reset` when the instance was replaced (a trapped one is replaced first).
   */
  protected assertLive(): void {
    if (this.isFreed) {
      throw packageError("disposed", `the ${this.#name} was used after free() or its using block`);
    }
    this.#instance.check(this.#generation, this.#name);
  }

  /** Runs `f` (calls into the glue) on this live handle; see {@link Instance.call}. */
  protected call<T>(f: (glue: G) => T): T {
    this.assertLive();
    return this.#instance.call(f);
  }

  /** Frees the WebAssembly side; called at most once, and only while {@link owned}. */
  protected abstract release(): void;

  /**
   * Frees the handle's memory now; without it, the garbage collector frees it later. A second
   * call does nothing, and any other call afterwards throws `disposed`. Prefer `using`, which
   * calls this at the end of the block.
   */
  free(): void {
    if (this.#freed) return;
    this.#freed = true;
    forget(this);
    // A replaced instance's memory is gone already, and its objects must not reach the new one.
    if (this.owned) this.release();
  }

  /** Frees the handle at the end of a `using` block (an alias of `free()`). */
  [Symbol.dispose](): void {
    this.free();
  }
}

/** Gives `target`'s prototype the dispose method under {@link DISPOSE} where it isn't `Symbol.dispose`. */
export function disposable(target: { readonly prototype: { free(): void } }): void {
  if (DISPOSE === Symbol.dispose) return;
  Object.defineProperty(target.prototype, DISPOSE, {
    value(this: { free(): void }) {
      this.free();
    },
    configurable: true,
    writable: true,
  });
}

disposable(Handle);
