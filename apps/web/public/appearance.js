// Sets the stored theme, accent and motion on the document element before the first paint, so a
// dark or blue reader never flashes the default look. A classic script loaded from this origin
// (the CSP allows no inline script); src/settings.tsx keeps the same key and applies the same
// attributes once the app runs.
(() => {
  // biome-ignore lint/style/noRestrictedGlobals: runs before the app, with nothing but the page
  const root = document.documentElement;
  let stored = {};
  try {
    // biome-ignore lint/style/noRestrictedGlobals: as above; src/settings.tsx writes this key
    stored = JSON.parse(localStorage.getItem("lamina.appearance") ?? "{}") ?? {};
  } catch {
    // Storage blocked or a damaged entry: the default look.
  }
  if (["light", "dark", "high-contrast"].includes(stored.theme)) root.setAttribute("data-theme", stored.theme);
  root.setAttribute("data-accent", ["teal", "blue", "plum", "green"].includes(stored.accent) ? stored.accent : "teal");
  if (stored.motion === "reduce") root.setAttribute("data-motion", "reduce");
})();
