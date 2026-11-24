// Fill & Sign mode (Paper P01), registered while a document is open: its toolbar with the
// banner a signature calls for, the conflict dialog, the Fields rail tab, the status bar's form
// note and the handlers of its commands. The controller does the work; this is what shows it.

import {
  Button,
  type CommandRegistry,
  type ModeRegistry,
  Notice,
  type RailTab,
  type ShellLayout,
  StatusBarItem,
} from "@laminapdf/ui";
import {
  FieldConflictDialog,
  type FieldRow,
  FieldsPanel,
  FormToolbar,
  laterCountMessage,
  laterMessage,
  registerFormCommands,
  restrictionsOf,
  SignatureDetails,
} from "@laminapdf/ui/editing";
import type { Viewer } from "@laminapdf/viewer";
import { type ReactNode, useEffect, useMemo, useState, useSyncExternalStore } from "react";
import type { SaveFlow } from "../save-flow.ts";
import type { Engine } from "../session.ts";
import { FormController, type FormState, reportError } from "./controller.ts";
import { fieldLabel, isFilled, requiredCount } from "./rules.ts";

/** Fill & Sign's id among the modes. */
export const FILL_SIGN = "fill-sign";

/** The Fields tab's id in the rail. */
const FIELDS_TAB = "fields";

/** Each viewer's controller, once Fill & Sign has registered for it. */
const controllers = new WeakMap<Viewer, FormController>();
/** Who hears a controller come or go: the status bar's note renders before the mode registers. */
const watchers = new Set<() => void>();

function setController(viewer: Viewer, controller: FormController | undefined): void {
  if (controller === undefined) controllers.delete(viewer);
  else controllers.set(viewer, controller);
  for (const watcher of [...watchers]) watcher();
}

/** `viewer`'s controller, kept up to date as Fill & Sign registers and goes. */
function useController(viewer: Viewer): FormController | undefined {
  return useSyncExternalStore(
    (listener) => {
      watchers.add(listener);
      return () => watchers.delete(listener);
    },
    () => controllers.get(viewer),
  );
}

/** The controller's state, kept up to date. */
function useFormState(controller: FormController): FormState {
  return useSyncExternalStore(
    (listener) => controller.subscribe(listener),
    () => controller.getState(),
  );
}

/** Props of `FormMode`. */
export interface FormModeProps {
  readonly engine: Engine;
  readonly viewer: Viewer;
  readonly modes: ModeRegistry;
  readonly registry: CommandRegistry;
  readonly flow: SaveFlow;
}

/** Fill & Sign, registered while a document is open. */
export function FormMode({ engine, viewer, modes, registry, flow }: FormModeProps): ReactNode {
  const [controller, setOwn] = useState<FormController | undefined>(undefined);
  // The banner's copy goes through saving as it is now; a new flow keeps the mode and its state.
  const saving = useMemo(() => ({ flow }), []);
  useEffect(() => {
    saving.flow = flow;
  }, [saving, flow]);
  useEffect(() => {
    const controller = new FormController(engine, viewer);
    setController(viewer, controller);
    const unregister = modes.register({
      id: FILL_SIGN,
      title: laterMessage("command.mode.fill-sign"),
      label: laterMessage("mode.fill-sign"),
      icon: "text-field",
      toolbar: () => <FormBar controller={controller} saving={saving} />,
      enter: () => controller.enter(),
      leave: () => controller.leave(),
    });
    const commands = registerFormCommands(registry);
    // The status bar's note needs the form before the mode is ever entered.
    void controller.peek().catch(reportError);
    setOwn(controller);
    return () => {
      setOwn(undefined);
      commands();
      controller.leave();
      unregister();
      setController(viewer, undefined);
    };
  }, [engine, viewer, modes, registry, saving]);
  return controller === undefined ? null : <Question controller={controller} />;
}

/** The conflict dialog while a change waits for an answer. */
function Question({ controller }: { controller: FormController }): ReactNode {
  const { question } = useFormState(controller);
  const asked = question === undefined ? undefined : { field: question.label, kind: question.kind };
  return <FieldConflictDialog question={asked} onAnswer={(yes) => controller.answer(yes)} />;
}

/** The toolbar, then the banner on a file whose signature forbids filling. */
function FormBar({ controller, saving }: { controller: FormController; saving: { flow: SaveFlow } }): ReactNode {
  const state = useFormState(controller);
  const copy = saving.flow.copyTitle().replace(/…$/, "");
  return (
    <div className="web-form-bar">
      <FormToolbar
        highlight={state.highlight}
        tool={state.tool}
        required={state.fields === undefined ? undefined : requiredCount(state.fields)}
      />
      {state.banner === undefined ? null : (
        <section className="web-form-banner" aria-label={laterMessage("form.banner")}>
          <Notice
            tone="danger"
            title={laterMessage(state.banner.kind === "certified" ? "form.certified.title" : "form.usageRights.title")}
          >
            {laterMessage(state.banner.kind === "certified" ? "form.certified.body" : "form.usageRights.body", {
              copy,
            })}
          </Notice>
          <Button size={30} onPress={() => void saving.flow.copy()}>
            {saving.flow.copyTitle()}
          </Button>
          <SignatureDetails restrictions={restrictionsOf(state.banner.violations)} />
        </section>
      )}
    </div>
  );
}

/**
 * The handlers of Fill & Sign's commands for the document `viewer` shows. Each switches to Fill &
 * Sign first, so the palette's Next field works in any mode, and finds the mode's controller when
 * it runs: the controller is made once the document is open.
 */
export function formActions(viewer: Viewer, modes: ModeRegistry, layout: ShellLayout) {
  const on = (run: (controller: FormController) => void) => () => {
    if (modes.getState().current !== FILL_SIGN) modes.switchTo(FILL_SIGN);
    const controller = controllers.get(viewer);
    if (controller !== undefined) run(controller);
  };
  return {
    "form.focus-next-field": on((c) => c.step(1)),
    "form.focus-previous-field": on((c) => c.step(-1)),
    "form.focus-next-required-field": on((c) => c.nextRequired()),
    "form.toggle-field-highlight": on((c) => c.toggleHighlight()),
    "form.add-text": on((c) => c.toggleTool("text")),
    "form.add-check": on((c) => c.toggleTool("check")),
    "form.add-date": on((c) => c.toggleTool("date")),
    "view.show-fields-tab": on(() => {
      layout.setRailTab(FIELDS_TAB);
      layout.setRailOpen(true);
    }),
  } as const;
}

/** The Fields tab (P01) while Fill & Sign is the mode; undefined otherwise. */
export function fieldsTab(viewer: Viewer, pageName: (index: number) => string): RailTab | undefined {
  const controller = controllers.get(viewer);
  if (controller === undefined) return undefined;
  return {
    id: FIELDS_TAB,
    label: laterMessage("form.fields"),
    panel: <Fields controller={controller} viewer={viewer} pageName={pageName} />,
  };
}

/** The Fields tab's panel: the form's stops in tab order. */
function Fields({
  controller,
  viewer,
  pageName,
}: {
  controller: FormController;
  viewer: Viewer;
  pageName: (index: number) => string;
}): ReactNode {
  const state = useFormState(controller);
  const page = useSyncExternalStore(
    (listener) => viewer.subscribe(listener),
    () => viewer.getState().currentPage,
  );
  const byId = new Map((state.fields ?? []).map((f) => [f.id, f]));
  const rows: FieldRow[] = state.order.flatMap((stop) => {
    const field = byId.get(stop.field);
    if (field === undefined) return [];
    const locked = state.locks.get(field.id);
    return [
      {
        widget: stop.widget,
        field: field.id,
        page: stop.page,
        label: fieldLabel(field),
        filled: isFilled(field),
        required: field.required,
        ...(locked === undefined ? {} : { locked }),
      },
    ];
  });
  return (
    <FieldsPanel
      rows={rows}
      current={state.focused}
      page={page}
      pageName={pageName}
      onFocus={(row) => controller.focusStop({ widget: row.widget, field: row.field, page: row.page })}
    />
  );
}

/**
 * The status bar's form note: the field count, which switches to Fill & Sign, and how a signed
 * file saves or that some answers show only while filling (`/NeedAppearances`).
 */
export function FormStatus({ viewer, modes }: { viewer: Viewer; modes: ModeRegistry }): ReactNode {
  const controller = useController(viewer);
  if (controller === undefined) return null;
  return <FormStatusItem controller={controller} modes={modes} />;
}

/** The status-bar item itself, once the controller is there. */
function FormStatusItem({ controller, modes }: { controller: FormController; modes: ModeRegistry }): ReactNode {
  const state = useFormState(controller);
  const count = state.order.length;
  if (state.fields === undefined || count === 0) return null;
  const notes = [
    laterCountMessage(count, { one: "form.status.one", many: "form.status.many" }),
    laterMessage("form.status.order"),
    ...(state.needAppearances ? [laterMessage("form.needAppearances")] : []),
    ...(state.signed ? [laterMessage("form.signed")] : []),
  ];
  return <StatusBarItem onPress={() => modes.switchTo(FILL_SIGN)}>{notes.join(" · ")}</StatusBarItem>;
}
