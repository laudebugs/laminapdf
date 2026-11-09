// Cross-origin isolation: does the webview honour app.security.headers (COOP/COEP), and do
// custom-scheme tiles and IPC still load under COEP? One CSV row per launch.
import { invoke } from "@tauri-apps/api/core";
import { SCHEME_BASE } from "./transports";

async function fetches(url: string): Promise<string> {
  try {
    const r = await fetch(url);
    return r.ok ? "ok" : `http ${r.status}`;
  } catch (e) {
    return `blocked (${e instanceof Error ? e.name : "error"})`;
  }
}

function loadsAsImage(url: string): Promise<string> {
  return new Promise((resolve) => {
    const img = new Image();
    img.onload = () => resolve("ok");
    img.onerror = () => resolve("blocked");
    img.src = url;
  });
}

export async function runCoi(): Promise<void> {
  let sab = "unavailable";
  if (typeof SharedArrayBuffer === "function") {
    try {
      new SharedArrayBuffer(1);
      sab = "ok";
    } catch {
      sab = "throws";
    }
  }
  let ipc = "ok";
  try {
    await invoke("plan");
  } catch (e) {
    ipc = `failed: ${String(e)}`;
  }
  const tile = `${SCHEME_BASE}tile/1/0/1333/0/0/256`;
  const row: Record<string, unknown> = {
    kind: "coi",
    file: "coi",
    origin: location.origin,
    scheme_base: SCHEME_BASE,
    cross_origin_isolated: String(self.crossOriginIsolated),
    shared_array_buffer: sab,
    ipc_invoke: ipc,
    user_agent: navigator.userAgent,
  };
  for (const corp of ["same-origin", "cross-origin", "none"]) {
    row[`fetch_corp_${corp}`] = await fetches(`${tile}?id=1&corp=${corp}`);
    row[`img_corp_${corp}`] = await loadsAsImage(`${tile}?id=1&fmt=png&corp=${corp}`);
  }
  document.getElementById("status")!.textContent = JSON.stringify(row);
  await invoke("metrics", { msg: row });
  await invoke("metrics", { msg: { kind: "done", code: 0 } });
}
