// The bookmark editor (O11) in the rail's Outline tab: the outline as an editable tree, with Add,
// Named destinations, the repair notice, and the selected bookmark's details (where it goes, Use
// current view, bold, italic, colour, whether its children show when it opens, Go to, Delete).
// It names what the user did; the app turns each into one outline op, so each is one undo step.

import { Icon } from "@laminapdf/icons/react";
import { type KeyboardEvent, type ReactNode, useRef, useState } from "react";
import { Button as AriaButton, Input, useDragAndDrop } from "react-aria-components";
import { Button, IconButton } from "../components/button.js";
import { Checkbox } from "../components/choice.js";
import { EmptyState } from "../components/empty-state.js";
import { Notice } from "../components/notice.js";
import { type Swatch, SwatchGroup } from "../components/swatches.js";
import { ToggleButton } from "../components/toggle.js";
import { laterMessage } from "../later-messages.js";
import { OutlineTree, type OutlineTreeEditing, type OutlineTreeItem } from "../reader/outline-tree.js";
import { type BookmarkMove, dropMove, keyMove, type MoveKey, placeOf } from "./bookmark-moves.js";

/** A bookmark's look, as `updateOutlineItem` takes it. */
export interface BookmarkStyle {
  readonly bold: boolean;
  readonly italic: boolean;
  /** `"#rrggbb"`, or `null` for the default. */
  readonly color: string | null;
}

/** One bookmark of the editor: a tree item, with its style and the name or address it goes to. */
export interface BookmarkItem extends OutlineTreeItem {
  /** Shown in bold, italic or a colour (`"#rrggbb"`), as the file styles it. */
  readonly bold?: boolean;
  readonly italic?: boolean;
  readonly color?: string;
  /** The named destination it goes to. */
  readonly destinationName?: string;
  /** The web address it opens. */
  readonly uri?: string;
  readonly children: readonly BookmarkItem[];
}

/** Where a bookmark goes, in words: its page, else its name or address. */
function goesTo(item: BookmarkItem): string {
  if (item.page !== undefined) return laterMessage("bookmarks.page", { page: item.page });
  if (item.destinationName !== undefined) return laterMessage("bookmarks.named.target", { name: item.destinationName });
  return item.uri ?? laterMessage("bookmarks.goesNowhere");
}

/** Props of `BookmarkEditor`. */
export interface BookmarkEditorProps {
  readonly items: readonly BookmarkItem[];
  /** The section that holds the reading position. */
  readonly current?: string;
  /** The outline was broken and the first edit rebuilt it. */
  readonly repaired?: boolean;
  /** Edits can't be made now (an encrypted file, say): the tree is read-only. */
  readonly readOnly?: boolean;
  /**
   * An edit is being applied and the tree read again: moves wait, since a move worked out from the
   * tree as it was would put the bookmark in the wrong place.
   */
  readonly busy?: boolean;
  /** Why the last change failed, shown until the next one. */
  readonly failure?: string;
  /** Whether ⌘ (Apple) or Ctrl moves bookmarks with Shift and the arrows. */
  readonly apple: boolean;
  /** Go to where bookmark `id` points. */
  readonly onNavigate: (id: string) => void;
  /** Add a bookmark for the current view, after `after` (at its level) or at the end. */
  readonly onAdd: (after: string | undefined) => void;
  /** Give bookmark `id` a new title. */
  readonly onRename: (id: string, title: string) => void;
  /** Move a bookmark (with its children) by keyboard or drag. */
  readonly onMove: (move: BookmarkMove) => void;
  /** Delete bookmark `id` and its children. */
  readonly onDelete: (id: string) => void;
  /** Set bookmark `id`'s bold, italic and colour. */
  readonly onStyle: (id: string, style: BookmarkStyle) => void;
  /** Show (`open`) or hide bookmark `id`'s children when the file opens. */
  readonly onShowChildren: (id: string, open: boolean) => void;
  /** Point bookmark `id` at the current view. */
  readonly onUseCurrentView: (id: string) => void;
  /** Open the named destinations dialog. */
  readonly onNamedDestinations: () => void;
}

type ColourId = "default" | "grey" | "red" | "blue" | "green";

/**
 * The bookmark editor's words that the app shows: the steps it adds to the edit session, by the
 * name of their label, a new bookmark's title (`bookmark.new`), and the Outline tab's Edit
 * bookmarks and Done editing (`bookmarks.start`, `bookmarks.done`).
 */
export type BookmarkStep =
  | "bookmarks.failed"
  | "bookmarks.noPage"
  | "bookmarks.start"
  | "bookmarks.done"
  | "bookmark.add"
  | "bookmark.new"
  | "bookmark.rename"
  | "bookmark.move"
  | "bookmark.delete"
  | "bookmark.style"
  | "bookmark.target"
  | "bookmark.open"
  | "destination.add"
  | "destination.rename"
  | "destination.delete";

/**
 * The words of step `step` as the History tab and Undo show it ("Rename bookmark “Results”"), or
 * of a new bookmark's title (`bookmark.new`); `values` fill in `{title}` and `{name}`.
 */
export function bookmarkWords(step: BookmarkStep, values: Readonly<Record<string, string>> = {}): string {
  return laterMessage(`edit.${step}`, values);
}

/** The colours a bookmark can take (§12.3.3 `/C`), as swatches and as the colour written. */
const COLOURS: readonly (Swatch<ColourId> & { readonly hex: string | null })[] = [
  { id: "default", label: laterMessage("bookmarks.colour.default"), color: "text", hex: null },
  { id: "grey", label: laterMessage("bookmarks.colour.grey"), color: "text-3", hex: "#6b6b6b" },
  { id: "red", label: laterMessage("bookmarks.colour.red"), color: "content-line-red", hex: "#c00000" },
  { id: "blue", label: laterMessage("bookmarks.colour.blue"), color: "content-line-blue", hex: "#0060c0" },
  { id: "green", label: laterMessage("bookmarks.colour.green"), color: "content-line-green", hex: "#007a33" },
];

/** The swatch of a bookmark's colour: the nearest that matches, else the default. */
function colourOf(color: string | undefined): ColourId {
  return COLOURS.find((c) => c.hex !== null && c.hex === color?.toLowerCase())?.id ?? "default";
}

/** The arrow keys that move a bookmark, when held with Shift and the platform's command key. */
const MOVE_KEYS = new Set<string>(["ArrowUp", "ArrowDown", "ArrowLeft", "ArrowRight"]);

/** The selected bookmark's details. */
function Details({ item, props }: { item: BookmarkItem; props: BookmarkEditorProps }): ReactNode {
  const style: BookmarkStyle = {
    bold: item.bold === true,
    italic: item.italic === true,
    color: item.color ?? null,
  };
  // While an edit is applied, the style shown may be the one before it: a change made from it
  // would undo that edit, so the controls wait.
  const still = props.readOnly === true || props.busy === true;
  const set = (change: Partial<BookmarkStyle>) => {
    if (!still) props.onStyle(item.id, { ...style, ...change });
  };
  return (
    <section className="paper-bookmark-details" aria-label={laterMessage("bookmarks.details")}>
      <p className="paper-bookmark-goes-to">
        {laterMessage("bookmarks.goesTo")} <strong>{goesTo(item)}</strong>
      </p>
      <Button size={26} onPress={() => props.onUseCurrentView(item.id)} isDisabled={still}>
        {laterMessage("bookmarks.useCurrentView")}
      </Button>
      <div className="paper-bookmark-style">
        <ToggleButton isSelected={style.bold} isDisabled={still} onChange={(bold) => set({ bold })}>
          {laterMessage("bookmarks.bold")}
        </ToggleButton>
        <ToggleButton isSelected={style.italic} isDisabled={still} onChange={(italic) => set({ italic })}>
          {laterMessage("bookmarks.italic")}
        </ToggleButton>
      </div>
      <SwatchGroup
        label={laterMessage("bookmarks.colour")}
        swatches={COLOURS}
        value={colourOf(item.color)}
        onChange={(id) => set({ color: COLOURS.find((c) => c.id === id)?.hex ?? null })}
      />
      <p className="paper-bookmark-hint">{laterMessage("bookmarks.colour.hint")}</p>
      <Checkbox
        isSelected={item.open === true}
        isDisabled={item.children.length === 0 || still}
        onChange={(open) => props.onShowChildren(item.id, open)}
      >
        {laterMessage("bookmarks.showChildren")}
      </Checkbox>
      <div className="paper-bookmark-actions">
        <Button size={26} onPress={() => props.onNavigate(item.id)}>
          {laterMessage("bookmarks.goTo")}
        </Button>
        <Button size={26} variant="danger" icon="delete" onPress={() => props.onDelete(item.id)} isDisabled={still}>
          {laterMessage("bookmarks.delete")}
        </Button>
      </div>
    </section>
  );
}

/** What a key on a selected row does: a move, a rename, a delete, or nothing (`undefined`). */
type KeyAction = { readonly move: BookmarkMove } | { readonly rename: true } | { readonly remove: true } | undefined;

/** The action of key event `e` on row `id`. */
function keyAction(e: KeyboardEvent, items: readonly BookmarkItem[], id: string, apple: boolean): KeyAction {
  const command = apple ? e.metaKey : e.ctrlKey;
  if (e.shiftKey && command && MOVE_KEYS.has(e.key)) {
    const move = keyMove(items, id, e.key as MoveKey);
    return move === undefined ? undefined : { move };
  }
  if (e.key === "F2") return { rename: true };
  if (e.key === "Delete" || (apple && e.key === "Backspace")) return { remove: true };
  return undefined;
}

/**
 * A bookmark's title as the file styles it (§12.3.3 `/C`, `/F`). The colour shows as a dot before
 * the title, not on its text: a file's colour is chosen for white paper and can't be read on the
 * dark theme.
 */
function StyledTitle({ item }: { item: BookmarkItem }): ReactNode {
  const style = {
    ...(item.bold === true ? { fontWeight: 600 } : {}),
    ...(item.italic === true ? { fontStyle: "italic" } : {}),
  };
  return (
    <span className="paper-outline-title" style={style}>
      {item.color === undefined ? null : (
        <span className="paper-bookmark-colour" style={{ backgroundColor: item.color }} aria-hidden />
      )}
      {item.title}
    </span>
  );
}

/** The rename field, in place of a row's title: Enter keeps it, Escape cancels, an empty title cancels. */
function RenameField({
  item,
  onEnd,
}: {
  item: OutlineTreeItem;
  onEnd: (title: string | undefined) => void;
}): ReactNode {
  const [title, setTitle] = useState(item.title);
  // Enter, then the blur as the field goes, both end it: only the first counts.
  const ended = useRef(false);
  const end = (value: string | undefined) => {
    if (ended.current) return;
    ended.current = true;
    onEnd(value);
  };
  const kept = () => (title.trim() === "" ? undefined : title);
  return (
    <Input
      className="paper-outline-rename"
      aria-label={laterMessage("bookmarks.rename")}
      value={title}
      autoFocus
      onChange={(e) => setTitle(e.target.value)}
      onBlur={() => end(kept())}
      onKeyDown={(e) => {
        // The tree reads letters and arrows to move between rows: in the field they type.
        e.stopPropagation();
        if (e.key === "Enter") end(kept());
        if (e.key === "Escape") end(undefined);
      }}
    />
  );
}

/** The editor's header: its title, Add and Named destinations. */
function Header({ props, selected }: { props: BookmarkEditorProps; selected: string | undefined }): ReactNode {
  return (
    <div className="paper-bookmarks-header">
      <h3 className="paper-bookmarks-title">{laterMessage("bookmarks.title")}</h3>
      <IconButton
        icon="bookmark"
        label={laterMessage("bookmarks.add")}
        isDisabled={props.readOnly === true}
        onPress={() => props.onAdd(selected)}
      />
      <IconButton icon="list-box" label={laterMessage("bookmarks.named")} onPress={props.onNamedDestinations} />
    </div>
  );
}

/** No bookmarks yet: say so, and offer to add one. */
function Empty({ props }: { props: BookmarkEditorProps }): ReactNode {
  return (
    <EmptyState
      title={laterMessage("bookmarks.none")}
      headingLevel={4}
      icon="bookmark"
      actions={
        <Button variant="primary" size={26} isDisabled={props.readOnly === true} onPress={() => props.onAdd(undefined)}>
          {laterMessage("bookmarks.add")}
        </Button>
      }
    >
      {laterMessage("bookmarks.none.hint")}
    </EmptyState>
  );
}

/** Drag and drop: a row dropped beside or onto another moves there, unless the editor is busy. */
function useBookmarkDrops(props: BookmarkEditorProps) {
  const { dragAndDropHooks } = useDragAndDrop<OutlineTreeItem>({
    getItems: (keys) => [...keys].map((key) => ({ "text/plain": String(key) })),
    onMove: (e) => {
      const [dragged] = [...e.keys];
      const move =
        dragged === undefined
          ? undefined
          : dropMove(props.items, String(dragged), String(e.target.key), e.target.dropPosition);
      if (move !== undefined) props.onMove(move);
    },
    isDisabled: props.readOnly === true || props.busy === true,
  });
  return dragAndDropHooks;
}

/** What `OutlineTree` draws in each row while editing: its title or rename field, and its grip. */
function rowParts(
  props: BookmarkEditorProps,
  renaming: string | undefined,
  setRenaming: (id: string | undefined) => void,
): Pick<OutlineTreeEditing, "row" | "title" | "after"> {
  const readOnly = props.readOnly === true;
  return {
    row: (item) => ({
      onDoubleClick: () => {
        if (!readOnly) setRenaming(item.id);
      },
    }),
    title: (item) =>
      renaming === item.id ? (
        <RenameField
          item={item}
          onEnd={(title) => {
            setRenaming(undefined);
            if (title !== undefined && title !== item.title) props.onRename(item.id, title);
          }}
        />
      ) : (
        <StyledTitle item={item as BookmarkItem} />
      ),
    after: (item) =>
      readOnly ? null : (
        <AriaButton
          slot="drag"
          className="paper-outline-grip"
          aria-label={laterMessage("bookmarks.drag", { title: item.title })}
        >
          <Icon name="drag-grip" />
        </AriaButton>
      ),
  };
}

/** The tree's editing: selection, rename in place, keys and drag and drop. */
function useEditing(props: BookmarkEditorProps): {
  editing: OutlineTreeEditing;
  selected: string | undefined;
  onKeyDownCapture: (e: KeyboardEvent) => void;
} {
  const { items, apple } = props;
  const [selected, setSelected] = useState<string | undefined>(undefined);
  const [renaming, setRenaming] = useState<string | undefined>(undefined);
  const readOnly = props.readOnly === true;
  const dragAndDropHooks = useBookmarkDrops(props);
  // The keys reach the focused row, which React Aria marks with its key. While a row is renamed,
  // its keys type into the field; while the editor is busy, a move is taken but not acted on.
  const onKeyDownCapture = (e: KeyboardEvent) => {
    const id = (e.target as Element).closest("[data-key]")?.getAttribute("data-key");
    const action =
      id === null || id === undefined || readOnly || renaming === id ? undefined : keyAction(e, items, id, apple);
    if (action === undefined || id === null || id === undefined) return;
    e.preventDefault();
    e.stopPropagation();
    if (props.busy === true) return;
    if ("move" in action) props.onMove(action.move);
    else if ("rename" in action) setRenaming(id);
    else props.onDelete(id);
  };
  const editing: OutlineTreeEditing = {
    tree: {
      selectionMode: "single",
      selectionBehavior: "replace",
      selectedKeys: selected === undefined ? [] : [selected],
      onSelectionChange: (keys) => {
        const [first] = keys === "all" ? [] : [...keys];
        setSelected(first === undefined ? undefined : String(first));
      },
      ...(readOnly ? {} : { dragAndDropHooks }),
    },
    ...rowParts(props, renaming, setRenaming),
  };
  return { editing, selected, onKeyDownCapture };
}

/** The bookmark editor: the Outline tab's content while the document can be edited. */
export function BookmarkEditor(props: BookmarkEditorProps): ReactNode {
  const { items, apple } = props;
  const { editing, selected, onKeyDownCapture } = useEditing(props);
  const place = selected === undefined ? undefined : placeOf(items, selected);
  const chosen = place?.siblings[place.index] as BookmarkItem | undefined;
  return (
    <div className="paper-bookmarks">
      <Header props={props} selected={selected} />
      {props.repaired === true ? <Notice tone="warning" title={laterMessage("bookmarks.repaired")} /> : null}
      {props.failure === undefined ? null : <Notice tone="danger" title={props.failure} />}
      {items.length === 0 ? (
        <Empty props={props} />
      ) : (
        <div onKeyDownCapture={onKeyDownCapture}>
          <OutlineTree
            items={items}
            {...(props.current === undefined ? {} : { current: props.current })}
            onNavigate={props.onNavigate}
            editing={editing}
          />
        </div>
      )}
      {items.length === 0 || props.readOnly === true ? null : (
        <p className="paper-bookmark-hint">{laterMessage(apple ? "bookmarks.keys.apple" : "bookmarks.keys.other")}</p>
      )}
      {chosen === undefined ? null : <Details item={chosen} props={props} />}
    </div>
  );
}
