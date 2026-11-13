// The desktop app's own commands: tabs, windows and quitting. A module of its own, which only the
// desktop app imports, so the web reader's startup carries none of it (#380).

import { laterMessage } from "../later-messages.js";
import { message } from "../messages.js";
import { delegated, desktop, documentOpen, registerAll, title } from "./reader-commands.js";
import type { CommandRegistry } from "./registry.js";
import type { AnyCommand, CommandContext } from "./types.js";

/**
 * `true` while the window shows a file (open, opening, asking for its password, or failed to
 * open), else why not: a tab can close, and the tabs can be stepped through, whatever its
 * document's state. Moving a tab needs its document open.
 */
function fileShown(ctx: CommandContext): true | string {
  const shown = ctx.reader === undefined ? ctx.viewer !== undefined : ctx.reader.getState().phase !== "empty";
  return shown ? true : message("command.reason.noDocument");
}

/**
 * Tabs, windows and quitting, for shells with more than one window (the desktop app), which
 * register them with {@link registerDesktopCommands}: kept out of {@link readerCommands} so the
 * web reader doesn't load them. Their homes are the tab strip and the native menus.
 */
export function desktopCommands(): AnyCommand[] {
  return [
    delegated({
      id: "tab.close",
      title: title("tab.close"),
      keywords: ["close", "tab", "document"],
      group: "file",
      shortcuts: [desktop("Mod+W")],
      home: ["tab-strip", "native-menu"],
      enabled: fileShown,
    }),
    delegated({
      id: "tab.next",
      title: title("tab.next"),
      keywords: ["tab", "next", "switch"],
      group: "navigate",
      shortcuts: [desktop("Ctrl+Tab")],
      home: ["tab-strip"],
      enabled: fileShown,
    }),
    delegated({
      id: "tab.previous",
      title: title("tab.previous"),
      keywords: ["tab", "previous", "switch"],
      group: "navigate",
      shortcuts: [desktop("Ctrl+Shift+Tab")],
      home: ["tab-strip"],
      enabled: fileShown,
    }),
    delegated({
      id: "tab.move-to-new-window",
      title: title("tab.move-to-new-window"),
      keywords: ["tab", "window", "move", "detach"],
      group: "file",
      home: ["tab-strip"],
      enabled: documentOpen,
    }),
    delegated({
      id: "window.new",
      title: title("window.new"),
      keywords: ["window", "new"],
      group: "file",
      shortcuts: [desktop("Mod+Shift+N")],
      home: ["tab-strip", "native-menu"],
    }),
    delegated({
      id: "window.close",
      title: laterMessage("command.window.close"),
      keywords: ["window", "close"],
      group: "file",
      shortcuts: [desktop("Mod+Shift+W")],
      home: ["native-menu"],
    }),
    // macOS quits from the app menu's own Quit (⌘Q), which the OS provides.
    delegated({
      id: "app.quit",
      title: laterMessage("command.app.quit"),
      keywords: ["quit", "exit", "close"],
      group: "app",
      shortcuts: [{ ...desktop("Ctrl+Q"), on: "other" }],
      home: ["native-menu"],
    }),
  ];
}

/** Registers the desktop's commands in `registry`; returns the function that removes them. */
export function registerDesktopCommands(registry: CommandRegistry): () => void {
  return registerAll(registry, desktopCommands());
}
