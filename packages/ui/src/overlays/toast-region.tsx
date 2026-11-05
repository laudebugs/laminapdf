// The toast region (§7.20): one polite live region, present before any toast arrives so screen
// readers announce each one, showing at most three, newest at the bottom. Each toast leaves after
// 4 s; hovering or focusing the region pauses every timer. There is no error toast: an error
// belongs where it happened, in a notice or a field.

import { MOTION } from "@laminapdf/tokens";
import { type ReactNode, useEffect, useRef, useState, useSyncExternalStore } from "react";
import { Toast, type ToastAction } from "../components/notice.js";

/** At most this many toasts show; a new one pushes the oldest out. */
const MAX_VISIBLE = 3;

/** What a toast says. */
export interface ToastOptions {
  /** What happened: "Copied", "Highlights exported". */
  message: string;
  /** One action, such as "Undo"; also offer it elsewhere (a menu, a shortcut), since a toast goes away. */
  action?: ToastAction;
  /** How long it stays, in ms; 4 s when left out. */
  timeout?: number;
}

/** A shown toast. */
export interface ToastHandle {
  /** Removes it now. */
  dismiss(): void;
}

interface Entry {
  readonly key: number;
  readonly options: ToastOptions;
}

/** The toasts waiting to be shown by one `ToastRegion`. */
export class ToastQueue {
  #entries: readonly Entry[] = [];
  #next = 0;
  readonly #listeners = new Set<() => void>();

  /** Shows a toast and returns the handle that dismisses it. */
  add(options: ToastOptions): ToastHandle {
    const key = this.#next++;
    this.#entries = [...this.#entries, { key, options }].slice(-MAX_VISIBLE);
    this.#emit();
    return { dismiss: () => this.#remove(key) };
  }

  /** For `ToastRegion`: calls `listener` when the toasts change; returns the unsubscribe. */
  readonly subscribe = (listener: () => void): (() => void) => {
    this.#listeners.add(listener);
    return () => this.#listeners.delete(listener);
  };

  /** For `ToastRegion`: the toasts now, oldest first. */
  readonly entries = (): readonly Entry[] => this.#entries;

  /** For `ToastRegion`: removes a toast by key. */
  remove(key: number): void {
    this.#remove(key);
  }

  #remove(key: number): void {
    const next = this.#entries.filter((e) => e.key !== key);
    if (next.length === this.#entries.length) return;
    this.#entries = next;
    this.#emit();
  }

  #emit(): void {
    for (const listener of this.#listeners) listener();
  }
}

/** Props of `ToastRegion`. */
export interface ToastRegionProps {
  /** The queue whose toasts it shows. */
  queue: ToastQueue;
  /** The region's name, for landmark navigation: "Notifications". */
  "aria-label": string;
}

/** The region toasts appear in. Render it once, near the root, before any toast is added. */
export function ToastRegion({ queue, "aria-label": label }: ToastRegionProps): ReactNode {
  const entries = useSyncExternalStore(queue.subscribe, queue.entries, queue.entries);
  const [hovered, setHovered] = useState(false);
  const [focused, setFocused] = useState(false);
  const paused = hovered || focused;
  return (
    <section
      aria-label={label}
      className="paper-toast-region"
      // React Aria leaves elements marked as its top layer outside a modal's inert, so a toast
      // shown over a dialog is still announced and its action still reachable.
      data-react-aria-top-layer=""
      onPointerEnter={() => setHovered(true)}
      onPointerLeave={() => setHovered(false)}
      onFocus={() => setFocused(true)}
      onBlur={(e) => {
        if (!e.currentTarget.contains(e.relatedTarget)) setFocused(false);
      }}
    >
      <div role="status" className="paper-toast-list">
        {entries.map((entry) => (
          <TimedToast key={entry.key} entry={entry} paused={paused} onDone={() => queue.remove(entry.key)} />
        ))}
      </div>
    </section>
  );
}

/** One toast with its own timer, which stops while `paused` and resumes with the time it had left. */
function TimedToast({ entry, paused, onDone }: { entry: Entry; paused: boolean; onDone: () => void }): ReactNode {
  const remaining = useRef(entry.options.timeout ?? MOTION.toast);
  const done = useRef(onDone);
  done.current = onDone;
  useEffect(() => {
    if (paused) return;
    const started = performance.now();
    const timer = setTimeout(() => done.current(), remaining.current);
    return () => {
      clearTimeout(timer);
      remaining.current -= performance.now() - started;
    };
  }, [paused]);
  const { message, action } = entry.options;
  return (
    <Toast
      {...(action === undefined
        ? {}
        : {
            action: {
              label: action.label,
              onPress: () => {
                action.onPress();
                done.current();
              },
            },
          })}
    >
      {message}
    </Toast>
  );
}
