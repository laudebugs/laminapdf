// Fill & Sign's toolbar (Paper P01), under the top bar while the mode is on: Previous and Next
// field, Highlight fields, Form data ▾ (a disabled placeholder until its menu comes), then the
// required count. Each command's control is its command's view, so it is disabled, with the
// reason, when the command can't run.

import type { ReactNode } from "react";
import { Tooltip, TooltipTrigger } from "react-aria-components";
import { type CommandView, useCommand } from "../commands/react.js";
import { Button } from "../components/button.js";
import { ToolToggle } from "../components/toggle.js";
import { laterMessage } from "../later-messages.js";

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

/** Fill & Sign's toolbar. */
export function FormToolbar({ highlight, tool, required }: FormToolbarProps): ReactNode {
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
      <TooltipTrigger>
        <Button size={30} isDisabled>
          {laterMessage("form.data")}
        </Button>
        <Tooltip className="paper-tooltip">{laterMessage("form.data.later")}</Tooltip>
      </TooltipTrigger>
      {required === undefined || required.total === 0 ? null : (
        <span className="paper-form-required" role="status">
          {laterMessage("form.required.count", required)}
        </span>
      )}
    </div>
  );
}
