// The floor page's one script: the app's first command, sent as soon as the webview runs any
// script. The CSP allows no inline script, so it is a file, and with no bundler it calls Tauri's
// internal invoke, which `@tauri-apps/api` wraps.
window.__TAURI_INTERNALS__.invoke("app_info");
