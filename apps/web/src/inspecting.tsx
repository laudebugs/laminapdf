// Whether the object inspector (O15) is open, and the handlers that open it: Inspect objects (the
// More menu, the palette, Document properties' Inspect…) and Go to object… with a number. The
// inspector itself loads with later.ts. It isn't offered on a phone, and it closes when the
// document does or the window narrows to one.

import { PHONE_QUERY, type Platform, type ToastQueue, useMediaQuery } from "@laminapdf/ui";
import type { Viewer } from "@laminapdf/viewer";
import { type ReactNode, useEffect, useState } from "react";
import type { InspectRequest } from "./inspector.tsx";
import type * as LaterParts from "./later.ts";
import type { Engine } from "./session.ts";

/** The inspector's state and its handlers. */
export interface Inspecting {
  /** What it was opened with, while it shows. */
  readonly request: InspectRequest | undefined;
  /** It can be opened: a document is open, on a screen wider than a phone's. */
  readonly available: boolean;
  /** Opens it, on Objects, at object `goTo` when given. */
  readonly open: (goTo?: number) => void;
  /** Back to pages. */
  readonly close: () => void;
  /** The commands' handlers, while it can be opened. */
  readonly actions: Partial<Record<"file.inspect-objects" | "file.go-to-object", (args?: unknown) => void>>;
}

/** The inspector's state for the document `file` (undefined while none is open). */
export function useInspecting(file: object | undefined): Inspecting {
  const phone = useMediaQuery(PHONE_QUERY);
  const [request, setRequest] = useState<InspectRequest | undefined>(undefined);
  const available = file !== undefined && !phone;
  // Another document, or none: back to pages.
  useEffect(() => setRequest(undefined), [file]);
  const open = (goTo?: number) => setRequest({ tab: "objects", goTo });
  return {
    request: available ? request : undefined,
    available,
    open,
    close: () => setRequest(undefined),
    actions: available
      ? {
          "file.inspect-objects": () => open(),
          "file.go-to-object": (num?: unknown) => open(typeof num === "number" ? num : undefined),
        }
      : {},
  };
}

/** The inspector in place of the pages, once later.ts has loaded; nothing while it is closed. */
export function InspectorCover({
  inspecting,
  later,
  engine,
  platform,
  toasts,
  viewer,
}: {
  inspecting: Inspecting;
  later: Pick<typeof LaterParts, "InspectorView"> | undefined;
  engine: Engine;
  platform: Platform;
  toasts: ToastQueue;
  viewer: Viewer | undefined;
}): ReactNode {
  const { request } = inspecting;
  if (request === undefined || later === undefined) return null;
  return (
    <later.InspectorView
      engine={engine}
      platform={platform}
      toasts={toasts}
      request={request}
      onBack={inspecting.close}
      onShowPage={(page) => {
        inspecting.close();
        viewer?.goToPage(page);
      }}
    />
  );
}

/** The inspector's state for the document `parts.viewer` shows, and what the canvas shows while it is open. */
export function useInspector(parts: {
  readonly engine: Engine;
  readonly platform: Platform;
  readonly toasts: ToastQueue;
  readonly later: Pick<typeof LaterParts, "InspectorView"> | undefined;
  readonly viewer: Viewer | undefined;
}): { inspecting: Inspecting; cover: ReactNode } {
  // Each document gets a viewer of its own, so a new viewer is a new document.
  const inspecting = useInspecting(parts.viewer);
  const cover = inspecting.request === undefined ? null : <InspectorCover inspecting={inspecting} {...parts} />;
  return { inspecting, cover };
}
