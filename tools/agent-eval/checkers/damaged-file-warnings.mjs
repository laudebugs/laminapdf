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
  // Warnings raised at open must all be there; reading pages may add more, which is fine as long
  // as each is a code Lamina PDF reported for this file once its pages are read too.
  const atOpen = codes(damaged.warnings);
  for (let i = 0; i < damaged.pageCount; i++) damaged.page(i);
  const want = codes(damaged.warnings);
  expect(
    Array.isArray(listed) && listed.every((c) => typeof c === "string"),
    `warnings.json is ${JSON.stringify(listed)}, not an array of codes`,
  );
  const missing = atOpen.filter((c) => !listed.includes(c));
  const unknown = listed.filter((/** @type {string} */ c) => !want.includes(c));
  expect(missing.length === 0, `warnings.json lacks ${missing.join(", ")}`);
  expect(unknown.length === 0, `warnings.json lists ${unknown.join(", ")}, which Lamina PDF doesn't report here`);
  const repaired = open("repaired.pdf");
  expect(
    repaired.pageCount === damaged.pageCount,
    `repaired.pdf has ${repaired.pageCount} pages, not ${damaged.pageCount}`,
  );
  const left = codes(repaired.warnings).filter((c) => want.includes(c));
  expect(left.length === 0, `repaired.pdf still reports ${left.join(", ")}`);
  return `warnings ${want.join(", ")}; repaired.pdf opens without them`;
});
