import { useI18n } from "../i18n/context";
import { X } from "lucide-react";
import { useWorkspace } from "../app/context";
import { Badge, Technical, ErrorNotice } from "../components/Common";
import { stateLabel, assignmentLabel } from "../lib/derive";
import { ManualWorkflow, ResultInspector } from "./ManualWorkflow";
import { api } from "../lib/api";
import type { TaskStatus } from "../types/ipc";
export function Inspector() {
  const { t } = useI18n();
  const {
    snapshot,
    selection,
    select,
    showForm,
    busy,
    mutate,
    error,
    clearError,
  } = useWorkspace();
  if (!snapshot || !selection) return null;
  let content;
  if (selection.kind === "assignment")
    content = <ManualWorkflow key={selection.id} assignmentId={selection.id} />;
  else if (selection.kind === "result") {
    const assignment = snapshot.assignments.find((a) => a.id === selection.id);
    content = (
      <>
        <h2>
          {snapshot.tasks.find((t) => t.id === assignment?.task_id)?.title ??
            t("Result")}
        </h2>
        <p>
          {
            snapshot.sessions.find((s) => s.id === assignment?.session_id)
              ?.label
          }
        </p>
        <ResultInspector key={selection.id} assignmentId={selection.id} />
      </>
    );
  } else if (selection.kind === "resource") {
    const resource = snapshot.resources.find((r) => r.id === selection.id);
    content = resource && (
      <>
        <h2>{resource.name}</h2>
        <p>{resource.provider}</p>
        <dl>
          <dt>{t("Access")}</dt>
          <dd>{stateLabel(resource.access_mode, t)}</dd>
          <dt>{t("Scarcity")}</dt>
          <dd>{stateLabel(resource.scarcity, t)}</dd>
          <dt>{t("Status")}</dt>
          <dd>
            <Badge state={resource.status} />
          </dd>
        </dl>
        <h3>{t("Sessions using this resource")}</h3>
        {snapshot.sessions
          .filter((s) => s.resource_id === resource.id)
          .map((session) => (
            <button
              className="entity-row"
              key={session.id}
              onClick={() => select({ kind: "session", id: session.id })}
            >
              {session.label}
            </button>
          ))}
        <Technical>
          <p>
            {t("Resource:")} {resource.id}
          </p>
        </Technical>
      </>
    );
  } else if (selection.kind === "agent") {
    const agent = snapshot.agents.find(
      (a) => a.id === selection.id && a.revision === selection.revision,
    );
    content = agent && (
      <>
        <h2>{agent.name}</h2>
        <p>
          {agent.role} {t("· revision")} {agent.revision}
        </p>
        <h3>{t("Mission")}</h3>
        <p className="preserve">{agent.mission}</p>
        <h3>{t("Instructions")}</h3>
        <ul>
          {agent.instructions.map((item, index) => (
            <li key={index}>{item}</li>
          ))}
        </ul>
        <h3>{t("Expertise")}</h3>
        <p>{agent.expertise.join(", ") || t("Not specified")}</p>
        <h3>{t("Delegation policy")}</h3>
        <p>
          {agent.delegation.allowed
            ? t("Can delegate")
            : t("Delegation not allowed")}{" "}
          {t("· maximum direct delegates:")}{" "}
          {agent.delegation.max_children ?? t("No limit")}
        </p>
        <Technical>
          <p>
            {" "}
            {t("Agent:")} {agent.id}@{agent.revision}
          </p>
        </Technical>
      </>
    );
  } else if (selection.kind === "session") {
    const session = snapshot.sessions.find((s) => s.id === selection.id);
    const agent = snapshot.agents.find(
      (a) =>
        a.id === session?.agent_spec_id &&
        a.revision === session?.agent_spec_revision,
    );
    content = session && (
      <>
        <h2>{session.label}</h2>
        <Badge state={session.status} />
        <dl>
          <dt>{t("Agent")}</dt>
          <dd>
            {agent?.name} · {agent?.role} {t("· revision")}{" "}
            {session.agent_spec_revision}
          </dd>
          <dt>{t("Resource")}</dt>
          <dd>
            {snapshot.resources.find((r) => r.id === session.resource_id)?.name}
          </dd>
          <dt>{t("External reference")}</dt>
          <dd className="reference">
            {session.external_ref ?? t("Not specified")}
          </dd>
        </dl>
        <p className="muted">
          {" "}
          {t(
            "The actual execution model is recorded on each result when known.",
          )}{" "}
        </p>
        <Technical>
          <p>
            {t("Session:")} {session.id}
          </p>
          <p>
            {t("Resource:")} {session.resource_id}
          </p>
          <p>
            {" "}
            {t("Agent:")} {session.agent_spec_id}@{session.agent_spec_revision}
          </p>
        </Technical>
      </>
    );
  } else {
    const task = snapshot.tasks.find((t) => t.id === selection.id);
    const assignments = snapshot.assignments.filter(
      (a) => a.task_id === task?.id,
    );
    content = task && (
      <>
        <h2>{task.title}</h2>
        <Badge state={task.status} />
        <h3>{t("Objective")}</h3>
        <p className="preserve">{task.objective}</p>
        <form
          className="compact-form"
          onSubmit={(event) => {
            event.preventDefault();
            const status = new FormData(event.currentTarget).get(
              "status",
            ) as TaskStatus;
            void mutate(
              () => api.status(task.id, status),
              "Task status recorded",
            ).catch(() => {});
          }}
        >
          <label htmlFor="task-status">{t("Declare task status")}</label>
          <select
            id="task-status"
            name="status"
            defaultValue={task.status}
            key={task.status}
          >
            <option value="pending">{t("Pending")}</option>
            <option value="in_progress">{t("In progress")}</option>
            <option value="completed">{t("Completed")}</option>
          </select>
          <button disabled={busy} type="submit">
            {" "}
            {t("Record status")}{" "}
          </button>
          <small>
            {t("This is your declaration. It does not verify a result.")}
          </small>
        </form>
        <dl>
          <dt>{t("Parent")}</dt>
          <dd>
            {snapshot.tasks.find((t) => t.id === task.parent)?.title ??
              t("No parent")}
          </dd>
          <dt>{t("Dependencies")}</dt>
          <dd>
            {task.dependencies
              .map((id) => snapshot.tasks.find((t) => t.id === id)?.title ?? id)
              .join(", ") || t("None")}
          </dd>
        </dl>
        <h3>{t("Assigned work")}</h3>
        {assignments.map((a) => (
          <button
            key={a.id}
            className="entity-row"
            onClick={() => select({ kind: "assignment", id: a.id })}
          >
            <span>
              {snapshot.sessions.find((s) => s.id === a.session_id)?.label}
              <small>{assignmentLabel(a.status, t)}</small>
            </span>
          </button>
        ))}
        <button
          className="primary"
          onClick={() => showForm({ kind: "assignment", task_id: task.id })}
        >
          {" "}
          {t("Assign to AI")}{" "}
        </button>
        <Technical>
          <p>
            {t("Task:")} {task.id}
          </p>
        </Technical>
      </>
    );
  }
  return (
    <aside className="inspector" aria-label={t("Selection inspector")}>
      <header className="inspector-heading">
        <span>{t("INSPECTOR")}</span>
        <button
          className="icon-button"
          aria-label={t("Close inspector")}
          onClick={() => select(null)}
        >
          <X size={18} />
        </button>
      </header>
      <div className="inspector-body">
        {error && <ErrorNotice error={error} dismiss={clearError} />}
        {content || (
          <p>{t("This record is no longer available. Refresh the project.")}</p>
        )}
      </div>
    </aside>
  );
}
