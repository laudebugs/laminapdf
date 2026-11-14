// The link editor (Paper O12): where a link goes (a page of the file, a named destination, a web
// address or an email address), its description, and Follow link and Delete link. A change is
// committed when it is complete and valid: a radio or a name at once, a typed field on Enter or
// when it loses focus. An address that isn't an `http` or `https` one is refused in place, and the
// link keeps the target it had. A link read from a file that runs a script is shown as such, and
// can only be pointed somewhere else or deleted. The panel floats by the link on the page.

import { SPACE } from "@laminapdf/tokens";
import {
  type FocusEvent,
  type KeyboardEvent,
  type ReactNode,
  type RefObject,
  useEffect,
  useRef,
  useState,
} from "react";
import { createPortal } from "react-dom";
import { Button } from "../components/button.js";
import { Radio, RadioGroup } from "../components/choice.js";
import { Select, TextArea, TextInput } from "../components/field.js";
import { Notice } from "../components/notice.js";
import { type LaterMessageId, laterMessage } from "../later-messages.js";
import { useAnchoredPlacement } from "../overlays/anchored.js";
import { LayerScope } from "../overlays/layers.js";
import { type Box, placeBelow } from "../overlays/place.js";

/** The link editor's words an app needs: the tool, Make link, and the history's step labels. */
export type LinkWord = Extract<LaterMessageId, `link.${string}` | `edit.link.${string}`>;

/** The link editor's word `id`, filled in with `values`. */
export function linkWords(id: LinkWord, values: Readonly<Record<string, string>> = {}): string {
  return laterMessage(id, values);
}

/** Where a link goes, as the editor shows it. */
export type LinkGoal =
  /** Page `page` of this file (0-based). */
  | { readonly kind: "page"; readonly page: number }
  /** A named destination. */
  | { readonly kind: "named"; readonly name: string }
  /** An `http` or `https` address. */
  | { readonly kind: "web"; readonly address: string }
  /** An email address, with an optional subject. */
  | { readonly kind: "email"; readonly address: string; readonly subject: string }
  /** A link read from a file that runs a script (`JavaScript` or another action), never run. */
  | { readonly kind: "script"; readonly action: string };

/** The kinds of goal the editor offers. */
type Kind = Exclude<LinkGoal["kind"], "script">;

/** Props of `LinkEditor`. */
export interface LinkEditorProps {
  /** The linked text, for a link made on text; a drawn area has none. */
  readonly text?: string;
  /** A new link on a box the user drew: the context line says so. */
  readonly drawn?: boolean;
  /** Where it goes; undefined for a new link that doesn't go anywhere yet. */
  readonly goal: LinkGoal | undefined;
  /** The file's page count. */
  readonly pageCount: number;
  /** The file's named destinations. */
  readonly names: readonly string[];
  /** Its description (`/Contents`). */
  readonly contents: string;
  /** A new goal, complete and valid. */
  readonly onGoal: (goal: LinkGoal) => void;
  /** A new description; empty removes it. */
  readonly onContents: (contents: string) => void;
  /** Follows the link; left out while it goes nowhere or runs a script. */
  readonly onFollow?: () => void;
  /** Deletes the link (a new one is dropped). */
  readonly onDelete: () => void;
  /** Checks a web or `mailto:` address before it is written. */
  readonly checkAddress: (address: string) => AddressCheck;
}

/**
 * What the app makes of an address typed for a link: the address to write (normalised, such as
 * a host in punycode), or why it can't be one. `script` is any scheme but the web's (a script
 * would run, or a file would open); `invalid` is text that isn't an address at all. The app owns
 * the rule, so the editor and the reader's links agree.
 */
export type AddressCheck =
  | { readonly ok: true; readonly address: string }
  | { readonly ok: false; readonly reason: "script" | "invalid" };

/**
 * An email address, loosely: something@something in printable ASCII (as a link's address is
 * stored), with nothing a `mailto:` address could add to.
 */
const EMAIL = /^[!-~]+@[!-~]+$/;
const MAIL_EXTRAS = /[@?&#].*@|@.*[@?&#]/;

/** The parts of a `mailto:` address. */
export function mailParts(uri: string): { address: string; subject: string } {
  const rest = uri.replace(/^mailto:/i, "");
  const [address = "", query = ""] = rest.split("?", 2);
  const subject = new URLSearchParams(query).get("subject") ?? "";
  return { address: decodeURIComponent(address), subject };
}

/** A `mailto:` address for `address` and `subject`. */
export function mailUri(address: string, subject: string): string {
  return subject === "" ? `mailto:${address}` : `mailto:${address}?subject=${encodeURIComponent(subject)}`;
}

/** The editor's fields. */
export function LinkEditor(props: LinkEditorProps): ReactNode {
  const { goal } = props;
  const [kind, setKind] = useState<Kind>(goal === undefined || goal.kind === "script" ? "web" : goal.kind);
  return (
    <div className="paper-link-editor">
      {props.text !== undefined || props.drawn === true ? (
        <p className="paper-link-context">
          {props.text === undefined ? laterMessage("link.area") : laterMessage("link.text", { text: props.text })}
        </p>
      ) : null}
      {goal?.kind === "script" ? <ScriptGoal action={goal.action} /> : null}
      <RadioGroup label={laterMessage("link.goesTo")} value={kind} onChange={(v) => setKind(v as Kind)}>
        <Radio value="page">{laterMessage("link.kind.page")}</Radio>
        <Radio value="named">{laterMessage("link.kind.named")}</Radio>
        <Radio value="web">{laterMessage("link.kind.web")}</Radio>
        <Radio value="email">{laterMessage("link.kind.email")}</Radio>
      </RadioGroup>
      <GoalFields kind={kind} {...props} />
      <TextArea
        label={laterMessage("link.description")}
        description={laterMessage("link.descriptionHelp")}
        defaultValue={props.contents}
        onBlur={(e) => {
          const { value } = e.target;
          if (value !== props.contents) props.onContents(value);
        }}
      />
      <div className="paper-link-actions">
        <Button
          variant="secondary"
          size={30}
          icon="external-link"
          isDisabled={props.onFollow === undefined}
          onPress={() => props.onFollow?.()}
        >
          {laterMessage("link.follow")}
        </Button>
        <Button variant="danger" size={30} icon="delete" onPress={props.onDelete}>
          {laterMessage("link.delete")}
        </Button>
      </div>
    </div>
  );
}

/** The flagged script link (O12): what it runs, in `mono`, and that Lamina PDF never runs it. */
function ScriptGoal({ action }: { action: string }): ReactNode {
  return (
    <div className="paper-link-script">
      <p className="paper-link-script-action">{laterMessage("link.script", { action })}</p>
      <Notice tone="danger" title={laterMessage("link.scriptNotice")} />
    </div>
  );
}

/** The fields of the chosen kind of goal. */
function GoalFields({ kind, ...props }: LinkEditorProps & { kind: Kind }): ReactNode {
  const { goal } = props;
  switch (kind) {
    case "page":
      return (
        <PageField
          pageCount={props.pageCount}
          page={goal?.kind === "page" ? goal.page : undefined}
          onPage={(page) => props.onGoal({ kind: "page", page })}
        />
      );
    case "named":
      return props.names.length === 0 ? (
        <p className="paper-link-hint">{laterMessage("link.noNames")}</p>
      ) : (
        <Select
          label={laterMessage("link.name")}
          options={props.names.map((name) => ({ value: name, label: name }))}
          value={goal?.kind === "named" ? goal.name : ""}
          onChange={(name) => props.onGoal({ kind: "named", name })}
        />
      );
    case "web":
      return (
        <WebField
          address={goal?.kind === "web" ? goal.address : ""}
          check={props.checkAddress}
          onAddress={(address) => props.onGoal({ kind: "web", address })}
        />
      );
    case "email":
      return (
        <EmailFields
          goal={goal?.kind === "email" ? goal : undefined}
          check={props.checkAddress}
          onEmail={(address, subject) => props.onGoal({ kind: "email", address, subject })}
        />
      );
  }
}

/** Calls `commit` with the field's value on Enter and when it loses focus. */
function committing(commit: (value: string) => void) {
  return {
    onBlur: (e: FocusEvent) => commit((e.target as HTMLInputElement).value),
    onKeyDown: (e: KeyboardEvent) => {
      if (e.key === "Enter") commit((e.target as HTMLInputElement).value);
    },
  };
}

function PageField({
  pageCount,
  page,
  onPage,
}: {
  pageCount: number;
  page: number | undefined;
  onPage: (page: number) => void;
}): ReactNode {
  const [error, setError] = useState<string>();
  const commit = (value: string) => {
    const n = Number(value.trim());
    if (!Number.isInteger(n) || n < 1 || n > pageCount) {
      setError(laterMessage("link.pageInvalid", { count: pageCount }));
      return;
    }
    setError(undefined);
    if (n - 1 !== page) onPage(n - 1);
  };
  return (
    <TextInput
      label={laterMessage("link.pageNumber")}
      description={laterMessage("link.pageHelp", { count: pageCount })}
      inputMode="numeric"
      defaultValue={page === undefined ? "" : String(page + 1)}
      {...(error === undefined ? {} : { errorMessage: error })}
      {...committing(commit)}
    />
  );
}

function WebField({
  address,
  check,
  onAddress,
}: {
  address: string;
  check: (address: string) => AddressCheck;
  onAddress: (address: string) => void;
}): ReactNode {
  const [error, setError] = useState<string>();
  const commit = (value: string) => {
    const typed = value.trim();
    if (typed === "" || typed === address) return setError(undefined);
    const checked = /^mailto:/i.test(typed) ? ({ ok: false, reason: "invalid" } as const) : check(typed);
    if (!checked.ok) {
      setError(laterMessage(checked.reason === "script" ? "link.addressRefused" : "link.addressInvalid"));
      return;
    }
    setError(undefined);
    if (checked.address !== address) onAddress(checked.address);
  };
  return (
    <>
      <TextInput
        label={laterMessage("link.address")}
        placeholder="https://"
        inputMode="url"
        defaultValue={address}
        {...(error === undefined ? {} : { errorMessage: error })}
        {...committing(commit)}
      />
      <p className="paper-link-hint">{laterMessage("link.webNote")}</p>
    </>
  );
}

function EmailFields({
  goal,
  check,
  onEmail,
}: {
  goal: { address: string; subject: string } | undefined;
  check: (address: string) => AddressCheck;
  onEmail: (address: string, subject: string) => void;
}): ReactNode {
  const [error, setError] = useState<string>();
  const [typed, setTyped] = useState({ address: goal?.address ?? "", subject: goal?.subject ?? "" });
  const commit = (next: { address: string; subject: string }) => {
    setTyped(next);
    if (next.address === "") return setError(undefined);
    if (!EMAIL.test(next.address) || MAIL_EXTRAS.test(next.address) || !check(mailUri(next.address, next.subject)).ok) {
      setError(laterMessage("link.emailInvalid"));
      return;
    }
    setError(undefined);
    if (next.address !== goal?.address || next.subject !== goal?.subject) onEmail(next.address, next.subject);
  };
  return (
    <>
      <TextInput
        label={laterMessage("link.emailAddress")}
        inputMode="email"
        defaultValue={typed.address}
        {...(error === undefined ? {} : { errorMessage: error })}
        {...committing((v) => commit({ ...typed, address: v.trim() }))}
      />
      <TextInput
        label={laterMessage("link.subject")}
        defaultValue={typed.subject}
        {...committing((v) => commit({ ...typed, subject: v }))}
      />
    </>
  );
}

/** Props of `LinkPanel`. */
export interface LinkPanelProps {
  /** The link's box in the viewport, or null while it is out of sight. */
  readonly anchor: () => Box | null;
  /** Escape, or the panel's own Done: the app closes it. */
  readonly onDismiss: () => void;
  /** The editor. */
  readonly children: ReactNode;
}

/** The gap between the link and the panel. */
const GAP = SPACE["8"];

/**
 * The link editor's panel (O12's inspector, by the link while the reader has no Edit mode): below
 * the link, or above it when there is no room, kept there as the page scrolls. Focus moves into it
 * when it opens; Escape closes it.
 */
export function LinkPanel({ anchor, onDismiss, children }: LinkPanelProps): ReactNode {
  const ref = useRef<HTMLElement>(null);
  const place = useAnchoredPlacement(ref, { anchor, place: placeBelow, gap: GAP });
  useFocusOnOpen(ref);
  // A field commits on blur: closing blurs the one in focus first, so Escape or Done keeps what was typed.
  const dismiss = () => {
    const active = document.activeElement;
    if (active instanceof HTMLElement && ref.current?.contains(active) === true) active.blur();
    onDismiss();
  };
  return createPortal(
    <LayerScope dismiss={dismiss}>
      <section
        ref={ref}
        role="dialog"
        aria-label={laterMessage("link.panel")}
        className="paper-link-panel"
        data-side={place?.side}
        style={place === null ? { visibility: "hidden" } : { top: place.top, left: place.left }}
      >
        <header className="paper-link-panel-header">
          <h2 className="paper-link-panel-title">{laterMessage("link.panel")}</h2>
          <Button variant="ghost" size={30} onPress={dismiss}>
            {laterMessage("link.done")}
          </Button>
        </header>
        {children}
      </section>
    </LayerScope>,
    document.body,
  );
}

/** Puts focus on the panel's first control once, when it opens. */
function useFocusOnOpen(ref: RefObject<HTMLElement | null>): void {
  useEffect(() => {
    ref.current?.querySelector<HTMLElement>("input, textarea, button")?.focus();
  }, [ref]);
}
