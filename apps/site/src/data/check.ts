// The checks the site's data readers share (progress.ts, proof.ts): each JSON file the build reads
// is checked field by field, and a file its type doesn't describe fails the build, naming the field.

/** A check of one value at `path`; throws when the value isn't what it describes. */
export type Check = (value: unknown, path: string) => void;

function fail(path: string, expected: string): never {
  throw new Error(`${path} is not ${expected}`);
}

const DATE = /^\d{4}-\d{2}-\d{2}$/;

export const text: Check = (value, path) => typeof value === "string" || fail(path, "a string");
export const flag: Check = (value, path) => typeof value === "boolean" || fail(path, "true or false");
export const count: Check = (value, path) => (Number.isInteger(value) && Number(value) >= 0) || fail(path, "a count");
export const date: Check = (value, path) => (typeof value === "string" && DATE.test(value)) || fail(path, "a date");

export const oneOf =
  (...allowed: readonly unknown[]): Check =>
  (value, path) =>
    allowed.includes(value) || fail(path, allowed.join(" or "));

export const nullable =
  (check: Check): Check =>
  (value, path) =>
    value === null || check(value, path);

export const list =
  (check: Check): Check =>
  (value, path) => {
    if (!Array.isArray(value)) fail(path, "a list");
    for (const [i, item] of value.entries()) check(item, `${path}[${i}]`);
  };

function object(value: unknown, path: string): Record<string, unknown> {
  if (typeof value !== "object" || value === null || Array.isArray(value)) fail(path, "an object");
  return value as Record<string, unknown>;
}

/** An object with these fields (others are allowed). */
export const record =
  (fields: Readonly<Record<string, Check>>): Check =>
  (value, path) => {
    const map = object(value, path);
    for (const [key, check] of Object.entries(fields)) check(map[key], `${path}.${key}`);
  };

/** An object whose every value passes `check`, whatever its keys. */
export const entries =
  (check: Check): Check =>
  (value, path) => {
    for (const [key, item] of Object.entries(object(value, path))) check(item, `${path}.${key}`);
  };

/** `value` as `T` when `check` passes; throws `<file>: <path> is not …` otherwise. */
export function read<T>(file: string, check: Check, value: unknown): T {
  try {
    check(value, "the file");
  } catch (error) {
    throw new Error(`${file}: ${(error as Error).message}`);
  }
  return value as T;
}
