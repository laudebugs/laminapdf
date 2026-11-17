// A stream in the object inspector (O15): its filters and sizes, a Decoded · Raw · Hex group, and
// a focusable monospaced block with the first lines and Show more. What the block holds is capped:
// the inspector reads at most `STREAM_CAP` bytes of a stream, and a stream that decodes past the
// document's limits is never decoded (the engine refuses it); the block says so, shows the raw
// bytes instead, and offers Save raw stream….

import type { ReactNode } from "react";
import { Button } from "../components/button.js";
import { chooseOnPress, PressedGroup } from "../components/pressed-group.js";
import { ToggleButton } from "../components/toggle.js";
import { laterMessage } from "../later-messages.js";

/** The most bytes of a stream the inspector reads and shows: 1 MiB. */
export const STREAM_CAP = 1 << 20;

/** How the block shows a stream. */
export type StreamMode = "decoded" | "raw" | "hex";

/** What was read of a stream, or why it couldn't be. */
export type StreamState =
  | { readonly status: "reading" }
  | { readonly status: "read"; readonly bytes: Uint8Array; readonly total: number }
  | { readonly status: "over-limit"; readonly limit: string }
  | { readonly status: "failed"; readonly reason: string };

/** Props of `StreamBlock`. */
export interface StreamBlockProps {
  /** The stream's object number. */
  readonly num: number;
  /** Whether it is the content stream of the page shown, or a stream of its own. */
  readonly content: boolean;
  /** Its `/Filter` names. */
  readonly filters: readonly string[];
  /** Its length in the file. */
  readonly length: number;
  /** Its decoded length, once known. */
  readonly decodedLength: number | undefined;
  readonly mode: StreamMode;
  readonly onMode: (mode: StreamMode) => void;
  readonly state: StreamState;
  /** The raw bytes read in place of a stream that couldn't be decoded. */
  readonly raw?: StreamState | undefined;
  /** How many lines the block shows. */
  readonly lines: number;
  /** Show more. */
  readonly onMore: () => void;
  /** Save raw stream…. */
  readonly onSaveRaw: () => void;
}

/** Bytes per hex line. */
const HEX_WIDTH = 16;

/** `bytes` as text, one byte per character (a content stream is ASCII; the rest shows as Latin-1). */
function asText(bytes: Uint8Array): string[] {
  return new TextDecoder("latin1").decode(bytes).split(/\r\n|\r|\n/);
}

/** `bytes` as hex lines: an offset, 16 bytes, and their printable characters. */
function asHex(bytes: Uint8Array): string[] {
  const lines: string[] = [];
  for (let at = 0; at < bytes.length; at += HEX_WIDTH) {
    const row = bytes.subarray(at, at + HEX_WIDTH);
    const hex = Array.from(row, (b) => b.toString(16).padStart(2, "0")).join(" ");
    const chars = Array.from(row, (b) => (b >= 0x20 && b < 0x7f ? String.fromCharCode(b) : ".")).join("");
    lines.push(`${at.toString(16).padStart(8, "0")}  ${hex.padEnd(HEX_WIDTH * 3 - 1)}  ${chars}`);
  }
  return lines;
}

/** The words over the block: filters, size in the file, size decoded. */
function sizes(props: StreamBlockProps): string {
  const parts = [props.filters.length === 0 ? laterMessage("inspect.stream.noFilter") : props.filters.join(", ")];
  parts.push(laterMessage("inspect.stream.inFile", { bytes: props.length.toLocaleString("en") }));
  if (props.decodedLength !== undefined) {
    parts.push(laterMessage("inspect.stream.decoded", { bytes: props.decodedLength.toLocaleString("en") }));
  }
  return parts.join(" · ");
}

/** The notice over the block when the stream wasn't read whole or couldn't be decoded. */
function Problem({ state, onSaveRaw }: { state: StreamState; onSaveRaw: () => void }): ReactNode {
  let text: string | undefined;
  if (state.status === "over-limit") text = laterMessage("inspect.stream.overLimit", { limit: state.limit });
  if (state.status === "failed") text = laterMessage("inspect.stream.failed", { reason: state.reason });
  if (state.status === "read" && state.total > state.bytes.length) {
    text = laterMessage("inspect.stream.capped", { size: (state.total / (1 << 20)).toFixed(1) });
  }
  if (text === undefined) return null;
  return (
    <div className="paper-inspect-problem" role="status">
      <span>{text}</span>
      <Button size={30} onPress={onSaveRaw}>
        {laterMessage("inspect.stream.saveRaw")}
      </Button>
    </div>
  );
}

/** A stream: its facts, the view group, and the first lines. */
export function StreamBlock(props: StreamBlockProps): ReactNode {
  const { num, mode, state } = props;
  const id = `inspect-stream-${num}`;
  const shownState = state.status === "read" ? state : props.raw;
  const read = shownState?.status === "read" ? shownState : undefined;
  const all = read === undefined ? [] : mode === "hex" ? asHex(read.bytes) : asText(read.bytes);
  const shown = all.slice(0, props.lines);
  // Raw bytes shown in place of a stream that couldn't be decoded are named as raw.
  const view = laterMessage(state.status === "read" ? `inspect.stream.mode.${mode}` : "inspect.stream.mode.raw");
  return (
    <section className="paper-inspect-section" aria-labelledby={id}>
      <div className="paper-inspect-stream-head">
        <div className="paper-inspect-title">
          <h3 id={id} className="paper-inspect-heading">
            {laterMessage(props.content ? "inspect.stream.content" : "inspect.stream", { num })}
          </h3>
          <span className="paper-inspect-note">{sizes(props)}</span>
        </div>
        <PressedGroup label={laterMessage("inspect.stream.view")} className="paper-inspect-modes">
          {(["decoded", "raw", "hex"] as const).map((m) => (
            <ToggleButton key={m} isSelected={m === mode} onChange={chooseOnPress(m, props.onMode)}>
              {laterMessage(`inspect.stream.mode.${m}`)}
            </ToggleButton>
          ))}
        </PressedGroup>
      </div>
      <Problem state={state} onSaveRaw={props.onSaveRaw} />
      {state.status === "reading" ? <p className="paper-inspect-note">{laterMessage("inspect.loading")}</p> : null}
      {read !== undefined ? (
        <>
          <section
            // A focusable block, named for what it shows, so its lines scroll with the keyboard
            // (O15's accessibility).
            // biome-ignore lint/a11y/noNoninteractiveTabindex: the design asks for a focusable block.
            tabIndex={0}
            className="paper-inspect-pre"
            aria-label={laterMessage("inspect.stream.label", { num, view, count: shown.length })}
          >
            <pre>{shown.join("\n")}</pre>
          </section>
          <span className="paper-inspect-note">
            {laterMessage("inspect.stream.lines", { shown: shown.length, count: all.length })}
            {shown.length < all.length ? (
              <>
                {" · "}
                <button type="button" className="paper-object-link" onClick={props.onMore}>
                  {laterMessage("inspect.stream.more")}
                </button>
              </>
            ) : null}
          </span>
        </>
      ) : null}
    </section>
  );
}
