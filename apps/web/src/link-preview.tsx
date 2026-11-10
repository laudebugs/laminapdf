// Link previews in the reader (R07, todo T5a): the viewer's `linkpreview` events become a
// `LinkPreview` card (a popover for the mouse and keyboard, a sheet for touch). The picture comes from
// `Viewer.preview`; the title is the deepest outline item at or before the destination, else its
// page label; the words say where the link goes. Nothing here makes a request: the picture is
// drawn by the document worker from the open file (ADR 0011).

import { LinkPreview, type LinkPreviewProps, message, outlineItemAt, useStoreState } from "@laminapdf/ui";
import type { LinkPreviewPicture, LinkPreviewRequest, Viewer } from "@laminapdf/viewer";
import { type ReactNode, useEffect, useState } from "react";
import { pageLabel, placeOf } from "./outline.ts";
import type { Engine } from "./session.ts";

/** The preview that shows: what asked for it, and its picture once drawn. */
interface Shown {
  readonly request: LinkPreviewRequest;
  /** `undefined` while it draws, `null` for a preview without one (a page action, another file). */
  readonly picture: LinkPreviewPicture | null | undefined;
}

/** The open document's link previews. */
export function LinkPreviews({ engine, viewer }: { engine: Engine; viewer: Viewer | undefined }): ReactNode {
  const [shown, setShown] = useState<Shown | undefined>(undefined);
  const pages = useStoreState(engine, (s) => s.pages);
  const outline = useStoreState(engine, (s) => s.outline);
  useEffect(() => {
    if (viewer === undefined) return;
    let drawing: AbortController | undefined;
    const stop = () => {
      drawing?.abort();
      drawing = undefined;
    };
    const offShow = viewer.on("linkpreview", (request) => {
      stop();
      const { shows } = request;
      if (shows.kind !== "target") {
        setShown({ request, picture: null });
        return;
      }
      setShown({ request, picture: undefined });
      const abort = new AbortController();
      drawing = abort;
      viewer.preview(shows.target, { signal: abort.signal }).then(
        (picture) => {
          if (!abort.signal.aborted) setShown((now) => (now?.request === request ? { request, picture } : now));
        },
        () => {
          // Aborted (the pointer left), or a page that can't be drawn: the card keeps its words.
          if (!abort.signal.aborted) setShown((now) => (now?.request === request ? { request, picture: null } : now));
        },
      );
    });
    const offEnd = viewer.on("linkpreviewend", ({ anchor }) => {
      setShown((now) => {
        if (now?.request.anchor !== anchor) return now;
        stop();
        return undefined;
      });
    });
    return () => {
      stop();
      offShow();
      offEnd();
      setShown(undefined);
    };
  }, [viewer]);
  // The title names the destination's section: the outline is read the first time a preview shows.
  useEffect(() => {
    if (shown !== undefined && outline === undefined) void engine.loadOutline();
  }, [engine, shown, outline]);
  if (viewer === undefined || shown === undefined) return null;
  const items = typeof outline === "object" ? outline.items : [];
  const card = cardProps(shown, pages, items);
  // On touch the sheet's Go button follows: "Enter to go" would be wrong there.
  const { caption, ...rest } = card;
  const props = shown.request.pointer === "touch" || caption === undefined ? rest : card;
  return (
    <LinkPreview
      {...props}
      anchor={shown.request.anchor}
      pointer={shown.request.pointer}
      closeLabel={message("link.preview.close")}
      onFollow={() => {
        setShown(undefined);
        viewer.followLinkPreview();
      }}
      onDismiss={() => {
        setShown(undefined);
        viewer.dismissLinkPreview();
      }}
      onHold={(held) => viewer.holdLinkPreview(held)}
    />
  );
}

type CardProps = Pick<LinkPreviewProps, "title" | "caption" | "picture" | "pictureLabel" | "text" | "goLabel">;

/** What the card says: its title, caption, picture and the sheet's words. */
function cardProps(
  { request, picture }: Shown,
  pages: Parameters<typeof pageLabel>[0],
  items: Parameters<typeof outlineItemAt>[0],
): CardProps {
  const { shows } = request;
  if (shows.kind === "note") return { title: shows.note, picture: null };
  if (shows.kind === "page") {
    const page = pageLabel(pages, shows.page);
    return {
      title: shows.note,
      caption: message("link.preview.caption", { page }),
      picture: null,
      goLabel: message("link.preview.go", { page }),
    };
  }
  const page = pageLabel(pages, shows.target.page);
  // Where the destination is, for the current-section rule: the crop's top once drawn.
  const height = pages[shows.target.page]?.height ?? 0;
  const at =
    picture === undefined || picture === null || height <= 0
      ? placeOf(shows.target, pages)
      : { page: picture.page, y: Math.min(1, Math.max(0, picture.y / height)) };
  const section = outlineItemAt(items, at)?.title;
  const top = message("link.preview.top", { page });
  return {
    title: section ?? message("link.preview.pageTitle", { page }),
    caption: message("link.preview.caption", { page }),
    picture,
    pictureLabel:
      picture === undefined || picture === null || picture.kind === "page" || picture.text === "" ? top : picture.text,
    ...(picture === undefined || picture === null || picture.text === "" ? {} : { text: picture.text }),
    goLabel: message("link.preview.go", { page }),
  };
}
