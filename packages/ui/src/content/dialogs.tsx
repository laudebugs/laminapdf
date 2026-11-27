// Edit mode's two dialogs (Paper A01, A02): the symbol picker, which puts a special character
// into the text being typed (A.12), and the notice for a file lamina can't place as a picture.
// Both are controlled; the app owns what they ask and does the work.

import { type ReactNode, useEffect, useState } from "react";
import { Button } from "../components/button.js";
import { TextInput } from "../components/field.js";
import { laterMessage } from "../later-messages.js";
import { Dialog } from "../overlays/dialog.js";

/** A symbol the picker offers. */
export interface SymbolChoice {
  readonly character: string;
  /** Its English name, which the search matches and the button's label says. */
  readonly name: string;
}

/** A heading and the symbols under it. */
export interface SymbolSection {
  readonly id: string;
  readonly label: string;
  readonly symbols: readonly SymbolChoice[];
}

/** Props of `SymbolPicker`. */
export interface SymbolPickerProps {
  readonly open: boolean;
  /** The symbols in their groups, shown while the search is empty. */
  readonly sections: readonly SymbolSection[];
  /** The symbols whose character or name matches a query. */
  readonly find: (query: string) => readonly SymbolChoice[];
  /** A symbol was chosen; the dialog stays open so several can be put in. */
  readonly onPick: (character: string) => void;
  /** Done, the close button or Escape. */
  readonly onClose: () => void;
  /** What putting a symbol in costs the first time ("Downloads Noto Sans Symbols 2, 660 KB"); none when nothing does. */
  readonly cost?: string | undefined;
}

/** The picker: a search field, then the groups of symbols, each a button named by what it is. */
export function SymbolPicker({ open, sections, find, onPick, onClose, cost }: SymbolPickerProps): ReactNode {
  const [query, setQuery] = useState("");
  useEffect(() => {
    if (open) setQuery("");
  }, [open]);
  const found = query.trim() === "" ? undefined : find(query);
  return (
    <Dialog
      isOpen={open}
      onOpenChange={(isOpen) => {
        if (!isOpen) onClose();
      }}
      title={laterMessage("content.symbols.title")}
      description={laterMessage("content.symbols.description")}
      closeLabel={laterMessage("content.symbols.close")}
      actions={
        <Button variant="primary" onPress={onClose}>
          {laterMessage("content.symbols.done")}
        </Button>
      }
    >
      <TextInput
        label={laterMessage("content.symbols.search")}
        value={query}
        onChange={setQuery}
        autoFocus
        {...(cost === undefined ? {} : { description: cost })}
      />
      <div className="paper-content-symbols">
        {found !== undefined ? (
          found.length === 0 ? (
            <p className="paper-content-symbols-empty">{laterMessage("content.symbols.none", { query })}</p>
          ) : (
            <SymbolGrid symbols={found} onPick={onPick} />
          )
        ) : (
          sections.map((section) => (
            <section key={section.id} aria-label={section.label}>
              <h3 className="paper-content-symbols-heading">{section.label}</h3>
              <SymbolGrid symbols={section.symbols} onPick={onPick} />
            </section>
          ))
        )}
      </div>
    </Dialog>
  );
}

/** A row of symbol buttons. */
function SymbolGrid({
  symbols,
  onPick,
}: {
  symbols: readonly SymbolChoice[];
  onPick: (character: string) => void;
}): ReactNode {
  return (
    <ul className="paper-content-symbol-grid">
      {symbols.map((symbol) => (
        <li key={symbol.character}>
          <button
            type="button"
            className="paper-content-symbol"
            aria-label={symbol.name}
            title={symbol.name}
            onClick={() => onPick(symbol.character)}
          >
            {symbol.character}
          </button>
        </li>
      ))}
    </ul>
  );
}

/** Props of `UnsupportedImageDialog`. */
export interface UnsupportedImageDialogProps {
  /** The file's name; undefined while nothing was refused. */
  readonly file: string | undefined;
  /** OK, the close button or Escape. */
  readonly onClose: () => void;
}

/** "lamina can’t place this file": no object exists to show an error on, so a small dialog names the formats it takes. */
export function UnsupportedImageDialog({ file, onClose }: UnsupportedImageDialogProps): ReactNode {
  return (
    <Dialog
      isOpen={file !== undefined}
      onOpenChange={(isOpen) => {
        if (!isOpen) onClose();
      }}
      title={laterMessage("content.unsupported.title")}
      description={laterMessage("content.unsupported.body", { file: file ?? "" })}
      closeLabel={laterMessage("content.unsupported.close")}
      actions={
        <Button variant="primary" autoFocus onPress={onClose}>
          {laterMessage("content.unsupported.ok")}
        </Button>
      }
    />
  );
}
