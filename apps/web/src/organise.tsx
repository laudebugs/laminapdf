// The Organise mode in the reader (Paper O06, E2h): the page grid over the viewer while the mode is
// current, its toolbar under the top bar, its inspector beside the grid, the page commands, and the
// dialogs. Loaded with later.ts once a document is open (#380). Each action is one step of the edit
// session; before one applies, the engine asks what its save would break (`wouldViolate`), so a
// change a certification forbids, or one to a page that holds a signature's box, is asked about
// first and changes nothing when declined. A certification's refusal offers Save a copy (the change
// written to a copy without the certification, the open file left as it was), Apply anyway… and
// Details (what the signature allows). Moves, deletes, duplicates and inserts follow the pages
// by ID (E2l), so the grid's selection does too.

import type { Op, Violation } from "@laminapdf/sdk";
import {
  Button,
  type CommandRegistry,
  type ExternalStore,
  IconButton,
  type KeyPlatform,
  type MenuEntry,
  type ModeRegistry,
  ModeSwitcher,
  type Platform,
  type ToastQueue,
  useStoreState,
} from "@laminapdf/ui";
import {
  BlankPagesDialog,
  type BlankSize,
  type BlankWhere,
  deletedMessage,
  duplicatedMessage,
  type GridCommand,
  type GridPage,
  insertedMessage,
  insertLabel,
  MoveToPageDialog,
  movedMessage,
  moveToGap,
  OrganiseToolbar,
  laterMessage as organiseWords,
  PageGrid,
  pageRanges,
  type RestrictionKind,
  registerOrganiseCommands,
  type SaveSignature,
  SignatureDetails,
  type SignatureRestriction,
  SignedPageDialog,
  selectionNote,
  stepLabel,
} from "@laminapdf/ui/editing";
import type { EditResult, Viewer } from "@laminapdf/viewer";
import { type ReactNode, useEffect, useMemo } from "react";
import { DocumentsController, DocumentsDialogs, documentsActions } from "./documents.tsx";
import type { PageInfo } from "./outline.ts";
import { PageSetupDialogs, pageSetupActions } from "./page-setup.tsx";
import { type SaveFlow, signature } from "./save-flow.ts";
import type { Engine } from "./session.ts";

/** A question before an edit applies: about a signature on the page, or the certification. */
type Ask =
  | {
      readonly kind: "signed";
      readonly action: "delete" | "rotate";
      readonly page: string;
      readonly proceed: () => void;
    }
  | {
      /** A certification or usage rights: no page change is permitted (E2 plan §6). */
      readonly kind: "noPageChanges";
      /** What the change would invalidate. */
      readonly breaks: "certification" | "usageRights";
      /** The refused change's ops, which Save a copy writes to the copy. */
      readonly ops: readonly object[];
      /** What its save would break: Details names them, and the copy's confirmation too. */
      readonly violations: readonly Violation[];
      readonly proceed: () => void;
    };

/** The dialog open over the grid: Organise's own, or one of Page setup's (page-setup.tsx). */
export type OrganiseDialog = "move" | "blank" | "crop" | "resize" | "boxes" | "labels";

/** What the grid, its toolbar and its dialogs show. */
export interface OrganiseState {
  /** The selected pages' IDs. */
  readonly selected: ReadonlySet<string>;
  /** Thumbnail size, 0.75–2. */
  readonly scale: number;
  readonly dialog: OrganiseDialog | undefined;
  readonly ask: Ask | undefined;
  /** Each page's drawing version: bumped when an edit redrew it, so its tile draws again. */
  readonly drawn: ReadonlyMap<string, number>;
  /** What the last move said ("Moved pages 5–6 after page 9."), for the live region. */
  readonly announced: string;
}

/** Page sizes in points for Blank pages…: US Letter and A4. */
const SIZES = { letter: { width: 612, height: 792 }, a4: { width: 595.28, height: 841.89 } } as const;

/** The Organise mode's state and actions over one open document. */
export class OrganiseController implements ExternalStore<OrganiseState> {
  readonly #engine: Engine;
  readonly #viewer: Viewer;
  readonly #toasts: ToastQueue;
  /** Saving's copies, for the certification banner's Save a copy. */
  readonly #copies: Pick<SaveFlow, "copyWith" | "copyTitle">;
  #state: OrganiseState = {
    selected: new Set(),
    scale: 1,
    dialog: undefined,
    ask: undefined,
    drawn: new Map(),
    announced: "",
  };
  readonly #listeners = new Set<() => void>();
  /** Insert from a file, Replace, Merge, Split and Extract (documents.tsx). */
  readonly documents: DocumentsController;

  constructor(
    engine: Engine,
    viewer: Viewer,
    toasts: ToastQueue,
    platform: Platform,
    copies: Pick<SaveFlow, "copyWith" | "copyTitle">,
  ) {
    this.#engine = engine;
    this.#viewer = viewer;
    this.#toasts = toasts;
    this.#copies = copies;
    this.documents = new DocumentsController({ engine, organise: this, toasts, platform });
    viewer.on("invalidate", ({ pages }) => {
      const ids = this.pages();
      const drawn = new Map(this.#state.drawn);
      for (const { index } of pages) {
        const id = ids[index]?.id;
        if (id !== undefined) drawn.set(id, (drawn.get(id) ?? 0) + 1);
      }
      this.#set({ drawn });
    });
  }

  getState(): OrganiseState {
    return this.#state;
  }

  subscribe(listener: () => void): () => void {
    this.#listeners.add(listener);
    return () => this.#listeners.delete(listener);
  }

  /** The open document's pages, in order. */
  pages(): readonly PageInfo[] {
    return this.#engine.getState().pages;
  }

  /** The selected pages' indices, in order. */
  selectedIndices(): number[] {
    return indicesOf(this.pages(), this.#state.selected);
  }

  select(ids: ReadonlySet<string>): void {
    this.#set({ selected: new Set(ids) });
  }

  selectAll(): void {
    this.select(new Set(this.pages().map((p) => p.id)));
  }

  setScale(scale: number): void {
    this.#set({ scale });
  }

  open(dialog: OrganiseDialog | undefined): void {
    this.#set({ dialog });
  }

  /** Says `message` in a toast, as a refusal that changed nothing. */
  say(message: string): void {
    this.#toasts.add({ message });
  }

  /**
   * Applies `ops` to the pages at `indices` as one step named `label`, asked about first when
   * their save would break a signature; `report` gives the toast's words.
   */
  edit(
    ops: readonly object[],
    indices: readonly number[],
    label: string,
    report?: (result: EditResult) => string | undefined,
  ): void {
    this.#guarded(ops, undefined, indices, () => this.#apply(ops, label, report));
  }

  /** The person's answer to the question before an edit. */
  answer(proceed: boolean): void {
    const ask = this.#state.ask;
    this.#set({ ask: undefined });
    if (proceed) ask?.proceed();
  }

  /**
   * Save a copy on the certification's banner: the refused change written to a copy without the
   * signatures it breaks, through saving's confirmation. The banner goes once the copy is written;
   * the document stays as it was.
   */
  saveCopy(): void {
    const ask = this.#state.ask;
    if (ask?.kind !== "noPageChanges") return;
    void this.#copies.copyWith(ask.ops as unknown as Op[], brokenSignatures(ask.violations)).then((written) => {
      if (written && this.#state.ask === ask) this.#set({ ask: undefined });
    });
  }

  /** Save a copy's name: "Save a copy…", or "Download a copy" where the browser downloads it. */
  copyTitle(): string {
    return this.#copies.copyTitle();
  }

  /** Turns the selection a quarter (`delta` 90 clockwise, -90 the other way). */
  rotate(delta: 90 | -90): void {
    const indices = this.selectedIndices();
    if (indices.length === 0) return;
    const op = { op: "rotatePages", args: { pages: this.#ids(indices), angle: delta, mode: "turn" } };
    this.#guarded([op], "rotate", indices, () =>
      this.#apply([op], stepLabel("rotate", this.#names(indices), indices.length)),
    );
  }

  /** Deletes the selection, unless it is every page. */
  delete(): void {
    const indices = this.selectedIndices();
    if (indices.length === 0) return;
    if (indices.length >= this.pages().length) {
      this.#toasts.add({ message: organiseWords("organise.lastPage") });
      return;
    }
    const op = { op: "deletePages", args: { pages: this.#ids(indices), references: "retarget" } };
    this.#guarded([op], "delete", indices, () =>
      this.#apply([op], stepLabel("delete", this.#names(indices), indices.length), (result) => {
        this.#set({ selected: new Set() });
        return deletedMessage(indices.length, result.references);
      }),
    );
  }

  /** Duplicates the selection, each copy after its page. */
  duplicate(): void {
    const indices = this.selectedIndices();
    if (indices.length === 0) return;
    const last = this.pages()[indices[indices.length - 1] as number];
    if (last === undefined) return;
    const op = { op: "duplicatePages", args: { pages: this.#ids(indices), at: { after: last.id } } };
    this.#guarded([op], undefined, indices, () =>
      this.#apply([op], stepLabel("duplicate", this.#names(indices), indices.length), (result) =>
        duplicatedMessage(indices.length, result.fields),
      ),
    );
  }

  /** Reverses the order of the selection, or of every page when nothing is selected. */
  reverse(): void {
    const chosen = this.selectedIndices();
    // One page reversed stays where it is.
    if (chosen.length === 1) return;
    const indices = chosen.length > 0 ? chosen : this.pages().map((_, i) => i);
    const op = { op: "reversePages", args: { pages: this.#ids(indices) } };
    this.#guarded([op], undefined, indices, () =>
      this.#apply([op], stepLabel("reverse", this.#names(indices), indices.length)),
    );
  }

  /** Moves the pages at `indices` (the selection, for a drag or a key) to gap `gap`. */
  move(indices: readonly number[], gap: number): void {
    const pages = this.pages();
    const order = pages.map((p) => p.id);
    const move = moveToGap(order, indices, gap);
    if (move === undefined) return;
    const op = { op: "movePages", args: { pages: move.pages, to: move.to } };
    // A page keeps its label through a move (PageLabels::remap); one without is named by its new place.
    const labelOf = (id: string) => pages.find((p) => p.id === id)?.label ?? String(move.order.indexOf(id) + 1);
    this.#guarded([op], undefined, indices, () =>
      this.#apply([op], stepLabel("move", this.#names(indices), indices.length), () => {
        this.#set({ selected: new Set(move.pages), announced: movedMessage(move.moved, move.order, labelOf) });
        return undefined;
      }),
    );
  }

  /** Inserts `count` blank pages of `size` beside the selection or at the end. */
  insertBlank(count: number, size: BlankSize, where: BlankWhere): void {
    const pages = this.pages();
    const indices = this.selectedIndices();
    const first = pages[indices[0] ?? -1];
    const last = pages[indices[indices.length - 1] ?? -1];
    const at =
      where === "before" && first !== undefined
        ? { before: first.id }
        : where === "after" && last !== undefined
          ? { after: last.id }
          : "end";
    const like = first ?? pages[0];
    const page =
      size === "match" && like !== undefined
        ? { width: like.width, height: like.height }
        : SIZES[size === "match" ? "letter" : size];
    const op = { op: "insertBlankPages", args: { at, count, size: page, rotate: 0 } };
    const before = new Set(pages.map((p) => p.id));
    this.#guarded([op], undefined, indices, () =>
      this.#apply([op], insertLabel(count), () => {
        this.#set({ selected: new Set(this.pages().flatMap((p) => (before.has(p.id) ? [] : [p.id]))) });
        return insertedMessage(count);
      }),
    );
  }

  /** Inserts one blank page before or after the page at `index`, its size. */
  insertBeside(index: number, where: "before" | "after"): void {
    const page = this.pages()[index];
    if (page === undefined) return;
    this.select(new Set([page.id]));
    this.insertBlank(1, "match", where);
  }

  #ids(indices: readonly number[]): string[] {
    const pages = this.pages();
    return indices.flatMap((i) => (pages[i] === undefined ? [] : [(pages[i] as PageInfo).id]));
  }

  #names(indices: readonly number[]): string {
    const pages = this.pages();
    return pageRanges(indices, (i) => pages[i]?.label ?? String(i + 1));
  }

  /**
   * Runs `go` unless saving `ops` would break a signature: then asks first, about the
   * certification or the signature whose box is on one of the pages (O06's states).
   */
  #guarded(
    ops: readonly object[],
    action: "delete" | "rotate" | undefined,
    indices: readonly number[],
    go: () => void,
  ): void {
    void this.#engine
      .wouldViolate(ops as unknown as Op[])
      .then((violations) => {
        const ask = askFor(violations, ops, action, this.#names(indices.slice(0, 1)), go);
        if (ask === undefined) go();
        else this.#set({ ask });
      })
      .catch(() => go());
  }

  /** Applies `ops` as one step named `label`; `report` gives the toast's words, offered with Undo. */
  #apply(ops: readonly object[], label: string, report?: (result: EditResult) => string | undefined): void {
    void this.#viewer.edits.apply(ops as unknown as Op[], label).then(
      (result) => {
        const text = report?.(result);
        if (text === undefined) return;
        this.#toasts.add({
          message: text,
          action: { label: organiseWords("organise.undo"), onPress: () => void this.#viewer.edits.undo() },
        });
      },
      // A failed edit is said by the session's failure toast (edit-toasts.ts).
      () => undefined,
    );
  }

  #set(change: Partial<OrganiseState>): void {
    this.#state = { ...this.#state, ...change };
    for (const listener of [...this.#listeners]) listener();
  }
}

/** The question `violations` ask before an edit, or undefined when its save would break nothing. */
function askFor(
  violations: readonly Violation[],
  ops: readonly object[],
  action: "delete" | "rotate" | undefined,
  page: string,
  proceed: () => void,
): Ask | undefined {
  if (violations.length === 0) return undefined;
  if (violations.some((v) => v.restriction === "docMdp"))
    return { kind: "noPageChanges", breaks: "certification", ops, violations, proceed };
  if (violations.some((v) => v.restriction === "usageRights"))
    return { kind: "noPageChanges", breaks: "usageRights", ops, violations, proceed };
  return { kind: "signed", action: action ?? "delete", page, proceed };
}

/** `violations` mapped by `map`, each key `keyOf` gives once, in order. */
function onceEach<T>(violations: readonly Violation[], keyOf: (v: Violation) => string, map: (v: Violation) => T): T[] {
  const seen = new Set<string>();
  return violations.flatMap((v) => {
    const key = keyOf(v);
    if (seen.has(key)) return [];
    seen.add(key);
    return [map(v)];
  });
}

/** A signature's identity: its kind and field. */
const signatureKey = ({ kind, field }: Violation["signature"]) => `${kind}:${field ?? ""}`;

/** Each signature `violations` break, once. */
function brokenSignatures(violations: readonly Violation[]): SaveSignature[] {
  return onceEach(
    violations,
    (v) => signatureKey(v.signature),
    (v) => signature(v.signature),
  );
}

/** Each restriction `violations` break, once, for Details. */
function restrictionsOf(violations: readonly Violation[]): SignatureRestriction[] {
  return onceEach(
    violations,
    (v) => `${v.restriction}:${signatureKey(v.signature)}`,
    ({ restriction, permission, signature: { revision } }) => ({
      // The WASM type says string; the planner names only these five.
      restriction: restriction as RestrictionKind,
      ...(permission === undefined ? {} : { permission }),
      ...(revision === undefined ? {} : { revision }),
    }),
  );
}

/** The indices of the pages in `selected`, in document order. */
function indicesOf(pages: readonly PageInfo[], selected: ReadonlySet<string>): number[] {
  return pages.flatMap((p, i) => (selected.has(p.id) ? [i] : []));
}

/** The handlers the page commands run while Organise is the mode. */
export function organiseActions(controller: OrganiseController) {
  return {
    "pages.rotate-right": () => controller.rotate(90),
    "pages.rotate-left": () => controller.rotate(-90),
    "pages.delete": () => controller.delete(),
    "pages.duplicate": () => controller.duplicate(),
    "pages.reverse": () => controller.reverse(),
    "pages.move-to": () => controller.open("move"),
    "pages.insert-blank": () => controller.open("blank"),
    "pages.select-all": () => controller.selectAll(),
    ...pageSetupActions(controller),
    ...documentsActions(controller.documents),
  } as const;
}

/** Registers the Organise mode and its commands for the document `viewer` shows. */
function useOrganiseMode(
  controller: OrganiseController,
  engine: Engine,
  modes: ModeRegistry,
  registry: CommandRegistry,
  keys: KeyPlatform,
): void {
  useEffect(() => {
    const unregister = modes.register({
      id: "organise",
      title: organiseWords("command.mode.organise"),
      label: organiseWords("mode.organise"),
      icon: "pages",
      toolbar: () => <OrganiseBar controller={controller} engine={engine} keys={keys} />,
      leave: () => controller.select(new Set()),
    });
    const commands = registerOrganiseCommands(registry);
    return () => {
      commands();
      unregister();
    };
  }, [controller, engine, modes, registry, keys]);
}

/** The toolbar under the top bar: the commands and the note on the selection. */
function OrganiseBar({
  controller,
  engine,
  keys,
}: {
  controller: OrganiseController;
  engine: Engine;
  keys: KeyPlatform;
}): ReactNode {
  const selected = useStoreState(controller, (s) => s.selected.size);
  const scale = useStoreState(controller, (s) => s.scale);
  const total = useStoreState(engine, (s) => s.pages.length);
  return (
    <OrganiseToolbar
      note={selectionNote(selected, total)}
      scale={scale}
      onScale={(value) => controller.setScale(value)}
      platform={keys}
    />
  );
}

/** The top bar's mode switcher (§7.2), once a second mode (Organise) is registered. */
export function ModeArea({ modes }: { modes: ModeRegistry }): ReactNode {
  const all = useStoreState(modes, (s) => s.modes);
  const current = useStoreState(modes, (s) => s.current);
  if (all.length < 2 || current === undefined) return null;
  return (
    <ModeSwitcher
      aria-label={organiseWords("organise.modes")}
      modes={all.map((m) => ({ id: m.id, label: m.label ?? m.title }))}
      value={current}
      onChange={(id) => modes.switchTo(id)}
    />
  );
}

/** Props of `OrganiseMode`. */
export interface OrganiseModeProps {
  readonly controller: OrganiseController;
  /** The open file's name, which the files Split and Merge write are named from. */
  readonly fileName: string;
  readonly engine: Engine;
  readonly viewer: Viewer;
  readonly modes: ModeRegistry;
  readonly registry: CommandRegistry;
  readonly keys: KeyPlatform;
}

/** The Organise mode: registered while a document is open, its grid over the viewer while current. */
export function OrganiseMode(props: OrganiseModeProps): ReactNode {
  const { controller, viewer, modes } = props;
  useOrganiseMode(controller, props.engine, modes, props.registry, props.keys);
  const current = useStoreState(modes, (s) => s.current === "organise");
  // The viewer stays mounted under the grid, hidden: out of reach of keys and assistive technology,
  // and drawing nothing while edits change its pages. It keeps its place for Read.
  useEffect(() => {
    viewer.element.hidden = current;
    return () => {
      viewer.element.hidden = false;
    };
  }, [viewer, current]);
  return current ? (
    <OrganiseCanvas {...props} />
  ) : (
    <OrganiseDialogs controller={controller} engine={props.engine} fileName={props.fileName} />
  );
}

/** The grid, the inspector beside it, the live region and the dialogs. */
function OrganiseCanvas({ controller, engine, viewer, modes, keys, fileName }: OrganiseModeProps): ReactNode {
  const state = useStoreState(controller, (s) => s);
  const pages = useStoreState(engine, (s) => s.pages);
  const grid = useMemo<GridPage[]>(
    () =>
      pages.map((p, i) => ({
        id: p.id,
        label: p.label ?? String(i + 1),
        width: p.width,
        height: p.height,
        drawn: `${p.rotation}:${state.drawn.get(p.id) ?? 0}`,
      })),
    [pages, state.drawn],
  );
  const draw = useMemo(() => thumbnailDrawer(engine), [engine]);
  const commands = useMemo(() => organiseActions(controller), [controller]);
  const onCommand = (command: GridCommand) => {
    if (command === "delete") controller.delete();
    else controller.rotate(command === "rotate-right" ? 90 : -90);
  };
  // The viewer shows again first: the jump needs its size.
  const open = (index: number) => {
    viewer.element.hidden = false;
    modes.switchTo("read");
    viewer.goToPage(index);
  };
  return (
    <div className="web-organise">
      <div className="web-organise-body">
        <PageGrid
          aria-label={organiseWords("organise.grid")}
          pages={grid}
          selected={state.selected}
          onSelectionChange={(ids) => controller.select(ids)}
          draw={draw}
          onMove={(indices, gap) => controller.move(indices, gap)}
          onCommand={onCommand}
          onOpen={open}
          menu={pageMenu(state.selected.size)}
          onMenuAction={(id) => runMenu(id, controller, commands)}
          platform={keys}
          scale={state.scale}
        />
        <OrganiseInspector controller={controller} pages={pages} />
      </div>
      <Announcement controller={controller} />
      <OrganiseDialogs controller={controller} engine={engine} fileName={fileName} />
    </div>
  );
}

/** The page menu's entries (O06, O09): insert beside, turn, duplicate, move, Page setup's, and delete last. */
type MenuId =
  | "insert-before"
  | "insert-after"
  | "rotate-left"
  | "rotate-right"
  | "duplicate"
  | "move-to"
  | "crop"
  | "resize"
  | "edit-boxes"
  | "make-landscape"
  | "make-portrait"
  | "edit-labels"
  | "delete";

function pageMenu(count: number): MenuEntry<MenuId>[] {
  const del = count > 1 ? organiseWords("organise.menu.delete.many", { count }) : organiseWords("organise.menu.delete");
  return [
    { id: "insert-before", label: organiseWords("organise.menu.insert-before"), icon: "insert-page" },
    { id: "insert-after", label: organiseWords("organise.menu.insert-after") },
    {
      id: "rotate-left",
      label: organiseWords("command.pages.rotate-left"),
      icon: "rotate-left",
      shortcut: { mac: "Shift+R", other: "Shift+R" },
    },
    {
      id: "rotate-right",
      label: organiseWords("command.pages.rotate-right"),
      icon: "rotate-right",
      shortcut: { mac: "R", other: "R" },
    },
    { id: "duplicate", label: organiseWords("command.pages.duplicate"), icon: "copy" },
    { id: "move-to", label: organiseWords("command.pages.move-to"), icon: "move-to-page" },
    { id: "crop", label: organiseWords("command.pages.crop"), icon: "crop" },
    { id: "resize", label: organiseWords("command.pages.resize"), icon: "resize" },
    { id: "edit-boxes", label: organiseWords("command.pages.edit-boxes") },
    { id: "make-landscape", label: organiseWords("command.pages.make-landscape") },
    { id: "make-portrait", label: organiseWords("command.pages.make-portrait") },
    { id: "edit-labels", label: organiseWords("command.pages.edit-labels"), icon: "tag" },
    { id: "delete", label: del, icon: "delete", isDestructive: true, shortcut: { mac: "Backspace", other: "Delete" } },
  ];
}

function runMenu(id: MenuId, controller: OrganiseController, commands: ReturnType<typeof organiseActions>): void {
  const first = controller.selectedIndices()[0];
  if (id === "insert-before" || id === "insert-after") {
    if (first !== undefined) controller.insertBeside(first, id === "insert-before" ? "before" : "after");
    return;
  }
  commands[`pages.${id}`]();
}

/** The inspector for the selection (O06): what it is, its rotation with the turn buttons, and Duplicate and Delete. */
function OrganiseInspector({
  controller,
  pages,
}: {
  controller: OrganiseController;
  pages: readonly PageInfo[];
}): ReactNode {
  const selected = useStoreState(controller, (s) => s.selected);
  const indices = pages.flatMap((p, i) => (selected.has(p.id) ? [i] : []));
  if (indices.length === 0) return null;
  const chosen = indices.map((i) => pages[i] as PageInfo);
  const names = pageRanges(indices, (i) => pages[i]?.label ?? String(i + 1));
  const rotations = new Set(chosen.map((p) => p.rotation));
  const sizes = new Set(chosen.map((p) => `${Math.round(p.width)} × ${Math.round(p.height)} pt`));
  const one = indices.length === 1;
  return (
    <aside className="web-organise-inspector" aria-label={organiseWords("organise.inspector.label")}>
      <h2 className="web-organise-inspector-title">
        {organiseWords(one ? "organise.inspector.title.one" : "organise.inspector.title", { pages: names })}
      </h2>
      <dl className="web-organise-facts">
        <dt>{organiseWords("organise.inspector.rotation")}</dt>
        <dd>
          <IconButton
            icon="rotate-left"
            label={organiseWords("command.pages.rotate-left")}
            size={32}
            onPress={() => controller.rotate(-90)}
          />
          <IconButton
            icon="rotate-right"
            label={organiseWords("command.pages.rotate-right")}
            size={32}
            onPress={() => controller.rotate(90)}
          />
          <span>{rotations.size === 1 ? `${[...rotations][0]}°` : organiseWords("organise.inspector.mixed")}</span>
        </dd>
        <dt>{organiseWords("organise.inspector.size")}</dt>
        <dd>{sizes.size === 1 ? [...sizes][0] : organiseWords("organise.inspector.mixed")}</dd>
        <dt>{organiseWords("organise.inspector.labels")}</dt>
        <dd>{names}</dd>
      </dl>
      <div className="web-organise-actions">
        <Button size={30} icon="copy" onPress={() => controller.duplicate()}>
          {organiseWords("command.pages.duplicate")}
        </Button>
        <Button size={30} variant="danger" icon="delete" onPress={() => controller.delete()}>
          {one
            ? organiseWords("organise.menu.delete")
            : organiseWords("organise.menu.delete.many", { count: indices.length })}
        </Button>
      </div>
    </aside>
  );
}

/** Says each move once it is made ("Moved pages 5–6 after page 9."), politely. */
function Announcement({ controller }: { controller: OrganiseController }): ReactNode {
  const text = useStoreState(controller, (s) => s.announced);
  return (
    <p className="paper-visually-hidden" role="status">
      {text}
    </p>
  );
}

/** Move to page…, Blank pages…, Page setup's and the dialogs across files, and the questions before an edit a signature forbids. */
function OrganiseDialogs({
  controller,
  engine,
  fileName,
}: {
  controller: OrganiseController;
  engine: Engine;
  fileName: string;
}): ReactNode {
  const state = useStoreState(controller, (s) => s);
  // Read through the stores, not the controller's methods, which the React Compiler would keep from the first render.
  const pages = useStoreState(engine, (s) => s.pages);
  const selected = indicesOf(pages, state.selected);
  const ask = state.ask;
  return (
    <>
      <MoveToPageDialog
        isOpen={state.dialog === "move"}
        onClose={() => controller.open(undefined)}
        pages={pages.map((p, i) => ({ id: p.id, label: p.label ?? String(i + 1) }))}
        selected={selected}
        onMove={(gap) => controller.move(selected, gap)}
      />
      <BlankPagesDialog
        isOpen={state.dialog === "blank"}
        onClose={() => controller.open(undefined)}
        hasSelection={state.selected.size > 0}
        onInsert={(count, size, where) => controller.insertBlank(count, size, where)}
      />
      <SignedPageDialog
        ask={ask?.kind === "signed" ? { action: ask.action, page: ask.page } : undefined}
        onAnswer={(proceed) => controller.answer(proceed)}
      />
      <PageSetupDialogs controller={controller} engine={engine} />
      <DocumentsDialogs documents={controller.documents} organise={controller} engine={engine} fileName={fileName} />
      <NoPageChangesNotice controller={controller} />
    </>
  );
}

/**
 * The banner on a certified file, or one with usage rights (O06): the change was refused, with
 * Save a copy (the change in a copy, the file left signed), Apply anyway… and Details.
 */
function NoPageChangesNotice({ controller }: { controller: OrganiseController }): ReactNode {
  const ask = useStoreState(controller, (s) => (s.ask?.kind === "noPageChanges" ? s.ask : undefined));
  if (ask === undefined) return null;
  return (
    <div className="web-organise-certified" role="alert">
      <span>{organiseWords(ask.breaks === "certification" ? "organise.certified" : "organise.usageRights")}</span>
      <Button size={30} onPress={() => controller.saveCopy()}>
        {controller.copyTitle()}
      </Button>
      <Button size={30} onPress={() => controller.answer(true)}>
        {organiseWords("organise.certified.apply")}
      </Button>
      <SignatureDetails restrictions={restrictionsOf(ask.violations)} />
      <IconButton
        icon="close"
        label={organiseWords("organise.close")}
        size={26}
        onPress={() => controller.answer(false)}
      />
    </div>
  );
}

/**
 * Draws page `id` into a tile's canvas for the grid: rendered at the tile's size in device
 * pixels, behind the pages in view, and drawn once it arrives unless the tile went away.
 */
function thumbnailDrawer(engine: Engine) {
  return async (id: string, canvas: HTMLCanvasElement, signal: AbortSignal): Promise<void> => {
    const box = canvas.parentElement;
    const pages = engine.getState().pages;
    const index = pages.findIndex((p) => p.id === id);
    const page = pages[index];
    if (box === null || page === undefined) return;
    const ratio = globalThis.devicePixelRatio || 1;
    const width = Math.max(1, Math.round(box.clientWidth * ratio));
    const height = Math.max(1, Math.round(box.clientHeight * ratio));
    const { bitmap } = await engine.thumbnail(index, width / page.width, signal);
    try {
      if (signal.aborted) return;
      canvas.width = width;
      canvas.height = height;
      canvas.getContext("2d")?.drawImage(bitmap, 0, 0, width, height);
    } finally {
      bitmap.close();
    }
  };
}
