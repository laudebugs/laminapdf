import type { FieldAction, SubmitRequest } from "@laminapdf/sdk";
import { describe, expect, it } from "vitest";
import {
  hideOps,
  moveTarget,
  PressGate,
  plain,
  planActions,
  problemOf,
  refusalOf,
  refusedPlan,
  submitPlan,
} from "./actions.ts";

/** A click on a button that runs `action`. */
function click(action: FieldAction["action"]): FieldAction {
  return { trigger: "click", action };
}

describe("planActions", () => {
  it("plans each click action in the file's order, with a submit's place in the list", () => {
    const steps = planActions([
      click({ type: "goTo", pageIndex: 2 }),
      click({ type: "named", action: "nextPage" }),
      click({ type: "uri", url: "https://example.com/" }),
      click({ type: "resetForm", fields: ["f1.0"] }),
      click({ type: "hide", targets: [{ field: "f2.0" }], hide: true }),
      click({ type: "submitForm", url: "https://example.com/submit" }),
    ]);
    expect(steps.map((s) => s.kind)).toEqual(["page", "move", "link", "reset", "hide", "submit"]);
    expect(steps[5]).toEqual({ kind: "submit", index: 5 });
  });

  it("names what it doesn't run and never runs it", () => {
    const steps = planActions([
      click({ type: "javaScript" }),
      click({ type: "launch" }),
      click({ type: "importData" }),
      click({ type: "goToR" }),
      click({ type: "goToE" }),
      click({ type: "other", name: "Movie" }),
      click({ type: "named", action: "other", name: "Print" }),
    ]);
    expect(steps.map((s) => (s.kind === "not-run" ? s.why : s.kind))).toEqual([
      "javaScript",
      "launch",
      "importData",
      "otherFile",
      "otherFile",
      "other",
      "other",
    ]);
    expect(steps[6]).toEqual({ kind: "not-run", why: "other", name: "Print" });
  });

  it("leaves the pointer and focus triggers alone", () => {
    const hover: FieldAction = { trigger: "mouseEnter", action: { type: "submitForm", url: "https://a.example/" } };
    const out: FieldAction = { trigger: "blur", action: { type: "uri", url: "https://a.example/" } };
    expect(planActions([hover, out, { trigger: "other", action: { type: "javaScript" } }])).toEqual([]);
  });

  it("keeps the index of a submit that follows an action that isn't a click", () => {
    const steps = planActions([
      { trigger: "mouseEnter", action: { type: "javaScript" } },
      click({ type: "submitForm", url: "https://a.example/" }),
    ]);
    expect(steps).toEqual([{ kind: "submit", index: 1 }]);
  });
});

describe("moveTarget", () => {
  it("stays inside the document", () => {
    expect(moveTarget("next", 4, 5)).toBe(4);
    expect(moveTarget("next", 1, 5)).toBe(2);
    expect(moveTarget("previous", 0, 5)).toBe(0);
    expect(moveTarget("first", 3, 5)).toBe(0);
    expect(moveTarget("last", 0, 5)).toBe(4);
    expect(moveTarget("last", 0, 0)).toBe(0);
  });
});

describe("hideOps", () => {
  const flags = new Map([
    ["a1.0", 4],
    ["a2.0", 4 | 2],
  ]);
  const flagsOf = (id: string) => flags.get(id);

  it("hides fields and annotations, each with the op that fits it", () => {
    const ops = hideOps([{ field: "f1.0" }, { annotation: "a1.0" }], true, flagsOf, "D:20261128000000Z");
    expect(ops).toEqual([
      { op: "setFieldHidden", args: { field: "f1.0", hidden: true } },
      { op: "updateAnnot", args: { id: "a1.0", flags: 6, modified: "D:20261128000000Z" } },
    ]);
  });

  it("shows them again, and skips an annotation already as asked or one it can't find", () => {
    const ops = hideOps(
      [{ annotation: "a2.0" }, { annotation: "a1.0" }, { annotation: "gone" }],
      false,
      flagsOf,
      "now",
    );
    expect(ops).toEqual([{ op: "updateAnnot", args: { id: "a2.0", flags: 4, modified: "now" } }]);
  });
});

describe("the submit dialog's plan", () => {
  const request = {
    url: "https://forms.example.com/a/submit?x=1",
    host: "https://forms.example.com",
    target: "web",
    method: "POST",
    format: "fdf",
    contentType: "application/vnd.fdf",
    fields: [{ name: "name", value: "Ada" }],
    emptyRequired: ["email"],
    body: new Uint8Array(),
  } as SubmitRequest;

  it("splits the address and lists every field", () => {
    expect(submitPlan("Send", request)).toEqual({
      button: "Send",
      to: { scheme: "https://", host: "forms.example.com", path: "/a/submit?x=1" },
      format: "fdf",
      fields: [{ name: "name", value: "Ada" }],
      emptyRequired: ["email"],
    });
  });

  it("reads the library's refusals and a failed send", () => {
    expect(refusalOf({ code: "submit-pdf-refused" })).toEqual({ kind: "pdf" });
    expect(refusalOf({ code: "submit-url-refused" })).toEqual({ kind: "insecure" });
    expect(refusalOf(new Error("x"))).toBeUndefined();
    expect(problemOf({ sent: true, status: 200 }, false)).toBeUndefined();
    expect(problemOf({ sent: false, reason: "unreachable", mayHaveSent: false }, false)).toEqual({
      kind: "blocked",
      mayHaveSent: false,
    });
    expect(problemOf({ sent: false, reason: "unreachable", mayHaveSent: true }, false)).toEqual({
      kind: "blocked",
      mayHaveSent: true,
    });
    expect(problemOf({ sent: false, reason: "unreachable", mayHaveSent: true }, true)).toEqual({
      kind: "unreachable",
      mayHaveSent: true,
    });
    expect(problemOf({ sent: false, reason: "timeout" }, false)).toEqual({ kind: "timeout" });
    expect(problemOf({ sent: false, reason: "cancelled" }, false)).toBeUndefined();
    expect(problemOf({ sent: false, reason: "status", status: 500 }, false)).toEqual({ kind: "status", status: 500 });
    expect(problemOf({ sent: false, reason: "refused" }, true)).toEqual({ kind: "insecure" });
  });
});

describe("what the dialog shows", () => {
  it("leaves out characters that make text read as other text", () => {
    expect(plain("Pay\u202Efdp.exe\u200B now\u0007")).toBe("Payfdp.exe now");
    expect(plain("two\nlines")).toBe("two\nlines");
  });

  it("shows the address's own query, and cleans every name and value", () => {
    const request = {
      url: "https://forms.example.com/s?token=abc",
      host: "https://forms.example.com",
      target: "web",
      method: "POST",
      format: "html",
      contentType: "x",
      fields: [{ name: "a\u202Eb", value: "v\u200B" }],
      emptyRequired: [],
      body: new Uint8Array(),
    } as SubmitRequest;
    const plan = submitPlan("Send\u202E", request);
    expect(plan.to).toEqual({ scheme: "https://", host: "forms.example.com", path: "/s?token=abc" });
    expect([plan.button, plan.fields]).toEqual(["Send", [{ name: "ab", value: "v" }]]);
    expect(refusedPlan("x", "not a url\u202E").to.host).toBe("not a url");
  });
});

describe("PressGate", () => {
  it("lets only the newest press go on", () => {
    const gate = new PressGate();
    const first = gate.begin();
    expect(gate.isCurrent(first)).toBe(true);
    const second = gate.begin();
    expect([gate.isCurrent(first), gate.isCurrent(second)]).toEqual([false, true]);
  });
});
