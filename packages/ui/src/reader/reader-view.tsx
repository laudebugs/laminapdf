// Reader mode (R14): a document's blocks reflowed into one reading column, at the reader's text
// size, line spacing and line length, with figures, formulas and tables kept as crops of their
// page, and the panel that changes those settings. The app makes the blocks (in the structure
// tree's order, or, approximate, the page layout's) and draws the crops; this only lays them out.
// The approximate view also says so, offers each block's original page, and shows the pages it
// couldn't read with confidence as their images.

import { type ReactNode, useEffect, useId, useRef } from "react";
import { Badge } from "../components/badge.js";
import { Button, IconButton } from "../components/button.js";
import { Radio, RadioGroup } from "../components/choice.js";
import { EmptyState } from "../components/empty-state.js";
import { Notice } from "../components/notice.js";
import { Skeleton } from "../components/progress.js";
import { message } from "../messages.js";
import { DockedPanel } from "./docked-panel.js";
import {
  DEFAULT_READER_SETTINGS,
  READER_TEXT_SIZES,
  type ReaderLineLength,
  type ReaderLineSpacing,
  type ReaderSettings,
  readerTextSize,
} from "./reader-settings.js";

/** One block of the reading view: what the app's reflow gives it. */
export interface ReaderBlock {
  /**
   * `heading`, `paragraph`, `list-item`, `quote`, `caption`, `note`, `figure`, `formula`, `table`,
   * `page-image` (a whole page, approximate only) or `other`.
   */
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
  /** Why a page image isn't reflowed: `scanned` or `uncertain`. */
  readonly why?: string;
}

/** Whether `block` is shown as a crop of its page rather than as text. */
export function isCropBlock(block: ReaderBlock): boolean {
  return ["figure", "formula", "table", "page-image"].includes(block.kind);
}

/** Why the view is approximate: the file has no tags, or its tags don't match its pages. */
export type ReaderApproximation = "untagged" | "tree";

/** What the view has to show. */
export type ReaderViewContent =
  | { readonly status: "loading" }
  /** No page has text or a picture to show: the empty state, with Back to page. */
  | { readonly status: "unavailable" }
  | {
      readonly status: "ready";
      readonly blocks: readonly ReaderBlock[];
      readonly lang?: string;
      /** Set when the order comes from the page layout, not the tags: the view says why. */
      readonly approximate?: ReaderApproximation;
    };

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
  /** Leaves reader mode at a block's place on its page, outlined: each approximate block's "View original page". */
  onViewOriginal?: (block: ReaderBlock, index: number) => void;
  /** Leaves reader mode at a page: a page image's "Open page n". */
  onOpenPage?: (page: number) => void;
  /** A page as people number it (its label, or its number); the 1-based number when left out. */
  nameOfPage?: (page: number) => string;
}

/** What each block needs besides itself: the approximate view's ways back to the page. */
interface BlockActions {
  readonly viewOriginal?: (block: ReaderBlock, index: number) => void;
  readonly openPage?: (page: number) => void;
  readonly nameOf: (page: number) => string;
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
      <CropText text={block.text} />
    </figure>
  );
}

/**
 * The text a crop takes in (a table's cells, a page image's text), for screen readers: the crop
 * is a picture, so without this its words would reach only those who can see it.
 */
function CropText({ text }: { text: string }): ReactNode {
  if (text.trim() === "") return null;
  return (
    <div className="paper-visually-hidden">
      {text.split("\n\n").map((part, i) => (
        // The parts never move, so their places are their keys.
        // biome-ignore lint/suspicious/noArrayIndexKey: a fixed list
        <p key={i}>{part}</p>
      ))}
    </div>
  );
}

/** A whole page shown as its image, with why and the way to open it. */
function PageImageBlock({
  block,
  actions,
  children,
}: {
  block: ReaderBlock;
  actions: BlockActions;
  children: ReactNode;
}): ReactNode {
  const page = actions.nameOf(block.page);
  const why = message(block.why === "scanned" ? "reader.mode.pageImage.scanned" : "reader.mode.pageImage.uncertain");
  return (
    <figure className="paper-reader-crop paper-reader-page-image">
      <div className="paper-reader-crop-image" role="img" aria-label={message("reader.mode.pageImage.label", { page })}>
        {children}
      </div>
      <figcaption className="paper-reader-alt">
        {why}{" "}
        {actions.openPage === undefined ? null : (
          <Button variant="secondary" onPress={() => actions.openPage?.(block.page)}>
            {message("reader.mode.openPage", { page })}
          </Button>
        )}
      </figcaption>
      <CropText text={block.text} />
    </figure>
  );
}

/** A block's "View original page", shown on hover or focus. */
function ViewOriginal({
  block,
  index,
  actions,
}: {
  block: ReaderBlock;
  index: number;
  actions: BlockActions;
}): ReactNode {
  if (actions.viewOriginal === undefined || block.kind === "page-image") return null;
  return (
    <span className="paper-reader-view-original">
      <Button
        variant="secondary"
        icon="single-page"
        aria-label={message("reader.mode.viewOriginal.label", { page: actions.nameOf(block.page) })}
        onPress={() => actions.viewOriginal?.(block, index)}
      >
        {message("reader.mode.viewOriginal")}
      </Button>
    </span>
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
function ListGroup({ group, actions }: { group: Group; actions: BlockActions }): ReactNode {
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
          <ViewOriginal block={block} index={index} actions={actions} />
        </li>
      ))}
    </ul>
  );
}

/** The column's blocks, each `data-block` its index, crops drawn by `renderCrop`. */
function Blocks({
  blocks,
  renderCrop,
  actions,
}: {
  blocks: readonly ReaderBlock[];
  renderCrop: ReaderViewProps["renderCrop"];
  actions: BlockActions;
}): ReactNode {
  return groups(blocks).map((group) =>
    group.list ? (
      <ListGroup key={group.entries[0]?.index} group={group} actions={actions} />
    ) : (
      group.entries.map(({ index, block }) => (
        <div key={index} className="paper-reader-block" data-block={index} data-kind={block.kind}>
          <OneBlock block={block} crop={() => renderCrop(block, index)} actions={actions} />
          <ViewOriginal block={block} index={index} actions={actions} />
        </div>
      ))
    ),
  );
}

/** One block that isn't a list item: its text, its crop, or its page image. */
function OneBlock({
  block,
  crop,
  actions,
}: {
  block: ReaderBlock;
  crop: () => ReactNode;
  actions: BlockActions;
}): ReactNode {
  if (block.kind === "page-image") {
    return (
      <PageImageBlock block={block} actions={actions}>
        {crop()}
      </PageImageBlock>
    );
  }
  return isCropBlock(block) ? <CropBlock block={block}>{crop()}</CropBlock> : <TextBlock block={block} />;
}

/** Placeholder lines while the blocks load. */
const LOADING = ["100%", "96%", "88%", "100%", "64%"].map((width, i) => ({ id: `line-${i}`, width }));

/** Leaves on Escape, for the column and the empty state alike. */
const escapeTo = (onBack: () => void) => (event: React.KeyboardEvent) => {
  if (event.key !== "Escape") return;
  event.stopPropagation();
  onBack();
};

/** Gives `element` the focus when it shows, so Escape reaches it and a screen reader starts there. */
function useFocusOnShow(element: React.RefObject<HTMLElement | null>): void {
  useEffect(() => {
    element.current?.focus({ preventScroll: true });
  }, [element]);
}

/** Why reader mode can't show the document, and the way back. */
function Unavailable({ onBack }: { onBack: () => void }): ReactNode {
  const root = useRef<HTMLElement>(null);
  useFocusOnShow(root);
  return (
    <section
      ref={root}
      tabIndex={-1}
      className="paper-reader-mode paper-reader-mode-empty"
      aria-label={message("reader.mode.title")}
      onKeyDown={escapeTo(onBack)}
    >
      <EmptyState
        title={message("reader.mode.unavailable.title")}
        icon="reader"
        actions={
          <Button variant="primary" onPress={onBack}>
            {message("reader.mode.backToPage")}
          </Button>
        }
      >
        {message("reader.mode.unavailable.body")}
      </EmptyState>
    </section>
  );
}

/** The approximate view's ways back to the page, from `ReaderView`'s props. */
function blockActions(
  approximate: ReaderApproximation | undefined,
  props: {
    readonly onViewOriginal: ReaderViewProps["onViewOriginal"] | undefined;
    readonly onOpenPage: ReaderViewProps["onOpenPage"] | undefined;
    readonly nameOfPage: ReaderViewProps["nameOfPage"] | undefined;
  },
): BlockActions {
  return {
    ...(approximate === undefined || props.onViewOriginal === undefined ? {} : { viewOriginal: props.onViewOriginal }),
    ...(props.onOpenPage === undefined ? {} : { openPage: props.onOpenPage }),
    nameOf: props.nameOfPage ?? ((page) => String(page + 1)),
  };
}

/** The column's head: Back to page and the source badge, then, approximate, the notice saying why. */
function ReaderHead({
  pageName,
  onBack,
  approximate,
}: {
  pageName: string;
  onBack: () => void;
  approximate: ReaderApproximation | undefined;
}): ReactNode {
  return (
    <>
      <div className="paper-reader-mode-head">
        <Button variant="secondary" icon="back" onPress={onBack}>
          {message("reader.mode.back", { page: pageName })}
        </Button>
        {approximate === undefined ? (
          <Badge tone="accent">{message("reader.mode.badge")}</Badge>
        ) : (
          <Badge tone="warning">{message("reader.mode.badge.approximate")}</Badge>
        )}
      </div>
      {approximate === undefined ? null : (
        <Notice tone="warning" title={message("reader.mode.approximate.title")}>
          {message(approximate === "untagged" ? "reader.mode.approximate.untagged" : "reader.mode.approximate.tree")}
        </Notice>
      )}
    </>
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
  onViewOriginal,
  onOpenPage,
  nameOfPage,
}: ReaderViewProps): ReactNode {
  const column = useRef<HTMLElement>(null);
  useFocusOnShow(column);
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
  if (content.status === "unavailable") return <Unavailable onBack={onBack} />;
  const approximate = content.status === "ready" ? content.approximate : undefined;
  const actions = blockActions(approximate, { onViewOriginal, onOpenPage, nameOfPage });
  return (
    <section
      ref={column}
      tabIndex={-1}
      className="paper-reader-mode"
      aria-label={message("reader.mode.title")}
      onKeyDown={escapeTo(onBack)}
    >
      <article
        className="paper-reader-column"
        aria-busy={blocks === undefined || undefined}
        {...(content.status === "ready" && content.lang !== undefined ? { lang: content.lang } : {})}
        data-line-spacing={settings.lineSpacing}
        data-line-length={settings.lineLength}
        style={{ fontSize: `${readerTextSize(settings.textSize)}px` }}
      >
        <ReaderHead pageName={pageName} onBack={onBack} approximate={approximate} />
        {blocks === undefined ? (
          <div className="paper-reader-loading">
            {LOADING.map((line) => (
              <Skeleton key={line.id} inlineSize={line.width} />
            ))}
          </div>
        ) : (
          <Blocks blocks={blocks} renderCrop={renderCrop} actions={actions} />
        )}
      </article>
    </section>
  );
}

/** Props of `ReaderSettingsPanel`. */
export interface ReaderSettingsPanelProps {
  /** The settings the panel shows. */
  settings: ReaderSettings;
  /** Called with the new settings when one control changes, or with the defaults on Reset. */
  onChange: (settings: ReaderSettings) => void;
  /** Called when the panel's close button is pressed. */
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
