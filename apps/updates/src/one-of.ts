// One guard for the Worker's fixed lists of strings (channels, targets, arches).

/** Whether `value` is one of `list`'s strings, narrowing it to that list's type. */
export function isOneOf<T extends string>(list: readonly T[], value: unknown): value is T {
  return typeof value === "string" && (list as readonly string[]).includes(value);
}
