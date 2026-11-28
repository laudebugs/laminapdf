import { describe, expect, it } from "vitest";
import { desktopCommands } from "../src/commands/desktop-commands.ts";
import { editCommands } from "../src/commands/edit-commands.ts";
import { readerCommands } from "../src/commands/reader-commands.ts";
import { CommandRegistry } from "../src/commands/registry.ts";
import { shortcutProblems } from "../src/commands/shortcuts.ts";
import type { CommandContext } from "../src/commands/types.ts";
import { commentCommands, registerCommentCommands } from "../src/comment/commands.ts";
import { FakeEdits, fakePlatform } from "./fakes.ts";

const context = (actions: NonNullable<CommandContext["actions"]>, edits?: FakeEdits): CommandContext => ({
  platform: fakePlatform(),
  ...(edits === undefined ? {} : { edits }),
  actions,
});

describe("Comment mode's commands", () => {
  it("each have a visible home besides the palette", () => {
    expect(commentCommands().filter((c) => c.home.length === 0)).toEqual([]);
  });

  it("refuse a comment tool, with the reason, on a file whose permissions forbid annotating, and still show the comments", () => {
    const registry = new CommandRegistry();
    registerCommentCommands(registry);
    const run = () => undefined;
    const forbidding = new FakeEdits([], 0, { forbidden: ["comment"] });
    const actions = {
      "comment.highlight": run,
      "comment.reply": run,
      "comments.flatten": run,
      "comments.toggle-panel": run,
      "comments.search": run,
      "comments.summary": run,
    };
    const reason =
      "This file's permissions don't allow adding comments. Open it with the permissions password to change that.";
    expect(
      [
        "comment.highlight",
        "comment.reply",
        "comments.flatten",
        "comments.toggle-panel",
        "comments.search",
        "comments.summary",
      ].map((id) => registry.availability(id as never, context(actions, forbidding))),
    ).toEqual([
      { state: "disabled", reason },
      { state: "disabled", reason },
      { state: "disabled", reason },
      { state: "available" },
      { state: "available" },
      { state: "available" },
    ]);
    // Opened with the permissions password, the session forbids nothing.
    expect(
      registry.availability("comment.highlight", context(actions, new FakeEdits([], 0, { forbidden: [] }))),
    ).toEqual({
      state: "available",
    });
  });

  it("give Highlight, Underline, Text box and Ink single keys on the page, Export highlights ⌥⌘E on macOS, and bold and italic ⌘B and ⌘I, that no other command takes", () => {
    const keyed = commentCommands().flatMap((c) => (c.shortcuts ?? []).map((b) => `${c.id} ${b.mac} ${b.scope}`));
    expect(keyed).toEqual([
      "comment.highlight H page",
      "comment.underline U page",
      "comment.ink P page",
      "comments.export-markdown Alt+Mod+E global",
      "comment.text-box T page",
      "annot.toggle-bold Mod+B global",
      "annot.toggle-italic Mod+I global",
    ]);
    const all = [...readerCommands(), ...editCommands(), ...desktopCommands(), ...commentCommands()];
    expect([...shortcutProblems(all, "web"), ...shortcutProblems(all, "desktop")]).toEqual([]);
  });

  it("say why a tool can't run: no document, a blocked session, or Comment not open", () => {
    const registry = new CommandRegistry();
    registerCommentCommands(registry);
    const run = () => undefined;
    expect([
      registry.availability("comment.highlight", context({ "comment.highlight": run })),
      registry.availability("comment.highlight", context({}, new FakeEdits())),
      registry.availability("comment.highlight", context({ "comment.highlight": run }, new FakeEdits())),
    ]).toEqual([
      { state: "disabled", reason: "Open a document to comment on it" },
      { state: "hidden" },
      { state: "available" },
    ]);
  });

  it("run the summary and Print comment summary through their own handlers, and Flatten only in an editable session", async () => {
    const registry = new CommandRegistry();
    registerCommentCommands(registry);
    const ran: string[] = [];
    const ctx = context(
      {
        "comments.summary": () => void ran.push("summary"),
        "comments.print-summary": () => void ran.push("print"),
      },
      new FakeEdits(),
    );
    await registry.run("comments.summary", ctx, {});
    await registry.run("comments.print-summary", ctx, {});
    expect(ran).toEqual(["summary", "print"]);
    // Flatten has no handler here, so it is left out; the reading commands need only a document.
    expect(registry.availability("comments.flatten", ctx).state).toBe("hidden");
    expect(registry.availability("comments.summary", ctx).state).toBe("available");
  });

  it("run their tool with the shape chosen", () => {
    const registry = new CommandRegistry();
    registerCommentCommands(registry);
    const chosen: unknown[] = [];
    const ctx = context({ "comment.shape": (args) => void chosen.push(args) }, new FakeEdits());
    void registry.run("comment.shape", ctx, { args: "arrow" });
    expect(chosen).toEqual(["arrow"]);
  });
});
