// The external-link confirmation (R07, §7.18): a link in the document wants to leave it for the
// web. The dialog shows the full address with its host in weight 600, as the reader's link rules
// normalise it (an international host in punycode), so link text on the page that claims another
// address can't hide where this one goes. Cancel has the focus; Open website opens the address in
// a new tab with no referrer, through the platform; Copy link copies it and leaves the dialog open.

import type { ReactNode } from "react";
import { Button } from "../components/button.js";
import { message } from "../messages.js";
import { Dialog } from "../overlays/dialog.js";

/** A link address to confirm: normalised, and split around its host. */
export interface ExternalLinkTarget {
  /** The address that opens. */
  readonly href: string;
  /** The address in three parts, the host (or a mail address's domain) emphasised. */
  readonly parts: { readonly before: string; readonly host: string; readonly after: string };
}

/** Props of `ExternalLinkDialog`. */
export interface ExternalLinkDialogProps {
  /** The link waiting for an answer; the dialog is closed without one. */
  link: ExternalLinkTarget | undefined;
  /** Open website was pressed: open `href`, then the dialog closes. */
  onOpen: (href: string) => void;
  /** Copy link was pressed. */
  onCopy: (href: string) => void;
  /** Cancel, Close, Escape or the scrim: the dialog closes and nothing opens. */
  onClose: () => void;
}

/** Asks before a link in the document opens a website. */
export function ExternalLinkDialog({ link, onOpen, onCopy, onClose }: ExternalLinkDialogProps): ReactNode {
  return (
    <Dialog
      isOpen={link !== undefined}
      onOpenChange={(open) => {
        if (!open) onClose();
      }}
      title={message("link.external.title")}
      description={message("link.external.description")}
      closeLabel={message("link.external.close")}
      actions={
        link === undefined ? null : (
          <>
            <Button variant="ghost" onPress={() => onCopy(link.href)}>
              {message("link.external.copy")}
            </Button>
            <Button autoFocus onPress={onClose}>
              {message("link.external.cancel")}
            </Button>
            <Button
              variant="primary"
              onPress={() => {
                onOpen(link.href);
                onClose();
              }}
            >
              {message("link.external.open")}
            </Button>
          </>
        )
      }
    >
      {link === undefined ? null : (
        <p className="paper-external-link-address">
          {link.parts.before}
          <strong>{link.parts.host}</strong>
          {link.parts.after}
        </p>
      )}
    </Dialog>
  );
}
