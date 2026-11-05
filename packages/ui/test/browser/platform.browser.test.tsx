import { afterEach, describe, expect, it } from "vitest";
import {
  type Capability,
  createWebPlatform,
  type ExternalStore,
  PlatformProvider,
  ReaderController,
  useCapability,
  useReaderState,
  useStoreState,
} from "../../src/index.ts";
import { fakeFile, fakePlatform } from "../fakes.ts";
import { cleanup, renderIn } from "./render.tsx";

afterEach(cleanup);

function Gate({ flag }: { flag: Capability }) {
  const gate = useCapability(flag);
  return <p data-flag={flag}>{gate.state === "disabled" ? `disabled: ${gate.reason}` : gate.state}</p>;
}

/** A store of two numbers whose methods need their `this`, like the viewer's. */
class TwoNumbers implements ExternalStore<{ page: number; scroll: number }> {
  #state = { page: 1, scroll: 0 };
  readonly #listeners = new Set<() => void>();
  getState() {
    return this.#state;
  }
  subscribe(listener: () => void) {
    this.#listeners.add(listener);
    return () => this.#listeners.delete(listener);
  }
  set(next: Partial<{ page: number; scroll: number }>) {
    this.#state = { ...this.#state, ...next };
    for (const listener of this.#listeners) listener();
  }
}

/** The store's page, counting its renders in `renders`. */
function Page({ store, renders }: { store: TwoNumbers; renders: { count: number } }) {
  renders.count++;
  return <p>{useStoreState(store, (state) => state.page)}</p>;
}

/** The reader's phase, or its page count once open. */
function Phase({ controller }: { controller: ReaderController }) {
  const state = useReaderState(controller);
  return <p>{state.phase === "open" ? `${state.pageCount} pages` : state.phase}</p>;
}

const settle = () => new Promise((resolve) => requestAnimationFrame(resolve));

describe("capability gates", () => {
  it("show a control, disable it with how to enable it, or hide it for good", () => {
    const platform = fakePlatform({
      flags: { canSaveAsDialog: true },
      reasons: { canWriteInPlace: "Use a browser that can save to your files." },
    });
    const host = renderIn(
      <PlatformProvider platform={platform}>
        <Gate flag="canSaveAsDialog" />
        <Gate flag="canWriteInPlace" />
        <Gate flag="hasNativeMenus" />
      </PlatformProvider>,
    );
    expect([...host.querySelectorAll("p")].map((p) => p.textContent)).toEqual([
      "available",
      "disabled: Use a browser that can save to your files.",
      "hidden",
    ]);
  });
});

describe("useStoreState", () => {
  it("re-renders when the selected part changes, not on every update", async () => {
    const store = new TwoNumbers();
    const renders = { count: 0 };
    const host = renderIn(<Page store={store} renders={renders} />);
    const before = renders.count;
    for (let scroll = 1; scroll <= 5; scroll++) store.set({ scroll });
    await settle();
    expect(renders.count).toBe(before);
    store.set({ page: 4 });
    await settle();
    expect(renders.count).toBe(before + 1);
    expect(host.textContent).toBe("4");
  });

  it("follows the reader controller through an open", async () => {
    const controller = new ReaderController(fakePlatform(), async () => ({ pageCount: 3 }));
    const host = renderIn(<Phase controller={controller} />);
    expect(host.textContent).toBe("empty");
    await controller.open(fakeFile());
    await settle();
    expect(host.textContent).toBe("3 pages");
  });
});

describe("the web platform in this browser", () => {
  const platform = createWebPlatform();
  const { capabilities } = platform;

  it("detects the pickers it finds, and says how to get the ones it doesn't", () => {
    const pickers = "showOpenFilePicker" in window;
    expect(capabilities.canSaveAsDialog).toBe("showSaveFilePicker" in window);
    expect(capabilities.canWriteInPlace).toBe(pickers);
    expect(platform.howToEnable("canWriteInPlace")).toEqual(pickers ? undefined : expect.any(String));
    expect(capabilities.canOpenUrl).toBe(true);
    expect(capabilities.hasNativePrint || capabilities.multiWindow || capabilities.ownsKeyboard).toBe(false);
  });

  it("keeps values in IndexedDB across platforms over the same window", async () => {
    await platform.storage.set("theme", { name: "dark" });
    expect(await createWebPlatform().storage.get("theme")).toEqual({ name: "dark" });
    await platform.storage.clear();
    expect(await platform.storage.get("theme")).toBeUndefined();
  });

  it("can reopen recent documents exactly where it says so", () => {
    // Reading a kept handle back is covered with fakes in Node: it needs the click that calls
    // reopen, and the sandbox-less Chromium of some CI containers crashes on it.
    expect(platform.recent.reopen !== undefined).toBe(capabilities.canReopenRecent);
  });
});
