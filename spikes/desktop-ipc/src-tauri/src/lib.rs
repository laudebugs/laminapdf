//! Spike D1a: four ways to move rendered tiles from Rust into the webview, measured by the
//! script in `src/bench.ts`. Every raw tile starts with a 16-byte header (little-endian id,
//! width, height, FNV-1a of the first row) that the webview checks.

mod encode;
mod probe;
mod tiles;

use std::fs::OpenOptions;
use std::io::Write as _;
use std::path::PathBuf;
use std::sync::{Arc, PoisonError};
use std::time::Instant;

use serde::Deserialize;
use serde_json::{Map, Value};
use tauri::http::{Request, Response as HttpResponse};
use tauri::ipc::{Channel, InvokeResponseBody, Response};
use tauri::{AppHandle, State};

use crate::tiles::{Tile, TileRequest, Tiles};

struct App {
    tiles: Arc<Tiles>,
    probe: probe::Probe,
    results: PathBuf,
    label: String,
    mode: String,
}

#[derive(Deserialize)]
struct TileJob {
    id: u32,
    #[serde(flatten)]
    req: TileRequest,
}

fn header(id: u32, tile: &Tile) -> [u8; 16] {
    let mut h = [0; 16];
    for (i, v) in [id, tile.width, tile.height, tile.first_row_sum()].into_iter().enumerate() {
        h[i * 4..i * 4 + 4].copy_from_slice(&v.to_le_bytes());
    }
    h
}

/// Header and pixels in one owned buffer: the copy a real transport would make too.
fn raw_payload(id: u32, tile: &Tile) -> Vec<u8> {
    let mut out = Vec::with_capacity(16 + tile.rgba.len());
    out.extend_from_slice(&header(id, tile));
    out.extend_from_slice(&tile.rgba);
    out
}

/// What the webview runs: the label, the plan (`quick`, `full` or `coi`) and the documents.
#[tauri::command]
fn plan(app: State<'_, App>) -> Value {
    serde_json::json!({ "label": app.label, "mode": app.mode, "docs": app.tiles.docs() })
}

/// Transport 2: one invoke per tile, answered with a raw body (no JSON).
#[tauri::command]
async fn tile(app: State<'_, App>, job: TileJob) -> Result<Response, String> {
    let tiles = Arc::clone(&app.tiles);
    let tile = tauri::async_runtime::spawn_blocking(move || tiles.get(&job.req))
        .await
        .map_err(|e| e.to_string())??;
    Ok(Response::new(raw_payload(job.id, &tile)))
}

/// Transport 1: a batch of tiles streamed back over a channel as they are ready. A failed
/// tile is sent as a header with width and height 0.
#[tauri::command]
async fn stream_tiles(
    app: State<'_, App>,
    jobs: Vec<TileJob>,
    on_tile: Channel<InvokeResponseBody>,
) -> Result<(), String> {
    let tiles = Arc::clone(&app.tiles);
    tauri::async_runtime::spawn_blocking(move || {
        for job in jobs {
            let payload = match tiles.get(&job.req) {
                Ok(tile) => raw_payload(job.id, &tile),
                Err(_) => header(job.id, &Tile { width: 0, height: 0, rgba: Vec::new() }).to_vec(),
            };
            on_tile.send(InvokeResponseBody::Raw(payload)).map_err(|e| e.to_string())?;
        }
        Ok(())
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Run control and results: `start` resets the counters and starts the probe, `result`, `coi`
/// and `note` append a CSV row to `results/<file>.csv`, `log` prints, `done` quits.
#[tauri::command]
fn metrics(handle: AppHandle, app: State<'_, App>, msg: Map<String, Value>) -> Result<(), String> {
    match msg.get("kind").and_then(Value::as_str) {
        Some("start") => {
            *app.tiles.timings.lock().unwrap_or_else(PoisonError::into_inner) = tiles::Timings::default();
            app.probe.start();
        }
        Some("result" | "coi" | "note") => {
            let mut row = msg.clone();
            if msg.get("kind").and_then(Value::as_str) == Some("result") {
                let sample = app.probe.stop();
                let t = *app.tiles.timings.lock().unwrap_or_else(PoisonError::into_inner);
                let ms = |ns: u64, n: u64| if n == 0 { 0.0 } else { ns as f64 / n as f64 / 1e6 };
                row.extend(serde_json::to_value(sample).map_err(|e| e.to_string())?.as_object().cloned().unwrap_or_default());
                row.insert("render_ms_avg".into(), ms(t.render_ns, t.renders).into());
                row.insert("renders".into(), t.renders.into());
                row.insert("encode_ms_avg".into(), ms(t.encode_ns, t.encodes).into());
            }
            row.insert("label".into(), app.label.clone().into());
            let file = msg.get("file").and_then(Value::as_str).unwrap_or("results");
            append_csv(&app.results.join(format!("{}-{file}.csv", app.label)), &row)?;
        }
        Some("log") => println!("[bench] {}", msg.get("text").and_then(Value::as_str).unwrap_or("")),
        Some("done") => {
            let code = msg.get("code").and_then(Value::as_i64).unwrap_or(0);
            handle.exit(i32::try_from(code).unwrap_or(1));
        }
        _ => return Err("unknown metrics message".into()),
    }
    Ok(())
}

fn csv_cell(v: &Value) -> String {
    let s = match v {
        Value::String(s) => s.clone(),
        other => other.to_string(),
    };
    if s.contains([',', '"', '\n']) { format!("\"{}\"", s.replace('"', "\"\"")) } else { s }
}

/// Appends `row` (keys in sorted order); writes the header when the file is new.
fn append_csv(path: &std::path::Path, row: &Map<String, Value>) -> Result<(), String> {
    let mut keys: Vec<&String> = row.keys().filter(|k| k.as_str() != "kind" && k.as_str() != "file").collect();
    keys.sort();
    let new = !path.exists();
    let mut f = OpenOptions::new().create(true).append(true).open(path).map_err(|e| e.to_string())?;
    let mut text = String::new();
    if new {
        text.push_str(&keys.iter().map(|k| k.as_str()).collect::<Vec<_>>().join(","));
        text.push('\n');
    }
    text.push_str(&keys.iter().map(|k| csv_cell(&row[*k])).collect::<Vec<_>>().join(","));
    text.push('\n');
    f.write_all(text.as_bytes()).map_err(|e| e.to_string())
}

/// Transports 3 and 4: `lamina://localhost/tile/{doc}/{page}/{scale}/{x}/{y}/{size}?id=&fmt=&corp=`
/// (`http://lamina.localhost/...` on Windows), and `/ping` for the isolation check.
fn scheme(tiles: &Tiles, request: &Request<Vec<u8>>) -> HttpResponse<Vec<u8>> {
    let uri = request.uri();
    let query: Vec<(&str, &str)> =
        uri.query().unwrap_or("").split('&').filter_map(|kv| kv.split_once('=')).collect();
    let param = |name: &str| query.iter().find(|(k, _)| *k == name).map(|(_, v)| *v);
    let corp = match param("corp") {
        Some("cross-origin") => "cross-origin",
        Some("none") => "",
        _ => "same-origin",
    };
    let mut response = HttpResponse::builder()
        .header("Cache-Control", "no-store")
        .header("Access-Control-Allow-Origin", "*")
        .header("Access-Control-Expose-Headers", "X-Tile");
    if !corp.is_empty() {
        response = response.header("Cross-Origin-Resource-Policy", corp);
    }
    let result = (|| -> Result<(Vec<u8>, &'static str, String), String> {
        let path: Vec<&str> = uri.path().trim_start_matches('/').split('/').collect();
        if path.first() == Some(&"ping") {
            return Ok((b"pong".to_vec(), "text/plain", String::new()));
        }
        let num = |i: usize| -> Result<u32, String> {
            path.get(i).and_then(|s| s.parse().ok()).ok_or_else(|| format!("bad path {}", uri.path()))
        };
        let req = TileRequest {
            doc: num(1)?,
            page: num(2)?,
            scale_milli: num(3)?,
            x: num(4)?,
            y: num(5)?,
            size: u16::try_from(num(6)?).map_err(|e| e.to_string())?,
        };
        let id: u32 = param("id").and_then(|s| s.parse().ok()).unwrap_or(0);
        let tile = tiles.get(&req)?;
        let info = format!("{id},{},{},{}", tile.width, tile.height, tile.first_row_sum());
        match param("fmt").and_then(encode::Format::parse) {
            None => Ok((raw_payload(id, &tile), "application/octet-stream", info)),
            Some(format) => {
                let start = Instant::now();
                let body = encode::encode(&tile, format)?;
                tiles.record(start, true);
                Ok((body, format.content_type(), info))
            }
        }
    })();
    let built = match result {
        Ok((body, content_type, info)) => {
            response.status(200).header("Content-Type", content_type).header("X-Tile", info).body(body)
        }
        Err(e) => response.status(500).header("Content-Type", "text/plain").body(e.into_bytes()),
    };
    built.unwrap_or_else(|_| HttpResponse::new(Vec::new()))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
#[expect(clippy::disallowed_types, reason = "tauri::generate_context! builds a HashMap")]
pub fn run() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let repo = std::env::var_os("LAMINA_SPIKE_REPO").map_or_else(|| root.join("../../.."), PathBuf::from);
    let results = std::env::var_os("LAMINA_SPIKE_RESULTS").map_or_else(|| root.join("../results"), PathBuf::from);
    let label = std::env::var("LAMINA_SPIKE_LABEL").unwrap_or_else(|_| std::env::consts::OS.to_owned());
    let mode = std::env::var("LAMINA_SPIKE_MODE").unwrap_or_else(|_| "quick".to_owned());
    let tiles = match Tiles::open(&repo) {
        Ok(tiles) => Arc::new(tiles),
        Err(e) => {
            eprintln!("desktop-ipc: {e}");
            std::process::exit(2);
        }
    };
    if let Err(e) = std::fs::create_dir_all(&results) {
        eprintln!("desktop-ipc: {}: {e}", results.display());
        std::process::exit(2);
    }
    let scheme_tiles = Arc::clone(&tiles);
    let result = tauri::Builder::default()
        .manage(App { tiles, probe: probe::Probe::default(), results, label, mode })
        .register_asynchronous_uri_scheme_protocol("lamina", move |_ctx, request, responder| {
            let tiles = Arc::clone(&scheme_tiles);
            // Rendering and encoding never run on the main thread.
            tauri::async_runtime::spawn_blocking(move || responder.respond(scheme(&tiles, &request)));
        })
        .invoke_handler(tauri::generate_handler![plan, tile, stream_tiles, metrics])
        .run(tauri::generate_context!());
    if let Err(e) = result {
        eprintln!("desktop-ipc: {e}");
        std::process::exit(1);
    }
}
