// The home page rendered from a fixture snapshot: every count, title and date on the page comes
// from progress.json, and an empty list shows a sentence instead of an empty box.

import { experimental_AstroContainer as AstroContainer } from "astro/container";
import { describe, expect, it } from "vitest";
import Home from "../src/components/home/Home.astro";
import { DASHBOARD } from "../src/data/copy.js";
import type { Progress } from "../src/data/progress.js";
import { roundName } from "../src/data/rounds.js";

const FIXTURE: Progress = {
  version: 1,
  date: "2031-02-03",
  totals: { todos: 47, done: 29, inProgress: 3, ready: 15 },
  rounds: [
    { letter: "B", heading: "Library — Round B", todos: 23, done: 19, inProgress: 1, ready: 3 },
    { letter: "C", heading: "Web reader — Round C", todos: 24, done: 10, inProgress: 2, ready: 12 },
  ],
  milestones: [
    { id: "M0", name: "Fixture foundations", rounds: "A", ships: "The `fixture-crate`", state: "done" },
    { id: "M2", name: "Fixture reader", rounds: "C1–C7", ships: "A reader", state: "under-way" },
    { id: "M3", name: "Fixture desktop", rounds: "D1–D7", ships: "Installers", state: "not-started" },
  ],
  inProgress: {
    source: "pull-requests",
    checked: "2031-02-03",
    todos: [{ id: "C9", title: "Fixture todo in flight" }],
  },
  nextUp: [{ id: "C10", title: "Fixture todo ready next" }],
  merged: { count: 61, recent: ["feat(fixture): the last merged title"] },
};

async function render(progress: Progress): Promise<string> {
  const container = await AstroContainer.create();
  return container.renderToString(Home, { props: { progress } });
}

/** `html` as text, tags removed and spaces collapsed. */
function text(html: string): string {
  return html
    .replace(/<[^>]+>/g, " ")
    .replace(/&#39;/g, "'")
    .replace(/\s+/g, " ");
}

describe("the home page", () => {
  it("shows every count, title and the date from the snapshot", async () => {
    const html = await render(FIXTURE);
    const page = text(html);
    expect(page).toContain("29 of 47 todos are done, across 61 merged pull requests.");
    expect(page).toContain("3 milestones: 1 done, 1 under way.");
    expect(page).toContain("Snapshot of 3 February 2031");
    expect(html).toContain('datetime="2031-02-03"');
    for (const round of FIXTURE.rounds) {
      expect(page).toContain(roundName(round.letter));
      expect(page).toContain(`${round.done} done · ${round.inProgress} in progress · ${round.ready} ready`);
    }
    for (const milestone of FIXTURE.milestones) expect(page).toContain(`${milestone.id} · ${milestone.name}`);
    expect(html).toContain("The <code>fixture-crate</code>");
    for (const title of ["Fixture todo in flight", "Fixture todo ready next", "feat(fixture): the last merged title"]) {
      expect(page).toContain(title);
    }
    expect(page).toContain("milestone M2, under way");
    expect(page).toContain("milestone M3, not started");
  });

  it("draws each area's bar from its counts", async () => {
    const html = await render(FIXTURE);
    expect(html).toContain('viewBox="0 0 24 1"');
    expect(html).toMatch(/class="bar-in-progress" x="10" y="0" width="2"/);
    expect(html).toMatch(/class="bar-ready" x="12" y="0" width="12"/);
  });

  it("says so in a sentence when a list is empty", async () => {
    const empty: Progress = {
      ...FIXTURE,
      inProgress: { source: "none", checked: null, todos: [] },
      nextUp: [],
      merged: { count: 0, recent: [] },
    };
    const page = text(await render(empty));
    expect(page).toContain(DASHBOARD.noneInProgress);
    expect(page).toContain(DASHBOARD.noneNext);
    expect(page).toContain(DASHBOARD.noneMerged);
  });

  it("fails on a roadmap without the milestones the page links to", async () => {
    await expect(render({ ...FIXTURE, milestones: FIXTURE.milestones.slice(0, 1) })).rejects.toThrow(
      "the roadmap has no milestone M2",
    );
  });
});
