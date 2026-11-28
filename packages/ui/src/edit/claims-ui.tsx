// The standards a tagged or PDF/A file claims, while editing (Paper A07): the strip under the top
// bar that says an edit would break a claim, the badges in the status bar, and Check before
// saving, which lists each claim and offers Save without the claim, Fix it first or closing. The
// app computes the claims and issues; these only show them.

import type { ReactNode } from "react";
import { Badge } from "../components/badge.js";
import { Button } from "../components/button.js";
import { Notice } from "../components/notice.js";
import { StatusBarItem } from "../components/status-bar.js";
import { laterCountMessage, laterList, laterMessage } from "../later-messages.js";
import { Dialog } from "../overlays/dialog.js";

/** A standard the file claims. */
export type ClaimId = "pdfa" | "pdfua";

/** One thing an edit broke: the claim it breaks, the sentence that says so, and whether adding alternative text fixes it. */
export interface ClaimIssue {
  readonly claim: ClaimId;
  readonly message: string;
  readonly needsAlt: boolean;
}

/** What the strip, the badges and the dialog read: the claims by name ("PDF/UA-1") and what breaks them. */
export interface ClaimsView {
  /** The name of each claim the file makes. */
  readonly names: Readonly<Partial<Record<ClaimId, string>>>;
  readonly issues: readonly ClaimIssue[];
}

/** The claims that have an issue, named. */
function brokenNames(view: ClaimsView): string[] {
  return (["pdfa", "pdfua"] as const)
    .filter((id) => view.issues.some((i) => i.claim === id))
    .flatMap((id) => view.names[id] ?? []);
}

/** The first sentence of an issue, begun with a capital. */
function sentence(text: string): string {
  return text.length === 0
    ? text
    : `${text.charAt(0).toLocaleUpperCase()}${text.slice(1)}${text.endsWith(".") ? "" : "."}`;
}

/** The strip under the toolbar: which claim would break and why, with Details and, for a figure, Add alternative text. */
export function ClaimsStrip({
  claims,
  onDetails,
  onAddAlt,
}: {
  claims: ClaimsView;
  onDetails: () => void;
  /** Selects the figure and puts the focus in Alternative text; absent when no issue is about a figure. */
  onAddAlt: (() => void) | undefined;
}): ReactNode {
  const broken = brokenNames(claims);
  if (broken.length === 0) return null;
  return (
    <section
      className="paper-save-strip paper-claims-strip"
      aria-label={laterMessage("claims.strip.label")}
      role="status"
    >
      <Notice tone="warning" title={laterMessage("claims.strip.title", { claims: laterList(broken) })}>
        {sentence(claims.issues[0]?.message ?? "")}
      </Notice>
      <Button variant="ghost" size={26} onPress={onDetails}>
        {laterMessage("claims.strip.details")}
      </Button>
      {onAddAlt === undefined ? null : (
        <Button size={26} onPress={onAddAlt}>
          {laterMessage("claims.strip.addAlt")}
        </Button>
      )}
    </section>
  );
}

/** The status bar's claim badge: one button, named by its visible text, that opens the check. */
export function ClaimBadges({ claims, onPress }: { claims: ClaimsView; onPress: () => void }): ReactNode {
  const broken = brokenNames(claims);
  if (broken.length === 0) return null;
  const count = claims.issues.length;
  const label =
    count === 1
      ? laterMessage("claims.badge.one", { claims: laterList(broken) })
      : laterMessage("claims.badge", { claims: laterList(broken), count });
  return (
    <StatusBarItem icon="warning" onPress={onPress}>
      {label}
    </StatusBarItem>
  );
}

/** How a person answers the check. */
export type ClaimsAnswer = "remove" | "fix" | "close";

/** One claim's row: its name, a badge with the word and an icon, and one line. */
function ClaimRow({ claim, name, issues }: { claim: ClaimId; name: string; issues: readonly ClaimIssue[] }): ReactNode {
  const broken = issues.filter((i) => i.claim === claim);
  return (
    <li className="paper-claims-row">
      <span className="paper-claims-name">{name}</span>
      {broken.length === 0 ? (
        <Badge tone="positive" icon="positive">
          {laterMessage("claims.dialog.holds")}
        </Badge>
      ) : (
        <Badge tone="warning" icon="warning">
          {laterCountMessage(broken.length, { one: "claims.dialog.issue.one", many: "claims.dialog.issue.many" })}
        </Badge>
      )}
      {broken.map((issue) => (
        <p key={issue.message} className="paper-claims-line">
          {sentence(issue.message)}
        </p>
      ))}
    </li>
  );
}

/**
 * Check before saving: each claim with what holds and what doesn't, why saving without the claim
 * is how a file stops promising what it no longer meets, and the three ways out. Without a figure
 * to fix, Fix it first only closes the check so the person can keep editing.
 */
export function ClaimsCheckDialog({
  claims,
  open,
  saving,
  onAnswer,
}: {
  claims: ClaimsView;
  open: boolean;
  /** Whether a save is waiting on the answer (Save without the claim then saves); false when the check was opened for a look. */
  saving: boolean;
  onAnswer: (answer: ClaimsAnswer) => void;
}): ReactNode {
  const broken = brokenNames(claims);
  const rows = (["pdfa", "pdfua"] as const).flatMap((id) => {
    const name = claims.names[id];
    return name === undefined ? [] : [{ id, name }];
  });
  return (
    <Dialog
      isOpen={open}
      onOpenChange={(isOpen) => {
        if (!isOpen) onAnswer("close");
      }}
      title={laterMessage("claims.dialog.title")}
      description={laterMessage("claims.dialog.body", { claims: laterList(broken) })}
      closeLabel={laterMessage("claims.dialog.close")}
      size="form"
      actions={
        <>
          {saving ? (
            <Button autoFocus onPress={() => onAnswer("remove")}>
              {laterMessage("claims.dialog.remove")}
            </Button>
          ) : null}
          <Button variant="primary" autoFocus={!saving} onPress={() => onAnswer("fix")}>
            {laterMessage("claims.dialog.fix")}
          </Button>
        </>
      }
    >
      <ul className="paper-claims-list" aria-label={laterMessage("claims.dialog.claims")}>
        {rows.map((row) => (
          <ClaimRow key={row.id} claim={row.id} name={row.name} issues={claims.issues} />
        ))}
      </ul>
    </Dialog>
  );
}
