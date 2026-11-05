// The reader chrome on one page, over a stand-in viewer and opener, for trying it by hand
// (`pnpm --filter @laminapdf/ui demo`) and for the axe check in the browser tests. Open any PDF;
// a name with "locked" asks for the password "lamina", "damaged" opens with repairs and "broken"
// can't be opened. `#debug` shows the debug menu.

import "../src/styles/index.css";
import { type ReactNode, useState } from "react";
import { createRoot } from "react-dom/client";
import {
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
  RepairDetails,
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

/** The chrome over `platform`: the reader's shell, its prompt and its pages. */
export function Demo({ platform, debug = false }: { platform: Platform; debug?: boolean }): ReactNode {
  const [reader] = useState(() => new ReaderController(platform, demoOpener));
  const [viewer] = useState(() => new DemoViewer());
  const [sidebar, setSidebar] = useState(true);
  const [repairsOpen, setRepairsOpen] = useState(true);
  const [backend, setBackend] = useState<DebugBackend>("pdfium");
  const state = useReaderState(reader);
  const more = useGatedEntries<MoreId>([
    { id: "open", label: "Open another file…" },
    { id: "rotate", label: "Rotate clockwise", icon: "rotate-right" },
    { id: "save", label: "Save a copy…", icon: "download", needs: "canSaveAsDialog" },
  ]);
  const damaged = state.phase === "open" && state.file.name.includes("damaged");
  return (
    <ReaderShell
      topBar={
        <ReaderTopBar
          reader={reader}
          viewer={viewer}
          keys="other"
          sidebar={{ isOpen: sidebar, onChange: setSidebar }}
          home={<span className="demo-wordmark">lamina</span>}
          debug={debug ? <DebugMenu backend={backend} onBackendChange={setBackend} keys="other" /> : undefined}
          more={{
            items: more,
            onAction: (id) => (id === "open" ? reader.close() : id === "rotate" && viewer.rotate()),
          }}
        />
      }
      {...(state.phase === "open" && sidebar ? { rail: <nav aria-label="Pages">Pages</nav> } : {})}
      {...(damaged && repairsOpen
        ? {
            inspector: (
              <RepairDetails
                fileName={state.file.name}
                repairs={[{ code: "xref-repaired" }, { code: "stream-length-recovered", pages: [3, 4, 9] }]}
                onClose={() => setRepairsOpen(false)}
                onCopy={() => void platform.copyText("xref-repaired, stream-length-recovered")}
              />
            ),
          }
        : {})}
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
  );
}

const root = document.getElementById("demo");
if (root !== null) {
  createRoot(root).render(
    <PlatformProvider platform={createWebPlatform()}>
      <Demo platform={createWebPlatform()} debug={location.hash === "#debug"} />
    </PlatformProvider>,
  );
}
