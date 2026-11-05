import { describe, expect, it } from "vitest";
import type { PlatformFile } from "../src/platform/types.ts";
import { type OpenedDocument, type Opener, ReaderController } from "../src/state/reader-controller.ts";
import { fakeFile, fakePlatform } from "./fakes.ts";

/** An error shaped like the SDK's LaminaPdfError: a message and an ADR 0026 code. */
const coded = (code: string) => Object.assign(new Error(code), { code });

/** An opener that answers each call from `answers` in turn, and records what it was given. */
function scripted(...answers: (number | Error)[]) {
  const calls: { file: PlatformFile; password: string | undefined }[] = [];
  const opener: Opener = async (file, password) => {
    calls.push({ file, password });
    const answer = answers.shift();
    if (answer instanceof Error) throw answer;
    return { pageCount: answer ?? 1 };
  };
  return { opener, calls };
}

/** A promise and the functions that settle it. */
function deferred<T>() {
  let resolve: (value: T) => void = () => undefined;
  let reject: (error: unknown) => void = () => undefined;
  const promise = new Promise<T>((res, rej) => {
    resolve = res;
    reject = rej;
  });
  return { promise, resolve, reject };
}

describe("ReaderController", () => {
  it("opens the file the picker returns and reports its page count", async () => {
    const file = fakeFile();
    const platform = fakePlatform({ picked: [file] });
    const { opener, calls } = scripted(12);
    const controller = new ReaderController(platform, opener);
    await controller.openFromPicker();
    expect(platform.calls.openFiles).toEqual([{ multiple: false }]);
    expect(calls).toEqual([{ file, password: undefined }]);
    expect(controller.getState()).toEqual({ phase: "open", file, pageCount: 12, mode: "select" });
  });

  it("does nothing when the picker is dismissed", async () => {
    const { opener, calls } = scripted();
    const controller = new ReaderController(fakePlatform(), opener);
    let notified = 0;
    controller.subscribe(() => notified++);
    await controller.openFromPicker();
    expect(calls).toEqual([]);
    expect(notified).toBe(0);
    expect(controller.getState().phase).toBe("empty");
  });

  it("is opening while the opener works, and tells subscribers of each change", async () => {
    const pending = deferred<OpenedDocument>();
    const controller = new ReaderController(fakePlatform(), () => pending.promise);
    const phases: string[] = [];
    controller.subscribe(() => phases.push(controller.getState().phase));
    const done = controller.open(fakeFile());
    expect(controller.getState().phase).toBe("opening");
    pending.resolve({ pageCount: 2 });
    await done;
    expect(phases).toEqual(["opening", "open"]);
  });

  it("asks for a password, passes the one given to the opener and says when it was wrong", async () => {
    const file = fakeFile();
    const { opener, calls } = scripted(coded("password-required"), coded("wrong-password"), 4);
    const controller = new ReaderController(fakePlatform(), opener);
    await controller.open(file);
    expect(controller.getState()).toEqual({ phase: "needs-password", file, wrong: false });
    await controller.submitPassword("guess");
    expect(controller.getState()).toEqual({ phase: "needs-password", file, wrong: true });
    await controller.submitPassword("right");
    expect(controller.getState()).toMatchObject({ phase: "open", pageCount: 4 });
    expect(calls.map((call) => call.password)).toEqual([undefined, "guess", "right"]);
    expect(JSON.stringify(controller.getState())).not.toMatch(/guess|right/);
  });

  it("ignores a password when none was asked for", async () => {
    const { opener, calls } = scripted();
    const controller = new ReaderController(fakePlatform(), opener);
    await controller.submitPassword("unasked");
    expect(calls).toEqual([]);
  });

  it("goes back to empty when the password prompt is cancelled", async () => {
    const controller = new ReaderController(fakePlatform(), scripted(coded("password-required")).opener);
    await controller.open(fakeFile());
    controller.cancel();
    expect(controller.getState().phase).toBe("empty");
  });

  it("shows the error's code and message when the document can't open", async () => {
    const file = fakeFile();
    const controller = new ReaderController(fakePlatform(), scripted(coded("invalid-header")).opener);
    await controller.open(file);
    expect(controller.getState()).toEqual({
      phase: "error",
      file,
      error: { code: "invalid-header", message: "invalid-header" },
    });
    controller.close();
    expect(controller.getState().phase).toBe("empty");
  });

  it("calls anything thrown without a code unknown", async () => {
    const opener: Opener = () => Promise.reject("worker gone");
    const controller = new ReaderController(fakePlatform(), opener);
    await controller.open(fakeFile());
    expect(controller.getState()).toMatchObject({ error: { code: "unknown", message: "worker gone" } });
  });

  it("keeps the later of two opens when the earlier finishes last", async () => {
    const first = deferred<{ pageCount: number }>();
    const second = deferred<{ pageCount: number }>();
    const answers = [first.promise, second.promise];
    const controller = new ReaderController(fakePlatform(), () => answers.shift() ?? first.promise);
    const later = fakeFile("later.pdf");
    const a = controller.open(fakeFile("earlier.pdf"));
    const b = controller.open(later);
    second.resolve({ pageCount: 9 });
    await b;
    first.resolve({ pageCount: 1 });
    await a;
    expect(controller.getState()).toEqual({ phase: "open", file: later, pageCount: 9, mode: "select" });
  });

  it("drops an open's outcome once it was cancelled", async () => {
    const pending = deferred<OpenedDocument>();
    const controller = new ReaderController(fakePlatform(), () => pending.promise);
    const done = controller.open(fakeFile());
    controller.cancel();
    pending.reject(coded("invalid-header"));
    await done;
    expect(controller.getState().phase).toBe("empty");
  });

  it("frees an open that nothing will show: superseded, cancelled, replaced or closed", async () => {
    const closed: string[] = [];
    const slow = deferred<OpenedDocument>();
    const opener: Opener = (file) =>
      file.name === "slow.pdf" ? slow.promise : Promise.resolve({ pageCount: 1, close: () => closed.push(file.name) });
    const controller = new ReaderController(fakePlatform(), opener);
    const superseded = controller.open(fakeFile("slow.pdf"));
    await controller.open(fakeFile("first.pdf"));
    slow.resolve({ pageCount: 1, close: () => closed.push("slow.pdf") });
    await superseded;
    expect(closed).toEqual(["slow.pdf"]);
    await controller.open(fakeFile("second.pdf"));
    expect(closed).toEqual(["slow.pdf", "first.pdf"]);
    controller.close();
    expect(closed).toEqual(["slow.pdf", "first.pdf", "second.pdf"]);
  });

  it("calls a DOMException's legacy numeric code unknown", async () => {
    const opener: Opener = () => Promise.reject(new DOMException("The file could not be read.", "NotReadableError"));
    const controller = new ReaderController(fakePlatform(), opener);
    await controller.open(fakeFile());
    expect(controller.getState()).toMatchObject({ error: { code: "unknown", message: "The file could not be read." } });
  });

  it("stops telling a listener once it unsubscribes", async () => {
    const controller = new ReaderController(fakePlatform(), scripted(1).opener);
    let notified = 0;
    const unsubscribe = controller.subscribe(() => notified++);
    unsubscribe();
    await controller.open(fakeFile());
    expect(notified).toBe(0);
  });
});
