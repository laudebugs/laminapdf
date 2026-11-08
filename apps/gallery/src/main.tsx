// The Paper gallery (todo U6f): every token, icon and `@laminapdf/ui` component in every state, in
// the theme, page appearance, density, direction and motion the URL chooses, so a screenshot can
// address any combination: `?route=buttons&theme=dark&density=touch&dir=rtl`. Dev-only.

import "@laminapdf/tokens/tokens.css";
import "@laminapdf/fonts/fonts.css";
import "@laminapdf/ui/styles.css";
import "./gallery.css";
import { FONT_PRELOADS } from "@laminapdf/fonts";
import { DensityProvider, PlatformProvider } from "@laminapdf/ui";
import { EXAMPLES, fakePlatform } from "@laminapdf/ui/testing";
import { type ReactNode, StrictMode, useEffect } from "react";
import { createRoot } from "react-dom/client";
import { Icons } from "./icons.tsx";
import { Dialogs, Menus, Palette, ReaderParts, Selection, Sheets, Toasts } from "./overlays.tsx";
import {
  APPEARANCES,
  DENSITIES,
  DIRECTIONS,
  MOTIONS,
  parseSettings,
  ROUTES,
  type Route,
  type Settings,
  settingsQuery,
  THEMES,
} from "./routes.ts";
import { Tokens } from "./tokens.tsx";

for (const font of FONT_PRELOADS) {
  const link = document.createElement("link");
  Object.assign(link, { rel: "preload", as: "font", type: font.type, href: font.href, crossOrigin: "anonymous" });
  document.head.append(link);
}

/** The states of `route`'s components, as a grid of labelled cells. */
function StateGrid({ route }: { route: Route }): ReactNode {
  const examples = EXAMPLES.filter((e) => route.components.includes(e.component));
  return (
    <ul className="g-grid">
      {examples.map((example) => (
        <li key={`${example.component}-${example.state}`} className="g-cell">
          <span className="g-label">
            {example.component} · {example.state}
          </span>
          <div className="g-sample">{example.element()}</div>
        </li>
      ))}
    </ul>
  );
}

/** What `route` shows beyond the state grid. */
function Extra({ route, state, theme }: { route: Route; state: string; theme: string }): ReactNode {
  const open = state || route.states?.[0] || "";
  switch (route.id) {
    case "tokens":
      return <Tokens theme={theme} />;
    case "icons":
      return <Icons />;
    case "reader-shell":
      return <ReaderParts />;
    case "dialogs":
      return <Dialogs state={open} />;
    case "sheets":
      return <Sheets state={open} />;
    case "menus":
      return <Menus state={open} />;
    case "selection-toolbar":
      return <Selection />;
    case "palette":
      return <Palette state={open} />;
    case "toasts":
      return <Toasts />;
    default:
      return null;
  }
}

/** A select that changes one setting through the URL. */
function Control<K extends keyof Settings>(props: {
  name: K;
  label: string;
  values: readonly string[];
  settings: Settings;
}): ReactNode {
  const { name, label, values, settings } = props;
  return (
    <label className="g-control">
      {label}
      <select
        id={`g-${name}`}
        value={settings[name]}
        onChange={(event) => {
          location.search = settingsQuery({ ...settings, [name]: event.target.value });
        }}
      >
        {values.map((v) => (
          <option key={v} value={v}>
            {v}
          </option>
        ))}
      </select>
    </label>
  );
}

function Index({ settings }: { settings: Settings }): ReactNode {
  return (
    <nav aria-label="Gallery pages">
      <ul className="g-index">
        {ROUTES.map((route) => (
          <li key={route.id}>
            <a href={settingsQuery({ ...settings, route: route.id, state: "" }) || "?"}>{route.title}</a>
            {route.states === undefined ? null : <span className="g-note"> ({route.states.join(", ")})</span>}
          </li>
        ))}
      </ul>
    </nav>
  );
}

function App({ settings }: { settings: Settings }): ReactNode {
  const route = ROUTES.find((r) => r.id === settings.route);
  // The reader route draws ReaderShell, which brings the page's banner and main landmarks itself:
  // the gallery's own header and stage would duplicate them, so there they are plain divisions.
  const shell = route?.renders?.includes("ReaderShell") === true;
  const Chrome = shell ? "div" : "header";
  const Stage = shell ? "div" : "main";
  useEffect(() => {
    const root = document.documentElement;
    root.dataset.theme = settings.theme;
    root.dir = settings.dir;
    root.dataset.motion = settings.motion;
    document.title = route === undefined ? "Paper gallery" : `${route.title} · Paper gallery`;
  }, [settings, route]);
  return (
    <DensityProvider density={settings.density}>
      <Chrome className="g-header">
        <h1>
          <a href={settingsQuery({ ...settings, route: "", state: "" }) || "?"}>Paper gallery</a>
          {route === undefined ? null : <span> · {route.title}</span>}
        </h1>
        <div className="g-controls">
          <Control name="theme" label="Theme" values={THEMES} settings={settings} />
          <Control name="appearance" label="Page" values={APPEARANCES} settings={settings} />
          <Control name="density" label="Density" values={DENSITIES} settings={settings} />
          <Control name="dir" label="Direction" values={DIRECTIONS} settings={settings} />
          <Control name="motion" label="Motion" values={MOTIONS} settings={settings} />
          {route?.states === undefined ? null : (
            <Control
              name="state"
              label="Open"
              values={route.states}
              settings={{ ...settings, state: settings.state || (route.states[0] ?? "") }}
            />
          )}
        </div>
      </Chrome>
      <Stage id="stage" data-page-appearance={route?.pageRelated ? settings.appearance : undefined}>
        {route === undefined ? (
          <Index settings={settings} />
        ) : (
          <>
            {route.components.length > 0 ? <StateGrid route={route} /> : null}
            <Extra route={route} state={settings.state} theme={settings.theme} />
          </>
        )}
      </Stage>
    </DensityProvider>
  );
}

/**
 * The platform the reader components read capabilities from: a stand-in that can do nothing
 * optional, so they draw their plain states and nothing touches the real file system.
 */
const PLATFORM = fakePlatform();

const root = document.getElementById("root");
if (root !== null) {
  createRoot(root).render(
    <StrictMode>
      <PlatformProvider platform={PLATFORM}>
        <App settings={parseSettings(location.search)} />
      </PlatformProvider>
    </StrictMode>,
  );
}
