// The bench page: one module worker (src/worker.ts) holds both engines. A file picked by hand is
// benchmarked with every measurement; Playwright calls `window.spike.run(…)` with fixture URLs
// served from out/public/fixtures/ (scripts/make-fixtures.mjs).

interface Job {
  url: string;
  name: string;
  password?: string;
  strategy?: "copy" | "custom";
  allPages?: boolean;
  reload?: boolean;
}

const worker = new Worker(new URL("./worker.ts", import.meta.url), { type: "module" });
let next = 0;
const pending = new Map<number, { resolve: (v: unknown) => void; reject: (e: Error) => void }>();
/** When each bench's first page was drawn on the main thread. */
const drawnAt = new Map<number, number>();

worker.onmessage = (event: MessageEvent) => {
  const { id, ok, result, error, bitmap, first } = event.data as {
    id: number;
    ok: boolean;
    result?: unknown;
    error?: string;
    bitmap?: ImageBitmap;
    first?: boolean;
  };
  if (first && bitmap !== undefined) {
    draw(bitmap);
    drawnAt.set(id, performance.now());
    return;
  }
  const p = pending.get(id);
  pending.delete(id);
  if (ok) p?.resolve(result);
  else p?.reject(new Error(error));
};

// A worker that dies (out of memory, a crash) settles every pending call instead of hanging it.
const failAll = (reason: string) => {
  for (const [, p] of pending) p.reject(new Error(reason));
  pending.clear();
};
worker.onerror = (e) => failAll(`worker error: ${e.message}`);
worker.onmessageerror = () => failAll("worker message could not be read");

function call(type: string, payload?: unknown, onId?: (id: number) => void): Promise<unknown> {
  const id = next++;
  onId?.(id);
  return new Promise((resolve, reject) => {
    pending.set(id, { resolve, reject });
    worker.postMessage({ id, type, payload });
  });
}

/** Draws a page bitmap on the page's canvas: the last step of "first page visible". */
function draw(bitmap: ImageBitmap): void {
  const canvas = document.getElementById("page") as HTMLCanvasElement;
  canvas.width = bitmap.width;
  canvas.height = bitmap.height;
  canvas.getContext("2d")?.drawImage(bitmap, 0, 0);
  bitmap.close();
}

let booted: Promise<unknown> | undefined;

/** Peak memory as the browser reports it (Chromium, cross-origin isolated only). */
async function uaMemory(): Promise<number | null> {
  const measure = (performance as unknown as { measureUserAgentSpecificMemory?: () => Promise<{ bytes: number }> })
    .measureUserAgentSpecificMemory;
  if (measure === undefined || !crossOriginIsolated) return null;
  try {
    return (await measure.call(performance)).bytes;
  } catch {
    return null;
  }
}

async function bench(file: File, job: Omit<Job, "url" | "name">): Promise<unknown> {
  const selected = performance.now();
  let id = -1;
  const result = (await call("bench", { file, ...job }, (i) => {
    id = i;
  })) as Record<string, unknown>;
  // From the file being handed over to page 1 drawn on the main thread's canvas.
  const drawn = drawnAt.get(id);
  result.firstPageVisibleMs = drawn === undefined ? null : drawn - selected;
  result.uaMemoryBytes = await uaMemory();
  return result;
}

const spike = {
  boot: () => {
    booted ??= call("boot");
    return booted;
  },
  /** Fetches each fixture, hands it to the worker as a File, and returns every result. */
  async run(jobs: Job[]): Promise<unknown[]> {
    await spike.boot();
    const out: unknown[] = [];
    for (const job of jobs) {
      const blob = await (await fetch(job.url)).blob();
      const file = new File([blob], job.name, { type: "application/pdf" });
      out.push(await bench(file, job));
    }
    return out;
  },
};

(window as unknown as { spike: typeof spike }).spike = spike;

document.getElementById("run")?.addEventListener("click", async () => {
  const input = document.getElementById("file") as HTMLInputElement;
  const file = input.files?.[0];
  if (file === undefined) return;
  const out = document.getElementById("out") as HTMLPreElement;
  out.textContent = "booting…";
  const boot = await spike.boot();
  out.textContent = JSON.stringify({ boot, result: await bench(file, {}) }, null, 2);
});
