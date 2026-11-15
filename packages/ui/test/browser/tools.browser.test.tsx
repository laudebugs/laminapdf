// The tools' cursors over the page (motion plan §3, 1): the tool's cursor on the view wins over the
// text layer's I-beam when the tool selects no text, the hand shows grab and, while a press it took
// is held, grabbing; tools that select text leave the I-beam over the words.

import { afterEach, describe, expect, it } from "vitest";
import { attachTools, handTool, ToolRegistry } from "../../src/tools/tools.ts";

/** The property the viewer's text layer reads its words' cursor from (TEXT_CURSOR_PROPERTY). */
const TEXT_CURSOR = "--test-text-cursor";

/** A view with one word in it, styled as the viewer's text layer styles its spans. */
function view() {
  const element = document.createElement("div");
  const word = document.createElement("span");
  word.textContent = "Widgets";
  word.style.cursor = `var(${TEXT_CURSOR}, text)`;
  element.append(word);
  // A synthetic press has no pointer to capture.
  element.setPointerCapture = () => {};
  document.body.append(element);
  return { element, word };
}

const cursor = (el: Element) => getComputedStyle(el).cursor;
const pointer = (type: string) => new PointerEvent(type, { button: 0, pointerId: 1, bubbles: true });

afterEach(() => document.body.replaceChildren());

describe("the tools' cursors", () => {
  it("leave the I-beam over the words while text can be selected", () => {
    const { element, word } = view();
    const tools = new ToolRegistry();
    attachTools(tools, { element, textCursorProperty: TEXT_CURSOR, scrollBy: () => {} });
    expect(cursor(word)).toBe("text");
    tools.use("textSelect");
    expect(cursor(word)).toBe("text");
  });

  it("show a creation tool's cursor over the words, and the I-beam again once it is off", () => {
    const { element, word } = view();
    const tools = new ToolRegistry();
    tools.register({ name: "link", cursor: "crosshair", selectsText: false });
    const stop = attachTools(tools, { element, textCursorProperty: TEXT_CURSOR, scrollBy: () => {} });
    tools.use("link");
    expect([cursor(element), cursor(word)]).toEqual(["crosshair", "crosshair"]);
    tools.use("select");
    expect(cursor(word)).toBe("text");
    tools.use("link");
    stop();
    expect([cursor(element), cursor(word)]).toEqual(["auto", "text"]);
  });

  it("show the hand's grab, and grabbing while it holds the page", () => {
    const { element, word } = view();
    const tools = new ToolRegistry([handTool()]);
    tools.use("hand");
    attachTools(tools, { element, textCursorProperty: TEXT_CURSOR, scrollBy: () => {} });
    expect([cursor(element), cursor(word)]).toEqual(["grab", "grab"]);
    word.dispatchEvent(pointer("pointerdown"));
    expect([cursor(element), cursor(word)]).toEqual(["grabbing", "grabbing"]);
    word.dispatchEvent(pointer("pointerup"));
    expect([cursor(element), cursor(word)]).toEqual(["grab", "grab"]);
  });
});
