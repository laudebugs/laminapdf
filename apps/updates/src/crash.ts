// A desktop crash report, the body of `POST /crash`. The app builds it from an allow-list and
// scrubs its free-text fields (apps/desktop/src-tauri/src/crash/); this module checks it again, as
// schemas/crash-report.schema.json describes it, and refuses free text where a path, an e-mail
// address or a long number is left. test/crash.test.ts holds this validator to the schema.
import { isOneOf } from "./one-of.ts";

/**
 * What a crash report's free text (`reason`, each frame's `function`) may never hold: a Unix path
 * (`/a/b`), a Windows drive path (`C:\`, `C:/`), a UNC path (`\\server\`), a home path (`~/`, `~\`),
 * a `file:` URL, an e-mail address, or a standalone run of 6 or more digits (hex such as
 * `0xc0000005` passes). The desktop's scrubber, apps/desktop/src-tauri/src/crash/scrub.rs, removes
 * the same things; the Worker refuses a report where one is left.
 */
const PATH_LIKE: readonly RegExp[] = [
  /\/[^\s/]+\//,
  /\b[A-Za-z]:[\\/]/,
  /\\\\[^\s\\]+\\/,
  /~[\\/]/,
  /\bfile:/i,
  /[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}/,
  /\b\d{6,}\b/,
];

/** A webview frame's `file`: a bundle asset with the app's origin stripped, or {@link EXTERNAL_FILE}. */
export const SCRIPT_FILE = /^assets\/[A-Za-z0-9._-]+\.(js|mjs)$/;

/** A webview frame's `file` for anything outside the app's bundle. */
const EXTERNAL_FILE = "<external>";

const CHANNELS = ["stable", "beta", "nightly", "dev"] as const;
const OSES = ["macos", "windows", "linux"] as const;
const ARCHES = ["x86_64", "aarch64"] as const;
const PROCESSES = ["main", "engine", "webview"] as const;
const RENDERERS = ["pdfium"] as const;
const LAYERS = [
  "off",
  "no_new_privs",
  "seccomp",
  "landlock",
  "memory_limit",
  "seatbelt",
  "job",
  "low_integrity",
  "restricted_token",
] as const;
const UPTIMES = ["under_1m", "1m_to_10m", "10m_to_1h", "over_1h"] as const;
const KINDS = ["panic", "signal", "exit", "error", "rejection"] as const;

/** A Rust frame: the module's file name and the address within it. */
interface NativeFrame {
  readonly module: string;
  readonly offset: string;
  readonly function: string | null;
}

/** A webview frame: a bundle asset (or `<external>`) and the place in it. */
interface ScriptFrame {
  readonly file: string;
  readonly line: number;
  readonly column: number;
  readonly function: string | null;
}

/** A loaded module a native frame refers to. */
interface Module {
  readonly name: string;
  readonly version: string | null;
  readonly debug_id: string | null;
}

/** A frame of a native crash's stack, from its minidump (D7d): the module and the address in it. */
interface DumpFrame {
  readonly module: string;
  readonly offset: number;
  readonly function: string | null;
}

/** A module a native crash's frames name, with the debug ID its symbols are archived under. */
interface DumpModule {
  readonly name: string;
  readonly version: string | null;
  readonly debug_id: string;
}

/** A native crash's stack, read on the computer from its minidump (D7d). */
interface Native {
  readonly crash_reason: string;
  readonly frames: readonly DumpFrame[];
  readonly modules: readonly DumpModule[];
}

/** A crash report as schemas/crash-report.schema.json describes it. */
export interface CrashReport {
  readonly schema: 1;
  /** A random UUID v4 made for this report only; the R2 key's file name. */
  readonly report_id: string;
  readonly app_version: string;
  readonly channel: (typeof CHANNELS)[number];
  readonly os: (typeof OSES)[number];
  readonly os_version: string;
  readonly arch: (typeof ARCHES)[number];
  readonly process: (typeof PROCESSES)[number];
  readonly renderer: (typeof RENDERERS)[number];
  readonly sandbox: readonly (typeof LAYERS)[number][];
  readonly uptime: (typeof UPTIMES)[number];
  readonly kind: (typeof KINDS)[number];
  readonly reason: string;
  /** Where a panic happened, only in lamina's own sources. */
  readonly location: { readonly file: string; readonly line: number; readonly column: number } | null;
  readonly frames: readonly (NativeFrame | ScriptFrame)[];
  readonly modules: readonly Module[];
  /** A native crash's stack; absent when the crash left no minidump. */
  readonly native?: Native;
}

/** The result of checking a report: the report, or every problem found, each with its JSON path. */
export type CheckedReport =
  | { readonly ok: true; readonly report: CrashReport }
  | { readonly ok: false; readonly errors: string[] };

type Json = Record<string, unknown>;

const REPORT_ID = /^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/;
const VERSION = /^[0-9A-Za-z.+-]{1,32}$/;
const OS_VERSION = /^[0-9A-Za-z. ()_-]{0,40}$/;
const SOURCE_FILE = /^(apps|crates)\/[A-Za-z0-9_-]+(\/[A-Za-z0-9_-]+)*\.rs$/;
const MODULE_NAME = /^[A-Za-z0-9._+-]{1,64}$/;
const OFFSET = /^0x[0-9a-f]{1,16}$/;
const DEBUG_ID = /^[0-9A-Fa-f]{32,40}$/;
const TOP_KEYS = [
  "schema",
  "report_id",
  "app_version",
  "channel",
  "os",
  "os_version",
  "arch",
  "process",
  "renderer",
  "sandbox",
  "uptime",
  "kind",
  "reason",
  "location",
  "frames",
  "modules",
];
/** Keys a report may leave out. */
const OPTIONAL_KEYS = ["native"];
const MAX_ITEMS = { sandbox: 8, frames: 64, modules: 64 } as const;
/** The largest native offset: JSON numbers stay exact up to 2^53 − 1. */
const MAX_OFFSET = Number.MAX_SAFE_INTEGER;

function isObject(value: unknown): value is Json {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

/** Whether `value` is a string matching `pattern`, at most `max` code points long. */
function isText(value: unknown, pattern: RegExp, max = Number.POSITIVE_INFINITY): value is string {
  return typeof value === "string" && [...value].length <= max && pattern.test(value);
}

/** The schema's `count`: an integer from 0 to 2^32 − 1. */
function isCount(value: unknown): boolean {
  return typeof value === "number" && Number.isInteger(value) && value >= 0 && value <= 4_294_967_295;
}

/** Whether `text` holds anything {@link PATH_LIKE} refuses. */
export function isPathLike(text: string): boolean {
  return PATH_LIKE.some((pattern) => pattern.test(text));
}

// Errors name the JSON path and the rule, never a key or a value from the body: they become the 400's text.
function exactKeys(
  value: Json,
  keys: readonly string[],
  at: string,
  errors: string[],
  optional: readonly string[] = [],
): void {
  if (Object.keys(value).some((key) => !keys.includes(key) && !optional.includes(key))) {
    errors.push(`${at}: has an unknown key`);
  }
  for (const key of keys) if (!(key in value)) errors.push(`${at}/${key}: required`);
}

/** A free-text field: a string of at most `max` code points with nothing path-like in it. */
function checkFreeText(value: unknown, max: number, at: string, errors: string[]): void {
  if (typeof value !== "string" || [...value].length > max) {
    errors.push(`${at}: must be a string of at most ${max} characters`);
  } else if (isPathLike(value)) {
    errors.push(`${at}: holds a path, an e-mail address or a long number`);
  }
}

function checkFunction(value: unknown, at: string, errors: string[]): void {
  if (value !== null) checkFreeText(value, 256, `${at}/function`, errors);
}

function checkFrame(frame: unknown, at: string, errors: string[]): void {
  if (!isObject(frame)) {
    errors.push(`${at}: must be an object`);
    return;
  }
  // A native frame has a module; a script frame has a file. No frame may have both.
  if ("module" in frame) {
    exactKeys(frame, ["module", "offset", "function"], at, errors);
    if (frame.module !== "<unknown>" && !isText(frame.module, MODULE_NAME)) {
      errors.push(`${at}/module: must be a file name or <unknown>`);
    }
    if (!isText(frame.offset, OFFSET)) errors.push(`${at}/offset: must be 0x and lower-case hex`);
  } else {
    exactKeys(frame, ["file", "line", "column", "function"], at, errors);
    if (frame.file !== EXTERNAL_FILE && !isText(frame.file, SCRIPT_FILE)) {
      errors.push(`${at}/file: must be assets/{name}.js or ${EXTERNAL_FILE}`);
    }
    if (!isCount(frame.line)) errors.push(`${at}/line: must be a count`);
    if (!isCount(frame.column)) errors.push(`${at}/column: must be a count`);
  }
  checkFunction(frame.function, at, errors);
}

function checkModule(module: unknown, at: string, errors: string[]): void {
  if (!isObject(module)) {
    errors.push(`${at}: must be an object`);
    return;
  }
  exactKeys(module, ["name", "version", "debug_id"], at, errors);
  if (!isText(module.name, MODULE_NAME)) errors.push(`${at}/name: must be a file name`);
  if (module.version !== null && !isText(module.version, VERSION)) errors.push(`${at}/version: must be a version`);
  if (module.debug_id !== null && !isText(module.debug_id, DEBUG_ID)) {
    errors.push(`${at}/debug_id: must be 32 to 40 hex digits`);
  }
}

/** A module name in a native stack: a file name or `<unknown>`. */
function isModuleName(value: unknown): boolean {
  return value === "<unknown>" || isText(value, MODULE_NAME);
}

/** Checks one list of a native stack: an array of at most 64, each item by `check`. */
function checkNativeList(value: unknown, at: string, check: (item: Json, at: string) => void, errors: string[]): void {
  if (!Array.isArray(value) || value.length > 64) {
    errors.push(`${at}: must be a list of at most 64`);
    return;
  }
  value.forEach((item: unknown, i: number) => {
    if (isObject(item)) check(item, `${at}/${i}`);
    else errors.push(`${at}/${i}: must be an object`);
  });
}

/** A native crash's stack (D7d), when the report has one. */
function checkNative(value: unknown, errors: string[]): void {
  if (value === undefined) return;
  if (!isObject(value)) {
    errors.push("/native: must be an object");
    return;
  }
  exactKeys(value, ["crash_reason", "frames", "modules"], "/native", errors);
  checkFreeText(value.crash_reason, 128, "/native/crash_reason", errors);
  checkNativeList(
    value.frames,
    "/native/frames",
    (frame, at) => {
      exactKeys(frame, ["module", "offset", "function"], at, errors);
      if (!isModuleName(frame.module)) errors.push(`${at}/module: must be a file name or <unknown>`);
      const offset = frame.offset;
      if (typeof offset !== "number" || !Number.isInteger(offset) || offset < 0 || offset > MAX_OFFSET) {
        errors.push(`${at}/offset: must be a whole number from 0 to 2^53 - 1`);
      }
      // A native frame carries no function name (ADR 0014 §10): the computer has no symbols.
      if (frame.function !== null) errors.push(`${at}/function: must be null`);
    },
    errors,
  );
  checkNativeList(
    value.modules,
    "/native/modules",
    (module, at) => {
      exactKeys(module, ["name", "version", "debug_id"], at, errors);
      if (!isModuleName(module.name)) errors.push(`${at}/name: must be a file name or <unknown>`);
      if (module.version !== null && !isText(module.version, VERSION)) errors.push(`${at}/version: must be a version`);
      if (!isText(module.debug_id, DEBUG_ID)) errors.push(`${at}/debug_id: must be 32 to 40 hex digits`);
    },
    errors,
  );
}

function checkLocation(value: unknown, errors: string[]): void {
  if (value === null) return;
  if (!isObject(value)) {
    errors.push("/location: must be null or an object");
    return;
  }
  exactKeys(value, ["file", "line", "column"], "/location", errors);
  if (!isText(value.file, SOURCE_FILE, 160)) errors.push("/location/file: must be a .rs file under apps/ or crates/");
  if (!isCount(value.line)) errors.push("/location/line: must be a count");
  if (!isCount(value.column)) errors.push("/location/column: must be a count");
}

/** Checks a list field's length, then each item with `check`. */
function checkList(
  value: unknown,
  name: keyof typeof MAX_ITEMS,
  check: (item: unknown, at: string, errors: string[]) => void,
  errors: string[],
): void {
  if (!Array.isArray(value) || value.length > MAX_ITEMS[name]) {
    errors.push(`/${name}: must be a list of at most ${MAX_ITEMS[name]}`);
    return;
  }
  value.forEach((item: unknown, i: number) => {
    check(item, `/${name}/${i}`, errors);
  });
}

function checkSandbox(value: unknown, errors: string[]): void {
  checkList(
    value,
    "sandbox",
    (layer, at) => {
      if (!isOneOf(LAYERS, layer)) errors.push(`${at}: not a sandbox layer`);
    },
    errors,
  );
  if (Array.isArray(value) && new Set(value).size !== value.length) errors.push("/sandbox: lists a layer twice");
}

function checkEnvironment(value: Json, errors: string[]): void {
  if (value.schema !== 1) errors.push("/schema: must be 1");
  if (!isText(value.report_id, REPORT_ID)) errors.push("/report_id: must be a lower-case UUID v4");
  if (!isText(value.app_version, VERSION)) errors.push("/app_version: must be a version");
  if (!isOneOf(CHANNELS, value.channel)) errors.push(`/channel: must be one of ${CHANNELS.join(", ")}`);
  if (!isOneOf(OSES, value.os)) errors.push(`/os: must be one of ${OSES.join(", ")}`);
  if (!isText(value.os_version, OS_VERSION)) errors.push("/os_version: must be a short version");
  if (!isOneOf(ARCHES, value.arch)) errors.push(`/arch: must be one of ${ARCHES.join(", ")}`);
  if (!isOneOf(PROCESSES, value.process)) errors.push(`/process: must be one of ${PROCESSES.join(", ")}`);
  if (!isOneOf(RENDERERS, value.renderer)) errors.push(`/renderer: must be one of ${RENDERERS.join(", ")}`);
  if (!isOneOf(UPTIMES, value.uptime)) errors.push(`/uptime: must be one of ${UPTIMES.join(", ")}`);
  if (!isOneOf(KINDS, value.kind)) errors.push(`/kind: must be one of ${KINDS.join(", ")}`);
}

/** Checks a parsed crash report. */
export function checkCrashReport(value: unknown): CheckedReport {
  if (!isObject(value)) return { ok: false, errors: ["/: must be an object"] };
  const errors: string[] = [];
  exactKeys(value, TOP_KEYS, "", errors, OPTIONAL_KEYS);
  checkEnvironment(value, errors);
  checkSandbox(value.sandbox, errors);
  checkFreeText(value.reason, 1024, "/reason", errors);
  checkLocation(value.location, errors);
  checkList(value.frames, "frames", checkFrame, errors);
  checkList(value.modules, "modules", checkModule, errors);
  checkNative(value.native, errors);
  // Every field the type names was checked above, so the value is a CrashReport; test/crash.test.ts
  // holds this validator to schemas/crash-report.schema.json, case by case.
  return errors.length === 0 ? { ok: true, report: value as unknown as CrashReport } : { ok: false, errors };
}
