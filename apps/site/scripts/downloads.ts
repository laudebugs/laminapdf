// Refreshes the download page's snapshot, src/data/downloads.json, from each channel's list on its
// download host (src/data/downloads.ts). Production deploys run it before the build (site.yml); a
// channel with no list yet (404) is `null`. Any other failure stops the deploy rather than show a
// page with a channel missing.
//
//   pnpm --filter @laminapdf/site run downloads
import { writeFileSync } from "node:fs";
import process from "node:process";
import { CHANNELS, type Channel, DOWNLOAD_HOSTS, type Downloads, parseDownloads } from "../src/data/downloads.ts";

async function fetchList(channel: Channel): Promise<Downloads | null> {
  const url = `${DOWNLOAD_HOSTS[channel]}/downloads/${channel}.json`;
  const answer = await fetch(url, { headers: { accept: "application/json" } });
  if (answer.status === 404) return null;
  if (!answer.ok) throw new Error(`${url} answered ${answer.status}`);
  const list = parseDownloads(await answer.json(), channel);
  if (typeof list === "string") throw new Error(`${url} is invalid: ${list}`);
  return list;
}

async function refresh(): Promise<void> {
  const snapshot: Record<string, Downloads | null> = {};
  for (const channel of CHANNELS) {
    snapshot[channel] = await fetchList(channel);
    console.log(`${channel}: ${snapshot[channel]?.version ?? "no release yet"}`);
  }
  writeFileSync(new URL("../src/data/downloads.json", import.meta.url), `${JSON.stringify(snapshot, null, 2)}\n`);
}

refresh().catch((error: unknown) => {
  console.error(`downloads: ${error instanceof Error ? error.message : String(error)}`);
  process.exit(1);
});
