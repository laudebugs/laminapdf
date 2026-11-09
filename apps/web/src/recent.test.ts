import { describe, expect, it } from "vitest";
import { mayRemember, parseChoices } from "./recent.tsx";

describe("the storage choices", () => {
  it("remember local files and not linked ones by default, and when the stored value is damaged", () => {
    const defaults = { rememberLocal: true, rememberRemote: false };
    expect(parseChoices(null)).toEqual(defaults);
    expect(parseChoices("not json")).toEqual(defaults);
    expect(parseChoices('{"rememberLocal":"yes"}')).toEqual(defaults);
    expect(parseChoices('{"rememberLocal":false,"rememberRemote":true}')).toEqual({
      rememberLocal: false,
      rememberRemote: true,
    });
  });

  it("list a linked file only when the user chose to", () => {
    const defaults = parseChoices(null);
    expect(mayRemember(defaults, undefined)).toBe(true);
    expect(mayRemember(defaults, "https://example.org/a.pdf")).toBe(false);
    expect(mayRemember({ ...defaults, rememberRemote: true }, "https://example.org/a.pdf")).toBe(true);
    expect(mayRemember({ rememberLocal: false, rememberRemote: false }, undefined)).toBe(false);
  });
});
