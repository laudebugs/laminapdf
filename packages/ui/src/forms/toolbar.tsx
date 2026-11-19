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

/** Props of `FormToolbar`. */
export interface FormToolbarProps {
  /** Whether the fields are tinted. */
  readonly highlight: boolean;
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

/** Fill & Sign's toolbar. */
export function FormToolbar({ highlight, required }: FormToolbarProps): ReactNode {
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
