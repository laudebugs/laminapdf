// The commands' argument parsers: a page by number or label, page ranges (`4-8`, `4,6,9-12`),
// a zoom (`150`, `150%`) and a choice among named values. Each returns the value with a short
// summary for the palette's row ("Go to page 12"), or an error saying what to type instead. The
// mini-language stays this small until a mode needs more (U1b's risks).

import { type MessageId, message } from "../messages.js";
import { MAX_ZOOM, MIN_ZOOM, pageFromInput } from "../reader/viewer-controls.js";
import type { ArgsSchema, CommandContext, ParseResult } from "./types.js";

const ok = <T>(value: T, summary: string): ParseResult<T> => ({ ok: true, value, summary });
const fail = <T>(error: string): ParseResult<T> => ({ ok: false, error });

/** The open document's page count, or 0 without one. */
const pageCount = (ctx: CommandContext) => ctx.viewer?.getState().pageCount ?? 0;

/**
 * The 0-based page `text` names, by label first and then by number (as the page input reads it),
 * or why it names none.
 */
export function parsePage(text: string, total: number, labels?: readonly string[]): ParseResult<number> {
  const typed = text.trim();
  if (typed === "") return fail(message("command.args.page.empty"));
  const page = pageFromInput(typed, total, labels);
  if (page === undefined) return fail(message("reader.page.invalid", { value: typed, total }));
  return ok(page, message("command.args.page.summary", { page: labels?.[page] ?? page + 1 }));
}

/**
 * The 0-based pages that `text` lists: numbers and ranges from 1 to `total`, separated by commas
 * (`4-8`, `4, 6, 9-12`), in the order written. Spaces don't matter; a range written backwards, a
 * page past the end and anything else are errors.
 */
export function parsePageRanges(text: string, total: number): ParseResult<readonly number[]> {
  const parts = text.replace(/\s+/g, "").split(",");
  if (parts.length === 0 || parts.some((part) => !/^\d+(?:-\d+)?$/.test(part))) {
    return fail(message("command.args.range.invalid"));
  }
  const pages: number[] = [];
  for (const part of parts) {
    const [from = 0, to = from] = part.split("-").map(Number);
    const outside = [from, to].find((page) => page < 1 || page > total);
    if (outside !== undefined) return fail(message("command.args.range.outOfRange", { value: outside, total }));
    if (to < from) return fail(message("command.args.range.backwards", { from, to }));
    for (let page = from; page <= to; page += 1) pages.push(page - 1);
  }
  return ok(pages, message("command.args.range.summary", { pages: parts.join(", ") }));
}

/** A zoom percentage from `text` (`150`, `150%`), from 10 to 800. */
export function parseZoomPercent(text: string): ParseResult<number> {
  const match = /^(\d+(?:\.\d+)?)\s*%?$/.exec(text.trim());
  const percent = match === null ? Number.NaN : Number(match[1]);
  if (!(percent >= MIN_ZOOM && percent <= MAX_ZOOM)) {
    return fail(message("command.args.zoom.invalid", { min: MIN_ZOOM, max: MAX_ZOOM }));
  }
  return ok(percent, message("command.args.zoom.summary", { percent }));
}

/** Each choice's words: its label, and its value with hyphens read as spaces. */
const choiceWords = (value: string, label: string) => [label.toLocaleLowerCase(), value.replace(/-/g, " ")];

/** One of `values`, typed as its label ("Two pages") or its value ("two-up"), without regard to case. */
export function parseChoice<Value extends string>(
  text: string,
  values: readonly Value[],
  label: (value: Value) => string,
): ParseResult<Value> {
  const typed = text.trim().toLocaleLowerCase().replace(/-/g, " ");
  const value = values.find((v) => choiceWords(v, label(v)).includes(typed));
  const choices = values.map(label).join(", ");
  return value === undefined ? fail(message("command.args.choice.invalid", { choices })) : ok(value, label(value));
}

/** A page by number or label, for `nav.go-to-page`. Built when registered, in the catalogue's words. */
export const pageArgs = (): ArgsSchema<number> => ({
  prompt: message("command.args.page.prompt"),
  prefixes: ["go to", "page"],
  parse: (text, ctx) => parsePage(text, pageCount(ctx), ctx.labels),
});

/** Page ranges, for `file.print`; without them every page prints. */
export const rangeArgs = (): ArgsSchema<readonly number[]> => ({
  prompt: message("command.args.range.prompt"),
  optional: true,
  parse: (text, ctx) => parsePageRanges(text, pageCount(ctx)),
});

/** A zoom percentage, for `view.set-zoom`. */
export const zoomArgs = (): ArgsSchema<number> => ({
  prompt: message("command.args.zoom.prompt", { min: MIN_ZOOM, max: MAX_ZOOM }),
  prefixes: ["zoom"],
  parse: (text) => parseZoomPercent(text),
});

/** A web address (`https://…`) for `file.open-url`. */
export function parseUrl(text: string): ParseResult<string> {
  try {
    const url = new URL(text.trim());
    if (url.protocol === "https:" || url.protocol === "http:") return ok(url.href, url.host);
  } catch {
    // Not a URL at all: the same message.
  }
  return fail(message("command.args.url.invalid"));
}

/** A web address, for `file.open-url`. */
export const urlArgs = (): ArgsSchema<string> => ({
  prompt: message("command.args.url.prompt"),
  parse: (text) => parseUrl(text),
});

/** A choice among `values`, labelled from the catalogue as `choice.<value>`. */
export function choiceArgs<Value extends string>(
  values: readonly Value[],
  options: { optional?: boolean } = {},
): ArgsSchema<Value> {
  const label = (value: Value) => message(`choice.${value}` as MessageId);
  return {
    prompt: message("command.args.choice.prompt", { choices: values.map(label).join(", ") }),
    ...(options.optional === true ? { optional: true } : {}),
    parse: (text) => parseChoice(text, values, label),
  };
}
