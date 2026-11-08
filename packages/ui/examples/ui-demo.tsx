// The reader chrome on one page, over a stand-in viewer and opener, for trying it by hand
// (`pnpm --filter @laminapdf/ui demo`) and for the axe check in the browser tests. Open any PDF;
// a name with "locked" asks for the password "lamina", "damaged" opens with repairs and "broken"
// can't be opened. Ctrl+K or Commands opens the palette (the demo writes keys as on Windows and
// Linux, so it is Ctrl+K on a Mac too): try "go to 12" or "zoom 150".
// `#debug` shows the debug menu.

import "../src/styles/index.css";
import { type ReactNode, useMemo, useState } from "react";
import { createRoot } from "react-dom/client";
import {
  type CommandContext,
  CommandRegistry,
  CommandsProvider,
  createWebPlatform,
  type DebugBackend,
  DebugMenu,
  NetworkIndicator,
  OpenDropZone,
  type OpenedDocument,
  OpenFailure,
  PasswordDialog,
  type Platform,
  PlatformProvider,
  ReaderController,
  ReaderShell,
  ReaderTopBar,
  RegistryPalette,
  RepairDetails,
  registerReaderCommands,
  useGatedEntries,
  useReaderState,
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
  shell: { toggleSidebar: () => void; openPalette: () => void },
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
        "view.toggle-sidebar": shell.toggleSidebar,
        "palette.open": shell.openPalette,
        "file.properties": () => undefined,
      },
    }),
    [platform, reader, viewer, shell.toggleSidebar, shell.openPalette],
  );
  return { registry, context };
}

/** The chrome over `platform`: the reader's shell, its prompt and its pages. */
export function Demo({ platform, debug = false }: { platform: Platform; debug?: boolean }): ReactNode {
  const [reader] = useState(() => new ReaderController(platform, demoOpener));
  const [viewer] = useState(() => new DemoViewer());
  const [sidebar, setSidebar] = useState(true);
  const [palette, setPalette] = useState(false);
  const [repairsOpen, setRepairsOpen] = useState(true);
  const [shell] = useState(() => ({
    toggleSidebar: () => setSidebar((open) => !open),
    openPalette: () => setPalette(true),
  }));
  const { registry, context } = useDemoCommands(platform, reader, viewer, shell);
  const state = useReaderState(reader);
  return (
    <CommandsProvider registry={registry} context={context} keys="other">
      <ReaderShell
        topBar={<DemoTopBar reader={reader} viewer={viewer} sidebar={sidebar} debug={debug} />}
        {...(state.phase === "open" && sidebar ? { rail: <nav aria-label="Pages">Pages</nav> } : {})}
        {...demoInspector(
          platform,
          repairsOpen && state.phase === "open" && state.file.name.includes("damaged") ? state.file.name : undefined,
          () => setRepairsOpen(false),
        )}
        status={<NetworkIndicator count="0 network requests for this document" explanation="· Offline-safe" />}
      >
        {state.phase === "empty" ? (
          <OpenDropZone reader={reader} keys="other">
            <p>Lamina PDF reads PDFs inside this browser tab and has no upload server.</p>
          </OpenDropZone>
        ) : null}
        {state.phase === "error" ? (
          <OpenFailure error={state.error} onTryAnother={() => reader.close()} onCopy={() => undefined} />
        ) : null}
        <PasswordDialog reader={reader} />
      </ReaderShell>
      <RegistryPalette isOpen={palette} onOpenChange={setPalette} />
    </CommandsProvider>
  );
}

/** The top bar, with the demo's More menu and, for `#debug`, the debug menu. */
function DemoTopBar(props: {
  reader: ReaderController;
  viewer: DemoViewer;
  sidebar: boolean;
  debug: boolean;
}): ReactNode {
  const [backend, setBackend] = useState<DebugBackend>("pdfium");
  const more = useGatedEntries<MoreId>([
    { id: "open", label: "Open another file…" },
    { id: "rotate", label: "Rotate clockwise", icon: "rotate-right" },
    { id: "save", label: "Save a copy…", icon: "download", needs: "canSaveAsDialog" },
  ]);
  return (
    <ReaderTopBar
      reader={props.reader}
      viewer={props.viewer}
      keys="other"
      sidebarOpen={props.sidebar}
      home={<span className="demo-wordmark">lamina</span>}
      debug={props.debug ? <DebugMenu backend={backend} onBackendChange={setBackend} keys="other" /> : undefined}
      more={{
        items: more,
        onAction: (id) => (id === "open" ? props.reader.close() : id === "rotate" && props.viewer.rotate()),
      }}
    />
  );
}

/** The repair details beside a damaged file, until closed. */
function demoInspector(
  platform: Platform,
  damaged: string | undefined,
  onClose: () => void,
): { inspector?: ReactNode } {
  return damaged === undefined
    ? {}
    : {
        inspector: (
          <RepairDetails
            fileName={damaged}
            repairs={[{ code: "xref-repaired" }, { code: "stream-length-recovered", pages: [3, 4, 9] }]}
            onClose={onClose}
            onCopy={() => void platform.copyText("xref-repaired, stream-length-recovered")}
          />
        ),
      };
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
