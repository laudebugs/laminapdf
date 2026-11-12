// The WebAssembly instances behind every handle. A trap (a bug, or growth past the memory maximum
// set when the modules are built) can leave an instance's memory half-written, so the call that
// trapped throws `wasm-trap` or `out-of-memory`, and the next call first replaces the instance with
// a fresh one of the same compiled module (wasm-bindgen's `__wbg_reset_state`); `reset()` replaces
// every one. Each replacement starts a new generation: a handle made in an older one throws
// `instance-reset` rather than reach into the new instance's memory.
import { isDiagnostic, isLaminaPdfError, LaminaPdfError, packageError } from "../errors.js";

/**
 * Whether `e` stopped WebAssembly code mid-call: a trap (`RuntimeError`), or the engine's stack
 * overflowing inside it (V8 and JavaScriptCore throw a `RangeError`, Firefox an `InternalError`).
 * Either leaves the instance's memory half-written.
 */
function stoppedMidCall(e: unknown): boolean {
  if (e instanceof WebAssembly.RuntimeError) return true;
  if (!(e instanceof Error)) return false;
  return (
    (e instanceof RangeError && /call stack/i.test(e.message)) ||
    (e.name === "InternalError" && /recursion/i.test(e.message))
  );
}

/**
 * Whether `e` is a trap: as {@link Instance.call} reports one (`wasm-trap`, `out-of-memory`), or
 * raw from a module the package doesn't guard (an extension's PDFium).
 */
export function isTrap(e: unknown): boolean {
  return stoppedMidCall(e) || isLaminaPdfError(e, "wasm-trap") || isLaminaPdfError(e, "out-of-memory");
}

/** The modules of `@laminapdf/wasm` that `@laminapdf/sdk` loads. */
export type ModuleName = "base" | "text" | "extract";

/** The exports every wasm-bindgen module has. */
interface Exports {
  readonly memory: WebAssembly.Memory;
}

/** What `@laminapdf/sdk` needs from a module's glue besides its classes. */
export interface Glue {
  /** Replaces the instance with a fresh one of the same compiled module. */
  __wbg_reset_state(): void;
  /** Whether an allocation failed in the current instance. */
  allocationFailed(): boolean;
  /** Once the module is loaded: the current instance's exports. */
  initSync(module: never): Exports;
}

/** One loaded module: its glue, and which of its instances is current. */
export class Instance<G extends Glue = Glue> {
  readonly name: ModuleName;
  readonly glue: G;
  #memory: WebAssembly.Memory;
  #generation = 0;
  /** Set by a trap: the next call replaces the instance first. */
  #poisoned = false;

  constructor(name: ModuleName, glue: G, exports: Exports) {
    this.name = name;
    this.glue = glue;
    this.#memory = exports.memory;
  }

  /** Which instance is current; a handle keeps the one it was made in. */
  get generation(): number {
    return this.#generation;
  }

  /** The size of the current instance's memory, in bytes. */
  get memoryBytes(): number {
    return this.#memory.buffer.byteLength;
  }

  /**
   * Whether objects made in `generation` can still be freed: it is current, and hasn't trapped
   * (a trapped instance is about to be replaced, and freeing into it could trap again).
   */
  owns(generation: number): boolean {
    return generation === this.#generation && !this.#poisoned;
  }

  /**
   * Throws `instance-reset` unless objects made in `generation` belong to the current instance,
   * after replacing an instance that trapped (so a trap's survivors never reach the fresh one).
   *
   * @param name - what holds the objects, for the message.
   */
  check(generation: number, name: string): void {
    this.live();
    if (generation !== this.#generation) {
      throw packageError(
        "instance-reset",
        `the ${name} belongs to a WebAssembly instance that reset() or a trap replaced`,
      );
    }
  }

  /** The glue, after replacing an instance that trapped. */
  live(): G {
    if (this.#poisoned) this.replace();
    return this.glue;
  }

  /** Replaces the instance with a fresh one: memory back to its initial size, every handle gone. */
  replace(): void {
    this.glue.__wbg_reset_state();
    // The glue's `initSync` returns early with the current exports once the module is loaded.
    this.#memory = this.glue.initSync(undefined as never).memory;
    this.#generation++;
    this.#poisoned = false;
  }

  /**
   * Runs one call into the glue, after replacing an instance that trapped.
   *
   * @throws {@link LaminaPdfError} for the error object the module throws; `wasm-trap` or
   *   `out-of-memory` when it traps or overflows the stack (the next call gets a fresh instance).
   */
  call<T>(f: (glue: G) => T): T {
    const glue = this.live();
    try {
      return f(glue);
    } catch (e) {
      throw this.#wrap(e);
    }
  }

  #wrap(e: unknown): unknown {
    if (isDiagnostic(e)) return new LaminaPdfError(e);
    if (!stoppedMidCall(e)) return e;
    this.#poisoned = true;
    const module = this.name;
    if (this.#allocationFailed()) {
      return packageError("out-of-memory", `the ${module} module ran out of memory; the next call starts afresh`, {
        data: { module, memoryBytes: this.memoryBytes },
        cause: e,
      });
    }
    return packageError("wasm-trap", `the ${module} module trapped; the next call starts afresh`, {
      data: { module },
      cause: e,
    });
  }

  /** Whether the trapped instance ran out of memory. Reading its flag is safe: a trap leaves memory as it was. */
  #allocationFailed(): boolean {
    try {
      return this.glue.allocationFailed();
    } catch {
      return false;
    }
  }
}
