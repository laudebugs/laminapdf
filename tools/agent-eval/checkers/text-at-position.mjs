// hello.pdf: one A4 page, "Hello from Lamina" starting at x = 72 with its baseline at y = 100
// (page space: points from the top-left corner, y down).
import { check, expect, near, runs } from "./lib.mjs";

const SIZE = 18;

await check(({ open, pdf }) => {
  pdf("hello.pdf");
  const doc = open("hello.pdf");
  expect(doc.pageCount === 1, `expected 1 page, found ${doc.pageCount}`);
  const { width, height } = doc.pageInfo(0);
  expect(near(width, 595.28) && near(height, 841.89), `expected A4, found ${width} × ${height}`);
  const run = runs(doc, 0).find((r) => r.text.includes("Hello from Lamina"));
  expect(run, "the text 'Hello from Lamina' is missing");
  expect(near(run.x, 72, 1.5), `the text starts at x = ${run.x.toFixed(1)}, not 72`);
  // The run's box spans the font's ascent and descent: its top is above the baseline, its
  // bottom below it, by less than the font size.
  const top = run.y;
  const bottom = run.y + run.height;
  const baselineInBox = top < 100 && top > 100 - SIZE * 1.2 && bottom > 100 && bottom < 100 + SIZE * 0.5;
  expect(baselineInBox, `the text's box spans y = ${top.toFixed(1)}…${bottom.toFixed(1)}: baseline not at 100`);
  expect(near(run.height, SIZE * 0.93, SIZE * 0.35), `the text's box is ${run.height.toFixed(1)} pt tall: not 18 pt`);
  return `text at x = ${run.x.toFixed(1)}, box ${top.toFixed(1)}…${bottom.toFixed(1)}`;
});
