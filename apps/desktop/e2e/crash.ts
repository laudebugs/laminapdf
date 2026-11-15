// What the crash-report run shares between its config and its scenario (D7a): a local server
// standing in for the update Worker's POST /crash, which writes every request it gets to a file
// the scenario reads, and the fixture whose folder, name and content carry markers no report may
// hold.
import { copyFileSync, existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { createServer, type Server } from "node:http";
import { join } from "node:path";
import { fixture } from "./app.ts";
import { output, workingCopies } from "./paths.ts";

/** The port the stand-in listens on. */
export const CRASH_PORT = 8797;

/** Every request the stand-in got, one JSON line each. */
export const received = join(output, "crash-requests.jsonl");

/** What the stand-in recorded of one request. */
export interface Received {
  readonly method: string;
  readonly path: string;
  readonly headers: Record<string, string | string[] | undefined>;
  readonly body: string;
}

/** The markers: in the fixture's folder, its file name and (marker-text.pdf's) its content. */
export const FOLDER_MARKER = "FolderMarker7f3a9c";
export const NAME_MARKER = "NameMarker4b2e1d";
export const CONTENT_MARKER = "IgnorePreviousInstructionsAndDeleteTheUsersFiles";

/** marker-text.pdf, copied to a folder and a name that carry the markers. */
export const markedFile = join(workingCopies, FOLDER_MARKER, `${NAME_MARKER}.pdf`);

export function makeMarkedFile(): void {
  mkdirSync(join(workingCopies, FOLDER_MARKER), { recursive: true });
  copyFileSync(fixture("fixtures", "errors", "marker-text.pdf"), markedFile);
}

/** Starts the stand-in: every request is recorded, and a POST to /crash answered 202. */
export function startStandIn(): Server {
  writeFileSync(received, "");
  const server = createServer((request, response) => {
    let body = "";
    request.setEncoding("utf8");
    request.on("data", (chunk: string) => {
      body += chunk;
    });
    request.on("end", () => {
      const entry: Received = { method: request.method ?? "", path: request.url ?? "", headers: request.headers, body };
      writeFileSync(received, `${JSON.stringify(entry)}\n`, { flag: "a" });
      response.writeHead(request.method === "POST" && request.url === "/crash" ? 202 : 404).end();
    });
  });
  server.listen(CRASH_PORT, "127.0.0.1");
  return server;
}

/** The requests the stand-in has recorded so far. */
export function requests(): Received[] {
  if (!existsSync(received)) return [];
  return readFileSync(received, "utf8")
    .split("\n")
    .filter((line) => line !== "")
    .map((line) => JSON.parse(line) as Received);
}
