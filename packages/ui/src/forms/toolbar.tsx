// Fill & Sign's toolbar (Paper P01), under the top bar while the mode is on: Previous and Next
// field, Highlight fields, Form data ▾ (import, export, reset and flatten), then the
// required count. Each command's control is its command's view, so it is disabled, with the
// reason, when the command can't run.

import type { ReactNode } from "react";
import { Tooltip, TooltipTrigger } from "react-aria-components";
import { type CommandView, useCommand } from "../commands/react.js";
import { Button } from "../components/button.js";
import { ToolToggle } from "../components/toggle.js";
import { laterMessage } from "../later-messages.js";
import { Menu, type MenuEntry, MenuTrigger } from "../overlays/popover.js";

/** Flat-form placement tools in Fill & Sign's toolbar (Paper P01). */
export type FlatFormTool = "text" | "check" | "date";

/** Props of `FormToolbar`. */
export interface FormToolbarProps {
  /** Whether the fields are tinted. */
  readonly highlight: boolean;
  /** Which flat-form placement tool is active, if any. */
  readonly tool?: FlatFormTool | undefined;
  /** Required fields filled and in all; undefined while the form is read. */
  readonly required: { readonly filled: number; readonly total: number } | undefined;
  /** Whether the form has fields: the Form data menu is off without them. */
  readonly hasFields?: boolean;
}

/** A command's button: its title, and the reason it can't run as a tooltip when it can't. */
function CommandButton({
  view,
  label,
  icon,
}: {
  view: CommandView;
  label: string;
  icon?: "chevron-up" | "chevron-down";
}): ReactNode {
  const disabled = view.availability.state !== "available";
  const button = (
    <Button size={30} isDisabled={disabled} onPress={() => view.run()} {...(icon === undefined ? {} : { icon })}>
      {label}
    </Button>
  );
  if (view.availability.state !== "disabled") return button;
  return (
    <TooltipTrigger>
      {button}
      <Tooltip className="paper-tooltip">{view.availability.reason}</Tooltip>
    </TooltipTrigger>
  );
}

/** The three flat-form placement tools in toolbar order. */
const FLAT_TOOLS = [
  { tool: "text", command: "form.add-text", icon: "add-text", label: "form.addText" },
  { tool: "check", command: "form.add-check", icon: "check", label: "form.addCheck" },
  { tool: "date", command: "form.add-date", icon: "date", label: "form.addDate" },
] as const;

/** A single flat-form placement toggle bound to its command. */
function FlatToolToggle({
  item,
  active,
}: {
  item: (typeof FLAT_TOOLS)[number];
  active: FlatFormTool | undefined;
}): ReactNode {
  const view = useCommand(item.command);
  return (
    <ToolToggle
      icon={item.icon}
      isSelected={active === item.tool}
      isDisabled={view.availability.state !== "available"}
      onChange={() => view.run()}
    >
      {laterMessage(item.label)}
    </ToolToggle>
  );
}

/** The Form data menu's commands in menu order; reset and flatten change the form, so they come last. */
const DATA_ITEMS = [
  { id: "form.import-data", icon: "download" },
  { id: "form.export-data", icon: "upload" },
  { id: "form.reset-fields", icon: "reset" },
  { id: "form.flatten-fields", icon: "flatten" },
] as const;

type DataId = (typeof DATA_ITEMS)[number]["id"];

/** The Form data ▾ menu: import, export, reset and flatten, with its note under the items. */
function FormDataMenu({ hasFields }: { hasFields: boolean }): ReactNode {
  const views: Record<DataId, CommandView> = {
    "form.import-data": useCommand("form.import-data"),
    "form.export-data": useCommand("form.export-data"),
    "form.reset-fields": useCommand("form.reset-fields"),
    "form.flatten-fields": useCommand("form.flatten-fields"),
  };
  const items: MenuEntry<DataId>[] = DATA_ITEMS.map(({ id, icon }) => {
    const { availability, title } = views[id];
    const noForm = !hasFields;
    const reason = availability.state === "disabled" ? availability.reason : undefined;
    return {
      id,
      label: title,
      icon,
      ...(reason !== undefined || noForm
        ? { isDisabled: true, disabledReason: reason ?? laterMessage("form.data.noForm") }
        : {}),
    };
  });
  return (
    <MenuTrigger>
      <Button size={30}>{laterMessage("form.data")}</Button>
      <Menu
        aria-label={laterMessage("form.data")}
        items={items}
        platform="other"
        note={laterMessage("form.data.note")}
        onAction={(id) => views[id].run()}
      />
    </MenuTrigger>
  );
}

/** Fill & Sign's toolbar. */
export function FormToolbar({ highlight, tool, required, hasFields = true }: FormToolbarProps): ReactNode {
  const previous = useCommand("form.focus-previous-field");
  const next = useCommand("form.focus-next-field");
  const tint = useCommand("form.toggle-field-highlight");
  return (
    <div className="paper-form-toolbar" role="toolbar" aria-label={laterMessage("form.toolbar")}>
      <CommandButton view={previous} label={laterMessage("form.previous")} icon="chevron-up" />
      <CommandButton view={next} label={laterMessage("form.next")} icon="chevron-down" />
      <ToolToggle
        icon="eye"
        isSelected={highlight}
        isDisabled={tint.availability.state !== "available"}
        onChange={() => tint.run()}
      >
        {laterMessage("form.highlight")}
      </ToolToggle>
      <span className="paper-form-toolbar-sep" aria-hidden="true" />
      {FLAT_TOOLS.map((item) => (
        <FlatToolToggle key={item.tool} item={item} active={tool} />
      ))}
      <span className="paper-form-toolbar-sep" aria-hidden="true" />
      <FormDataMenu hasFields={hasFields} />
      {required === undefined || required.total === 0 ? null : (
        <span className="paper-form-required" role="status">
          {laterMessage("form.required.count", required)}
        </span>
      )}
    </div>
  );
}
