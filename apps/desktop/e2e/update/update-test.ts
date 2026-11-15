// The desktop update's end-to-end test: an installed Lamina PDF 0.0.1 refuses a tampered
// 0.0.2, then updates itself to the real one, from the update Worker's `staging` channel.
//
// The workflow (.github/workflows/desktop-update-e2e.yml) builds both versions with the `e2e`
// feature and one throwaway updater key: 0.0.1 trusts its public half, and 0.0.2's bundle is
// signed with its private half. This script then:
//
// 1. serves the Worker: `wrangler dev` with local R2 (`local` mode), or the deployed staging Worker
//    and its bucket (`staging` mode), with a proxy in front that records each request;
// 2. installs 0.0.1 (macOS: the .app copied out, Windows: the NSIS installer, Linux: the AppImage);
// 3. serves 0.0.2 with one byte flipped and its real signature, launches the app, and expects it to
//    report the failed check and stay on 0.0.1;
// 4. serves the real 0.0.2, launches the app, and expects it to install and restart as 0.0.2;
// 5. checks that every request carried the install ID in the header and never in the URL.
//
// The app's `e2e` hooks (src-tauri/src/update/e2e.rs), turned on by `update-test.json` in its
// config folder, point it at the proxy, check at once, install
// as soon as an update is ready, and write each step to a report file.
//
// Environment: UPDATE_TEST_MODE (`local` or `staging`), UPDATE_TEST_OLD (0.0.1's .app, installer
// or AppImage) and UPDATE_TEST_NEW (0.0.2's updater bundle, with its `.sig` beside it). Staging
// mode also needs CLOUDFLARE_API_TOKEN (the staging bucket's token) and CLOUDFLARE_ACCOUNT_ID.
import type { Buffer } from "node:buffer";
import { type ChildProcess, execFileSync, spawn, spawnSync } from "node:child_process";
import { randomUUID } from "node:crypto";
import {
  appendFileSync,
  chmodSync,
  cpSync,
  existsSync,
  mkdirSync,
  readdirSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { createServer, type Server } from "node:http";
import { basename, dirname, join } from "node:path";
import { arch, env, exit, kill, platform } from "node:process";
import { setTimeout as sleep } from "node:timers/promises";
import { root, updateTestFile } from "../paths.ts";
import {
  CHANNEL,
  emptyManifest,
  endpointPath,
  INSTALL_ID_HEADER,
  manifestFor,
  NEW_VERSION,
  OLD_VERSION,
  platformOf,
  requestProblems,
  type SeenRequest,
  tampered,
} from "./plan.ts";

const mode = env.UPDATE_TEST_MODE === "staging" ? "staging" : "local";
const oldBuild = required("UPDATE_TEST_OLD");
const newBundle = required("UPDATE_TEST_NEW");
const signature = readFileSync(`${newBundle}.sig`, "utf8");

/** The staging channel's bucket and its download host (apps/updates/README.md). */
const BUCKET = "lamina-pdf-staging";
const DOWNLOADS = "https://staging-downloads.laminapdf.com";
const STAGING_WORKER = "https://staging-updates.laminapdf.com";
const LOCAL_PORT = 8787;

const updates = join(root, "apps", "updates");
const out = join(root, "target", "desktop-update-e2e");
const run = env.GITHUB_RUN_ID === undefined ? randomUUID() : `${env.GITHUB_RUN_ID}-${env.GITHUB_RUN_ATTEMPT ?? "1"}`;
const here = platformOf(platform, arch);
const key = `${here.target}-${here.arch}`;
/** The bundle's name as served: what the updater unpacks depends on its ending. */
const servedName = bundleName(newBundle);

function required(name: string): string {
  const value = env[name];
  if (value === undefined || value === "") throw new Error(`set ${name}`);
  return value;
}

function bundleName(path: string): string {
  for (const ending of [".app.tar.gz", "-setup.exe", ".AppImage"]) {
    if (path.endsWith(ending)) return `Lamina-PDF${ending}`;
  }
  throw new Error(`${path} isn't a macOS, Windows or Linux updater bundle`);
}

function log(line: string): void {
  const stamped = `${new Date().toISOString()} update-test: ${line}`;
  console.log(stamped);
  appendFileSync(join(out, "update-test.log"), `${stamped}\n`);
}

// --- The servers -----------------------------------------------------------------------------

/** The bundles `local` mode serves in place of the download host, by path. */
const files = new Map<string, Uint8Array>();
const seen: SeenRequest[] = [];

function listen(server: Server): Promise<number> {
  return new Promise((resolve, reject) => {
    server.once("error", reject);
    server.listen(0, "127.0.0.1", () => {
      const address = server.address();
      if (address === null || typeof address === "string") reject(new Error("no port"));
      else resolve(address.port);
    });
  });
}

/** `local` mode's download host: the bundles in {@link files}. */
const fileServer = createServer((request, response) => {
  const body = files.get(new URL(request.url ?? "/", "http://x").pathname);
  if (body === undefined) {
    response.writeHead(404).end();
    return;
  }
  response.writeHead(200, { "content-type": "application/octet-stream", "content-length": body.length }).end(body);
});

/** The proxy the app asks: it records each request and passes it on to the Worker. */
function proxy(worker: string, filesOrigin: string | undefined): Server {
  return createServer((request, response) => {
    const header = request.headers[INSTALL_ID_HEADER];
    const installId = Array.isArray(header) ? header[0] : header;
    seen.push({ url: request.url ?? "", installId });
    const headers: Record<string, string> = {};
    if (installId !== undefined) headers[INSTALL_ID_HEADER] = installId;
    fetch(`${worker}${request.url ?? "/"}`, { headers })
      .then(async (answer) => {
        let body = await answer.text();
        // Local mode has no download host: its bundles come from the file server.
        if (filesOrigin !== undefined) body = body.replaceAll(DOWNLOADS, filesOrigin);
        response
          .writeHead(answer.status, { "content-type": answer.headers.get("content-type") ?? "text/plain" })
          .end(body);
      })
      .catch((error: unknown) => {
        log(`the proxy couldn't reach the Worker: ${String(error)}`);
        response.writeHead(502).end();
      });
  });
}

let wranglerDev: ChildProcess | undefined;

/** `wrangler dev` on local R2, once it answers its health check. */
async function startLocalWorker(): Promise<string> {
  const origin = `http://127.0.0.1:${LOCAL_PORT}`;
  const logFile = join(out, "wrangler-dev.log");
  wranglerDev = spawn(
    "pnpm",
    ["exec", "wrangler", "dev", "--local", "--ip", "127.0.0.1", "--port", String(LOCAL_PORT)],
    {
      cwd: updates,
      shell: platform === "win32",
      stdio: ["ignore", "pipe", "pipe"],
    },
  );
  logTo(wranglerDev, logFile);
  for (let tries = 0; tries < 120; tries++) {
    try {
      if ((await fetch(`${origin}/healthz`)).ok) return origin;
    } catch {
      // Not listening yet.
    }
    await sleep(1000);
  }
  throw new Error(`wrangler dev didn't answer within 120 s (${logFile})`);
}

/** Appends `child`'s output to `file`. */
function logTo(child: ChildProcess, file: string): void {
  child.stdout?.on("data", (chunk: Buffer) => appendFileSync(file, chunk));
  child.stderr?.on("data", (chunk: Buffer) => appendFileSync(file, chunk));
}

function pnpm(args: string[], cwd: string): void {
  const result = spawnSync("pnpm", args, { cwd, shell: platform === "win32", encoding: "utf8", timeout: 180_000 });
  if (result.status !== 0) throw new Error(`pnpm ${args.join(" ")} failed:\n${result.stdout}\n${result.stderr}`);
}

/** Writes the staging manifest with the manifest tool, which checks it with the Worker's validator. */
function putManifest(manifest: object): void {
  const file = join(out, "staging.json");
  writeFileSync(file, `${JSON.stringify(manifest, null, 2)}\n`);
  pnpm(["run", "manifest", "put", CHANNEL, file, ...(mode === "local" ? ["--local"] : [])], updates);
}

/** Serves `bytes` as step `step`'s bundle and offers it on the staging channel. */
async function publish(worker: string, step: string, bytes: Uint8Array): Promise<void> {
  const path = `/e2e/${run}/${step}/${servedName}`;
  if (mode === "local") {
    files.set(path, bytes);
  } else {
    const file = join(out, `${step}-${servedName}`);
    writeFileSync(file, bytes);
    pnpm(["exec", "wrangler", "r2", "object", "put", `${BUCKET}${path}`, "--file", file, "--remote"], updates);
  }
  putManifest(manifestFor(key, `${DOWNLOADS}${path}`, signature, new Date()));
  // The Worker caches a manifest for up to 30 s: wait until it offers this step's bundle.
  for (let tries = 0; tries < 60; tries++) {
    const answer = await fetch(`${worker}${endpointPath(here, OLD_VERSION)}`, {
      headers: { [INSTALL_ID_HEADER]: randomUUID() },
    });
    if (answer.status === 200 && (await answer.text()).includes(path)) return;
    await sleep(2000);
  }
  throw new Error(`the Worker didn't offer ${path} within 120 s`);
}

// --- The app ---------------------------------------------------------------------------------

/** Installs 0.0.1 and returns the program to launch. */
function install(): string {
  const apps = join(out, "installed");
  mkdirSync(apps, { recursive: true });
  if (platform === "darwin") {
    const app = join(apps, basename(oldBuild));
    cpSync(oldBuild, app, { recursive: true });
    spawnSync("xattr", ["-dr", "com.apple.quarantine", app]);
    const plist = join(app, "Contents", "Info.plist");
    const program = execFileSync("plutil", ["-extract", "CFBundleExecutable", "raw", plist], {
      encoding: "utf8",
    }).trim();
    return join(app, "Contents", "MacOS", program);
  }
  if (platform === "win32") {
    log(`running ${basename(oldBuild)} /S`);
    execFileSync(oldBuild, ["/S"], { stdio: "ignore", timeout: 300_000 });
    const dir = join(required("LOCALAPPDATA"), "Lamina PDF");
    const program = readdirSync(dir).find((name) => /\.exe$/i.test(name) && !/^uninstall|engine/i.test(name));
    if (program === undefined) throw new Error(`no program in ${dir}: ${readdirSync(dir).join(", ")}`);
    return join(dir, program);
  }
  const image = join(apps, basename(oldBuild));
  cpSync(oldBuild, image);
  chmodSync(image, 0o755);
  return image;
}

/** Launches the app at `program` for step `step`, reporting to its own file. */
function launch(program: string, server: string, step: string): string {
  log(`launching the app for the ${step} step`);
  const report = join(out, `${step}.report`);
  rmSync(report, { force: true });
  // A file, not the environment: Windows' installer relaunches the updated app as a fresh process.
  mkdirSync(dirname(updateTestFile), { recursive: true });
  writeFileSync(updateTestFile, JSON.stringify({ server, report }));
  const logFile = join(out, `${step}.log`);
  const app = spawn(program, [], {
    env: {
      ...env,
      // GitHub's runners have no FUSE: the AppImage runs from a copy it unpacks.
      APPIMAGE_EXTRACT_AND_RUN: "1",
      RUST_LOG: "info,laminapdf_desktop=debug,tauri_plugin_updater=debug",
    },
    stdio: ["ignore", "pipe", "pipe"],
  });
  logTo(app, logFile);
  const started = Date.now();
  app.once("exit", (code, signal) => {
    // The single-instance plugin hands a second launch to a copy already running and exits.
    if (Date.now() - started < 5000 && lines(report).length === 0) {
      log(`the app exited at once (${code ?? signal}): is another copy of the e2e build running?`);
    }
  });
  return report;
}

/**
 * Stops the test's copy of the app, which after an update isn't this script's child: on macOS the
 * processes under the installed .app, on Linux the AppImage's unpacked copy (named after the
 * image), on Windows the installed program. Nothing else is stopped: a developer's own copy of
 * Lamina PDF keeps running.
 */
function stopApp(program: string): void {
  log("stopping the app");
  if (platform === "win32") {
    const path = program.replaceAll("'", "''");
    spawnSync(
      "powershell",
      ["-NoProfile", "-Command", `Get-Process | Where-Object { $_.Path -eq '${path}' } | Stop-Process -Force`],
      { timeout: 60_000 },
    );
  } else if (platform === "darwin") {
    spawnSync("pkill", ["-f", join(out, "installed")]);
  } else {
    // The AppImage's unpacked copy runs from a temporary folder; its environment names the image.
    for (const pid of readdirSync("/proc").filter((name) => /^\d+$/.test(name))) {
      try {
        if (readFileSync(`/proc/${pid}/environ`, "utf8").split("\0").includes(`APPIMAGE=${program}`)) {
          kill(Number(pid), "SIGKILL");
        }
      } catch {
        // Gone already, or not ours to read.
      }
    }
  }
}

function lines(report: string): string[] {
  return existsSync(report)
    ? readFileSync(report, "utf8")
        .split("\n")
        .filter((line) => line !== "")
    : [];
}

/** Waits for a line of `report` that `done` accepts, or fails at `limitMs`. */
async function waitFor(
  report: string,
  done: (line: string) => boolean,
  what: string,
  limitMs: number,
): Promise<string[]> {
  const end = Date.now() + limitMs;
  while (Date.now() < end) {
    const found = lines(report);
    if (found.some(done)) return found;
    await sleep(500);
  }
  throw new Error(
    `the app didn't report ${what} within ${limitMs / 1000} s; it reported: ${JSON.stringify(lines(report))}`,
  );
}

// --- The run ---------------------------------------------------------------------------------

/** The installed program, for the clean-up when the run fails half-way. */
let installed: string | undefined;

async function main(): Promise<void> {
  rmSync(out, { recursive: true, force: true });
  mkdirSync(out, { recursive: true });
  const worker = mode === "local" ? await startLocalWorker() : STAGING_WORKER;
  const filesOrigin = mode === "local" ? `http://127.0.0.1:${await listen(fileServer)}` : undefined;
  const server = `http://127.0.0.1:${await listen(proxy(worker, filesOrigin))}`;
  log(`${mode} mode, ${key}: the Worker at ${worker}, the app asks ${server}`);

  const program = install();
  installed = program;
  log(`installed ${OLD_VERSION}: ${program}`);
  const bundle = new Uint8Array(readFileSync(newBundle));

  // A tampered bundle is refused, and the app stays on 0.0.1.
  await publish(worker, "tampered", tampered(bundle));
  const refused = launch(program, server, "tampered");
  const after = await waitFor(
    refused,
    (line) => line.startsWith("failed: ") || line.startsWith("ready "),
    "the failed check",
    180_000,
  );
  await sleep(5000);
  stopApp(program);
  const failure = after.find((line) => line.startsWith("failed: "));
  if (failure === undefined || !/signature|minisign/i.test(failure)) {
    throw new Error(`the tampered bundle wasn't refused for its signature: ${JSON.stringify(lines(refused))}`);
  }
  if (lines(refused).some((line) => line === `started ${NEW_VERSION}`))
    throw new Error("the tampered bundle installed");
  log(`refused the tampered bundle: ${failure}`);
  const tamperedRequests = seen.length;

  // The real bundle installs, and the app restarts as 0.0.2.
  await publish(worker, "update", bundle);
  const updated = launch(program, server, "update");
  await waitFor(
    updated,
    (line) => {
      if (line.startsWith("failed: ")) throw new Error(`the update failed: ${line}`);
      return line === `started ${NEW_VERSION}`;
    },
    `started ${NEW_VERSION}`,
    300_000,
  );
  stopApp(program);
  // The refused update left a working 0.0.1, which is what updated.
  if (lines(updated)[0] !== `started ${OLD_VERSION}`) {
    throw new Error(`the update step didn't start on ${OLD_VERSION}: ${JSON.stringify(lines(updated))}`);
  }
  log(`updated: ${JSON.stringify(lines(updated))}`);

  // Every request, the restarted 0.0.2's included: the header, never the URL.
  const problems = requestProblems(seen, here, [OLD_VERSION, NEW_VERSION]);
  if (tamperedRequests === 0) problems.push("the app made no request while the tampered bundle was offered");
  if (problems.length > 0) throw new Error(`the update requests were wrong:\n  ${problems.join("\n  ")}`);
  log(`${seen.length} update request(s), each with the install ID in the header only`);
}

let failed = false;
try {
  await main();
} catch (error) {
  failed = true;
  console.error(`update-test: ${error instanceof Error ? error.message : String(error)}`);
} finally {
  if (installed !== undefined) stopApp(installed);
  if (mode === "staging") {
    try {
      putManifest(emptyManifest());
      for (const step of ["tampered", "update"]) {
        pnpm(
          ["exec", "wrangler", "r2", "object", "delete", `${BUCKET}/e2e/${run}/${step}/${servedName}`, "--remote"],
          updates,
        );
      }
    } catch (error) {
      console.error(`update-test: couldn't clean the staging channel up: ${String(error)}`);
      failed = true;
    }
  }
  rmSync(updateTestFile, { force: true });
  wranglerDev?.kill();
  fileServer.close();
}
exit(failed ? 1 : 0);
