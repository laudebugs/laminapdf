// The submit confirmation (Paper P01, E8 plan §2.3): a form's button asks to send its answers
// to another computer. The dialog names the button, the address, the format and every field and
// value that leaves the device, and says what isn't sent. Nothing is sent before Send, and Don't
// send has the focus, so Enter never sends by accident. A refused address or a send the server
// didn't accept shows inside the dialog, with Export form data… as the way out.

import type { ReactNode } from "react";
import { Button } from "../components/button.js";
import { Notice } from "../components/notice.js";
import { laterCountMessage, laterMessage } from "../later-messages.js";
import { Dialog } from "../overlays/dialog.js";

/** One field of a send. */
export interface SubmitField {
  /** The fully qualified name. */
  readonly name: string;
  /** The value that leaves the device; empty for a field the action sends without an answer. */
  readonly value: string;
}

/** Why nothing can be sent, or a send that failed. */
export type SubmitProblem =
  /** The address isn't `https:`: Send is gone. */
  | { readonly kind: "insecure" }
  /** The button would send the whole PDF: Send is gone. */
  | { readonly kind: "pdf" }
  /** The server didn't accept a send from a web page (no CORS); `mayHaveSent`: the request may have reached it. */
  | { readonly kind: "blocked"; readonly mayHaveSent: boolean }
  /** No connection, or the server is down. */
  | { readonly kind: "unreachable"; readonly mayHaveSent: boolean }
  /** No answer in time: the request may have reached the server. */
  | { readonly kind: "timeout" }
  /** The server answered with an error: it may have kept the answers. */
  | { readonly kind: "status"; readonly status: number };

/** Whether the answers may have reached the server although the send failed: the dialog then warns and offers Send again. */
export function mayHaveSent(problem: SubmitProblem | undefined): boolean {
  if (problem === undefined) return false;
  return problem.kind === "timeout" || problem.kind === "status" || ("mayHaveSent" in problem && problem.mayHaveSent);
}

/** What the dialog shows. */
export interface SubmitPlan {
  /** The button's name, as a person reads it. */
  readonly button: string;
  /** The address in three parts: the scheme muted, the host in weight 600, then the path. */
  readonly to: { readonly scheme: string; readonly host: string; readonly path: string };
  /** `html`, `fdf` or `xfdf`. */
  readonly format: "html" | "fdf" | "xfdf";
  /** Every field in the body, in order. */
  readonly fields: readonly SubmitField[];
  /** Required fields that are still empty: the send waits until they are filled. */
  readonly emptyRequired: readonly string[];
}

/** Props of `SubmitFormDialog`. */
export interface SubmitFormDialogProps {
  /** What the button would send; the dialog is closed without it. */
  readonly plan: SubmitPlan | undefined;
  /** Why it can't be sent, or how the last send failed. */
  readonly problem?: SubmitProblem | undefined;
  /** A send is in progress. */
  readonly sending?: boolean;
  /** Send was pressed: the one request the dialog describes. */
  readonly onSend: () => void;
  /** Don't send, Close, Escape: nothing leaves the device. */
  readonly onCancel: () => void;
  /** Export form data… was pressed. */
  readonly onExport: () => void;
}

/** The notice's words for a problem. */
function words(problem: SubmitProblem, host: string): { readonly title: string; readonly body: string } {
  switch (problem.kind) {
    case "insecure":
      return {
        title: laterMessage("form.submit.refused.insecure.title"),
        body: laterMessage("form.submit.refused.insecure.body"),
      };
    case "pdf":
      return {
        title: laterMessage("form.submit.refused.pdf.title"),
        body: laterMessage("form.submit.refused.pdf.body"),
      };
    case "blocked":
      return problem.mayHaveSent
        ? {
            title: laterMessage("form.submit.error.maybe.title"),
            body: laterMessage("form.submit.error.blocked.maybe", { host }),
          }
        : {
            title: laterMessage("form.submit.error.blocked.title"),
            body: laterMessage("form.submit.error.blocked.body", { host }),
          };
    case "unreachable":
      return problem.mayHaveSent
        ? {
            title: laterMessage("form.submit.error.maybe.title"),
            body: laterMessage("form.submit.error.unreachable.maybe", { host }),
          }
        : {
            title: laterMessage("form.submit.error.unreachable.title"),
            body: laterMessage("form.submit.error.unreachable.body", { host }),
          };
    case "timeout":
      return {
        title: laterMessage("form.submit.error.maybe.title"),
        body: laterMessage("form.submit.error.timeout.body", { host }),
      };
    default:
      return {
        title: laterMessage("form.submit.error.maybe.title"),
        body: laterMessage("form.submit.error.status.body", { host, status: problem.status }),
      };
  }
}

/** "Send this form to another computer?": the one confirmation before a form's answers leave. */
export function SubmitFormDialog(props: SubmitFormDialogProps): ReactNode {
  const { plan, problem, sending = false, onSend, onCancel, onExport } = props;
  const notSendable = problem?.kind === "insecure" || problem?.kind === "pdf";
  const waiting = plan !== undefined && plan.emptyRequired.length > 0;
  const again = mayHaveSent(problem);
  return (
    <Dialog
      isOpen={plan !== undefined}
      onOpenChange={(open) => {
        if (!open) onCancel();
      }}
      title={laterMessage("form.submit.title")}
      description={
        plan === undefined ? "" : laterMessage("form.submit.description", { button: plan.button, host: plan.to.host })
      }
      closeLabel={laterMessage("form.submit.close")}
      size="form"
      actions={
        <>
          {problem === undefined && !waiting ? null : (
            <Button onPress={onExport}>{laterMessage("form.submit.export")}</Button>
          )}
          <Button autoFocus onPress={onCancel}>
            {laterMessage("form.submit.cancel")}
          </Button>
          {notSendable ? null : (
            <Button variant={again ? "secondary" : "primary"} isDisabled={sending || waiting} onPress={onSend}>
              {laterMessage(sending ? "form.submit.sending" : again ? "form.submit.again" : "form.submit.send")}
            </Button>
          )}
        </>
      }
    >
      {plan === undefined ? null : <SubmitBody plan={plan} problem={problem} />}
    </Dialog>
  );
}

/** The dialog's content: the problem, the required fields still empty, what is sent and where. */
function SubmitBody({ plan, problem }: { plan: SubmitPlan; problem: SubmitProblem | undefined }): ReactNode {
  const said = problem === undefined ? undefined : words(problem, plan.to.host);
  return (
    <>
      {said === undefined ? null : (
        <div role="alert">
          <Notice tone="danger" title={said.title}>
            {said.body}
          </Notice>
        </div>
      )}
      {plan.emptyRequired.length === 0 ? null : (
        <Notice tone="warning" title={laterMessage("form.submit.required.title")}>
          {laterMessage("form.submit.required.body", { names: plan.emptyRequired.join(", ") })}
        </Notice>
      )}
      <dl className="paper-form-submit">
        <dt>{laterMessage("form.submit.button")}</dt>
        <dd>{plan.button}</dd>
        <dt>{laterMessage("form.submit.to")}</dt>
        <dd>
          <span className="paper-form-submit-scheme" data-insecure={problem?.kind === "insecure" || undefined}>
            {plan.to.scheme}
          </span>
          <strong>{plan.to.host}</strong>
          {plan.to.path}
        </dd>
        <dt>{laterMessage("form.submit.format")}</dt>
        <dd>{laterMessage(`form.submit.format.${plan.format}`)}</dd>
        <dt>{laterMessage("form.submit.fields")}</dt>
        <dd>
          <FieldList fields={plan.fields} />
        </dd>
        <dt>{laterMessage("form.submit.notSent")}</dt>
        <dd>{laterMessage("form.submit.notSent.body")}</dd>
      </dl>
      <p className="paper-form-dialog-help">
        {laterMessage(plan.format === "html" ? "form.submit.trust" : "form.submit.trust.asks")}
      </p>
    </>
  );
}

/** Every field and value that leaves the device, behind one disclosure. */
function FieldList({ fields }: { fields: readonly SubmitField[] }): ReactNode {
  if (fields.length === 0) return laterMessage("form.submit.fields.none");
  return (
    <details>
      <summary>
        {laterCountMessage(fields.length, { one: "form.submit.show.one", many: "form.submit.show.many" })}
      </summary>
      <table className="paper-form-submit-fields">
        <tbody>
          {fields.map((field) => (
            <tr key={field.name}>
              <th scope="row">{field.name}</th>
              <td>{field.value === "" ? laterMessage("form.submit.empty") : field.value}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </details>
  );
}
