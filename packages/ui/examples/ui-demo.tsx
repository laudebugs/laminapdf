// The reader chrome on one page, over a stand-in viewer and opener, for trying it by hand
// (`pnpm --filter @laminapdf/ui demo`) and for the axe check in the browser tests. Open any PDF;
// a name with "locked" asks for the password "lamina", "damaged" opens with repairs and "broken"
// can't be opened. Ctrl+K or Commands opens the palette (the demo writes keys as on Windows and
// Linux, so it is Ctrl+K on a Mac too): try "go to 12" or "zoom 150".
// The sidebar toggle opens the rail (fake Pages and Outline tabs); "Select the heading" on the stand-in
// page opens the inspector beside the canvas; Ctrl+\ turns focus mode on and off.
// `#debug` shows the debug menu.

import "../src/styles/index.css";
import { type ReactNode, useEffect, useMemo, useState } from "react";
import { createRoot } from "react-dom/client";
import {
  type CommandContext,
  CommandRegistry,
  CommandsProvider,
  createWebPlatform,
  type DebugBackend,
  DebugMenu,
  FocusModeItem,
  Inspector,
  LeftRail,
  type ModeRegistry,
  type MoreItems,
  NetworkIndicator,
  OpenDropZone,
  type OpenedDocument,
  OpenFailure,
  PasswordDialog,
  PhoneBottomBar,
  type Platform,
  PlatformProvider,
  type RailTab,
  ReaderController,
  ReaderShell,
  ReaderTopBar,
  RegistryPalette,
  RepairDetails,
  registerReaderCommands,
  type ShellLayout,
  shellActions,
  useGatedEntries,
  useReaderState,
  useShell,
  useStoreState,
  type ViewerControls,
  type ViewerSlice,
  type Zoom,
} from "../src/index.ts";

/** A viewer that only keeps its state, standing in for `@laminapdf/viewer`'s. */
class DemoViewer implements ViewerControls {
  #state: ViewerSlice = { pageCount: 24, currentPage: 0, zoom: "fit-width", percent: 118 };
  readonly #listeners = new Set<() => void>();
  getState = (): ViewerSlice => this.#state;
  subscribe = (listener: () => void) => {
    this.#listeners.add(listener);
    return () => this.#listeners.delete(listener);
  };
  goToPage = (index: number) => this.#set({ currentPage: index });
  // A stand-in records no history: a jump is a plain move.
  jumpToPage = (index: number) => this.goToPage(index);
  setZoom = (zoom: Zoom) => this.#set({ zoom, percent: typeof zoom === "number" ? zoom : 100 });
  rotate = () => undefined;
  #set(next: Partial<ViewerSlice>) {
    this.#state = { ...this.#state, ...next };
    for (const listener of this.#listeners) listener();
  }
}

/** Opens by the file's name: see the header. */
async function demoOpener(file: { name: string }, password: string | undefined): Promise<OpenedDocument> {
  if (file.name.includes("broken")) throw Object.assign(new Error("no /Root"), { code: "missing-root" });
  if (file.name.includes("locked") && password !== "lamina") {
    throw Object.assign(new Error("locked"), { code: password === undefined ? "password-required" : "wrong-password" });
  }
  return { pageCount: 24 };
}

type MoreId = "open" | "rotate" | "save";

/** The registry with the reader's commands, and the context they run in for this demo. */
function useDemoCommands(
  platform: Platform,
  reader: ReaderController,
  viewer: DemoViewer,
  shell: { layout: ShellLayout; modes: ModeRegistry; openPalette: () => void },
): { registry: CommandRegistry; context: CommandContext } {
  const [registry] = useState(() => {
    const commands = new CommandRegistry();
    registerReaderCommands(commands);
    return commands;
  });
  const context = useMemo<CommandContext>(
    () => ({
      platform,
      reader,
      viewer,
      actions: {
        ...shellActions(shell.layout, shell.modes, { rail: true }),
        "palette.open": shell.openPalette,
        "file.properties": () => undefined,
      },
    }),
    [platform, reader, viewer, shell],
  );
  return { registry, context };
}

/** The rail's stand-in tabs. */
const RAIL_TABS: readonly RailTab[] = [
  {
    id: "pages",
    label: "Pages",
    panel: (
      <ol className="demo-rail-list">
        {["Page 1", "Page 2", "Page 3", "Page 4", "Page 5", "Page 6"].map((page) => (
          <li key={page}>{page}</li>
        ))}
      </ol>
    ),
  },
  {
    id: "outline",
    label: "Outline",
    panel: (
      <ul className="demo-rail-list">
        <li>1 Summary</li>
        <li>2 Regions</li>
        <li>3 Outlook</li>
      </ul>
    ),
  },
];

/** The chrome over `platform`: the reader's shell, its prompt and its pages. */
export function Demo({ platform, debug = false }: { platform: Platform; debug?: boolean }): ReactNode {
  const [reader] = useState(() => new ReaderController(platform, demoOpener));
  const [viewer] = useState(() => new DemoViewer());
  const [palette, setPalette] = useState(false);
  const { layout, modes } = useShell(platform.storage);
  const [shell] = useState(() => ({ layout, modes, openPalette: () => setPalette(true) }));
  const { registry, context } = useDemoCommands(platform, reader, viewer, shell);
  const more = useDemoMore(reader, viewer);
  const state = useReaderState(reader);
  const sidebar = useStoreState(layout, (s) => s.rail.open);
  const open = state.phase === "open";
  const damaged = open && state.file.name.includes("damaged") ? state.file.name : undefined;
  // A damaged file opens with its repairs docked, as the web reader does.
  useEffect(() => {
    if (damaged !== undefined) layout.openInspector({ id: "repairs", selection: false });
  }, [damaged, layout]);
  return (
    <CommandsProvider registry={registry} context={context} keys="other">
      <ReaderShell
        layout={layout}
        modes={modes}
        topBar={<DemoTopBar reader={reader} viewer={viewer} sidebar={sidebar} debug={debug} more={more} />}
        bottomBar={<PhoneBottomBar layout={layout} rail={open} more={more} keys="other" />}
        {...(open ? { rail: <LeftRail layout={layout} tabs={RAIL_TABS} /> } : {})}
        inspector={<DemoInspector platform={platform} layout={layout} damaged={damaged} />}
        status={
          <>
            <NetworkIndicator count="0 network requests for this document" explanation="· Offline-safe" />
            <FocusModeItem layout={layout} />
          </>
        }
      >
        {state.phase === "empty" ? (
          <OpenDropZone reader={reader} keys="other">
            <p>Lamina PDF reads PDFs inside this browser tab and has no upload server.</p>
          </OpenDropZone>
        ) : null}
        {state.phase === "error" ? (
          <OpenFailure error={state.error} onTryAnother={() => reader.close()} onCopy={() => undefined} />
        ) : null}
        {open ? <DemoPage layout={layout} /> : null}
        <PasswordDialog reader={reader} />
      </ReaderShell>
      <RegistryPalette isOpen={palette} onOpenChange={setPalette} />
    </CommandsProvider>
  );
}

/** A stand-in for the viewer's page, with a heading to select. */
function DemoPage({ layout }: { layout: ShellLayout }): ReactNode {
  return (
    <div className="demo-page">
      <button type="button" onClick={() => layout.openInspector({ id: "heading", selection: true })}>
        Select the heading
      </button>
    </div>
  );
}

/** The demo's More items: the top bar's menu, and the phone's More sheet. */
function useDemoMore(reader: ReaderController, viewer: DemoViewer): MoreItems<MoreId> {
  const items = useGatedEntries<MoreId>([
    { id: "open", label: "Open another file…" },
    { id: "rotate", label: "Rotate clockwise", icon: "rotate-right" },
    { id: "save", label: "Save a copy…", icon: "download", needs: "canSaveAsDialog" },
  ]);
  return { items, onAction: (id) => (id === "open" ? reader.close() : id === "rotate" && viewer.rotate()) };
}

/** The top bar, with the demo's More menu and, for `#debug`, the debug menu. */
function DemoTopBar(props: {
  reader: ReaderController;
  viewer: DemoViewer;
  sidebar: boolean;
  debug: boolean;
  more: MoreItems<MoreId>;
}): ReactNode {
  const [backend, setBackend] = useState<DebugBackend>("pdfium");
  return (
    <ReaderTopBar
      reader={props.reader}
      viewer={props.viewer}
      keys="other"
      sidebarOpen={props.sidebar}
      home={<span className="demo-wordmark">lamina</span>}
      debug={props.debug ? <DebugMenu backend={backend} onBackendChange={setBackend} keys="other" /> : undefined}
      more={props.more}
    />
  );
}

/** What the inspector shows: the repair details of a damaged file, or the selected heading. */
function DemoInspector({
  platform,
  layout,
  damaged,
}: {
  platform: Platform;
  layout: ShellLayout;
  damaged: string | undefined;
}): ReactNode {
  const shown = useStoreState(layout, (s) => s.inspector?.id);
  if (shown === "repairs" && damaged !== undefined) {
    return (
      <Inspector layout={layout}>
        <RepairDetails
          fileName={damaged}
          repairs={[{ code: "xref-repaired" }, { code: "stream-length-recovered", pages: [3, 4, 9] }]}
          onClose={() => layout.closeInspector()}
          onCopy={() => void platform.copyText("xref-repaired, stream-length-recovered")}
        />
      </Inspector>
    );
  }
  if (shown === "heading") {
    return (
      <Inspector layout={layout} label="Heading">
        <h2 className="demo-inspector-title">Heading</h2>
        <p>Quarterly figures, on page 1.</p>
      </Inspector>
    );
  }
  return null;
}

const root = document.getElementById("demo");
if (root !== null) {
  // One platform for the page: two would each open the store and take the launch queue.
  const platform = createWebPlatform();
  createRoot(root).render(
    <PlatformProvider platform={platform}>
      <Demo platform={platform} debug={location.hash === "#debug"} />
    </PlatformProvider>,
  );
}
