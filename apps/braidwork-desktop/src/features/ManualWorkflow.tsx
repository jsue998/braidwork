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
  const perform = async (action: () => Promise<unknown>, message: string) => {
    try {
      await action();
      notify(message);
    } catch (error) {
      report(error);
    }
  };
  if (error) return <ErrorNotice error={error} />;
  if (loading || !detail) return <p role="status">Loading assigned work…</p>;
  return (
    <div className="workflow">
      <h2>{detail.task.title}</h2>
      <p className="muted">
        {detail.agent.name} · {detail.session.label}
      </p>
      <ol className="workflow-steps" aria-label="Manual workflow">
        <li>Assigned</li>
        <li className={detail.instructions ? "recorded" : ""}>Instructions</li>
        <li
          className={detail.assignment.status !== "prepared" ? "recorded" : ""}
        >
          Dispatch
        </li>
        <li className={detail.result ? "recorded" : ""}>Result</li>
      </ol>
      <dl>
        <dt>State</dt>
        <dd>{assignmentLabel(detail.assignment.status)}</dd>
        <dt>Resource</dt>
        <dd>{detail.resource.name}</dd>
        <dt>Agent revision</dt>
        <dd>
          {detail.agent.name} · {detail.agent.revision}
        </dd>
      </dl>
      <section className="inspector-section">
        <h3>Instructions</h3>
        {detail.instructions ? (
          <>
            <p className="muted">
              Declared budget: {detail.instructions.max_estimated_tokens}{" "}
              estimated tokens. Not counted.
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
                <Copy size={15} /> Copy instructions
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
            <p>Snapshot the context and outputs this specialist needs.</p>
            <button
              className="primary"
              disabled={busy}
              onClick={() => setPreparing(true)}
            >
              Prepare instructions
            </button>
          </>
        )}
      </section>
      <section className="inspector-section">
        <h3>Dispatch</h3>
        <p className="muted">
          Copy the instructions into your external AI session. Braidwork does
          not contact the provider.
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
                  <ExternalLink size={15} /> Open external chat
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
                  <Copy size={15} /> Copy reference
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
            <Send size={15} /> Mark dispatched
          </button>
        )}
        {detail.assignment.status === "dispatched" && (
          <p>Dispatch recorded. Waiting for you to import the response.</p>
        )}
      </section>
      <section className="inspector-section">
        <h3>Result</h3>
        {detail.result ? (
          <>
            <p className="result-received">Result received</p>
            <ResultInspector assignmentId={assignmentId} />
          </>
        ) : detail.instructions ? (
          <IngestForm assignmentId={assignmentId} />
        ) : (
          <p className="muted">
            Prepare instructions before importing a response.
          </p>
        )}
      </section>
      <Technical>
        <dl>
          <dt>Assignment</dt>
          <dd>{detail.assignment.id}</dd>
          <dt>Task</dt>
          <dd>{detail.task.id}</dd>
          <dt>Session</dt>
          <dd>{detail.session.id}</dd>
          <dt>Agent</dt>
          <dd>
            {detail.agent.id}@{detail.agent.revision}
          </dd>
          {detail.instructions && (
            <>
              <dt>Capsule</dt>
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
              ? [{ kind: "inline", label: "Selected context", text: context }]
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
      const chosen = await chooseFiles(true);
      if (chosen.length)
        setFiles((previous) => [...new Set([...previous, ...chosen])]);
    } catch (error) {
      report(error);
    }
  };
  return (
    <form className="compact-form" onSubmit={(event) => void submit(event)}>
      <Field label="Context text">
        {(id) => <textarea id={id} name="context" rows={4} />}
      </Field>
      <button type="button" onClick={() => void pick()}>
        <FileUp size={15} /> Choose context files
      </button>
      {files.length > 0 && (
        <div className="file-selection">
          <p>
            {files.length} selected UTF-8 files (read in Rust and snapshotted
            inline)
          </p>
          {files.map((path) => (
            <div className="selected-file" key={path}>
              <span>{path}</span>
              <button
                type="button"
                aria-label={`Remove ${path}`}
                onClick={() =>
                  setFiles((files) => files.filter((file) => file !== path))
                }
              >
                Remove
              </button>
            </div>
          ))}
        </div>
      )}
      <Field label="Constraints" hint="One per line.">
        {(id) => <textarea id={id} name="constraints" rows={2} />}
      </Field>
      <Field
        label="Acceptance criteria"
        hint="One per line. Import does not verify these automatically."
      >
        {(id) => <textarea id={id} name="acceptance" rows={2} />}
      </Field>
      <Field label="Expected output kind">
        {(id) => (
          <select id={id} name="kind" defaultValue="text">
            {artifactKinds.map((kind) => (
              <option key={kind} value={kind}>
                {stateLabel(kind)}
              </option>
            ))}
          </select>
        )}
      </Field>
      <Field
        label="Expected outputs"
        hint="One description per line, using the selected kind."
      >
        {(id) => (
          <textarea
            id={id}
            name="output"
            required
            defaultValue="Provide the task result"
            rows={2}
          />
        )}
      </Field>
      <Field
        label="Context budget (estimated tokens)"
        hint="Provider-independent declaration. Zero permits no selected context."
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
          Cancel
        </button>
        <button type="submit" className="primary" disabled={busy}>
          {busy ? "Preparing…" : "Prepare instructions"}
        </button>
      </div>
    </form>
  );
}
function IngestForm({ assignmentId }: { assignmentId: string }) {
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
      const [file] = await chooseFiles(false);
      if (file) setPath(file);
    } catch (error) {
      report(error);
    }
  };
  return (
    <form className="compact-form" onSubmit={(event) => void submit(event)}>
      <div className="tabs" aria-label="Result source">
        <button
          type="button"
          aria-current={mode === "text" ? "page" : undefined}
          onClick={() => setMode("text")}
        >
          Paste text
        </button>
        <button
          type="button"
          aria-current={mode === "file" ? "page" : undefined}
          onClick={() => setMode("file")}
        >
          Import file
        </button>
      </div>
      {mode === "text" ? (
        <Field label="External AI result">
          {(id) => (
            <textarea
              id={id}
              name="result"
              required
              rows={7}
              placeholder="Paste the actual response here…"
            />
          )}
        </Field>
      ) : (
        <>
          <button type="button" onClick={() => void pick()}>
            <FileUp size={15} /> Choose result file
          </button>
          <p className="reference">{path ?? "No file selected"}</p>
        </>
      )}
      <details>
        <summary>Advanced observations</summary>
        <Field label="Model ID (optional)">
          {(id) => <input id={id} name="model" />}
        </Field>
        <Field label="Artifact kind">
          {(id) => (
            <select id={id} name="kind" defaultValue="text">
              {artifactKinds.map((kind) => (
                <option key={kind} value={kind}>
                  {stateLabel(kind)}
                </option>
              ))}
            </select>
          )}
        </Field>
        <Field label="Media type">
          {(id) => <input id={id} name="media" defaultValue="text/markdown" />}
        </Field>
        {[
          ["input_tokens", "Input tokens"],
          ["output_tokens", "Output tokens"],
          ["cost", "Cost micros"],
        ].map(([name, label]) => (
          <Field key={name} label={`${label} (optional)`}>
            {(id) => (
              <input id={id} name={name} inputMode="numeric" pattern="[0-9]+" />
            )}
          </Field>
        ))}
        <Field label="Currency (required if cost is known)">
          {(id) => <input id={id} name="currency" placeholder="USD" />}
        </Field>
        <p className="muted">
          Leave unknown observations empty. A recorded zero is a known zero.
        </p>
      </details>
      <p className="muted">
        This records a received result. Verification is not performed; task
        status stays unchanged.
      </p>
      <button
        className="primary"
        type="submit"
        disabled={busy || (mode === "file" && !path)}
      >
        {busy ? "Importing…" : "Ingest result"}
      </button>
    </form>
  );
}
export function ResultInspector({ assignmentId }: { assignmentId: string }) {
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
  if (loading || !detail) return <p role="status">Loading result…</p>;
  const receipt = detail.receipt;
  return (
    <>
      <dl>
        <dt>Model</dt>
        <dd>{modelLabel(receipt.model_id)}</dd>
        <dt>Execution</dt>
        <dd>{stateLabel(receipt.execution.outcome)}</dd>
        <dt>Verification</dt>
        <dd>{stateLabel(receipt.verification.decision)}</dd>
        <dt>Transport</dt>
        <dd>{stateLabel(receipt.access_mode)}</dd>
      </dl>
      <p className="muted">{usageLabel(receipt.usage)}</p>
      {detail.artifacts.map((preview) => (
        <section className="inspector-section" key={preview.artifact.id}>
          <h3>{stateLabel(preview.artifact.kind)}</h3>
          <p className="muted">
            {preview.artifact.media_type ?? "Unknown media type"} ·{" "}
            {preview.artifact.size_bytes ?? "Unknown"} bytes
          </p>
          {preview.status === "text" ? (
            <ContentPreview text={preview.text ?? ""} />
          ) : (
            <p className="preview-unavailable">
              {preview.status === "too_large"
                ? "Preview not loaded because this result is large (over 1 MiB)."
                : preview.status === "binary"
                  ? "Preview not loaded: media type is not known to be text-like."
                  : "Preview not loaded: content is not valid UTF-8."}{" "}
              The complete artifact remains on disk.
            </p>
          )}
          <Technical>
            <p>Artifact: {preview.artifact.id}</p>
          </Technical>
        </section>
      ))}
      <Technical>
        <dl>
          <dt>Receipt</dt>
          <dd>{receipt.id}</dd>
          <dt>Task</dt>
          <dd>{receipt.task_id}</dd>
          <dt>Capsule</dt>
          <dd>{receipt.capsule_id}</dd>
          <dt>Resource</dt>
          <dd>{receipt.resource_id}</dd>
        </dl>
      </Technical>
    </>
  );
}
