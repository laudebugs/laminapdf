// The reader v0.1 command set (ADR 0024's appendix): 39 commands, with the design language's
// shortcuts (§8.8) and each one's visible home. Commands the reader and the viewer can do run
// here; the rest are delegated to the shell, which gives a handler in `CommandContext.actions`
// once it has the feature (find, print, the sidebar), and stays out of every view until then.

import { message } from "../messages.js";
import {
  MAX_ZOOM,
  MIN_ZOOM,
  type ViewerControls,
  type ViewerSlice,
  type Zoom,
  zoomIn,
  zoomOut,
} from "../reader/viewer-controls.js";
import { choiceArgs, pageArgs, rangeArgs, urlArgs, zoomArgs } from "./args.js";
import type { CommandRegistry } from "./registry.js";
import type { AnyCommand, Command, CommandContext, CommandId, Shortcut, ShortcutScope } from "./types.js";

/** A binding with the same keys on macOS and elsewhere, or `mac` and `other` apart. */
function keys(scope: ShortcutScope, mac: string, other = mac, extra: Partial<Shortcut> = {}): Shortcut {
  return { mac, other, scope, ...extra };
}

/** A binding only the desktop app takes: the browser keeps these keys on the web. */
const desktop = (mac: string, other = mac) => keys("global", mac, other, { requires: "ownsKeyboard" });

/** `true` with a document open (by the reader's state, else by a viewer being there), else why not. */
function documentOpen(ctx: CommandContext): true | string {
  const open = ctx.reader === undefined ? ctx.viewer !== undefined : ctx.reader.getState().phase === "open";
  return open ? true : message("command.reason.noDocument");
}

/** `documentOpen`, then `check` on the viewer's state. */
function withViewer(check: (state: ViewerSlice) => true | string) {
  return (ctx: CommandContext) => {
    const open = documentOpen(ctx);
    if (open !== true) return open;
    return ctx.viewer === undefined ? message("command.reason.noDocument") : check(ctx.viewer.getState());
  };
}

/** A document open in the viewer: what the commands that drive the viewer need. */
const viewerOpen = withViewer(() => true);

/** A command the shell does: its handler in `ctx.actions`, given the parsed arguments. */
function delegated<Args = undefined>(
  command: Omit<Command<Args>, "run" | "delegated"> & { run?: never },
): Command<Args> {
  return { ...command, delegated: true, run: (ctx, args) => ctx.actions?.[command.id]?.(args) };
}

/** Runs `act` on the viewer and its state, when there is a viewer. */
const viewerRun = (act: (viewer: ViewerControls, state: ViewerSlice) => void) => (ctx: CommandContext) => {
  if (ctx.viewer !== undefined) act(ctx.viewer, ctx.viewer.getState());
};

const title = (id: CommandId) => message(`command.${id}` as Parameters<typeof message>[0]);

/** The reader's commands, built in the catalogue's words. */
/** Opening files and unlocking them. */
function fileOpenCommands(): AnyCommand[] {
  return [
    {
      id: "file.open",
      title: title("file.open"),
      keywords: ["file", "pdf"],
      group: "file",
      icon: "folder",
      shortcuts: [keys("global", "Mod+O")],
      home: ["home", "file-menu"],
      enabled: (ctx) => (ctx.reader === undefined ? message("command.reason.noReader") : true),
      run: (ctx) => ctx.reader?.openFromPicker(),
    },
    delegated({
      id: "file.open-url",
      title: title("file.open-url"),
      keywords: ["link", "web"],
      group: "file",
      icon: "link",
      requires: "canOpenUrl",
      home: ["home"],
      args: urlArgs(),
    }),
    delegated({
      id: "file.open-recent",
      title: title("file.open-recent"),
      group: "file",
      icon: "clock",
      requires: "canReopenRecent",
      home: ["home", "file-menu"],
    }),
    delegated({ id: "file.clear-recent", title: title("file.clear-recent"), group: "file", home: ["home"] }),
    delegated({
      id: "file.clear-reading-positions",
      title: title("file.clear-reading-positions"),
      keywords: ["forget", "position", "page", "privacy"],
      group: "file",
      home: ["settings"],
    }),
    {
      id: "file.unlock",
      title: title("file.unlock"),
      keywords: ["password", "decrypt"],
      group: "file",
      icon: "key",
      home: ["password-prompt"],
      args: {
        prompt: message("command.reason.passwordPrompt"),
        secret: true,
        parse: () => ({ ok: false, error: message("command.reason.passwordPrompt") }),
      },
      enabled: (ctx) =>
        ctx.reader?.getState().phase === "needs-password" ? true : message("command.reason.notLocked"),
      run: (ctx, password: string) => ctx.reader?.submitPassword(password),
    } satisfies Command<string>,
  ];
}

/** What can be done with the open file. */
function fileDocumentCommands(): AnyCommand[] {
  return [
    delegated({
      id: "file.save-copy",
      title: (ctx) =>
        message(ctx.platform.capabilities.canSaveAsDialog ? "command.file.save-copy.save" : "command.file.save-copy"),
      keywords: ["download", "export"],
      group: "file",
      icon: "download",
      home: ["file-menu"],
      enabled: documentOpen,
    }),
    delegated<readonly number[]>({
      id: "file.print",
      title: title("file.print"),
      group: "file",
      icon: "printer",
      shortcuts: [keys("global", "Mod+P")],
      home: ["file-menu"],
      args: rangeArgs(),
      enabled: documentOpen,
    }),
    delegated({
      id: "file.properties",
      title: title("file.properties"),
      keywords: ["info", "metadata"],
      group: "file",
      icon: "info",
      home: ["file-menu"],
      enabled: documentOpen,
    }),
    delegated({
      id: "file.show-repairs",
      title: title("file.show-repairs"),
      keywords: ["damaged", "repaired"],
      group: "file",
      icon: "repaired",
      home: ["status-bar"],
      enabled: documentOpen,
    }),
  ];
}

/** Moving between pages. */
function pageCommands(): AnyCommand[] {
  return [
    {
      id: "nav.next-page",
      title: title("nav.next-page"),
      group: "navigate",
      icon: "chevron-down",
      shortcuts: [keys("page", "PageDown")],
      home: ["page-indicator"],
      enabled: withViewer((s) => (s.currentPage < s.pageCount - 1 ? true : message("command.reason.lastPage"))),
      run: viewerRun((viewer, s) => viewer.goToPage(s.currentPage + 1)),
    },
    {
      id: "nav.previous-page",
      title: title("nav.previous-page"),
      group: "navigate",
      icon: "chevron-up",
      shortcuts: [keys("page", "PageUp")],
      home: ["page-indicator"],
      enabled: withViewer((s) => (s.currentPage > 0 ? true : message("command.reason.firstPage"))),
      run: viewerRun((viewer, s) => viewer.goToPage(s.currentPage - 1)),
    },
    {
      id: "nav.first-page",
      title: title("nav.first-page"),
      group: "navigate",
      shortcuts: [keys("page", "Home")],
      home: ["view-menu"],
      enabled: withViewer((s) => (s.currentPage > 0 ? true : message("command.reason.firstPage"))),
      // The first and last page are jumps, as going to a page is; the next and previous are steps.
      run: viewerRun((viewer) => viewer.jumpToPage(0)),
    },
    {
      id: "nav.last-page",
      title: title("nav.last-page"),
      group: "navigate",
      shortcuts: [keys("page", "End")],
      home: ["view-menu"],
      enabled: withViewer((s) => (s.currentPage < s.pageCount - 1 ? true : message("command.reason.lastPage"))),
      run: viewerRun((viewer, s) => viewer.jumpToPage(s.pageCount - 1)),
    },
    {
      id: "nav.go-to-page",
      title: title("nav.go-to-page"),
      keywords: ["jump", "page"],
      group: "navigate",
      home: ["page-indicator"],
      args: pageArgs(),
      enabled: viewerOpen,
      // Going to a page is a jump: Back returns to where the reader was.
      run: (ctx, page: number) => ctx.viewer?.jumpToPage(page),
    } satisfies Command<number>,
  ];
}

/** History, and stepping the zoom. */
function historyAndZoomCommands(): AnyCommand[] {
  return [
    delegated({
      id: "nav.back",
      title: title("nav.back"),
      group: "navigate",
      icon: "back",
      shortcuts: [desktop("Mod+[", "Alt+ArrowLeft")],
      home: ["view-menu"],
      enabled: documentOpen,
    }),
    delegated({
      id: "nav.forward",
      title: title("nav.forward"),
      group: "navigate",
      icon: "forward",
      shortcuts: [desktop("Mod+]", "Alt+ArrowRight")],
      home: ["view-menu"],
      enabled: documentOpen,
    }),
    // Its homes are the link itself and the link preview's sheet (R07); Enter on a link follows it.
    delegated({
      id: "nav.follow-link",
      title: title("nav.follow-link"),
      group: "navigate",
      home: ["layer"],
      enabled: (ctx) => {
        const open = documentOpen(ctx);
        if (open !== true) return open;
        return ctx.viewer?.canFollowLink?.() === true ? true : message("command.reason.noLink");
      },
    }),
    {
      id: "view.zoom-in",
      title: title("view.zoom-in"),
      keywords: ["bigger", "magnify"],
      group: "view",
      icon: "zoom-in",
      shortcuts: [keys("page", "+"), desktop("Mod++")],
      home: ["zoom-control"],
      enabled: withViewer((s) => (s.percent < MAX_ZOOM ? true : message("command.reason.maxZoom"))),
      run: viewerRun((viewer, s) => viewer.setZoom(zoomIn(s.percent))),
    },
    {
      id: "view.zoom-out",
      title: title("view.zoom-out"),
      keywords: ["smaller"],
      group: "view",
      icon: "zoom-out",
      shortcuts: [keys("page", "-"), desktop("Mod+-")],
      home: ["zoom-control"],
      enabled: withViewer((s) => (s.percent > MIN_ZOOM ? true : message("command.reason.minZoom"))),
      run: viewerRun((viewer, s) => viewer.setZoom(zoomOut(s.percent))),
    },
  ];
}

/** Setting the zoom, and rotating the view. */
function zoomModeCommands(): AnyCommand[] {
  return [
    {
      id: "view.set-zoom",
      title: title("view.set-zoom"),
      keywords: ["percent", "magnification"],
      group: "view",
      home: ["zoom-control"],
      args: zoomArgs(),
      enabled: viewerOpen,
      run: (ctx, zoom: Zoom) => ctx.viewer?.setZoom(zoom),
    } satisfies Command<Zoom>,
    {
      id: "view.actual-size",
      title: title("view.actual-size"),
      keywords: ["100%", "zoom"],
      group: "view",
      shortcuts: [keys("page", "0"), desktop("Mod+0")],
      home: ["zoom-control"],
      enabled: viewerOpen,
      run: viewerRun((viewer) => viewer.setZoom("actual-size")),
    },
    {
      id: "view.fit-width",
      title: title("view.fit-width"),
      keywords: ["zoom"],
      group: "view",
      icon: "fit-width",
      home: ["zoom-control"],
      enabled: viewerOpen,
      run: viewerRun((viewer) => viewer.setZoom("fit-width")),
    },
    {
      id: "view.fit-page",
      title: title("view.fit-page"),
      keywords: ["zoom", "whole page"],
      group: "view",
      icon: "fit-page",
      home: ["zoom-control"],
      enabled: viewerOpen,
      run: viewerRun((viewer) => viewer.setZoom("fit-page")),
    },
    {
      id: "view.rotate-display",
      title: title("view.rotate-display"),
      keywords: ["turn", "rotate"],
      group: "view",
      icon: "rotate-right",
      home: ["view-menu"],
      args: choiceArgs(["clockwise", "counter-clockwise"] as const, { optional: true }),
      enabled: viewerOpen,
      run: (ctx, way: "clockwise" | "counter-clockwise" | undefined) =>
        ctx.viewer?.rotate(way === "counter-clockwise" ? -90 : 90),
    } satisfies Command<"clockwise" | "counter-clockwise" | undefined>,
  ];
}

/** The View settings popover and the Settings dialog. */
function settingsCommands(): AnyCommand[] {
  return [
    delegated({
      id: "view.settings",
      title: title("view.settings"),
      keywords: ["theme", "accent", "appearance", "dark"],
      group: "view",
      icon: "view-settings",
      home: ["top-bar"],
    }),
    delegated({
      id: "app.settings",
      title: title("app.settings"),
      keywords: ["preferences", "options", "motion", "accessibility"],
      group: "app",
      icon: "settings",
      shortcuts: [desktop("Mod+,")],
      home: ["view-settings", "help-menu"],
    }),
    // R15: the app's handlers exist only while the browser offers an install or an update waits.
    delegated({
      id: "app.install",
      title: title("app.install"),
      keywords: ["install", "app", "offline", "open with"],
      group: "app",
      icon: "download",
      home: ["file-menu"],
    }),
    delegated({
      id: "app.reload-update",
      title: title("app.reload-update"),
      keywords: ["update", "new version", "reload"],
      group: "app",
      home: ["status-bar"],
    }),
  ];
}

/** The view settings. */
function viewSettingCommands(): AnyCommand[] {
  return [
    delegated({
      id: "view.page-layout",
      title: title("view.page-layout"),
      keywords: ["two pages", "continuous", "spread"],
      group: "view",
      icon: "two-pages",
      home: ["view-settings"],
      args: choiceArgs(["single", "two-up", "continuous"] as const),
    }),
    delegated({
      id: "view.page-appearance",
      title: title("view.page-appearance"),
      keywords: ["sepia", "dark", "contrast", "colour"],
      group: "view",
      home: ["view-settings"],
      args: choiceArgs(["original", "sepia", "dark", "high-contrast"] as const),
    }),
    delegated({
      id: "view.theme",
      title: title("view.theme"),
      keywords: ["dark", "light", "appearance"],
      group: "view",
      home: ["view-settings"],
      args: choiceArgs(["light", "dark", "high-contrast", "system"] as const),
    }),
    delegated({
      id: "view.accent",
      title: title("view.accent"),
      keywords: ["colour", "color"],
      group: "view",
      home: ["view-settings"],
      args: choiceArgs(["teal", "blue", "plum", "green"] as const),
    }),
    delegated({
      id: "view.toggle-sidebar",
      title: title("view.toggle-sidebar"),
      keywords: ["thumbnails", "outline", "rail"],
      group: "view",
      icon: "sidebar",
      home: ["top-bar"],
      enabled: documentOpen,
    }),
    delegated({
      id: "view.focus-mode",
      title: title("view.focus-mode"),
      keywords: ["distraction", "hide"],
      group: "view",
      icon: "focus-mode",
      shortcuts: [keys("global", "Mod+\\")],
      home: ["view-menu"],
      enabled: documentOpen,
    }),
  ];
}

/** Reader mode (R14): its toggle and the settings of its reading column. */
function readerModeCommands(): AnyCommand[] {
  const setting = (id: CommandId, keywords: string[]) =>
    delegated({ id, title: title(id), keywords, group: "view", home: ["view-settings"], enabled: documentOpen });
  return [
    delegated({
      id: "view.reader-mode",
      title: title("view.reader-mode"),
      keywords: ["reflow", "read", "text", "large text"],
      group: "view",
      icon: "reader",
      shortcuts: [keys("page", "R")],
      home: ["top-bar"],
      enabled: documentOpen,
    }),
    delegated({
      id: "view.reader-settings",
      title: title("view.reader-settings"),
      keywords: ["text size", "line spacing", "line length"],
      group: "view",
      home: ["top-bar"],
      enabled: documentOpen,
    }),
    setting("view.reader-text-smaller", ["font", "size", "zoom out"]),
    setting("view.reader-text-larger", ["font", "size", "zoom in"]),
    delegated({
      id: "view.reader-line-spacing",
      title: title("view.reader-line-spacing"),
      keywords: ["leading", "line height"],
      group: "view",
      home: ["view-settings"],
      args: choiceArgs(["tight", "normal", "loose"] as const),
      enabled: documentOpen,
    }),
    delegated({
      id: "view.reader-line-length",
      title: title("view.reader-line-length"),
      keywords: ["measure", "column width"],
      group: "view",
      home: ["view-settings"],
      args: choiceArgs(["narrow", "medium", "wide"] as const),
      enabled: documentOpen,
    }),
    setting("view.reader-reset", ["defaults"]),
  ];
}

/** Read mode and find. */
function findCommands(): AnyCommand[] {
  return [
    delegated({
      id: "mode.read",
      title: title("mode.read"),
      group: "view",
      icon: "reader",
      home: ["mode-switcher"],
    }),
    delegated({
      id: "find.open",
      title: title("find.open"),
      keywords: ["search", "text"],
      group: "find",
      icon: "find",
      shortcuts: [keys("global", "Mod+F")],
      home: ["top-bar"],
      enabled: documentOpen,
    }),
    delegated({
      id: "find.next",
      title: title("find.next"),
      group: "find",
      shortcuts: [keys("find-field", "Enter"), keys("find-field", "F3")],
      home: ["find-bar"],
      enabled: documentOpen,
    }),
    delegated({
      id: "find.previous",
      title: title("find.previous"),
      group: "find",
      shortcuts: [keys("find-field", "Shift+Enter"), keys("find-field", "Shift+F3")],
      home: ["find-bar"],
      enabled: documentOpen,
    }),
  ];
}

/** The selection's commands besides Copy (UI/UX plan §5.3): its other copy, search, share and settings. */
function selectionCommands(): AnyCommand[] {
  return [
    delegated({
      id: "edit.copy-exact",
      title: title("edit.copy-exact"),
      keywords: ["clipboard", "line breaks", "layout"],
      group: "edit",
      icon: "copy",
      home: ["mini-toolbar", "context-menu"],
      enabled: documentOpen,
    }),
    delegated({
      id: "edit.select-column-mode",
      title: title("edit.select-column-mode"),
      keywords: ["rectangle", "column", "select", "Alt drag"],
      group: "edit",
      icon: "select",
      home: ["context-menu"],
      enabled: documentOpen,
    }),
    delegated({
      id: "selection.search",
      title: title("selection.search"),
      keywords: ["find", "selected text"],
      group: "find",
      icon: "find",
      home: ["mini-toolbar", "context-menu"],
      enabled: documentOpen,
    }),
    delegated({
      id: "selection.toggle-toolbar",
      title: title("selection.toggle-toolbar"),
      keywords: ["mini toolbar", "settings", "selection"],
      group: "view",
      home: ["view-settings"],
    }),
    delegated({
      id: "selection.share",
      title: title("selection.share"),
      keywords: ["send", "selected text"],
      group: "edit",
      icon: "share",
      requires: "canShare",
      home: ["context-menu"],
      enabled: documentOpen,
    }),
  ];
}

/** Copying, privacy, the palette, help, layers and the selection. */
function appCommands(): AnyCommand[] {
  return [
    delegated({
      id: "edit.copy",
      title: title("edit.copy"),
      keywords: ["clipboard", "text"],
      group: "edit",
      icon: "copy",
      shortcuts: [keys("page", "Mod+C")],
      home: ["mini-toolbar", "context-menu"],
      enabled: documentOpen,
    }),
    delegated({
      id: "privacy.network-activity",
      title: title("privacy.network-activity"),
      keywords: ["privacy", "requests", "offline"],
      group: "privacy",
      icon: "network-shield",
      home: ["status-bar"],
    }),
    delegated({
      id: "palette.open",
      title: title("palette.open"),
      keywords: ["commands", "actions"],
      group: "app",
      shortcuts: [keys("global", "Mod+K")],
      home: ["top-bar"],
    }),
    delegated({
      id: "help.shortcuts",
      title: title("help.shortcuts"),
      keywords: ["keys", "keyboard", "help"],
      group: "help",
      icon: "keyboard",
      shortcuts: [keys("page", "?"), desktop("F1")],
      home: ["help-menu"],
    }),
    delegated({
      id: "help.capabilities",
      title: title("help.capabilities"),
      keywords: ["features", "supported", "matrix", "status"],
      group: "help",
      icon: "info",
      home: ["help-menu"],
    }),
    delegated({
      id: "layer.close",
      title: title("layer.close"),
      keywords: ["dismiss", "escape"],
      group: "app",
      icon: "close",
      // The layer stack closes the innermost layer on Escape itself; the binding is for the help.
      shortcuts: [keys("global", "Escape", "Escape", { dispatch: false })],
      home: ["layer"],
    }),
    delegated({
      id: "selection.actions",
      title: title("selection.actions"),
      keywords: ["context menu"],
      group: "edit",
      icon: "more",
      shortcuts: [keys("selection", "Shift+F10")],
      home: ["context-menu"],
    }),
  ];
}

/** The reader's commands, built in the catalogue's words. */
export function readerCommands(): readonly AnyCommand[] {
  return [
    ...fileOpenCommands(),
    ...fileDocumentCommands(),
    ...pageCommands(),
    ...historyAndZoomCommands(),
    ...zoomModeCommands(),
    ...viewSettingCommands(),
    ...readerModeCommands(),
    ...settingsCommands(),
    ...findCommands(),
    ...appCommands(),
    ...selectionCommands(),
  ];
}

/** Registers the reader's commands in `registry`; returns the function that removes them all. */
export function registerReaderCommands(registry: CommandRegistry): () => void {
  const removers = readerCommands().map((command) => registry.register(command));
  return () => {
    for (const remove of removers) remove();
  };
}
