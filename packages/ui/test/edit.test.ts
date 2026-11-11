import { describe, expect, it } from "vitest";
import { editCommands } from "../src/commands/edit-commands.ts";
import { readerCommands } from "../src/commands/reader-commands.ts";
import { CommandRegistry } from "../src/commands/registry.ts";
import { shortcutProblems } from "../src/commands/shortcuts.ts";
import type { CommandContext } from "../src/commands/types.ts";
import { inSentence, saveStateText, sessionStarted } from "../src/edit/edit-controls.ts";
import { revertedText } from "../src/edit/outcome.ts";
import { FakeEdits, fakePlatform } from "./fakes.ts";

function registry(): CommandRegistry {
  const commands = new CommandRegistry();
  for (const command of editCommands()) commands.register(command);
  return commands;
}

const context = (edits?: FakeEdits, actions: CommandContext["actions"] = {}): CommandContext => ({
  platform: fakePlatform(),
  ...(edits === undefined ? {} : { edits }),
  actions,
});

describe("the edit session's words", () => {
  it("say how many changes are unsaved", () => {
    const texts = [0, 1, 4].map((n) => saveStateText(new FakeEdits(["a", "b", "c", "d"], n).getState()));
    expect(texts).toEqual(["No unsaved changes", "1 unsaved change", "4 unsaved changes"]);
  });

  it("put a step's label in a sentence, keeping a name's capitals", () => {
    expect([inSentence("Rotate page 5"), inSentence("PDF/A metadata")]).toEqual(["rotate page 5", "PDF/A metadata"]);
  });

  it("say what an undo did, and what it left alone", () => {
    const step = { id: 1, label: "Rotate page 5", pages: [4], at: 0 };
    expect([
      revertedText({ step, way: "undo", outcome: "applied" }),
      revertedText({ step, way: "redo", outcome: "applied" }),
      revertedText({ step, way: "undo", outcome: "partial" }),
      revertedText({ step, way: "undo", outcome: "skipped" }),
    ]).toEqual([
      "Undid “Rotate page 5”",
      "Redid “Rotate page 5”",
      "Undid part of “Rotate page 5”: a later change replaced the rest, so it stays",
      "Couldn't undo “Rotate page 5”: later changes replaced all of it",
    ]);
  });

  it("start a session at the first edit", () => {
    expect([sessionStarted(new FakeEdits().getState()), sessionStarted(new FakeEdits(["a"], 0).getState())]).toEqual([
      false,
      true,
    ]);
  });
});

describe("the edit commands", () => {
  it("each have a visible home besides the palette", () => {
    expect(editCommands().filter((c) => c.home.length === 0)).toEqual([]);
  });

  it("bind Undo to Mod+Z and Redo to Shift+Mod+Z and Ctrl+Y, without colliding with the reader's", () => {
    const keys = Object.fromEntries(
      editCommands().map((c) => [c.id, (c.shortcuts ?? []).map((b) => [b.mac, b.other])]),
    );
    expect([keys["edit.undo"], keys["edit.redo"]]).toEqual([
      [["Mod+Z", "Mod+Z"]],
      [
        ["Mod+Shift+Z", "Mod+Y"],
        ["Mod+Shift+Z", "Mod+Shift+Z"],
      ],
    ]);
    const all = [...readerCommands(), ...editCommands()];
    expect([...shortcutProblems(all, "web"), ...shortcutProblems(all, "desktop")]).toEqual([]);
  });

  it("name the step Undo and Redo would take", () => {
    const commands = registry();
    const ctx = context(new FakeEdits(["Rotate page 5", "Rotate page 2"], 1));
    const title = (id: "edit.undo" | "edit.redo") => commands.title(commands.get(id) as never, ctx);
    expect([title("edit.undo"), title("edit.redo")]).toEqual(["Undo rotate page 5", "Redo rotate page 2"]);
  });

  it("disable Undo with nothing done and Redo with nothing undone", () => {
    const commands = registry();
    const ctx = context(new FakeEdits(["a"], 1));
    expect([commands.availability("edit.undo", ctx).state, commands.availability("edit.redo", ctx)]).toEqual([
      "available",
      { state: "disabled", reason: "There is nothing to redo." },
    ]);
  });

  it("disable every edit for an encrypted document, saying why", () => {
    const commands = registry();
    const ctx = context(new FakeEdits(["a"], 1, { blocked: "encrypted" }), { "pages.rotate-right": () => undefined });
    expect(commands.availability("pages.rotate-right", ctx)).toEqual({
      state: "disabled",
      reason: "This file is password-protected. Editing protected files comes in a later version.",
    });
  });

  it("undo, redo and go to a step through the session", async () => {
    const commands = registry();
    const edits = new FakeEdits(["a", "b", "c"], 3);
    const ctx = context(edits);
    await commands.run("edit.undo", ctx);
    await commands.run("edit.redo", ctx);
    await commands.run("edit.go-to-step", ctx, { args: 0 });
    expect(edits.went).toEqual([2, 3, 0]);
  });

  it("read a step number for Go to step, from 0 to the steps kept", () => {
    const args = editCommands().find((c) => c.id === "edit.go-to-step")?.args;
    const ctx = context(new FakeEdits(["a", "b"], 2));
    expect([args?.parse("1", ctx), args?.parse("3", ctx)]).toEqual([
      { ok: true, value: 1, summary: "Step 1" },
      { ok: false, error: "There is no step 3. Enter a step from 0 to 2." },
    ]);
  });

  it("leave Show history and the page turns to the shell, out of view without its handlers", () => {
    const commands = registry();
    const ctx = context(new FakeEdits());
    expect(commands.availability("edit.history", ctx).state).toBe("hidden");
  });
});
