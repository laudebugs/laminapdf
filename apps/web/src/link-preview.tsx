// Link previews in the reader (R07): the viewer's `linkpreview` events become a `LinkPreview` card
// (a popover for the mouse and keyboard, a sheet for touch). The picture comes from
// `Viewer.preview`; the title is the deepest outline item at or before the destination, else its
// page label; the words say where the link goes. Nothing here makes a request: the picture is
// drawn by the document worker from the open file (ADR 0011).

import {
  LinkPreview,
  type LinkPreviewProps,
  message,
  type OutlineTreeItem,
  outlineItemAt,
  useStoreState,
} from "@laminapdf/ui";
import type { LinkPreviewPicture, LinkPreviewRequest, Viewer } from "@laminapdf/viewer";
import { type ReactNode, useEffect, useState } from "react";
import { type PageInfo, pageLabel, placeOf } from "./outline.ts";
import type { Engine } from "./session.ts";

/** The preview that shows: what asked for it, and its picture once drawn. */
interface Shown {
  readonly request: LinkPreviewRequest;
  /** `undefined` while it draws, `null` for a preview without one (a page action, another file). */
  readonly picture: LinkPreviewPicture | null | undefined;
}

/** The viewer's link previews: the request that shows, and its picture as it arrives. */
function usePreviewRequests(viewer: Viewer | undefined): Shown | undefined {
  const [shown, setShown] = useState<Shown | undefined>(undefined);
  useEffect(() => {
    if (viewer === undefined) return;
    // The request that shows, and the render drawing its picture.
    let current: LinkPreviewRequest | undefined;
    let drawing: AbortController | undefined;
    const hide = () => {
      drawing?.abort();
      drawing = undefined;
      current = undefined;
      setShown(undefined);
    };
    const offShow = viewer.on("linkpreview", (request) => {
      drawing?.abort();
      current = request;
      const { shows } = request;
      if (shows.kind !== "target") {
        setShown({ request, picture: null });
        return;
      }
      setShown({ request, picture: undefined });
      const abort = new AbortController();
      drawing = abort;
      const settle = (picture: LinkPreviewPicture | null) => {
        if (!abort.signal.aborted && current === request) setShown({ request, picture });
      };
      // A failed render (a page that can't be drawn) leaves the card with its words.
      viewer.preview(shows.target, { signal: abort.signal }).then(settle, () => settle(null));
    });
    const offEnd = viewer.on("linkpreviewend", ({ anchor }) => {
      if (current?.anchor === anchor) hide();
    });
    return () => {
      offShow();
      offEnd();
      hide();
    };
  }, [viewer]);
  return shown;
}

/** The open document's link previews. */
export function LinkPreviews({ engine, viewer }: { engine: Engine; viewer: Viewer | undefined }): ReactNode {
  const shown = usePreviewRequests(viewer);
  const pages = useStoreState(engine, (s) => s.pages);
  const outline = useStoreState(engine, (s) => s.outline);
  // The title names the destination's section: the outline is read the first time a preview shows.
  useEffect(() => {
    if (shown !== undefined && outline === undefined) void engine.loadOutline();
  }, [engine, shown, outline]);
  if (viewer === undefined || shown === undefined) return null;
  const items = typeof outline === "object" ? outline.items : [];
  return (
    <LinkPreview
      {...cardProps(shown, pages, items)}
      anchor={shown.request.anchor}
      pointer={shown.request.pointer}
      closeLabel={message("link.preview.close")}
      onFollow={() => viewer.followLinkPreview()}
      onDismiss={() => viewer.dismissLinkPreview()}
      onHold={(held) => viewer.holdLinkPreview(held)}
    />
  );
}

type CardProps = Pick<LinkPreviewProps, "title" | "caption" | "picture" | "pictureLabel" | "text" | "goLabel">;

/** What the card says: its title, caption, picture and the sheet's words. */
function cardProps(
  { request, picture }: Shown,
  pages: readonly PageInfo[],
  items: readonly OutlineTreeItem[],
): CardProps {
  const { shows, pointer } = request;
  // On touch the sheet's Go button follows the link: no "Enter to go".
  const caption = (page: string) =>
    message(pointer === "touch" ? "link.preview.caption" : "link.preview.captionKeyboard", { page });
  if (shows.kind === "note") return { title: shows.note, picture: null };
  if (shows.kind === "page") {
    const page = pageLabel(pages, shows.page);
    return { title: shows.note, caption: caption(page), picture: null, goLabel: message("link.preview.go", { page }) };
  }
  const page = pageLabel(pages, shows.target.page);
  // The section at the destination itself, not at the crop's top a little above it.
  const section = outlineItemAt(items, placeOf(shows.target, pages))?.title;
  const text = picture?.text ?? "";
  return {
    title: section ?? message("link.preview.pageTitle", { page }),
    caption: caption(page),
    picture,
    pictureLabel: picture?.kind === "page" || text === "" ? message("link.preview.top", { page }) : text,
    ...(text === "" ? {} : { text }),
    goLabel: message("link.preview.go", { page }),
  };
}
