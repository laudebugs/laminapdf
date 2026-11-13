// Links in the reader (Paper O12), while it has no Edit mode: the top bar's Link tool shows every
// link on the page with a dashed outline; a click selects one and a drag draws a new one. Make
// link (the selection toolbar's, or the palette's) makes one on the selected text. The link
// editor opens by the link, in a panel. A new link is written once it has somewhere to go, so a
// box that never gets a target leaves nothing behind. Loaded with later.ts once a document is
// open, so none of it is in the reader's startup script.

import type { Op, OutlineTarget, PageLink } from "@laminapdf/sdk";
import { type CommandRegistry, message, type ToastQueue } from "@laminapdf/ui";
import {
  type AddressCheck,
  LinkEditor,
  type LinkGoal,
  LinkPanel,
  type LinkWord,
  linkWords,
  mailParts,
  mailUri,
} from "@laminapdf/ui/editing";
import { externalLink, type Viewer } from "@laminapdf/viewer";
import { type ReactNode, useEffect, useRef, useState } from "react";
import { toolsOf } from "./edit-tools.tsx";
import type { Engine } from "./session.ts";

/** The event the selection toolbar's Make link dispatches on the viewer's element. */
const MAKE_LINK = "lamina-make-link";

/** A box in page space: left, top, right, bottom, in points from the page's top-left. */
type Box = readonly [number, number, number, number];

/** The link the editor shows: a new one until it has an ID. */
interface Editing {
  readonly page: number;
  readonly box: Box;
  /** A new link on text: its lines' boxes. */
  readonly lines?: readonly Box[];
  /** The linked text, for the editor's context line. */
  readonly text?: string;
  /** A new link on a box the user drew. */
  readonly drawn?: boolean;
  readonly id?: string;
  readonly goal: LinkGoal | undefined;
  readonly contents: string;
}

/** Smaller than this (in points) is a click, not a drawn box. */
const SMALLEST = 4;

/** A point of the page (fractions 0 to 1) as shown in a view turned clockwise by `rotation`. */
function turn(x: number, y: number, rotation: number): [number, number] {
  switch (rotation) {
    case 90:
      return [1 - y, x];
    case 180:
      return [1 - x, 1 - y];
    case 270:
      return [y, 1 - x];
    default:
      return [x, y];
  }
}

/** A point of the page as shown, turned back to the page's own orientation: the turn the other way. */
const unturn = (x: number, y: number, rotation: number) => turn(x, y, (360 - rotation) % 360);

/** A scheme whose address runs a script or opens a file, rather than a page. */
const SCRIPT_SCHEME = /^\s*(javascript|vbscript|data|file|blob):/i;

/**
 * What the reader makes of an address typed for a link, by the rule its links follow when they
 * open (`externalLink`): the address as it will open, a host in punycode. A refused address is a
 * script when its scheme runs one, else just not an address a link can hold.
 */
function checkAddress(address: string): AddressCheck {
  const link = externalLink(address);
  if (link.kind === "open") return { ok: true, address: link.href };
  return { ok: false, reason: SCRIPT_SCHEME.test(address) ? "script" : "invalid" };
}

/** Whether a link read from the file would run a script, or open something the reader never opens. */
const runsScript = (link: PageLink) =>
  link.kind === "unsupported" || (link.kind === "uri" && externalLink(link.uri).kind === "refused");

/** The goal a link read from the file has, for the editor. */
function goalOf(link: PageLink): LinkGoal | undefined {
  switch (link.kind) {
    case "dest":
      return link.target === undefined ? undefined : { kind: "page", page: link.target.page };
    case "named":
      return { kind: "named", name: link.name };
    case "uri":
      if (runsScript(link)) return { kind: "script", action: link.uri };
      return /^mailto:/i.test(link.uri)
        ? { kind: "email", ...mailParts(link.uri) }
        : { kind: "web", address: link.uri };
    case "unsupported":
      return { kind: "script", action: link.action };
    default:
      return undefined;
  }
}

/** The page outlines of the Link tool: each mounted page's links, dashed, the selected one solid. */
class LinkBoxes {
  readonly #viewer: Viewer;
  readonly #engine: Engine;
  readonly #links = new Map<number, readonly PageLink[]>();
  readonly #slots = new Map<number, HTMLElement>();
  /** The link selected, or the box being drawn: drawn solid. */
  marked: { readonly page: number; readonly box: Box; readonly id?: string } | undefined;
  /** Whether the file's links show (the Link tool is on); the marked one always does. */
  showLinks = false;

  constructor(viewer: Viewer, engine: Engine) {
    this.#viewer = viewer;
    this.#engine = engine;
  }

  readonly layer = {
    name: "link-edit",
    onMount: (page: number, slot: HTMLElement) => {
      this.#slots.set(page, slot);
      void this.#load(page);
    },
    onUnmount: (page: number, slot: HTMLElement) => {
      this.#slots.delete(page);
      slot.replaceChildren();
    },
  };

  /** Page `page`'s links, as last read. */
  links(page: number): readonly PageLink[] {
    return this.#links.get(page) ?? [];
  }

  /** The link under page-space point (`x`, `y`) of `page`, the last drawn first. */
  linkAt(page: number, x: number, y: number): PageLink | undefined {
    const hits = this.links(page).filter(({ rect: [x0, y0, x1, y1] }) => x >= x0 && x <= x1 && y >= y0 && y <= y1);
    return hits[hits.length - 1];
  }

  /** Called with each page once its links are read again. */
  onLoaded: ((page: number) => void) | undefined;

  /** Reads the mounted pages' links again (an edit, undo or redo may have changed them). */
  reload(): void {
    for (const page of this.#slots.keys()) void this.#load(page);
  }

  draw(page: number): void {
    const slot = this.#slots.get(page);
    const size = this.#engine.getState().pages[page];
    if (slot === undefined || size === undefined) return;
    const rotation = this.#viewer.getState().rotation;
    const box = ([x0, y0, x1, y1]: Box, flags: { selected?: boolean; script?: boolean }) => {
      const [ax, ay] = turn(x0 / size.width, y0 / size.height, rotation);
      const [bx, by] = turn(x1 / size.width, y1 / size.height, rotation);
      const el = document.createElement("div");
      el.className = "paper-link-box";
      if (flags.selected === true) el.dataset.selected = "";
      if (flags.script === true) el.dataset.script = "";
      Object.assign(el.style, {
        left: `${Math.min(ax, bx) * 100}%`,
        top: `${Math.min(ay, by) * 100}%`,
        width: `${Math.abs(bx - ax) * 100}%`,
        height: `${Math.abs(by - ay) * 100}%`,
      });
      return el;
    };
    const marked = this.marked?.page === page ? this.marked : undefined;
    slot.replaceChildren(
      ...(this.showLinks ? this.links(page) : [])
        .filter((link) => marked?.id === undefined || link.id !== marked.id)
        .map((link) => box(link.rect, { script: runsScript(link) })),
      ...(marked === undefined ? [] : [box(marked.box, { selected: true })]),
    );
  }

  /** Where `box` of `page` is in the viewport, or null while the page isn't mounted. */
  viewportBox(
    page: number,
    [x0, y0, x1, y1]: Box,
  ): { top: number; left: number; width: number; height: number } | null {
    const slot = this.#slots.get(page);
    const size = this.#engine.getState().pages[page];
    if (slot === undefined || size === undefined) return null;
    const r = slot.getBoundingClientRect();
    const rotation = this.#viewer.getState().rotation;
    const [ax, ay] = turn(x0 / size.width, y0 / size.height, rotation);
    const [bx, by] = turn(x1 / size.width, y1 / size.height, rotation);
    return {
      left: r.left + Math.min(ax, bx) * r.width,
      top: r.top + Math.min(ay, by) * r.height,
      width: Math.abs(bx - ax) * r.width,
      height: Math.abs(by - ay) * r.height,
    };
  }

  async #load(page: number): Promise<void> {
    const { client } = await this.#engine.view();
    const links = await client.pageLinks(page).catch(() => undefined);
    // A failed read keeps what was shown: it says nothing about which links went away.
    if (links === undefined) return;
    this.#links.set(page, links);
    this.draw(page);
    this.onLoaded?.(page);
  }
}

/** The page-space point under the pointer, with its page. */
function pointOn(viewer: Viewer, engine: Engine, event: PointerEvent) {
  const r = viewer.element.getBoundingClientRect();
  const at = viewer.pointAt(event.clientX - r.left, event.clientY - r.top);
  const size = at === undefined ? undefined : engine.getState().pages[at.page];
  if (at === undefined || size === undefined) return undefined;
  const [x, y] = unturn(at.x, at.y, viewer.getState().rotation);
  return { page: at.page, x: x * size.width, y: y * size.height };
}

/** The selected text's lines on its first page, in page space. */
function selectedLines(viewer: Viewer, engine: Engine, rects: readonly DOMRect[]) {
  const r = viewer.element.getBoundingClientRect();
  const rotation = viewer.getState().rotation;
  const lines: Box[] = [];
  let page: number | undefined;
  for (const rect of rects) {
    const a = viewer.pointAt(rect.left - r.left, rect.top - r.top);
    const b = viewer.pointAt(rect.right - r.left, rect.bottom - r.top);
    if (a === undefined || b === undefined || a.page !== b.page) continue;
    page ??= a.page;
    const size = engine.getState().pages[a.page];
    if (a.page !== page || size === undefined) continue;
    const [ax, ay] = unturn(a.x, a.y, rotation);
    const [bx, by] = unturn(b.x, b.y, rotation);
    const box: Box = [
      Math.min(ax, bx) * size.width,
      Math.min(ay, by) * size.height,
      Math.max(ax, bx) * size.width,
      Math.max(ay, by) * size.height,
    ];
    // A line the browser split into pieces is one line: join a piece that shares its band.
    const same = lines.findIndex((l) => Math.abs((l[1] + l[3]) / 2 - (box[1] + box[3]) / 2) < (box[3] - box[1]) / 2);
    const line = lines[same];
    if (line === undefined) lines.push(box);
    else
      lines[same] = [
        Math.min(line[0], box[0]),
        Math.min(line[1], box[1]),
        Math.max(line[2], box[2]),
        Math.max(line[3], box[3]),
      ];
  }
  if (page === undefined || lines.length === 0) return undefined;
  const bounds: Box = [
    Math.min(...lines.map((l) => l[0])),
    Math.min(...lines.map((l) => l[1])),
    Math.max(...lines.map((l) => l[2])),
    Math.max(...lines.map((l) => l[3])),
  ];
  return { page, box: bounds, lines };
}

/** Props of `LinkEditing`. */
export interface LinkEditingProps {
  readonly engine: Engine;
  readonly viewer: Viewer;
  readonly registry: CommandRegistry;
  readonly toasts: ToastQueue;
}

/** The Link tool, Make link and the link editor for the document `viewer` shows. */
export function LinkEditing({ engine, viewer, registry, toasts }: LinkEditingProps): ReactNode {
  const [editing, setEditing] = useState<Editing>();
  const [names, setNames] = useState<readonly string[]>([]);
  const [boxes] = useState(() => new LinkBoxes(viewer, engine));
  const current = useRef(editing);
  current.current = editing;
  // The editor's edits run one at a time, so a second goal waits for the link the first one adds.
  const queue = useRef(Promise.resolve());
  useLinkTool(viewer, engine, boxes, setEditing);
  useMakeLink(viewer, engine, registry, setEditing);
  // The outlines follow the selection, and every edit, undo or redo reads the links again.
  useEffect(() => {
    boxes.marked = editing === undefined ? undefined : { page: editing.page, box: editing.box, ...idOf(editing) };
    boxes.reload();
  }, [boxes, editing]);
  useEffect(() => viewer.edits.subscribe(() => boxes.reload()), [viewer, boxes]);
  // An undo that took the link away closes its editor.
  useEffect(() => {
    boxes.onLoaded = (page) => {
      const at = current.current;
      if (at?.id !== undefined && at.page === page && !boxes.links(page).some((l) => l.id === at.id)) {
        setEditing(undefined);
      }
    };
    return () => {
      boxes.onLoaded = undefined;
    };
  }, [boxes]);
  useEffect(() => {
    if (editing === undefined) return;
    void engine
      .view()
      .then(({ client }) => client.call("namedDestinations", []))
      .then((list) => setNames(list.map((d) => d.name)))
      .catch(() => setNames([]));
  }, [viewer, editing?.page, editing?.id, editing]);
  if (editing === undefined) return null;

  const { onGoal, onContents, onDelete, onClose } = linkActions(engine, viewer, toasts, { current, queue }, setEditing);
  const onFollow = followOf(engine, viewer, editing);
  return (
    <LinkPanel anchor={() => boxes.viewportBox(editing.page, editing.box)} onDismiss={onClose}>
      <LinkEditor
        key={editing.id ?? editing.box.join()}
        {...(editing.text === undefined ? {} : { text: editing.text })}
        drawn={editing.drawn === true}
        goal={editing.goal}
        pageCount={engine.getState().pages.length}
        names={names}
        contents={editing.contents}
        onGoal={onGoal}
        onContents={onContents}
        onDelete={onDelete}
        checkAddress={checkAddress}
        {...(onFollow === undefined ? {} : { onFollow })}
      />
    </LinkPanel>
  );
}

/** Where `goal` goes, as the link ops take it; a script goal is never written. */
async function targetOf(engine: Engine, goal: LinkGoal): Promise<OutlineTarget | undefined> {
  switch (goal.kind) {
    case "page":
      return { destination: await (await engine.view()).client.call("destinationAt", [goal.page, { kind: "fit" }]) };
    case "named":
      return { named: goal.name };
    case "web":
      return { uri: goal.address };
    case "email":
      return { uri: mailUri(goal.address, goal.subject) };
    default:
      return undefined;
  }
}

/**
 * The editor's changes as edits, each one step and one at a time: a new link is added on its
 * first goal, and later goals, descriptions and Delete link change or remove it. A failure is a
 * toast.
 */
function linkActions(
  engine: Engine,
  viewer: Viewer,
  toasts: ToastQueue,
  refs: { readonly current: { current: Editing | undefined }; readonly queue: { current: Promise<void> } },
  setEditing: (editing: Editing | undefined) => void,
) {
  const { current, queue } = refs;
  const fail = (error: unknown) => {
    const reason = error instanceof Error ? error.message : String(error);
    toasts.add({ message: linkWords("link.failed", { reason }) });
  };
  const next = (work: () => Promise<unknown>) => {
    queue.current = queue.current.then(work).then(() => undefined, fail);
  };
  // Seen at once by the next queued edit, before React renders the change.
  const show = (editing: Editing | undefined) => {
    current.current = editing;
    setEditing(editing);
  };
  const apply = (ops: Op[], label: LinkWord) => viewer.edits.apply(ops, linkWords(label));
  const add = async (at: Editing, goal: LinkGoal, to: OutlineTarget) => {
    const { client } = await engine.view();
    const args =
      at.lines === undefined
        ? await client.call("linkOnArea", [at.page, [...at.box] as [number, number, number, number], to])
        : await client.call("linkOnText", [
            at.page,
            at.lines.map((l) => [...l] as [number, number, number, number]),
            to,
          ]);
    const made = await apply(
      [{ op: "addLink", args: at.contents === "" ? args : { ...args, contents: at.contents } }],
      "edit.link.add",
    );
    const id = made.createdAnnotations?.[0];
    show({ ...at, goal, ...(id === undefined ? {} : { id }) });
  };
  const onGoal = (goal: LinkGoal) =>
    next(async () => {
      const at = current.current;
      const to = await targetOf(engine, goal);
      if (at === undefined || to === undefined) return;
      if (at.id === undefined) return add(at, goal, to);
      await apply([{ op: "updateLink", args: { link: at.id, target: to } }], "edit.link.target");
      show({ ...at, goal });
    });
  const onContents = (contents: string) =>
    next(async () => {
      const at = current.current;
      if (at === undefined) return;
      show({ ...at, contents });
      if (at.id === undefined) return;
      await apply(
        [{ op: "updateLink", args: { link: at.id, contents: contents === "" ? null : contents } }],
        "edit.link.description",
      );
    });
  const onDelete = () =>
    next(async () => {
      const at = current.current;
      show(undefined);
      if (at?.id !== undefined) await apply([{ op: "deleteLink", args: { link: at.id } }], "edit.link.delete");
    });
  // Closing waits for the edits queued before it (a field's commit on blur among them), so none is
  // lost and none reopens the panel.
  const onClose = () => next(async () => show(undefined));
  return { onGoal, onContents, onDelete, onClose };
}

const idOf = (editing: Editing) => (editing.id === undefined ? {} : { id: editing.id });

/** Following the link the editor shows: a page or a name in the viewer, an address after the reader confirms it. */
function followOf(engine: Engine, viewer: Viewer, editing: Editing): (() => void) | undefined {
  const { goal } = editing;
  if (editing.id === undefined || goal === undefined) return undefined;
  switch (goal.kind) {
    case "page":
      return () => viewer.jumpToPage(goal.page);
    case "named":
      return () =>
        void engine
          .view()
          .then(({ client }) => client.namedDestination(goal.name))
          .then((to) => to !== undefined && viewer.jumpToDestination(to));
    case "web":
    case "email": {
      const link = externalLink(goal.kind === "web" ? goal.address : mailUri(goal.address, goal.subject));
      return link.kind === "open" ? () => engine.confirmExternalLink(link) : undefined;
    }
    default:
      return undefined;
  }
}

/** The Link tool: a click on a link selects it, a drag elsewhere draws a new one. */
function useLinkTool(
  viewer: Viewer,
  engine: Engine,
  boxes: LinkBoxes,
  select: (editing: Editing | undefined) => void,
): void {
  useEffect(() => {
    const tools = toolsOf(viewer.edits);
    let drawing: { page: number; x: number; y: number } | undefined;
    // The file's links show while the tool is on; the layer stays, for the link being edited.
    const off = viewer.addLayer(boxes.layer);
    const follow = () => {
      const on = tools.getState().current === "link";
      if (on === boxes.showLinks) return;
      boxes.showLinks = on;
      boxes.reload();
    };
    const unregister = tools.register({
      name: "link",
      cursor: "crosshair",
      selectsText: false,
      pointerDown: (event) => {
        const at = pointOn(viewer, engine, event);
        if (at === undefined || event.button !== 0) return false;
        const hit = boxes.linkAt(at.page, at.x, at.y);
        if (hit?.id !== undefined) {
          select({ page: at.page, box: hit.rect, id: hit.id, goal: goalOf(hit), contents: hit.contents ?? "" });
          return true;
        }
        drawing = at;
        return true;
      },
      pointerMove: (event) => {
        const at = pointOn(viewer, engine, event);
        if (drawing === undefined || at === undefined || at.page !== drawing.page) return;
        const box: Box = [
          Math.min(drawing.x, at.x),
          Math.min(drawing.y, at.y),
          Math.max(drawing.x, at.x),
          Math.max(drawing.y, at.y),
        ];
        boxes.marked = { page: drawing.page, box };
        boxes.draw(drawing.page);
      },
      pointerUp: () => {
        const box = boxes.marked?.page === drawing?.page && boxes.marked?.id === undefined ? boxes.marked : undefined;
        if (drawing !== undefined && box !== undefined) {
          const [x0, y0, x1, y1] = box.box;
          if (x1 - x0 >= SMALLEST && y1 - y0 >= SMALLEST) {
            select({ page: drawing.page, box: box.box, drawn: true, goal: undefined, contents: "" });
          }
        }
        drawing = undefined;
      },
    });
    follow();
    const stop = tools.subscribe(follow);
    return () => {
      stop();
      unregister();
      off();
    };
  }, [viewer, engine, boxes, select]);
}

/** Make link: on the selected text, from the selection toolbar's button and the palette. */
function useMakeLink(
  viewer: Viewer,
  engine: Engine,
  registry: CommandRegistry,
  select: (editing: Editing | undefined) => void,
): void {
  useEffect(() => {
    let selection: { rects: readonly DOMRect[]; text: string } | undefined;
    const stopSelection = viewer.on("selectionchange", ({ selection: next }) => {
      selection = next === undefined ? undefined : { rects: next.rects, text: next.text };
    });
    const make = () => {
      if (selection === undefined) return;
      const on = selectedLines(viewer, engine, selection.rects);
      if (on === undefined) return;
      // The linked text is the link's description until it gets another: PDF/UA asks every link for one.
      const text = selection.text.replace(/\s+/g, " ").trim();
      select({ ...on, text, goal: undefined, contents: text });
    };
    viewer.element.addEventListener(MAKE_LINK, make);
    const stopCommand = registry.register({
      id: "edit.make-link",
      title: message("command.edit.make-link"),
      keywords: ["hyperlink", "url"],
      group: "edit",
      icon: "link",
      home: ["mini-toolbar"],
      enabled: () => (selection === undefined ? linkWords("link.toolHelp") : true),
      run: make,
    });
    return () => {
      stopSelection();
      stopCommand();
      viewer.element.removeEventListener(MAKE_LINK, make);
    };
  }, [viewer, engine, registry, select]);
}
