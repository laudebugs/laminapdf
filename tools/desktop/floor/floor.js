// The floor page's one script: the app's first command, sent as soon as the webview runs any
// script. The CSP allows no inline script, so it is a file.
window.__TAURI_INTERNALS__.invoke("app_info");
