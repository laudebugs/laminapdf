// The capability page over the reader, loaded with later.ts (#380): its data (the matrix `cargo
// xtask capabilities` writes, bundled when the reader is built, so the page makes no request (O05),
// and the reader's version), the reader made inert behind it, and Back through the history.

import { message, usePlatform } from "@laminapdf/ui";
import { type CapabilityMatrix, CapabilityPage } from "@laminapdf/ui/capabilities";
import { type ReactNode, useEffect } from "react";
import { createPortal } from "react-dom";
import matrix from "../../../docs/capabilities/matrix.json" with { type: "json" };
import { CAPABILITY_HISTORY_KEY } from "./capability-address.ts";

/** How the statuses are proven: the matrix's own page, with its rules for each status. */
const HOW_TESTED = "https://github.com/laudebugs/laminapdf/blob/main/docs/capabilities/README.md";

/**
 * The page over the reader in `win`, with Back naming the open file when there is one. The reader
 * behind it can't take focus or clicks while it shows.
 */
export function CapabilityRoutePage({
  win,
  fileName,
  onLeft,
}: {
  win: Window;
  fileName: string | undefined;
  /** The page left without a history move: the reader took its address. */
  onLeft: () => void;
}): ReactNode {
  const platform = usePlatform();
  useEffect(() => {
    const app = win.document.getElementById("app");
    if (app === null) return;
    app.inert = true;
    return () => {
      app.inert = false;
    };
  }, [win]);
  const back = () => {
    const state: unknown = win.history.state;
    if (typeof state === "object" && state !== null && CAPABILITY_HISTORY_KEY in state) {
      win.history.back();
      return;
    }
    // Opened at this address (a link, a reload): there is no entry to go back to, so the reader
    // takes the address's place.
    win.history.replaceState(null, "", `/${win.location.search}${win.location.hash}`);
    onLeft();
  };
  return createPortal(
    <CapabilityPage
      matrix={matrix as CapabilityMatrix}
      version={__APP_VERSION__}
      backLabel={
        fileName === undefined ? message("capabilities.back.reader") : message("capabilities.back", { name: fileName })
      }
      onBack={back}
      onHowTested={() => void platform.openExternal(HOW_TESTED).catch(() => undefined)}
    />,
    win.document.body,
  );
}
