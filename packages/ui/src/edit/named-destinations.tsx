// The named destinations dialog (O11, third artboard): the places in the file that links and
// bookmarks point to by name, each with where it goes and how many bookmarks use it; rename one in
// place (Enter keeps, Escape cancels), delete one (asking first when bookmarks use it), or add one
// for the current view. Each change is one op the app applies, so each is one undo step.

import { type ReactNode, useState } from "react";
import { Input } from "react-aria-components";
import { Button, IconButton } from "../components/button.js";
import { TextInput } from "../components/field.js";
import { laterCountMessage, laterMessage } from "../later-messages.js";
import { Dialog } from "../overlays/dialog.js";

/** One named destination, as the dialog lists it. */
export interface NamedDestinationRow {
  readonly name: string;
  /** The label of the page it goes to, or nothing when it names no page here. */
  readonly page?: string;
  /** How many bookmarks of the file go to it. */
  readonly usedBy: number;
}

/** Props of `NamedDestinationsDialog`. */
export interface NamedDestinationsDialogProps {
  readonly isOpen: boolean;
  readonly onOpenChange: (isOpen: boolean) => void;
  readonly destinations: readonly NamedDestinationRow[];
  readonly onGoTo: (name: string) => void;
  readonly onRename: (from: string, to: string) => void;
  readonly onDelete: (name: string) => void;
  /** Add `name` for the current view. */
  readonly onAdd: (name: string) => void;
}

/** One row's name: as text, or the rename field. */
function NameCell({
  row,
  taken,
  renaming,
  onEnd,
}: {
  row: NamedDestinationRow;
  taken: (name: string) => boolean;
  renaming: boolean;
  onEnd: (to: string | undefined) => void;
}): ReactNode {
  const [value, setValue] = useState(row.name);
  if (!renaming) return <code className="paper-named-name">{row.name}</code>;
  const clash = value !== row.name && taken(value);
  return (
    <span className="paper-named-rename">
      <Input
        className="paper-named-input"
        aria-label={laterMessage("named.renameField", { name: row.name })}
        aria-invalid={clash || undefined}
        value={value}
        autoFocus
        onChange={(e) => setValue(e.target.value)}
        onKeyDown={(e) => {
          if (e.key === "Enter" && value.trim() !== "" && !clash) onEnd(value === row.name ? undefined : value);
          if (e.key === "Escape") {
            // Escape cancels the rename, not the dialog.
            e.stopPropagation();
            onEnd(undefined);
          }
        }}
      />
      <span className="paper-named-hint">{clash ? laterMessage("named.taken") : laterMessage("named.renameHint")}</span>
    </span>
  );
}

/** One destination's row: its name (or the rename field), where it goes, its uses, Rename and Delete. */
function Row({
  row,
  props,
  renaming,
  setRenaming,
  remove,
}: {
  row: NamedDestinationRow;
  props: NamedDestinationsDialogProps;
  renaming: boolean;
  setRenaming: (name: string | undefined) => void;
  remove: (row: NamedDestinationRow) => void;
}): ReactNode {
  const taken = (name: string) => props.destinations.some((d) => d.name === name);
  return (
    <tr>
      <td>
        <NameCell
          row={row}
          taken={taken}
          renaming={renaming}
          onEnd={(to) => {
            setRenaming(undefined);
            if (to !== undefined) props.onRename(row.name, to);
          }}
        />
      </td>
      <td>
        {row.page === undefined ? (
          "—"
        ) : (
          <Button variant="ghost" size={26} onPress={() => props.onGoTo(row.name)}>
            {laterMessage("bookmarks.page", { page: row.page })}
          </Button>
        )}
      </td>
      <td>
        {laterCountMessage(row.usedBy, {
          zero: "named.usedBy.none",
          one: "named.usedBy.one",
          many: "named.usedBy.many",
        })}
      </td>
      <td className="paper-named-actions">
        <IconButton
          icon="rename"
          label={laterMessage("named.rename", { name: row.name })}
          onPress={() => setRenaming(row.name)}
        />
        <IconButton
          icon="delete"
          label={laterMessage("named.delete", { name: row.name })}
          onPress={() => remove(row)}
        />
      </td>
    </tr>
  );
}

/** The table of destinations, or a line saying there are none. */
function Table({
  props,
  remove,
}: {
  props: NamedDestinationsDialogProps;
  remove: (row: NamedDestinationRow) => void;
}): ReactNode {
  const [renaming, setRenaming] = useState<string | undefined>(undefined);
  if (props.destinations.length === 0) return <p>{laterMessage("named.none")}</p>;
  return (
    <table className="paper-named-table">
      <thead>
        <tr>
          <th scope="col">{laterMessage("named.name")}</th>
          <th scope="col">{laterMessage("named.goesTo")}</th>
          <th scope="col">{laterMessage("named.usedBy")}</th>
          <th scope="col">
            <span className="paper-visually-hidden">{laterMessage("named.actions")}</span>
          </th>
        </tr>
      </thead>
      <tbody>
        {props.destinations.map((row) => (
          <Row
            key={row.name}
            row={row}
            props={props}
            renaming={renaming === row.name}
            setRenaming={setRenaming}
            remove={remove}
          />
        ))}
      </tbody>
    </table>
  );
}

/** A new name, added for the current view. */
function AddName({ props }: { props: NamedDestinationsDialogProps }): ReactNode {
  const [adding, setAdding] = useState("");
  const name = adding.trim();
  const usable = name !== "" && !props.destinations.some((d) => d.name === name);
  return (
    <div className="paper-named-add">
      <TextInput label={laterMessage("named.newName")} value={adding} onChange={setAdding} />
      <Button
        icon="bookmark"
        isDisabled={!usable}
        onPress={() => {
          props.onAdd(name);
          setAdding("");
        }}
      >
        {laterMessage("named.add")}
      </Button>
    </div>
  );
}

/** Asks before deleting a name bookmarks use. */
function ConfirmDelete({
  row,
  onAnswer,
}: {
  row: NamedDestinationRow | undefined;
  onAnswer: (remove: boolean) => void;
}): ReactNode {
  const description =
    row === undefined
      ? ""
      : row.usedBy === 0
        ? laterMessage("named.confirmDelete.none", { name: row.name })
        : row.usedBy === 1
          ? laterMessage("named.confirmDelete.one", { name: row.name })
          : laterMessage("named.confirmDelete", { name: row.name, count: row.usedBy });
  return (
    <Dialog
      isOpen={row !== undefined}
      onOpenChange={() => onAnswer(false)}
      role="alertdialog"
      title={laterMessage("named.deleteName")}
      description={description}
      closeLabel={laterMessage("named.cancel")}
      actions={
        <>
          <Button autoFocus onPress={() => onAnswer(false)}>
            {laterMessage("named.cancel")}
          </Button>
          <Button variant="danger-filled" onPress={() => onAnswer(true)}>
            {laterMessage("named.deleteName")}
          </Button>
        </>
      }
    />
  );
}

/** The named destinations dialog. */
export function NamedDestinationsDialog(props: NamedDestinationsDialogProps): ReactNode {
  const [confirming, setConfirming] = useState<NamedDestinationRow | undefined>(undefined);
  // Always asked: links, which the count leaves out, and other files may use the name too.
  const remove = (row: NamedDestinationRow) => setConfirming(row);
  return (
    <>
      <Dialog
        isOpen={props.isOpen && confirming === undefined}
        onOpenChange={props.onOpenChange}
        title={laterMessage("named.title")}
        description={laterMessage("named.intro")}
        closeLabel={laterMessage("named.close")}
        size="form"
        actions={
          <Button variant="primary" onPress={() => props.onOpenChange(false)}>
            {laterMessage("named.done")}
          </Button>
        }
      >
        <Table props={props} remove={remove} />
        <AddName props={props} />
      </Dialog>
      <ConfirmDelete
        row={confirming}
        onAnswer={(yes) => {
          if (yes && confirming !== undefined) props.onDelete(confirming.name);
          setConfirming(undefined);
        }}
      />
    </>
  );
}
