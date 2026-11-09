// Reader mode (R14): a document's blocks reflowed into one reading column, at the reader's text
// size, line spacing and line length, with figures, formulas and tables kept as crops of their
// page, and the panel that changes those settings. The app makes the blocks (the structure tree's
// order) and draws the crops; this only lays them out.

import { type ReactNode, useEffect, useId, useRef } from "react";
import { Badge } from "../components/badge.js";
import { Button, IconButton } from "../components/button.js";
import { Radio, RadioGroup } from "../components/choice.js";
import { EmptyState } from "../components/empty-state.js";
import { Skeleton } from "../components/progress.js";
import { message } from "../messages.js";
import { DockedPanel } from "./docked-panel.js";

/** How far apart a paragraph's lines are. */
export type ReaderLineSpacing = "tight" | "normal" | "loose";

/** How wide the column is: about 55, 68 or 85 characters. */
export type ReaderLineLength = "narrow" | "medium" | "wide";

/** The reader's settings: global, kept by the app. */
export interface ReaderSettings {
  /** The text size in CSS px, from {@link READER_TEXT_SIZES}' first to its last. */
  readonly textSize: number;
  readonly lineSpacing: ReaderLineSpacing;
  readonly lineLength: ReaderLineLength;
}

/** The smallest and largest text size, in CSS px. */
export const READER_TEXT_SIZES = [14, 32] as const;

/** The settings Reset brings back. */
export const DEFAULT_READER_SETTINGS: ReaderSettings = { textSize: 19, lineSpacing: "normal", lineLength: "medium" };

/** `size` kept between the smallest and largest text size, in whole pixels. */
export function readerTextSize(size: number): number {
  const [least, most] = READER_TEXT_SIZES;
  return Math.min(most, Math.max(least, Math.round(size)));
}

/** One block of the reading view: what the app's reflow gives it. */
export interface ReaderBlock {
  /** `heading`, `paragraph`, `list-item`, `quote`, `caption`, `note`, `figure`, `formula`, `table` or `other`. */
  readonly kind: string;
  readonly text: string;
  /** The page it is on, 0-based. */
  readonly page: number;
  /** A heading's level, 1 to 6. */
  readonly level?: number;
  /** A list item's marker and depth (0 for the outermost list). */
  readonly marker?: string;
  readonly depth?: number;
  /** A crop's alternate text. */
  readonly alt?: string;
}

/** Whether `block` is shown as a crop of its page rather than as text. */
export function isCropBlock(block: ReaderBlock): boolean {
  return block.kind === "figure" || block.kind === "formula" || block.kind === "table";
}

/** What the view has to show. */
export type ReaderViewContent =
  | { readonly status: "loading" }
  /**
   * The document can't be reflowed, because it has no tags or its tree can't be trusted: the
   * empty state, with Back to page.
   */
  | { readonly status: "unavailable"; readonly untagged: boolean }
  | { readonly status: "ready"; readonly blocks: readonly ReaderBlock[]; readonly lang?: string };

/** Props of `ReaderView`. */
export interface ReaderViewProps {
  content: ReaderViewContent;
  settings: ReaderSettings;
  /** The page "Back to page" names, as people number it (its label, or its number). */
  pageName: string;
  /** Leaves reader mode: "Back to page", and Escape in the column. */
  onBack: () => void;
  /** Draws a crop: the app renders the block's part of its page. */
  renderCrop: (block: ReaderBlock, index: number) => ReactNode;
  /** The block to bring to the top when the blocks first show. */
  startAt?: number;
  /** Called with the index of the block at the top of the column as it scrolls. */
  onTopBlock?: (index: number) => void;
}

/** A text block as its element. */
function TextBlock({ block }: { block: ReaderBlock }): ReactNode {
  switch (block.kind) {
    case "heading": {
      // The column's own title is the document's, so the tree's levels start one below it.
      const level = Math.min(6, Math.max(2, (block.level ?? 1) + 1));
      const Heading = `h${level}` as "h2";
      return <Heading className="paper-reader-heading">{block.text}</Heading>;
    }
    case "quote":
      return (
        <blockquote className="paper-reader-quote">
          <p>{block.text}</p>
        </blockquote>
      );
    case "caption":
      return <p className="paper-reader-caption">{block.text}</p>;
    case "note":
      return (
        <aside className="paper-reader-note" aria-label={message("reader.mode.note")}>
          <p>{block.text}</p>
        </aside>
      );
    default:
      return <p>{block.text}</p>;
  }
}

/** A run of blocks: consecutive list items as one list, or blocks shown one by one. */
interface Group {
  readonly list: boolean;
  /** Each block with its index in the reflow, which is its key and its `data-block`. */
  readonly entries: { readonly index: number; readonly block: ReaderBlock }[];
}

/** `blocks` in runs, consecutive list items together. */
function groups(blocks: readonly ReaderBlock[]): Group[] {
  const out: Group[] = [];
  blocks.forEach((block, index) => {
    const list = block.kind === "list-item";
    const last = out.at(-1);
    if (last?.list && list) last.entries.push({ index, block });
    else out.push({ list, entries: [{ index, block }] });
  });
  return out;
}

/** A crop: the app's drawing of the block's part of the page, named by its alternate text. */
function CropBlock({ block, children }: { block: ReaderBlock; children: ReactNode }): ReactNode {
  const alt = block.alt ?? message("reader.mode.noAlt");
  return (
    <figure className="paper-reader-crop">
      <div className="paper-reader-crop-image" role="img" aria-label={alt}>
        {children}
      </div>
      <figcaption className="paper-reader-alt">{alt}</figcaption>
    </figure>
  );
}

/** The blocks' indices as the column scrolls: which one is at the top. */
function useTopBlock(column: React.RefObject<HTMLElement | null>, onTopBlock: ((index: number) => void) | undefined) {
  useEffect(() => {
    const root = column.current;
    if (root === null || onTopBlock === undefined) return;
    const report = () => {
      // The column's top is below its sticky head.
      const head = root.querySelector(".paper-reader-mode-head")?.getBoundingClientRect().bottom;
      const top = head ?? root.getBoundingClientRect().top;
      const blocks = root.querySelectorAll<HTMLElement>("[data-block]");
      for (const block of blocks) {
        if (block.getBoundingClientRect().bottom > top + 1) {
          onTopBlock(Number(block.dataset.block));
          return;
        }
      }
    };
    // Only a scroll moves the top block: until then it is the one the app started at.
    root.addEventListener("scroll", report, { passive: true });
    return () => root.removeEventListener("scroll", report);
  }, [column, onTopBlock]);
}

/** Consecutive list items as one list. */
function ListGroup({ group }: { group: Group }): ReactNode {
  return (
    <ul className="paper-reader-list">
      {group.entries.map(({ index, block }) => (
        <li
          key={index}
          className="paper-reader-block"
          data-block={index}
          data-kind="list-item"
          data-depth={block.depth ?? 0}
        >
          {block.marker === undefined || block.marker === "" ? null : (
            <span className="paper-reader-marker" aria-hidden="true">
              {block.marker}
            </span>
          )}
          {block.text}
        </li>
      ))}
    </ul>
  );
}

/** The column's blocks, each `data-block` its index, crops drawn by `renderCrop`. */
function Blocks({
  blocks,
  renderCrop,
}: {
  blocks: readonly ReaderBlock[];
  renderCrop: ReaderViewProps["renderCrop"];
}): ReactNode {
  return groups(blocks).map((group) =>
    group.list ? (
      <ListGroup key={group.entries[0]?.index} group={group} />
    ) : (
      group.entries.map(({ index, block }) => (
        <div key={index} className="paper-reader-block" data-block={index} data-kind={block.kind}>
          {isCropBlock(block) ? (
            <CropBlock block={block}>{renderCrop(block, index)}</CropBlock>
          ) : (
            <TextBlock block={block} />
          )}
        </div>
      ))
    ),
  );
}

/** Placeholder lines while the blocks load. */
const LOADING = ["100%", "96%", "88%", "100%", "64%"].map((width, i) => ({ id: `line-${i}`, width }));

/** Why reader mode can't show the document, and the way back. */
function Unavailable({ untagged, onBack }: { untagged: boolean; onBack: () => void }): ReactNode {
  return (
    <div className="paper-reader-mode paper-reader-mode-empty">
      <EmptyState
        title={message("reader.mode.unavailable.title")}
        icon="reader"
        actions={
          <Button variant="primary" onPress={onBack}>
            {message("reader.mode.backToPage")}
          </Button>
        }
      >
        {message(untagged ? "reader.mode.untagged.body" : "reader.mode.unavailable.body")}
      </EmptyState>
    </div>
  );
}

/** The reading view: the reflowed column, its loading lines, or why it can't be shown. */
export function ReaderView({
  content,
  settings,
  pageName,
  onBack,
  renderCrop,
  startAt,
  onTopBlock,
}: ReaderViewProps): ReactNode {
  const column = useRef<HTMLElement>(null);
  const blocks = content.status === "ready" ? content.blocks : undefined;
  useTopBlock(column, blocks === undefined ? undefined : onTopBlock);
  // Brings the start block to the top once, when the blocks first show.
  useEffect(() => {
    // The first block needs no scroll: the column's head (Back, the badge) stays in view.
    if (blocks === undefined || startAt === undefined || startAt === 0) return;
    const root = column.current;
    const block = root?.querySelector<HTMLElement>(`[data-block="${startAt}"]`);
    const head = root?.querySelector(".paper-reader-mode-head")?.getBoundingClientRect().height ?? 0;
    // Just below the sticky head: what the top block is measured from.
    if (root !== null && root !== undefined && block !== null && block !== undefined) {
      root.scrollTop += block.getBoundingClientRect().top - root.getBoundingClientRect().top - head;
    }
  }, [blocks, startAt]);
  if (content.status === "unavailable") return <Unavailable untagged={content.untagged} onBack={onBack} />;
  return (
    <section
      ref={column}
      className="paper-reader-mode"
      aria-label={message("reader.mode.title")}
      onKeyDown={(event) => {
        if (event.key !== "Escape") return;
        event.stopPropagation();
        onBack();
      }}
    >
      <article
        className="paper-reader-column"
        aria-busy={blocks === undefined || undefined}
        {...(content.status === "ready" && content.lang !== undefined ? { lang: content.lang } : {})}
        data-line-spacing={settings.lineSpacing}
        data-line-length={settings.lineLength}
        style={{ fontSize: `${readerTextSize(settings.textSize)}px` }}
      >
        <div className="paper-reader-mode-head">
          <Button variant="secondary" icon="back" onPress={onBack}>
            {message("reader.mode.back", { page: pageName })}
          </Button>
          <Badge tone="accent">{message("reader.mode.badge")}</Badge>
        </div>
        {blocks === undefined ? (
          <div className="paper-reader-loading">
            {LOADING.map((line) => (
              <Skeleton key={line.id} inlineSize={line.width} />
            ))}
          </div>
        ) : (
          <Blocks blocks={blocks} renderCrop={renderCrop} />
        )}
      </article>
    </section>
  );
}

/** Props of `ReaderSettingsPanel`. */
export interface ReaderSettingsPanelProps {
  settings: ReaderSettings;
  onChange: (settings: ReaderSettings) => void;
  onClose: () => void;
}

/** The reader's settings, docked in the inspector: text size, line spacing, line length, reset. */
export function ReaderSettingsPanel({ settings, onChange, onClose }: ReaderSettingsPanelProps): ReactNode {
  const size = useId();
  const [least, most] = READER_TEXT_SIZES;
  const set = (patch: Partial<ReaderSettings>) => onChange({ ...settings, ...patch });
  return (
    <DockedPanel
      title={message("reader.settings.title")}
      subtitle={message("reader.settings.subtitle")}
      closeLabel={message("reader.settings.close")}
      onClose={onClose}
    >
      <div className="paper-reader-settings">
        <fieldset className="paper-reader-stepper" aria-labelledby={size}>
          <legend id={size} className="paper-field-label">
            {message("reader.settings.textSize")}
          </legend>
          <div className="paper-reader-stepper-row">
            <IconButton
              icon="zoom-out"
              label={message("command.view.reader-text-smaller")}
              isDisabled={settings.textSize <= least}
              onPress={() => set({ textSize: readerTextSize(settings.textSize - 1) })}
            />
            <output aria-live="polite">{message("reader.settings.px", { size: String(settings.textSize) })}</output>
            <IconButton
              icon="zoom-in"
              label={message("command.view.reader-text-larger")}
              isDisabled={settings.textSize >= most}
              onPress={() => set({ textSize: readerTextSize(settings.textSize + 1) })}
            />
          </div>
        </fieldset>
        <RadioGroup
          label={message("reader.settings.lineSpacing")}
          value={settings.lineSpacing}
          onChange={(value) => set({ lineSpacing: value as ReaderLineSpacing })}
        >
          <Radio value="tight">{message("choice.tight")}</Radio>
          <Radio value="normal">{message("choice.normal")}</Radio>
          <Radio value="loose">{message("choice.loose")}</Radio>
        </RadioGroup>
        <RadioGroup
          label={message("reader.settings.lineLength")}
          value={settings.lineLength}
          onChange={(value) => set({ lineLength: value as ReaderLineLength })}
        >
          <Radio value="narrow">{message("choice.narrow")}</Radio>
          <Radio value="medium">{message("choice.medium")}</Radio>
          <Radio value="wide">{message("choice.wide")}</Radio>
        </RadioGroup>
        <Button variant="ghost" onPress={() => onChange(DEFAULT_READER_SETTINGS)}>
          {message("command.view.reader-reset")}
        </Button>
      </div>
    </DockedPanel>
  );
}
