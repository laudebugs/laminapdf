// What the Paragraph inspector shows of the lines of the page's own text that Edit text has picked
// (Paper A04), and what each of its fields does: one change, one op, one undo step. Apart from the
// DOM, so the notice and the facts it reads off a run can be tested in Node.

import type { ContentTextRun } from "@laminapdf/sdk";
import type { ParagraphInspectorProps, ParagraphNotice } from "@laminapdf/ui/editing";
import { laterList, laterMessage } from "@laminapdf/ui/editing";
import type { ContentController } from "./controller.ts";
import { fontOptions, rgbOfColour } from "./inspector-props.ts";
import type { TextPick, TextTyping } from "./text-edit.ts";
import {
  fontFacts,
  isEditable,
  REASONS,
  readableFont,
  substitutedCharacters,
  substituteFont,
  type TextPlan,
} from "./text-runs.ts";

/** What the inspector needs besides the pick. */
export interface TextInspectorOptions {
  readonly typing: TextTyping | undefined;
  readonly locked: string | undefined;
}

/** The longest excerpt of a line's text the inspector shows. */
const EXCERPT = 80;

/** The words that say why text can't be edited in place: the engine's reason, in the catalogue's sentence. */
export function refusalBody(reason: string): string {
  const known = (REASONS as readonly string[]).includes(reason);
  return known
    ? laterMessage(`content.text.refused.${reason as "type3-font"}`)
    : laterMessage("content.text.refused.other", { reason });
}

/** The notice for what the engine said of the text being typed: a substitution or a character no font has. */
export function noticeOf(plan: TextPlan | undefined, shown: boolean): ParagraphNotice | undefined {
  if (plan?.kind === "substituted") {
    const characters = substitutedCharacters(plan.substitutions);
    const quoted = laterList(characters.map((c) => `“${c}”`));
    return {
      tone: "warning",
      title: laterMessage(
        characters.length === 1 ? "content.text.substitution.title" : "content.text.substitutions.title",
        { count: characters.length },
      ),
      body: `${laterMessage("content.text.substitution.body", {
        characters: quoted,
        them: laterMessage(
          characters.length === 1 ? "content.text.substitution.them.one" : "content.text.substitution.them.many",
        ),
        font: substituteFont(plan.substitutions) ?? "",
      })}${shown ? "" : ` ${laterMessage("content.text.substitution.confirm")}`}`,
    };
  }
  if (plan?.kind === "missing") {
    const quoted = laterList(plan.characters.map((c) => `“${c}”`));
    return {
      tone: "warning",
      title: laterMessage("content.text.missing.title", { characters: quoted }),
      body: laterMessage("content.text.missing.body"),
    };
  }
  return undefined;
}

/** The excerpt of what the picked lines say. */
function excerptOf(runs: readonly ContentTextRun[]): string {
  const text = runs.map((r) => r.text).join(" ");
  return text.length > EXCERPT ? `${text.slice(0, EXCERPT)}…` : text;
}

/** Why the picked text can't be edited in place: no line of it can be, or the engine refused the text typed. */
function refusalReason(pick: TextPick, plan: TextPlan | undefined): string | undefined {
  if (plan?.kind === "refused") return plan.reason;
  if (pick.runs.some(isEditable)) return undefined;
  const why = pick.runs[0]?.readOnly;
  return why === undefined || why === null ? undefined : String(why);
}

/** The fields that style the picked lines: font, size, Bold, Italic, Underline, alignment and colour. */
function styleProps(
  controller: ContentController,
  first: ContentTextRun | undefined,
): Pick<ParagraphInspectorProps, "font" | "size" | "bold" | "italic" | "onUnderline" | "onAlign" | "onColour"> {
  const tool = controller.text;
  const facts = fontFacts(first?.fontName ?? "");
  return {
    font: {
      options: [
        { id: "file", label: laterMessage("content.text.fontInFile") },
        ...fontOptions(controller, "file").filter((o) => o.id !== "file"),
      ],
      onChange: (id) => void tool.setFont(id),
      onAddFont: () => void controller.addFont(),
    },
    size: { value: first?.fontSize ?? 12, onChange: (size) => void tool.style({ size }) },
    bold: { value: facts.bold, onChange: (bold) => void tool.style({ bold }) },
    italic: { value: facts.italic, onChange: (italic) => void tool.style({ italic }) },
    onUnderline: () => void tool.style({ underline: true }),
    onAlign: (align) => void tool.style({ align }),
    onColour: (id) => void tool.style({ color: rgbOfColour(id) }),
  };
}

/** The inspector's props for the picked lines. */
export function textInspectorProps(
  controller: ContentController,
  pick: TextPick,
  options: TextInspectorOptions,
): ParagraphInspectorProps {
  const tool = controller.text;
  const [first] = pick.runs;
  const typing = options.typing;
  const plan = typing?.plan;
  const reason = refusalReason(pick, plan);
  const one = pick.runs.length === 1 ? first : undefined;
  const notice = noticeOf(plan, typing?.shown === typing?.text);
  return {
    title: laterMessage("content.text.inspector"),
    context: laterMessage("content.text.context", { page: pick.page + 1 }),
    excerpt: excerptOf(pick.runs),
    fileFont: laterMessage("content.text.fileFont.value", {
      font: readableFont(first?.fontName ?? ""),
      size: Math.round((first?.fontSize ?? 0) * 10) / 10,
    }),
    ...(notice === undefined ? {} : { notice }),
    ...(reason === undefined
      ? {}
      : {
          refusal: {
            title: laterMessage("content.text.refused.title"),
            body: refusalBody(reason),
            detail: laterMessage("content.text.refused.detail", { reason }),
            onSelect: () => tool.selectBlock(),
          },
        }),
    ...styleProps(controller, first),
    editing: typing !== undefined,
    ...(one === undefined || !isEditable(one) ? {} : { onEdit: () => controller.editText() }),
    onDone: () => controller.finishText(),
    onRevert: () => controller.revertText(),
    deleteLabel:
      pick.runs.length > 1
        ? laterMessage("content.text.deleteTexts", { count: pick.runs.length })
        : laterMessage("content.text.deleteText"),
    onDelete: () => controller.deleteSelection(),
    ...(options.locked === undefined ? {} : { locked: options.locked }),
    notes: [
      ...(typing?.escaped === true ? [laterMessage("content.text.escapeAgain")] : []),
      laterMessage("content.text.wholeLine"),
      laterMessage("content.text.noReadBack"),
      laterMessage("content.text.history"),
    ],
  };
}
