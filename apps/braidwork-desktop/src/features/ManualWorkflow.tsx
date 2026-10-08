import type { MessageKey } from "../i18n";
import { useI18n } from "../i18n/context";
import { useEffect, useState, type FormEvent } from "react";
import { Copy, ExternalLink, FileUp, Send } from "lucide-react";
import { useWorkspace } from "../app/context";
import { api, asError } from "../lib/api";
import { copyText, chooseFiles } from "../lib/native";
import {
  assignmentLabel,
  lines,
  stateLabel,
  modelLabel,
  usageLabel,
} from "../lib/derive";
import {
  Field,
  ErrorNotice,
  Technical,
  ContentPreview,
} from "../components/Common";
import type {
  AssignmentDetail,
  ArtifactKind,
  IngestInput,
  IpcError,
  ResultDetail,
} from "../types/ipc";
const artifactKinds: ArtifactKind[] = [
  "text",
  "analysis",
  "finding",
  "documentation",
  "question",
  "data",
  "patch",
  "test_result",
];
export function ManualWorkflow({ assignmentId }: { assignmentId: string }) {
  const { t } = useI18n();
  const { revision, busy, mutate, report, notify } = useWorkspace();
  const [detail, setDetail] = useState<AssignmentDetail | null>(null);
  const [error, setError] = useState<IpcError | null>(null);
  const [loading, setLoading] = useState(true);
  const [preparing, setPreparing] = useState(false);
  useEffect(() => {
    let active = true;
    setLoading(true);
    setError(null);
    void api
      .assignmentDetail(assignmentId)
      .then((value) => {
        if (active) setDetail(value);
      })
      .catch((error) => {
        if (active) setError(asError(error));
      })
      .finally(() => {
        if (active) setLoading(false);
      });
    return () => {
      active = false;
    };
  }, [assignmentId, revision]);
  const perform = async (
    action: () => Promise<unknown>,
    message: MessageKey,
  ) => {
    try {
      await action();
      notify(message);
    } catch (error) {
      report(error);
    }
  };
  if (error) return <ErrorNotice error={error} />;
  if (loading || !detail)
    return <p role="status">{t("Loading assigned work…")}</p>;
  return (
    <div className="workflow">
      <h2>{detail.task.title}</h2>
      <p className="muted">
        {detail.agent.name} · {detail.session.label}
      </p>
      <ol className="workflow-steps" aria-label={t("Manual workflow")}>
        <li>{t("Assigned")}</li>
        <li className={detail.instructions ? "recorded" : ""}>
          {t("Instructions")}
        </li>
        <li
          className={detail.assignment.status !== "prepared" ? "recorded" : ""}
        >
          {" "}
          {t("Dispatch")}{" "}
        </li>
        <li className={detail.result ? "recorded" : ""}>{t("Result")}</li>
      </ol>
      <dl>
        <dt>{t("State")}</dt>
        <dd>{assignmentLabel(detail.assignment.status, t)}</dd>
        <dt>{t("Resource")}</dt>
        <dd>{detail.resource.name}</dd>
        <dt>{t("Agent revision")}</dt>
        <dd>
          {detail.agent.name} · {detail.agent.revision}
        </dd>
      </dl>
      <section className="inspector-section">
        <h3>{t("Instructions")}</h3>
        {detail.instructions ? (
          <>
            <p className="muted">
              {" "}
              {t("Declared budget:")} {detail.instructions.max_estimated_tokens}{" "}
              {t("estimated tokens. Not counted.")}{" "}
            </p>
            <ContentPreview text={detail.instructions.rendered} instructions />
            <div className="actions">
              <button
                onClick={() =>
                  void perform(
                    () => copyText(detail.instructions?.rendered ?? ""),
                    "Instructions copied",
                  )
                }
              >
                <Copy size={15} /> {t("Copy instructions")}{" "}
              </button>
            </div>
          </>
        ) : preparing ? (
          <PrepareForm
            assignmentId={assignmentId}
            done={() => setPreparing(false)}
          />
        ) : (
          <>
            <p>
              {t("Snapshot the context and outputs this specialist needs.")}
            </p>
            <button
              className="primary"
              disabled={busy}
              onClick={() => setPreparing(true)}
            >
              {" "}
              {t("Prepare instructions")}{" "}
            </button>
          </>
        )}
      </section>
      <section className="inspector-section">
        <h3>{t("Dispatch")}</h3>
        <p className="muted">
          {" "}
          {t(
            "Copy the instructions into your external AI session. Braidwork does not contact the provider.",
          )}{" "}
        </p>
        {detail.session.external_ref && (
          <>
            <p className="reference">{detail.session.external_ref}</p>
            <div className="actions">
              {detail.external_url ? (
                <button
                  onClick={() =>
                    void perform(
                      () => api.openExternal(detail.session.id),
                      "External reference opened",
                    )
                  }
                >
                  <ExternalLink size={15} /> {t("Open external chat")}{" "}
                </button>
              ) : (
                <button
                  onClick={() =>
                    void perform(
                      () => copyText(detail.session.external_ref ?? ""),
                      "Reference copied",
                    )
                  }
                >
                  <Copy size={15} /> {t("Copy reference")}{" "}
                </button>
              )}
            </div>
          </>
        )}
        {detail.assignment.status === "prepared" && (
          <button
            className="primary"
            disabled={busy || !detail.instructions}
            onClick={() =>
              void perform(
                () =>
                  mutate(() => api.dispatch(assignmentId), "Dispatch recorded"),
                "Dispatch recorded",
              )
            }
          >
            <Send size={15} /> {t("Mark dispatched")}{" "}
          </button>
        )}
        {detail.assignment.status === "dispatched" && (
          <p>
            {t("Dispatch recorded. Waiting for you to import the response.")}
          </p>
        )}
      </section>
      <section className="inspector-section">
        <h3>{t("Result")}</h3>
        {detail.result ? (
          <>
            <p className="result-received">{t("Result received")}</p>
            <ResultInspector assignmentId={assignmentId} />
          </>
        ) : detail.instructions ? (
          <IngestForm assignmentId={assignmentId} />
        ) : (
          <p className="muted">
            {" "}
            {t("Prepare instructions before importing a response.")}{" "}
          </p>
        )}
      </section>
      <Technical>
        <dl>
          <dt>{t("Assignment")}</dt>
          <dd>{detail.assignment.id}</dd>
          <dt>{t("Task")}</dt>
          <dd>{detail.task.id}</dd>
          <dt>{t("Session")}</dt>
          <dd>{detail.session.id}</dd>
          <dt>{t("Agent")}</dt>
          <dd>
            {detail.agent.id}@{detail.agent.revision}
          </dd>
          {detail.instructions && (
            <>
              <dt>{t("Capsule")}</dt>
              <dd>{detail.instructions.capsule_id}</dd>
            </>
          )}
        </dl>
      </Technical>
    </div>
  );
}
function PrepareForm({
  assignmentId,
  done,
}: {
  assignmentId: string;
  done: () => void;
}) {
  const { t } = useI18n();
  const { busy, mutate, report } = useWorkspace();
  const [files, setFiles] = useState<string[]>([]);
  const submit = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    const data = new FormData(event.currentTarget);
    const context = String(data.get("context") ?? "");
    try {
      await mutate(
        () =>
          api.prepare(assignmentId, {
            inputs: context
              ? [
                  {
                    kind: "inline",
                    label: t("Selected context"),
                    text: context,
                  },
                ]
              : [],
            context_files: files,
            constraints: lines(String(data.get("constraints") ?? "")),
            acceptance_criteria: lines(String(data.get("acceptance") ?? "")),
            expected_outputs: lines(String(data.get("output"))).map(
              (description) => ({
                kind: String(data.get("kind")) as ArtifactKind,
                description,
              }),
            ),
            max_context_tokens: String(data.get("budget")),
          }),
        "Instructions prepared",
      );
      done();
    } catch {
      /* Keep user input for correction. */
    }
  };
  const pick = async () => {
    try {
      const chosen = await chooseFiles(true, t("Choose context files (UTF-8)"));
      if (chosen.length)
        setFiles((previous) => [...new Set([...previous, ...chosen])]);
    } catch (error) {
      report(error);
    }
  };
  return (
    <form className="compact-form" onSubmit={(event) => void submit(event)}>
      <Field label={t("Context text")}>
        {(id) => <textarea id={id} name="context" rows={4} />}
      </Field>
      <button type="button" onClick={() => void pick()}>
        <FileUp size={15} /> {t("Choose context files")}{" "}
      </button>
      {files.length > 0 && (
        <div className="file-selection">
          <p>
            {files.length}{" "}
            {t(
              "selected UTF-8 files (read in Rust and snapshotted inline)",
            )}{" "}
          </p>
          {files.map((path) => (
            <div className="selected-file" key={path}>
              <span>{path}</span>
              <button
                type="button"
                aria-label={t("Remove {path}", { path })}
                onClick={() =>
                  setFiles((files) => files.filter((file) => file !== path))
                }
              >
                {" "}
                {t("Remove")}{" "}
              </button>
            </div>
          ))}
        </div>
      )}
      <Field label={t("Constraints")} hint={t("One per line.")}>
        {(id) => <textarea id={id} name="constraints" rows={2} />}
      </Field>
      <Field
        label={t("Acceptance criteria")}
        hint={t("One per line. Import does not verify these automatically.")}
      >
        {(id) => <textarea id={id} name="acceptance" rows={2} />}
      </Field>
      <Field label={t("Expected output kind")}>
        {(id) => (
          <select id={id} name="kind" defaultValue="text">
            {artifactKinds.map((kind) => (
              <option key={kind} value={kind}>
                {stateLabel(kind, t)}
              </option>
            ))}
          </select>
        )}
      </Field>
      <Field
        label={t("Expected outputs")}
        hint={t("One description per line, using the selected kind.")}
      >
        {(id) => (
          <textarea
            id={id}
            name="output"
            required
            defaultValue={t("Provide the task result")}
            rows={2}
          />
        )}
      </Field>
      <Field
        label={t("Context budget (estimated tokens)")}
        hint={t(
          "Provider-independent declaration. Zero permits no selected context.",
        )}
      >
        {(id) => (
          <input
            id={id}
            name="budget"
            inputMode="numeric"
            pattern="[0-9]+"
            required
            defaultValue="4096"
          />
        )}
      </Field>
      <div className="actions">
        <button type="button" disabled={busy} onClick={done}>
          {" "}
          {t("Cancel")}{" "}
        </button>
        <button type="submit" className="primary" disabled={busy}>
          {busy ? t("Preparing…") : t("Prepare instructions")}
        </button>
      </div>
    </form>
  );
}
function IngestForm({ assignmentId }: { assignmentId: string }) {
  const { t } = useI18n();
  const { busy, mutate, report } = useWorkspace();
  const [mode, setMode] = useState<"text" | "file">("text");
  const [path, setPath] = useState<string | null>(null);
  const submit = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    const data = new FormData(event.currentTarget);
    const optional = (key: string) => String(data.get(key) ?? "") || null;
    const input: IngestInput = {
      model_id: optional("model"),
      kind: String(data.get("kind")) as ArtifactKind,
      media_type: optional("media"),
      input_tokens: optional("input_tokens"),
      output_tokens: optional("output_tokens"),
      cost_micros: optional("cost"),
      currency: optional("currency"),
    };
    try {
      if (mode === "text")
        await mutate(
          () =>
            api.ingestText(
              assignmentId,
              String(data.get("result") ?? ""),
              input,
            ),
          "Result ingested",
        );
      else if (path)
        await mutate(
          () => api.ingestFile(assignmentId, path, input),
          "Result ingested",
        );
    } catch {
      /* Typed error displayed; source remains untouched. */
    }
  };
  const pick = async () => {
    try {
      const [file] = await chooseFiles(false, t("Choose a result file"));
      if (file) setPath(file);
    } catch (error) {
      report(error);
    }
  };
  return (
    <form className="compact-form" onSubmit={(event) => void submit(event)}>
      <div className="tabs" aria-label={t("Result source")}>
        <button
          type="button"
          aria-current={mode === "text" ? "page" : undefined}
          onClick={() => setMode("text")}
        >
          {" "}
          {t("Paste text")}{" "}
        </button>
        <button
          type="button"
          aria-current={mode === "file" ? "page" : undefined}
          onClick={() => setMode("file")}
        >
          {" "}
          {t("Import file")}{" "}
        </button>
      </div>
      {mode === "text" ? (
        <Field label={t("External AI result")}>
          {(id) => (
            <textarea
              id={id}
              name="result"
              required
              rows={7}
              placeholder={t("Paste the actual response here…")}
            />
          )}
        </Field>
      ) : (
        <>
          <button type="button" onClick={() => void pick()}>
            <FileUp size={15} /> {t("Choose result file")}{" "}
          </button>
          <p className="reference">{path ?? t("No file selected")}</p>
        </>
      )}
      <details>
        <summary>{t("Advanced observations")}</summary>
        <Field label={t("Model ID (optional)")}>
          {(id) => <input id={id} name="model" />}
        </Field>
        <Field label={t("Artifact kind")}>
          {(id) => (
            <select id={id} name="kind" defaultValue="text">
              {artifactKinds.map((kind) => (
                <option key={kind} value={kind}>
                  {stateLabel(kind, t)}
                </option>
              ))}
            </select>
          )}
        </Field>
        <Field label={t("Media type")}>
          {(id) => <input id={id} name="media" defaultValue="text/markdown" />}
        </Field>
        {[
          ["input_tokens", t("Input tokens")],
          ["output_tokens", t("Output tokens")],
          ["cost", t("Cost micros")],
        ].map(([name, label]) => (
          <Field
            key={name}
            label={t("{label} (optional)", { label: label ?? "" })}
          >
            {(id) => (
              <input id={id} name={name} inputMode="numeric" pattern="[0-9]+" />
            )}
          </Field>
        ))}
        <Field label={t("Currency (required if cost is known)")}>
          {(id) => <input id={id} name="currency" placeholder={t("USD")} />}
        </Field>
        <p className="muted">
          {" "}
          {t(
            "Leave unknown observations empty. A recorded zero is a known zero.",
          )}{" "}
        </p>
      </details>
      <p className="muted">
        {" "}
        {t(
          "This records a received result. Verification is not performed; task status stays unchanged.",
        )}{" "}
      </p>
      <button
        className="primary"
        type="submit"
        disabled={busy || (mode === "file" && !path)}
      >
        {busy ? t("Importing…") : t("Ingest result")}
      </button>
    </form>
  );
}
export function ResultInspector({ assignmentId }: { assignmentId: string }) {
  const { t } = useI18n();
  const { revision } = useWorkspace();
  const [detail, setDetail] = useState<ResultDetail | null>(null);
  const [error, setError] = useState<IpcError | null>(null);
  const [loading, setLoading] = useState(true);
  useEffect(() => {
    let active = true;
    setLoading(true);
    setError(null);
    void api
      .resultDetail(assignmentId)
      .then((value) => {
        if (active) setDetail(value);
      })
      .catch((error) => {
        if (active) setError(asError(error));
      })
      .finally(() => {
        if (active) setLoading(false);
      });
    return () => {
      active = false;
    };
  }, [assignmentId, revision]);
  if (error) return <ErrorNotice error={error} />;
  if (loading || !detail) return <p role="status">{t("Loading result…")}</p>;
  const receipt = detail.receipt;
  return (
    <>
      <dl>
        <dt>{t("Model")}</dt>
        <dd>{modelLabel(receipt.model_id, t)}</dd>
        <dt>{t("Execution")}</dt>
        <dd>{stateLabel(receipt.execution.outcome, t)}</dd>
        <dt>{t("Verification")}</dt>
        <dd>{stateLabel(receipt.verification.decision, t)}</dd>
        <dt>{t("Transport")}</dt>
        <dd>{stateLabel(receipt.access_mode, t)}</dd>
      </dl>
      <p className="muted">{usageLabel(receipt.usage, t)}</p>
      {detail.artifacts.map((preview) => (
        <section className="inspector-section" key={preview.artifact.id}>
          <h3>{stateLabel(preview.artifact.kind, t)}</h3>
          <p className="muted">
            {preview.artifact.media_type ?? t("Unknown media type")} ·{" "}
            {preview.artifact.size_bytes ?? t("Unknown")} {t("bytes")}{" "}
          </p>
          {preview.status === "text" ? (
            <ContentPreview text={preview.text ?? ""} />
          ) : (
            <p className="preview-unavailable">
              {preview.status === "too_large"
                ? t(
                    "Preview not loaded because this result is large (over 1 MiB).",
                  )
                : preview.status === "binary"
                  ? t(
                      "Preview not loaded: media type is not known to be text-like.",
                    )
                  : t("Preview not loaded: content is not valid UTF-8.")}{" "}
              {t("The complete artifact remains on disk.")}{" "}
            </p>
          )}
          <Technical>
            <p>
              {t("Artifact:")} {preview.artifact.id}
            </p>
          </Technical>
        </section>
      ))}
      <Technical>
        <dl>
          <dt>{t("Receipt")}</dt>
          <dd>{receipt.id}</dd>
          <dt>{t("Task")}</dt>
          <dd>{receipt.task_id}</dd>
          <dt>{t("Capsule")}</dt>
          <dd>{receipt.capsule_id}</dd>
          <dt>{t("Resource")}</dt>
          <dd>{receipt.resource_id}</dd>
        </dl>
      </Technical>
    </>
  );
}
