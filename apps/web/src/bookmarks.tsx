// The bookmark editor in the reader (O11): the Outline tab's editable tree and the named
// destinations dialog, each action one op through the engine (`editOutline`), so each is one step
// of the edit session. Loaded with later.ts once a document is open, so none of it is in the
// startup script. "Use current view" and the new bookmarks take the place at the top of the view.

import type { ExplicitDestination, Op } from "@laminapdf/sdk";
import { Button, type KeyPlatform, useStoreState } from "@laminapdf/ui";
import {
  BookmarkEditor,
  type BookmarkItem,
  type BookmarkMove,
  type BookmarkStyle,
  bookmarkWords,
  type NamedDestinationRow,
  NamedDestinationsDialog,
  placeOf,
  type SiblingAt,
} from "@laminapdf/ui/editing";
import type { Viewer } from "@laminapdf/viewer";
import { type ReactNode, useEffect, useState, useSyncExternalStore } from "react";
import { KEYS } from "./keys.ts";
import { downThePage, pageLabel, TOP_PROBE } from "./outline.ts";
import type { OutlineEditingProps } from "./rail.tsx";
import type { Engine } from "./session.ts";

/** What `updateOutlineItem` takes. */
type UpdateArgs = Extract<Op, { op: "updateOutlineItem" }>["args"];

/** Item `id` and its parent's id (`null` at the top level), wherever it is in the tree. */
function find(items: readonly BookmarkItem[], id: string): { item: BookmarkItem; parent: string | null } | undefined {
  const place = placeOf(items, id);
  const item = place?.siblings[place.index] as BookmarkItem | undefined;
  return place === undefined || item === undefined ? undefined : { item, parent: place.parent?.id ?? null };
}

/** How many bookmarks go to each named destination. */
function uses(items: readonly BookmarkItem[], counts = new Map<string, number>()): Map<string, number> {
  for (const item of items) {
    if (item.destinationName !== undefined) {
      counts.set(item.destinationName, (counts.get(item.destinationName) ?? 0) + 1);
    }
    uses(item.children, counts);
  }
  return counts;
}

/** Props of `BookmarkPanel`. */
export interface BookmarkPanelProps {
  readonly engine: Engine;
  readonly items: readonly BookmarkItem[];
  readonly current: string | undefined;
  readonly onNavigate: (id: string) => void;
  /** How the move shortcuts are written: ⌘ on Apple, Ctrl elsewhere. */
  readonly keys: KeyPlatform;
}

/**
 * When the document was opened whose outline an edit found broken and rebuilt, by engine (O11's
 * notice): another document opened since has its own outline.
 */
const repaired = new WeakMap<Engine, number | undefined>();

/** Whether an edit rebuilt the open document's broken outline. */
const wasRepaired = (engine: Engine): boolean =>
  repaired.has(engine) && repaired.get(engine) === engine.getState().openedAt;

/**
 * Applies bookmark or named-destination `ops` to the open document as one step of the edit
 * session, named `label`, then reads the outline again. Returns the IDs of the bookmarks it added.
 */
async function editOutline(engine: Engine, ops: readonly Op[], label: string): Promise<readonly string[]> {
  const { viewer } = await engine.view();
  const result = await viewer.edits.apply(ops as Op[], label);
  if (result.outlineRepaired === true) repaired.set(engine, engine.getState().openedAt);
  await engine.loadOutline(true);
  return result.createdOutlineItems ?? [];
}

/**
 * The place at the top of the view as a destination ("use the current view"): its page, and the
 * point just under the viewport's top in page space, keeping the reader's zoom. Undefined without
 * a page in view.
 */
async function destinationHere(engine: Engine): Promise<ExplicitDestination | undefined> {
  const { viewer, client } = await engine.view();
  const top = viewer.pointAt(viewer.element.clientWidth / 2, TOP_PROBE);
  const page = top === undefined ? undefined : engine.getState().pages[top.page];
  if (top === undefined || page === undefined) return undefined;
  const clamp = (v: number) => Math.min(1, Math.max(0, v));
  const down = downThePage(clamp(top.x), clamp(top.y), viewer.getState().rotation);
  return client.call("destinationAt", [top.page, { kind: "at", y: down * page.height }]);
}

/** Goes to the named destination `name`, when the document has it. */
async function goToName(engine: Engine, name: string): Promise<void> {
  const { viewer, client } = await engine.view();
  const target = await client.namedDestination(name);
  if (target !== undefined) viewer.jumpToDestination(target);
}

/** The editor's actions, each one op: what `BookmarkEditor` calls. */
function bookmarkActions(
  items: readonly BookmarkItem[],
  apply: (ops: readonly Op[], label: string) => Promise<readonly string[]>,
  here: () => Promise<ExplicitDestination | undefined>,
) {
  const title = (id: string) => find(items, id)?.item.title ?? "";
  const update = (item: string, args: Omit<UpdateArgs, "item">, label: string) =>
    void apply([{ op: "updateOutlineItem", args: { item, ...args } }], label);
  return {
    onAdd: async (after: string | undefined) => {
      const destination = await here();
      if (destination === undefined) return;
      const parent = after === undefined ? null : (find(items, after)?.parent ?? null);
      const at: SiblingAt = after === undefined ? "last" : { after };
      const args = { parent, at, title: bookmarkWords("bookmark.new"), target: { destination } };
      await apply([{ op: "addOutlineItem", args }], bookmarkWords("bookmark.add"));
    },
    onRename: (id: string, t: string) =>
      update(id, { title: t }, bookmarkWords("bookmark.rename", { title: title(id) })),
    onMove: (move: BookmarkMove) =>
      void apply(
        [{ op: "moveOutlineItem", args: { item: move.item, parent: move.parent, at: move.at } }],
        bookmarkWords("bookmark.move", { title: title(move.item) }),
      ),
    onDelete: (id: string) =>
      void apply(
        [{ op: "deleteOutlineItem", args: { item: id, children: "delete" } }],
        bookmarkWords("bookmark.delete", { title: title(id) }),
      ),
    onStyle: (id: string, style: BookmarkStyle) =>
      update(id, { style }, bookmarkWords("bookmark.style", { title: title(id) })),
    onShowChildren: (id: string, open: boolean) =>
      update(id, { open }, bookmarkWords("bookmark.open", { title: title(id) })),
    onUseCurrentView: async (id: string) => {
      const destination = await here();
      if (destination !== undefined) {
        update(id, { target: { destination } }, bookmarkWords("bookmark.target", { title: title(id) }));
      }
    },
  };
}

/** The named destinations dialog's rows while it is open (`undefined` while closed), read again after each change. */
function useNamedDestinations(
  engine: Engine,
  items: readonly BookmarkItem[],
  apply: (ops: readonly Op[], label: string) => Promise<readonly string[]>,
) {
  const [named, setNamed] = useState<readonly NamedDestinationRow[] | undefined>(undefined);
  const read = async () => {
    const counts = uses(items);
    const pages = engine.getState().pages;
    const list = await (await engine.view()).client.call("namedDestinations", []);
    setNamed(
      list.map((d) => ({
        name: d.name,
        usedBy: counts.get(d.name) ?? 0,
        ...(d.target === undefined ? {} : { page: pageLabel(pages, d.target.page) }),
      })),
    );
  };
  // An open dialog follows the bookmarks it counts.
  useEffect(() => {
    if (named !== undefined) void read();
  }, [items]);
  // The edit reads the outline again, and the effect above the names with it: one read, with the
  // bookmarks as the edit left them.
  const change = (op: Op, label: string) => apply([op], label);
  return { named, open: () => void read(), close: () => setNamed(undefined), change };
}

/** The Outline tab's editor and its named destinations dialog. */
export function BookmarkPanel({ engine, items, current, onNavigate, keys }: BookmarkPanelProps): ReactNode {
  const { viewer } = engine.getState();
  // Edits in flight: moves wait until the outline is read again after them.
  const [inFlight, setInFlight] = useState(0);
  // Why the last change failed, until the next one.
  const [failure, setFailure] = useState<string | undefined>(undefined);
  const apply = async (ops: readonly Op[], label: string): Promise<readonly string[]> => {
    setInFlight((n) => n + 1);
    setFailure(undefined);
    try {
      return await editOutline(engine, ops, label);
    } catch (error) {
      setFailure(bookmarkWords("bookmarks.failed"));
      reportError(error);
      return [];
    } finally {
      setInFlight((n) => n - 1);
    }
  };
  const here = async () => {
    const destination = await destinationHere(engine);
    setFailure(destination === undefined ? bookmarkWords("bookmarks.noPage") : undefined);
    return destination;
  };
  const actions = bookmarkActions(items, apply, here);
  const names = useNamedDestinations(engine, items, apply);
  return (
    <>
      <BookmarkEditor
        items={items}
        {...(current === undefined ? {} : { current })}
        repaired={wasRepaired(engine)}
        readOnly={viewer?.edits.getState().blocked !== undefined}
        busy={inFlight > 0}
        {...(failure === undefined ? {} : { failure })}
        apple={keys === "mac"}
        onNavigate={onNavigate}
        {...actions}
        onAdd={(after) => void actions.onAdd(after)}
        onUseCurrentView={(id) => void actions.onUseCurrentView(id)}
        onNamedDestinations={names.open}
      />
      <NamedDestinationsDialog
        isOpen={names.named !== undefined}
        onOpenChange={(open) => {
          if (!open) names.close();
        }}
        destinations={names.named ?? []}
        onGoTo={(name) => {
          names.close();
          void goToName(engine, name);
        }}
        onRename={(from, to) =>
          void names.change(
            { op: "renameNamedDestination", args: { from, to } },
            bookmarkWords("destination.rename", { name: from }),
          )
        }
        onDelete={(name) =>
          void names.change(
            { op: "deleteNamedDestination", args: { name } },
            bookmarkWords("destination.delete", { name }),
          )
        }
        onAdd={async (name) => {
          const destination = await here();
          if (destination !== undefined) {
            await names.change(
              { op: "setNamedDestination", args: { name, destination } },
              bookmarkWords("destination.add", { name }),
            );
          }
        }}
      />
    </>
  );
}

/** A key that changes whenever the edit session makes, undoes or redoes a step. */
function useEditStep(viewer: Viewer): string {
  return useSyncExternalStore(
    (listener) => viewer.edits.subscribe(listener),
    () => {
      const { history } = viewer.edits.getState();
      return `${history.dropped}:${history.steps.length}:${history.position}`;
    },
  );
}

/**
 * The Outline tab's editing part: the read view with Edit bookmarks, or the editor with Done
 * editing; the outline reads again after every edit, undo or redo, which may change the bookmarks.
 */
export function OutlineEditing(props: OutlineEditingProps): ReactNode {
  const { engine } = props;
  // The Outline tab shows only while a view is open, so the view is there.
  const viewer = useStoreState(engine, (s) => s.viewer) as Viewer;
  const outline = useStoreState(engine, (s) => s.outline);
  const items = typeof outline === "object" ? outline.items : [];
  const [on, setOn] = useState(false);
  const step = useEditStep(viewer);
  useEffect(() => {
    void engine.loadOutline(true);
  }, [engine, step]);
  return (
    <>
      {on ? (
        <BookmarkPanel
          engine={engine}
          items={items}
          current={props.current}
          onNavigate={props.onNavigate}
          keys={KEYS}
        />
      ) : (
        props.view
      )}
      <Button size={26} variant="ghost" icon="bookmark" onPress={() => setOn(!on)}>
        {bookmarkWords(on ? "bookmarks.done" : "bookmarks.start")}
      </Button>
    </>
  );
}
