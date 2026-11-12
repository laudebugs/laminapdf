// Initial view (O13): how the file opens for anyone, in Lamina PDF and in other PDF readers. The
// page and zoom, the page layout, what shows beside the page and the reader window's requests,
// read from the file and applied as one edit (`setInitialView`). A file whose open action runs an
// action instead (a script, a named action) says so and keeps it; the page and zoom are locked.
// Nothing set shows each control at the reader's default, and Apply waits for a change.

import { type ReactNode, useEffect, useState } from "react";
import { Button } from "../components/button.js";
import { Checkbox, Radio, RadioGroup } from "../components/choice.js";
import { Select, TextInput } from "../components/field.js";
import { Notice } from "../components/notice.js";
import { laterMessage } from "../later-messages.js";
import { Dialog } from "../overlays/dialog.js";
import {
  changeOf,
  formFrom,
  type InitialLayout,
  type InitialMode,
  type InitialViewChange,
  type InitialViewData,
  type InitialViewForm,
  type InitialZoom,
} from "./initial-view-form.js";

/** Props of `InitialViewDialog`. */
export interface InitialViewDialogProps {
  readonly isOpen: boolean;
  /** The file's initial view; `undefined` while it is read. */
  readonly view: InitialViewData | undefined;
  readonly pageCount: number;
  /** Where the reader is now, for Use the current view: the page (0-based) and its zoom. */
  readonly current: () => { readonly page: number; readonly zoom: InitialZoom };
  /** Says under Fit the window that it has no effect here: a web app has no window of its own to resize. */
  readonly fitWindowHasNoEffect?: boolean | undefined;
  /** Apply: one edit; the dialog closes once it resolves and says so if it rejects. */
  readonly onApply: (change: InitialViewChange) => Promise<void>;
  /** Cancel, Close, Escape or the scrim. */
  readonly onClose: () => void;
}

/** The zooms offered; a file's own percentage joins them. */
const PERCENTS = [50, 75, 100, 125, 150, 200] as const;

const LAYOUTS: readonly InitialLayout[] = [
  "unset",
  "singlePage",
  "oneColumn",
  "twoPageLeft",
  "twoColumnLeft",
  "twoPageRight",
  "twoColumnRight",
];

const MODES: readonly InitialMode[] = [
  "unset",
  "outlines",
  "none",
  "thumbnails",
  "attachments",
  "optionalContent",
  "fullScreen",
];

/** A zoom as the select's value. */
const zoomValue = (zoom: InitialZoom): string => String(zoom);

/** The select's value as a zoom. */
function zoomFrom(value: string): InitialZoom {
  return value === "default" || value === "fitPage" || value === "fitWidth" ? value : Number(value);
}

/** The zoom select's options, with `extra` when it is a percentage not offered. */
function zoomOptions(extra: InitialZoom): { value: string; label: string }[] {
  const percents: number[] = [...PERCENTS];
  if (typeof extra === "number" && !percents.includes(extra)) percents.push(extra);
  percents.sort((a, b) => a - b);
  return [
    { value: "fitWidth", label: laterMessage("initialView.zoom.fitWidth") },
    { value: "default", label: laterMessage("initialView.zoom.default") },
    { value: "fitPage", label: laterMessage("initialView.zoom.fitPage") },
    ...percents.map((p) => ({ value: String(p), label: laterMessage("initialView.zoom.percent", { percent: p }) })),
  ];
}

/** The page field's text as a 0-based page, `undefined` when empty, or an error. */
function pageFrom(text: string, pageCount: number): { page: number | undefined } | { error: string } {
  const value = text.trim();
  if (value === "") return { page: undefined };
  const number = /^\d+$/.test(value) ? Number(value) : Number.NaN;
  if (Number.isInteger(number) && number >= 1 && number <= pageCount) return { page: number - 1 };
  return { error: laterMessage("initialView.page.invalid", { pages: pageCount }) };
}

/** The dialog's state: the form as read, as edited, the page field's text, and Apply's progress. */
function useInitialViewForm(props: InitialViewDialogProps) {
  const { view, isOpen } = props;
  const [held, setHeld] = useState<{ from: InitialViewData; form: InitialViewForm; pageText: string }>();
  const [applying, setApplying] = useState(false);
  const [failed, setFailed] = useState(false);
  useEffect(() => {
    if (!isOpen) {
      setHeld(undefined);
      setFailed(false);
    }
  }, [isOpen]);
  const read = view === undefined ? undefined : formFrom(view);
  const edited = view !== undefined && held?.from === view ? held : undefined;
  const form = edited?.form ?? read;
  const pageText = edited?.pageText ?? (read?.page === undefined ? "" : String(read.page + 1));
  const parsed = pageFrom(pageText, props.pageCount);
  const keepOpen = view?.open.kind === "action";
  const change =
    read === undefined || form === undefined || "error" in parsed ? undefined : changeOf(read, form, keepOpen);
  const set = (next: Partial<InitialViewForm>, text = pageText) => {
    if (view === undefined || form === undefined) return;
    const merged = { ...form, ...next };
    const page = pageFrom(text, props.pageCount);
    setHeld({ from: view, form: "error" in page ? merged : { ...merged, page: page.page }, pageText: text });
  };
  const apply = () => {
    if (change === undefined) return;
    setApplying(true);
    setFailed(false);
    props.onApply(change).then(
      () => {
        setApplying(false);
        props.onClose();
      },
      () => {
        setApplying(false);
        setFailed(true);
      },
    );
  };
  return {
    form,
    pageText,
    error: "error" in parsed ? parsed.error : undefined,
    keepOpen,
    change,
    set,
    apply,
    applying,
    failed,
  };
}

type FormState = ReturnType<typeof useInitialViewForm>;

/** The page and zoom, or the open action that stays. */
function OpenFields({ props, state }: { props: InitialViewDialogProps; state: FormState }): ReactNode {
  const { form } = state;
  if (form === undefined) return null;
  const action = props.view?.open.kind === "action" ? props.view.open.action : undefined;
  return (
    <>
      {action === undefined ? null : <Notice title={laterMessage("initialView.action", { action })} />}
      <div className="paper-initial-view-open">
        <TextInput
          label={laterMessage("initialView.page")}
          placeholder="1"
          description={laterMessage("initialView.page.help", { pages: props.pageCount })}
          value={state.pageText}
          isDisabled={state.keepOpen}
          onChange={(text) => state.set({}, text)}
          {...(state.error === undefined ? {} : { errorMessage: state.error })}
        />
        <Select
          label={laterMessage("initialView.zoom")}
          value={zoomValue(form.zoom)}
          isDisabled={state.keepOpen}
          onChange={(value) => state.set({ zoom: zoomFrom(value) })}
          options={zoomOptions(form.zoom)}
        />
      </div>
    </>
  );
}

/** Page layout, what shows beside the page, and the reader window's requests. */
function ViewFields({ props, state }: { props: InitialViewDialogProps; state: FormState }): ReactNode {
  const { form } = state;
  if (form === undefined) return null;
  const setWindow = (next: Partial<InitialViewForm["window"]>) => state.set({ window: { ...form.window, ...next } });
  return (
    <>
      <RadioGroup
        label={laterMessage("initialView.layout")}
        value={form.layout}
        onChange={(value) => state.set({ layout: value as InitialLayout })}
      >
        {LAYOUTS.map((layout) => (
          <Radio key={layout} value={layout}>
            {laterMessage(`initialView.layout.${layout}`)}
          </Radio>
        ))}
      </RadioGroup>
      <Select
        label={laterMessage("initialView.mode")}
        value={form.mode}
        onChange={(value) => state.set({ mode: value })}
        options={MODES.map((mode) => ({ value: mode, label: laterMessage(`initialView.mode.${mode}`) }))}
      />
      <fieldset className="paper-initial-view-window">
        <legend>{laterMessage("initialView.window")}</legend>
        <Checkbox isSelected={form.window.displayDocTitle} onChange={(on) => setWindow({ displayDocTitle: on })}>
          {laterMessage("initialView.window.title")}
        </Checkbox>
        <Checkbox isSelected={form.window.fitWindow} onChange={(on) => setWindow({ fitWindow: on })}>
          {laterMessage("initialView.window.fit")}
        </Checkbox>
        {props.fitWindowHasNoEffect === true ? (
          <p className="paper-field-help">{laterMessage("initialView.window.fit.web")}</p>
        ) : null}
        <Checkbox isSelected={form.window.hideControls} onChange={(on) => setWindow({ hideControls: on })}>
          {laterMessage("initialView.window.hide")}
        </Checkbox>
        <p className="paper-field-help">{laterMessage("initialView.window.requests")}</p>
      </fieldset>
    </>
  );
}

/** The name of the edit-session step Apply makes, for the History tab. */
export const initialViewStep = (): string => laterMessage("initialView.step");

/** Asks how the file should open, and makes the answer one edit. */
export function InitialViewDialog(props: InitialViewDialogProps): ReactNode {
  const state = useInitialViewForm(props);
  const takeCurrent = () => {
    const now = props.current();
    state.set({ page: now.page, zoom: now.zoom }, String(now.page + 1));
  };
  return (
    <Dialog
      isOpen={props.isOpen}
      onOpenChange={(open) => {
        if (!open) props.onClose();
      }}
      size="form"
      title={laterMessage("initialView.title")}
      description={laterMessage("initialView.intro")}
      closeLabel={laterMessage("initialView.close")}
      actions={
        <>
          <Button variant="ghost" isDisabled={state.keepOpen || state.form === undefined} onPress={takeCurrent}>
            {laterMessage("initialView.useCurrent")}
          </Button>
          <Button onPress={props.onClose}>{laterMessage("initialView.cancel")}</Button>
          <Button
            variant="primary"
            isDisabled={state.change === undefined}
            isPending={state.applying}
            onPress={state.apply}
          >
            {laterMessage("initialView.apply")}
          </Button>
        </>
      }
    >
      <div className="paper-initial-view">
        {state.failed ? <Notice tone="danger" title={laterMessage("initialView.failed")} /> : null}
        <OpenFields props={props} state={state} />
        <ViewFields props={props} state={state} />
      </div>
    </Dialog>
  );
}
