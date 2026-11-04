// warnings.json: the warning codes reported while reading damaged.pdf, as Lamina PDF reports them;
// repaired.pdf: opens without any of them.
import { CheckFailure, check, expect, taskInput } from "./lib.mjs";

/** @param {readonly { code: string }[]} warnings */
function codes(warnings) {
  return [...new Set(warnings.map((w) => w.code))];
}

await check(({ sdk, read, open, pdf }) => {
  pdf("repaired.pdf");
  let listed;
  try {
    listed = JSON.parse(new TextDecoder().decode(read("warnings.json")));
  } catch {
    throw new CheckFailure("warnings.json is not JSON");
  }
  const damaged = sdk.Document.open(taskInput("damaged-file-warnings", "damaged.pdf"));
  const want = codes(damaged.warnings);
  expect(
    JSON.stringify(listed) === JSON.stringify(want),
    `warnings.json is ${JSON.stringify(listed)}, expected ${JSON.stringify(want)}`,
  );
  const repaired = open("repaired.pdf");
  expect(
    repaired.pageCount === damaged.pageCount,
    `repaired.pdf has ${repaired.pageCount} pages, not ${damaged.pageCount}`,
  );
  const left = codes(repaired.warnings).filter((c) => want.includes(c));
  expect(left.length === 0, `repaired.pdf still reports ${left.join(", ")}`);
  return `warnings ${want.join(", ")}; repaired.pdf opens without them`;
});
