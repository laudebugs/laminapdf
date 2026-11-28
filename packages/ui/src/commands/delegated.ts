// A command a mode delegates (ADR 0024): its handler is registered while the mode is on, so the
// palette lists it with a reason when it isn't. Comment mode's commands and Edit mode's are made
// here; they differ in their words and keywords only.

import type { IconName } from "@laminapdf/icons";
import { blockedReason } from "../edit/outcome.js";
import { type LaterMessageId, laterMessage } from "../later-messages.js";
import type { ArgsSchema, Command, CommandContext, CommandHome, CommandId, Shortcut } from "./types.js";

/** What a mode's delegated command is made of. */
export interface DelegatedOptions<Args> {
  /** What the palette searches besides the title. */
  readonly keywords: readonly string[];
  /** Why the command can't run without an edit session ("Open a document first"). */
  readonly noSession: LaterMessageId;
  /** Why it can't run while its mode isn't the one on ("Switch to Edit mode first"). */
  readonly notInMode: LaterMessageId;
  readonly icon?: IconName | undefined;
  readonly shortcuts?: Shortcut[] | undefined;
  readonly args?: ArgsSchema<Args> | undefined;
  readonly home: CommandHome[];
  /** The palette's section; "edit" when left out. */
  readonly group?: Command["group"];
  /** The command only reads the document, so a blocked session still runs it. */
  readonly readOnly?: boolean;
}

/** Whether the session can take an edit now, and the shell gave command `id` a handler (its mode is on). */
function delegatedAvailable(
  id: CommandId,
  options: Pick<DelegatedOptions<unknown>, "noSession" | "notInMode" | "readOnly">,
) {
  return (ctx: CommandContext): true | string => {
    if (ctx.edits === undefined) return laterMessage(options.noSession);
    const blocked = options.readOnly === true ? undefined : blockedReason(ctx.edits.getState().blocked);
    if (blocked !== undefined) return blocked;
    return ctx.actions?.[id] === undefined ? laterMessage(options.notInMode) : true;
  };
}

/** The delegated command `id`, titled by the later catalogue's `command.<id>`. */
export function delegatedCommand<Args>(id: CommandId, options: DelegatedOptions<Args>): Command<Args> {
  return {
    id,
    title: laterMessage(`command.${id}` as LaterMessageId),
    keywords: options.keywords,
    group: options.group ?? "edit",
    ...(options.icon === undefined ? {} : { icon: options.icon }),
    ...(options.shortcuts === undefined ? {} : { shortcuts: options.shortcuts }),
    ...(options.args === undefined ? {} : { args: options.args }),
    delegated: true,
    home: options.home,
    enabled: delegatedAvailable(id, options),
    run: (ctx, args) => ctx.actions?.[id]?.(args),
  };
}
