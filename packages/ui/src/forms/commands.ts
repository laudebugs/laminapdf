// Fill & Sign's commands (Paper P01 and P03, design language §10): the mode, moving between fields in
// the form's order, the field tint, the Fields tab and the Form data menu's four commands. Each is
// delegated: Fill & Sign registers its handler while the mode is on, so the palette lists them with a
// reason otherwise. Tab and Shift Tab move between fields inside the form layer itself; their
// bindings here are shown, never dispatched.

import type { IconName } from "@laminapdf/icons";
import { delegatedCommand } from "../commands/delegated.js";
import type { CommandRegistry } from "../commands/registry.js";
import type { Command, CommandHome, CommandId, Shortcut } from "../commands/types.js";
import { laterMessage } from "../later-messages.js";

/** A delegated command of Fill & Sign. */
function formCommand(
  id: CommandId,
  extra: {
    icon?: IconName;
    shortcuts?: Shortcut[];
    home: CommandHome[];
    group?: Command["group"];
    keywords?: readonly string[];
  },
): Command<undefined> {
  return delegatedCommand(id, {
    ...extra,
    keywords: ["form", "fill", "field", ...(extra.keywords ?? [])],
    noSession: "form.unavailable",
    notInMode: "form.inFillSign",
  });
}

/** Every command Fill & Sign registers. */
export function formCommands(): Command<undefined>[] {
  return [
    {
      id: "mode.fill-sign",
      title: laterMessage("command.mode.fill-sign"),
      keywords: ["form", "fill", "sign", "fields"],
      group: "view",
      icon: "text-field",
      delegated: true,
      home: ["mode-switcher"],
      run: (ctx) => ctx.actions?.["mode.fill-sign"]?.(undefined),
    },
    formCommand("form.focus-next-field", {
      icon: "chevron-down",
      home: ["form-toolbar"],
      shortcuts: [{ mac: "Tab", other: "Tab", scope: "page", dispatch: false }],
    }),
    formCommand("form.focus-previous-field", {
      icon: "chevron-up",
      home: ["form-toolbar"],
      shortcuts: [{ mac: "Shift+Tab", other: "Shift+Tab", scope: "page", dispatch: false }],
    }),
    formCommand("form.focus-next-required-field", { home: ["form-toolbar"] }),
    formCommand("form.toggle-field-highlight", { home: ["form-toolbar"], group: "view" }),
    formCommand("form.add-text", { icon: "add-text", home: ["form-toolbar"], keywords: ["flat", "text"] }),
    formCommand("form.add-check", { icon: "check", home: ["form-toolbar"], keywords: ["flat", "check", "tick"] }),
    formCommand("form.add-date", { icon: "date", home: ["form-toolbar"], keywords: ["flat", "date", "today"] }),
    formCommand("sign.add-signature-image", {
      icon: "signature-image",
      home: ["form-toolbar"],
      keywords: ["signature", "sign", "image", "picture", "handwritten", "draw", "type"],
    }),
    formCommand("form.import-data", {
      icon: "download",
      home: ["form-toolbar"],
      keywords: ["data", "json", "xfdf", "fdf"],
    }),
    formCommand("form.export-data", {
      icon: "upload",
      home: ["form-toolbar"],
      keywords: ["data", "json", "xfdf", "fdf"],
    }),
    formCommand("form.reset-fields", {
      icon: "reset",
      home: ["form-toolbar"],
      keywords: ["clear", "empty", "default"],
    }),
    formCommand("form.flatten-fields", {
      icon: "flatten",
      home: ["form-toolbar"],
      keywords: ["print", "burn", "lock"],
    }),
    formCommand("view.show-fields-tab", { home: ["form-toolbar"], group: "view" }),
  ];
}

/** Registers Fill & Sign's commands in `registry`; returns the function that removes them. */
export function registerFormCommands(registry: CommandRegistry): () => void {
  const removers = formCommands().map((command) => registry.register(command as Command<unknown>));
  return () => {
    for (const remove of removers) remove();
  };
}
