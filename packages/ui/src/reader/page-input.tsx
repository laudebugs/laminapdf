// The top bar's page input (§7.1): the current page, or its label, then "/ N". Typing a page and
// pressing Enter goes there; a page that doesn't exist is announced and the field goes back to the
// current page. With page labels, the physical position follows in muted text: "A-1 (22 / 24)".

import { type CSSProperties, type ReactNode, useId, useState } from "react";
import { Input } from "react-aria-components";
import { message } from "../messages.js";
import { useStoreState } from "../state/hooks.js";
import { pageFromInput, type ViewerControls } from "./viewer-controls.js";

/** Props of `PageInput`. */
export interface PageInputProps {
  viewer: ViewerControls;
  /** The document's page labels, one per page, when it has any (C6a reads them). */
  labels?: readonly string[];
  isDisabled?: boolean;
}

/** The current page as an editable field, and the page count. */
export function PageInput({ viewer, labels, isDisabled = false }: PageInputProps): ReactNode {
  const current = useStoreState(viewer, (state) => state.currentPage);
  const total = useStoreState(viewer, (state) => state.pageCount);
  const shown = labels?.[current] ?? String(current + 1);
  // What the user is typing, until Enter, Escape or leaving the field; the current page otherwise.
  const [draft, setDraft] = useState<string | undefined>(undefined);
  const [problem, setProblem] = useState("");
  const help = useId();
  const go = () => {
    if (draft === undefined) return;
    const page = pageFromInput(draft, total, labels);
    setDraft(undefined);
    if (page === undefined) setProblem(message("reader.page.invalid", { value: draft.trim(), total }));
    else {
      setProblem("");
      viewer.goToPage(page);
    }
  };
  const labelled = labels !== undefined && shown !== String(current + 1);
  return (
    <div className="paper-page-input" data-disabled={isDisabled || undefined}>
      <Input
        className="paper-page-input-field"
        aria-label={message("reader.page")}
        aria-describedby={problem === "" ? undefined : help}
        aria-invalid={problem === "" ? undefined : true}
        inputMode={labels === undefined ? "numeric" : "text"}
        disabled={isDisabled}
        value={draft ?? shown}
        // As wide as its text, which reader.css reads: a page number fits the 30 px minimum, a label widens it.
        style={{ "--page-input-chars": (draft ?? shown).length } as CSSProperties}
        onChange={(event) => {
          setDraft(event.target.value);
          setProblem("");
        }}
        onFocus={(event) => event.target.select()}
        onBlur={() => setDraft(undefined)}
        onKeyDown={(event) => {
          if (event.key === "Enter") go();
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
      {/* Announced as it changes; empty until a typed page doesn't exist. */}
      <span id={help} role="status" className="paper-visually-hidden">
        {problem}
      </span>
    </div>
  );
}
