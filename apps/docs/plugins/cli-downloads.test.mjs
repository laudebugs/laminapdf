// Tests for cli-downloads.mjs: `node --test plugins/` (the docs site's `test` script).

import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { test } from "node:test";
import { archiveName, distTargets, downloadProblems, INSTALL_PAGE } from "./cli-downloads.mjs";

const ROOT = new URL("../../../", import.meta.url);
const page = await readFile(new URL(INSTALL_PAGE, ROOT), "utf8");
const targets = distTargets(await readFile(new URL("dist-workspace.toml", ROOT), "utf8"));

test("the install page links every target the release builds, and no other", () => {
  assert.deepEqual(downloadProblems(page, targets), []);
});

test("a target the page lacks fails the check", () => {
  const problems = downloadProblems(page, [...targets, "riscv64gc-unknown-linux-gnu"]);
  assert.equal(problems.length, 2);
  assert.match(problems[0], /laminapdf-cli-riscv64gc-unknown-linux-gnu\.tar\.xz$/);
  assert.match(problems[1], /\.tar\.xz\.sha256$/);
});

test("a row for a target the release dropped fails the check", () => {
  const problems = downloadProblems(
    page,
    targets.filter((target) => target !== "x86_64-apple-darwin"),
  );
  assert.deepEqual(problems, ["links x86_64-apple-darwin, which the release doesn't build"]);
});

test("a missing checksum link fails the check", () => {
  const archive = "laminapdf-cli-aarch64-apple-darwin.tar.xz.sha256)";
  assert.ok(page.includes(archive));
  const problems = downloadProblems(page.replace(archive, "elsewhere)"), targets);
  assert.equal(problems.length, 1);
  assert.match(problems[0], /aarch64-apple-darwin\.tar\.xz\.sha256$/);
});

test("Windows archives are zips", () => {
  assert.equal(archiveName("x86_64-pc-windows-msvc"), "laminapdf-cli-x86_64-pc-windows-msvc.zip");
  assert.equal(archiveName("aarch64-apple-darwin"), "laminapdf-cli-aarch64-apple-darwin.tar.xz");
});
