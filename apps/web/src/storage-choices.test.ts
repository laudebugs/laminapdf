import { describe, expect, it } from "vitest";
import { parseChoices } from "./storage-choices.ts";

describe("the storage choices", () => {
  it("remember local files and reading positions by default, and when the stored value is damaged", () => {
    const both = { rememberLocal: true, rememberPositions: true };
    expect(parseChoices(null)).toEqual(both);
    expect(parseChoices("not json")).toEqual(both);
    expect(parseChoices('{"rememberLocal":"yes"}')).toEqual(both);
    expect(parseChoices('{"rememberLocal":false}')).toEqual({ ...both, rememberLocal: false });
    expect(parseChoices('{"rememberPositions":false}')).toEqual({ ...both, rememberPositions: false });
  });

  it("keep no choice about linked files: their addresses are never stored", () => {
    expect(parseChoices('{"rememberLocal":true,"rememberRemote":true}')).toEqual({
      rememberLocal: true,
      rememberPositions: true,
    });
  });
});
