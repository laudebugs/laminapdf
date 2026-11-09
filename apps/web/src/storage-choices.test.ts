import { describe, expect, it } from "vitest";
import { parseChoices } from "./storage-choices.ts";

describe("the storage choices", () => {
  it("remember local files by default, and when the stored value is damaged", () => {
    expect(parseChoices(null)).toEqual({ rememberLocal: true });
    expect(parseChoices("not json")).toEqual({ rememberLocal: true });
    expect(parseChoices('{"rememberLocal":"yes"}')).toEqual({ rememberLocal: true });
    expect(parseChoices('{"rememberLocal":false}')).toEqual({ rememberLocal: false });
  });

  it("keep no choice about linked files: their addresses are never stored", () => {
    expect(parseChoices('{"rememberLocal":true,"rememberRemote":true}')).toEqual({ rememberLocal: true });
  });
});
