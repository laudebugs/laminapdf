// The four tile transports (raw over a channel, raw invoke responses, raw over the custom
// scheme, compressed over the custom scheme). Each turns a tile request into a checked
// ImageBitmap, the reader's paint input.
import { Channel, convertFileSrc, invoke } from "@tauri-apps/api/core";
import { decodeQoi } from "./qoi";

export const TRANSPORTS = ["channel", "response", "scheme-raw", "scheme-png", "scheme-webp", "scheme-qoi"] as const;
export type TransportName = (typeof TRANSPORTS)[number];

export interface TileReq {
  doc: number;
  page: number;
  scaleMilli: number;
  x: number;
  y: number;
  size: number;
}

export interface Delivered {
  bitmap: ImageBitmap;
  /** Bytes that crossed the transport (header included for raw tiles). */
  bytes: number;
}

export interface Transport {
  fetch(id: number, req: TileReq, verify: boolean): Promise<Delivered>;
}

// "lamina://localhost/" on macOS and Linux, "http://lamina.localhost/" on Windows.
const SCHEME_BASE = convertFileSrc("", "lamina");

export function fnv1a(bytes: Uint8Array): number {
  let h = 0x811c9dc5;
  for (let i = 0; i < bytes.length; i++) h = Math.imul(h ^ bytes[i], 0x01000193);
  return h >>> 0;
}

export class CorruptTile extends Error {}

async function fromRaw(id: number, raw: ArrayBuffer | number[]): Promise<Delivered> {
  const buf = raw instanceof ArrayBuffer ? raw : new Uint8Array(raw).buffer;
  const view = new DataView(buf);
  const [gotId, w, h, sum] = [0, 4, 8, 12].map((o) => view.getUint32(o, true));
  if (w === 0) throw new Error(`tile ${id} failed in Rust`);
  if (gotId !== id || buf.byteLength !== 16 + w * h * 4) throw new CorruptTile(`tile ${id}: bad header or length`);
  // Raw tiles are always checked: it costs one row.
  if (fnv1a(new Uint8Array(buf, 16, w * 4)) !== sum) throw new CorruptTile(`tile ${id}: first row differs`);
  const image = new ImageData(new Uint8ClampedArray(buf, 16, w * h * 4), w, h);
  return { bitmap: await createImageBitmap(image), bytes: buf.byteLength };
}

/** Reads back a decoded tile's first row, for the verify pass of compressed tiles. */
function firstRowSum(bitmap: ImageBitmap): number {
  const canvas = new OffscreenCanvas(bitmap.width, 1);
  const ctx = canvas.getContext("2d", { willReadFrequently: true });
  if (!ctx) throw new Error("no 2d context");
  ctx.drawImage(bitmap, 0, 0);
  return fnv1a(new Uint8Array(ctx.getImageData(0, 0, bitmap.width, 1).data.buffer));
}

function tileUrl(id: number, r: TileReq, fmt: string): string {
  const q = fmt === "raw" ? `id=${id}` : `id=${id}&fmt=${fmt}`;
  return `${SCHEME_BASE}tile/${r.doc}/${r.page}/${r.scaleMilli}/${r.x}/${r.y}/${r.size}?${q}`;
}

/** Batches the tiles asked for in one task into one `stream_tiles` call with its own channel. */
class ChannelTransport implements Transport {
  private queue: { id: number; req: TileReq; resolve: (d: Delivered) => void; reject: (e: unknown) => void }[] = [];

  fetch(id: number, req: TileReq): Promise<Delivered> {
    return new Promise((resolve, reject) => {
      if (this.queue.length === 0) queueMicrotask(() => this.flush());
      this.queue.push({ id, req, resolve, reject });
    });
  }

  private flush(): void {
    const batch = this.queue;
    this.queue = [];
    const waiting = new Map(batch.map((j) => [j.id, j]));
    // A channel per batch: message indexes restart with every Rust-side Channel.
    const channel = new Channel<ArrayBuffer>();
    channel.onmessage = (message) => {
      const id = new DataView(message instanceof ArrayBuffer ? message : new Uint8Array(message).buffer).getUint32(0, true);
      const job = waiting.get(id);
      if (!job) return;
      waiting.delete(id);
      fromRaw(id, message).then(job.resolve, job.reject);
    };
    const jobs = batch.map((j) => ({ id: j.id, ...j.req }));
    invoke("stream_tiles", { jobs, onTile: channel }).catch((e) => {
      for (const job of waiting.values()) job.reject(e);
    });
  }
}

class ResponseTransport implements Transport {
  async fetch(id: number, req: TileReq): Promise<Delivered> {
    return fromRaw(id, await invoke<ArrayBuffer>("tile", { job: { id, ...req } }));
  }
}

class SchemeTransport implements Transport {
  constructor(private readonly fmt: "raw" | "png" | "webp" | "qoi") {}

  async fetch(id: number, req: TileReq, verify: boolean): Promise<Delivered> {
    const response = await fetch(tileUrl(id, req, this.fmt));
    if (!response.ok) throw new Error(`tile ${id}: HTTP ${response.status} ${await response.text()}`);
    if (this.fmt === "raw") return fromRaw(id, await response.arrayBuffer());
    const [, w, h, sum] = (response.headers.get("X-Tile") ?? "").split(",").map(Number);
    let bitmap: ImageBitmap;
    let bytes: number;
    const options: ImageBitmapOptions = { colorSpaceConversion: "none", premultiplyAlpha: "none" };
    if (this.fmt === "qoi") {
      const body = await response.arrayBuffer();
      bytes = body.byteLength;
      bitmap = await createImageBitmap(decodeQoi(body), options);
    } else {
      const blob = await response.blob();
      bytes = blob.size;
      bitmap = await createImageBitmap(blob, options);
    }
    if (bitmap.width !== w || bitmap.height !== h) throw new CorruptTile(`tile ${id}: ${bitmap.width}×${bitmap.height}, expected ${w}×${h}`);
    if (verify && firstRowSum(bitmap) !== sum) throw new CorruptTile(`tile ${id}: decoded first row differs`);
    return { bitmap, bytes };
  }
}

export function transport(name: TransportName): Transport {
  switch (name) {
    case "channel":
      return new ChannelTransport();
    case "response":
      return new ResponseTransport();
    case "scheme-raw":
      return new SchemeTransport("raw");
    case "scheme-png":
      return new SchemeTransport("png");
    case "scheme-webp":
      return new SchemeTransport("webp");
    case "scheme-qoi":
      return new SchemeTransport("qoi");
  }
}

export { SCHEME_BASE };
