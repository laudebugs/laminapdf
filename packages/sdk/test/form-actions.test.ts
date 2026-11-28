// A form's actions and the sends they ask for: doc.formActions and doc.submitRequest over
// fixtures/forms/actions.pdf (tools/fixtures/forms.py). Nothing runs and nothing is sent. A test's
// title starts with the capability it proves (its tag), which `cargo xtask capabilities` counts.
import { Document, init, isLaminaPdfError } from "@laminapdf/sdk";
import { beforeAll, describe, expect, it } from "vitest";
import { read, thrown } from "./helpers.js";

beforeAll(() => init({ extract: true }));

function open(): Document {
  return Document.open(read("fixtures/forms/actions.pdf"));
}

/** The ID of the field named `name`. */
function idOf(doc: Document, name: string): string {
  const field = doc.formFields().fields.find((f) => f.name === name);
  if (field === undefined) throw new Error(`no field named ${name}`);
  return field.id;
}

describe("form actions", () => {
  it("[cap E.04 pdf read] reads a push button's action as typed data", () => {
    using doc = open();
    expect(doc.formActions(idOf(doc, "web"))).toEqual([
      { trigger: "click", action: { type: "uri", url: "https://example.com/help" } },
    ]);
    expect(doc.formActions(idOf(doc, "next"))).toEqual([{ trigger: "click", action: { type: "goTo", pageIndex: 1 } }]);
    expect(doc.formActions(idOf(doc, "nextpage"))).toEqual([
      { trigger: "click", action: { type: "named", action: "nextPage" } },
    ]);
  });

  it("resolves the fields a reset names, and a hide's targets", () => {
    using doc = open();
    const [reset] = doc.formActions(idOf(doc, "resetname"));
    expect(reset?.action).toEqual({ type: "resetForm", fields: [idOf(doc, "name")] });
    const [hide] = doc.formActions(idOf(doc, "hide"));
    expect(hide?.action.type).toBe("hide");
    if (hide?.action.type === "hide") {
      expect(hide.action.hide).toBe(true);
      expect(hide.action.targets[0]).toEqual({ field: idOf(doc, "notes") });
      expect(Object.keys(hide.action.targets[1] ?? {})).toEqual(["annotation"]);
    }
  });

  it("names the actions it never runs, and follows /Next", () => {
    using doc = open();
    const types = (name: string) => doc.formActions(idOf(doc, name)).map((a) => a.action.type);
    expect(types("script")).toEqual(["javaScript"]);
    expect(types("launch")).toEqual(["launch"]);
    expect(types("import")).toEqual(["importData"]);
    expect(types("chain")).toEqual(["resetForm", "hide"]);
  });

  it("[cap E.16 pdf read] builds the request of each format without sending it", () => {
    using doc = open();
    const request = (name: string) => doc.submitRequest(idOf(doc, name), 0);
    const html = request("sendhtml");
    expect([html.method, html.format, html.contentType, html.host]).toEqual([
      "POST",
      "html",
      "application/x-www-form-urlencoded",
      "https://forms.example.com",
    ]);
    expect(new TextDecoder().decode(html.body)).toBe("name=Ada+Lovelace&email=ada%40example.com&notes=Hello");
    expect(html.fields.map((f) => f.name)).toEqual(["name", "email", "notes"]);
    expect(html.emptyRequired).toEqual(["empty"]);
    expect(request("send").format).toBe("fdf");
    expect(new TextDecoder().decode(request("sendxfdf").body)).toContain(
      '<field name="name"><value>Ada Lovelace</value></field>',
    );
    const get = request("sendget");
    expect([get.method, get.body.length, get.url]).toEqual([
      "GET",
      0,
      "https://forms.example.com/submit?name=Ada+Lovelace&email=ada%40example.com&notes=Hello",
    ]);
  });

  it("refuses a send of the whole PDF and an address that isn't https, and sends no data by mail", () => {
    using doc = open();
    const code = (name: string) => {
      const error = thrown(() => doc.submitRequest(idOf(doc, name), 0));
      return isLaminaPdfError(error) ? error.code : undefined;
    };
    expect(code("sendpdf")).toBe("submit-pdf-refused");
    expect(code("sendhttp")).toBe("submit-url-refused");
    const mail = doc.submitRequest(idOf(doc, "sendmail"), 0);
    expect([mail.target, mail.url, mail.body.length, mail.fields]).toEqual(["mail", "mailto:forms@example.com", 0, []]);
  });

  it("asks for an index that is a submit action", () => {
    using doc = open();
    const error = thrown(() => doc.submitRequest(idOf(doc, "web"), 0));
    expect(isLaminaPdfError(error) && error.code).toBe("invalid-argument");
  });
});
