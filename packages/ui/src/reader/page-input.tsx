// The top bar's page input (§7.1): the current page, or its label, then "/ N". Typing a page and
// pressing Enter goes there; a page that doesn't exist is announced and the field goes back to the
// current page. With page labels, the physical position follows in muted text: "A-1 (22 / 24)".
// While the document is locked (R03) the field shows, disabled, before there are pages to count.

import {
  type CSSProperties,
  type InputHTMLAttributes,
  type ReactNode,
  useEffect,
  useId,
  useRef,
  useState,
} from "react";
import { Input } from "react-aria-components";
import { useCommand } from "../commands/react.js";
import { message } from "../messages.js";
import { useStoreState } from "../state/hooks.js";
import { pageFromInput, type ViewerControls } from "./viewer-controls.js";

/** Props of `PageInput`. */
export interface PageInputProps {
  /** The viewer, whose page the field shows: the commands' context's viewer, where `nav.go-to-page` goes. */
  viewer: ViewerControls;
  /** The document's page labels, one per page, when it has any (read by the reader once it shows page labels). */
  labels?: readonly string[];
}

/** A typed page that doesn't exist, numbered so the same mistake twice is announced twice. */
interface Problem {
  readonly text: string;
  readonly count: number;
}

/** The current page as an editable field, and the page count. */
export function PageInput({ viewer, labels }: PageInputProps): ReactNode {
  const goToPage = useCommand("nav.go-to-page");
  const current = useStoreState(viewer, (state) => state.currentPage);
  const total = useStoreState(viewer, (state) => state.pageCount);
  const shown = labels?.[current] ?? String(current + 1);
  // What the user is typing, until Enter, Escape or leaving the field; the current page otherwise.
  const [draft, setDraft] = useState<string | undefined>(undefined);
  const [problem, setProblem] = useState<Problem | undefined>(undefined);
  const help = useId();
  const field = useRef<HTMLInputElement>(null);
  const keepSelection = useSelectionKeptOnClick();
  // After a mistake the field shows the current page again, selected, so typing replaces it.
  useEffect(() => {
    if (problem !== undefined) field.current?.select();
  }, [problem]);
  // A page reached any other way ends what an earlier typed one got wrong.
  // biome-ignore lint/correctness/useExhaustiveDependencies: runs when the current page changes
  useEffect(() => setProblem(undefined), [current]);
  const go = () => {
    if (draft === undefined) return;
    setDraft(undefined);
    const typed = draft.trim();
    if (typed === "") return setProblem(undefined);
    const page = pageFromInput(typed, total, labels);
    if (page !== undefined) {
      setProblem(undefined);
      return goToPage.run(page);
    }
    setProblem((last) => ({
      text: message("reader.page.invalid", { value: typed, total }),
      count: (last?.count ?? 0) + 1,
    }));
  };
  const labelled = labels !== undefined && shown !== String(current + 1);
  return (
    <div className="paper-page-input">
      <Input
        ref={field}
        className="paper-page-input-field"
        aria-label={message("reader.page")}
        aria-describedby={problem === undefined ? undefined : help}
        aria-invalid={problem === undefined ? undefined : true}
        inputMode={labels === undefined ? "numeric" : "text"}
        value={draft ?? shown}
        style={width(draft ?? shown)}
        onChange={(event) => {
          setDraft(event.target.value);
          setProblem(undefined);
        }}
        onFocus={(event) => event.target.select()}
        {...keepSelection}
        onBlur={() => {
          setDraft(undefined);
          setProblem(undefined);
        }}
        onKeyDown={(event) => {
          // Enter that confirms an input method's composition isn't a jump.
          if (event.key === "Enter" && !event.nativeEvent.isComposing) go();
          if (event.key === "Escape" && draft !== undefined) {
            event.stopPropagation();
            setDraft(undefined);
          }
        }}
      />
      <span className="paper-page-input-total">
        {labelled
          ? message("reader.page.position", { page: current + 1, total })
          : message("reader.page.total", { total })}
      </span>
      {/* A new node for each mistake, so screen readers announce a repeated one. */}
      <span id={help} role="status" className="paper-visually-hidden">
        {problem === undefined ? null : <span key={problem.count}>{problem.text}</span>}
      </span>
    </div>
  );
}

/**
 * Handlers that keep a field's text selected after the click that focuses it. Safari clears the
 * selection on the mouseup that ends that click, so typing would add to the page instead of
 * replacing it: that one mouseup is cancelled.
 */
function useSelectionKeptOnClick(): Pick<InputHTMLAttributes<HTMLInputElement>, "onPointerDown" | "onMouseUp"> {
  const focusing = useRef(false);
  return {
    onPointerDown: (event) => {
      focusing.current = document.activeElement !== event.currentTarget;
    },
    onMouseUp: (event) => {
      if (focusing.current) event.preventDefault();
      focusing.current = false;
    },
  };
}

/** The page input of a locked document: disabled but announced, before there are pages to count. */
export function LockedPageInput(): ReactNode {
  return (
    <div className="paper-page-input">
      <input
        className="paper-page-input-field"
        aria-label={message("reader.page")}
        aria-disabled="true"
        readOnly
        value=""
        style={width("")}
      />
      <span className="paper-page-input-total">{message("reader.page.unknown")}</span>
    </div>
  );
}

/** As wide as its text, which reader.css reads: a page number fits the 30 px minimum, a label widens it. */
function width(text: string): CSSProperties {
  return { "--page-input-chars": text.length } as CSSProperties;
}
