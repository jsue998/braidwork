import { useI18n } from "../i18n/context";
import { useEffect, useRef, type FormEvent } from "react";
import { X } from "lucide-react";
import { useWorkspace } from "../app/context";
import { api } from "../lib/api";
import { lines, assignmentName, stateLabel } from "../lib/derive";
import type { AccessMode, ResourceStatus, Scarcity } from "../types/ipc";
import { ErrorNotice, Field } from "../components/Common";
const text = (data: FormData, key: string) => String(data.get(key) ?? "");
export function CreationForms() {
  const { t } = useI18n();
  const {
    snapshot,
    form,
    busy,
    showForm,
    mutate,
    select,
    error,
    clearError,
    report,
  } = useWorkspace();
  const dialog = useRef<HTMLDialogElement>(null);
  useEffect(() => {
    const node = dialog.current;
    node?.showModal();
    return () => node?.close();
  }, [form?.kind]);
  if (!snapshot || !form) return null;
  const titles = {
    resource: t("Create AI resource"),
    agent: t("Create agent"),
    session: t("Create session"),
    task: t("Create work"),
    assignment: t("Assign to AI"),
    delegation: t("Delegate work"),
  };
  const submit = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    const data = new FormData(event.currentTarget);
    try {
      switch (form.kind) {
        case "resource": {
          const resource = await mutate(
            () =>
              api.resource({
                name: text(data, "name"),
                provider: text(data, "provider"),
                access_mode: text(data, "access_mode") as AccessMode,
                scarcity: text(data, "scarcity") as Scarcity,
                status: text(data, "status") as ResourceStatus,
              }),
            "Resource created",
          );
          select({ kind: "resource", id: resource.id });
          break;
        }
        case "agent": {
          const max = text(data, "max_children");
          const agent = await mutate(
            () =>
              api.agent({
                name: text(data, "name"),
                role: text(data, "role"),
                mission: text(data, "mission"),
                instructions: lines(text(data, "instructions")),
                expertise: lines(text(data, "expertise")),
                delegation: {
                  allowed: data.has("allowed"),
                  max_children: max ? Number(max) : null,
                },
              }),
            "Agent created",
          );
          select({ kind: "agent", id: agent.id, revision: agent.revision });
          break;
        }
        case "session": {
          const agent = snapshot.agents[Number(text(data, "agent"))];
          if (!agent) throw new Error(t("Choose an existing agent revision."));
          const session = await mutate(
            () =>
              api.session({
                label: text(data, "label"),
                resource_id: text(data, "resource"),
                agent_spec_id: agent.id,
                agent_spec_revision: agent.revision,
                external_ref: text(data, "external_ref") || null,
              }),
            "Session created",
          );
          select({ kind: "session", id: session.id });
          break;
        }
        case "task": {
          const task = await mutate(
            () =>
              api.task({
                title: text(data, "title"),
                objective: text(data, "objective"),
                parent: text(data, "parent") || null,
                dependencies: data.getAll("dependency").map(String),
              }),
            "Work created",
          );
          select({ kind: "task", id: task.id });
          break;
        }
        case "assignment": {
          const assignment = await mutate(
            () => api.assignment(text(data, "task"), text(data, "session")),
            "Work assigned",
          );
          select({ kind: "assignment", id: assignment.id });
          break;
        }
        case "delegation": {
          await mutate(
            () => api.delegation(text(data, "from"), text(data, "to")),
            "Delegation recorded",
          );
          break;
        }
      }
      showForm(null);
    } catch (error) {
      report(error);
      /* Provider displays the typed error; preserve the form for correction. */
    }
  };
  return (
    <dialog
      ref={dialog}
      className="modal"
      onCancel={(event) => {
        if (busy) event.preventDefault();
        else showForm(null);
      }}
      aria-labelledby="creation-title"
    >
      <header>
        <h2 id="creation-title">{titles[form.kind]}</h2>
        <button
          className="icon-button"
          aria-label={t("Close form")}
          disabled={busy}
          onClick={() => showForm(null)}
        >
          <X size={18} />
        </button>
      </header>
      {error && <ErrorNotice error={error} dismiss={clearError} />}
      <form onSubmit={(event) => void submit(event)}>
        {form.kind === "resource" && (
          <>
            <Field label={t("Name")}>
              {(id) => (
                <input
                  id={id}
                  name="name"
                  required
                  placeholder={t("Primary AI account")}
                />
              )}
            </Field>
            <Field label={t("Provider")}>
              {(id) => (
                <input
                  id={id}
                  name="provider"
                  required
                  placeholder={t("Provider name")}
                />
              )}
            </Field>
            <details>
              <summary>{t("Advanced")}</summary>
              <EnumField
                name="access_mode"
                label={t("Access mode")}
                values={["manual", "api", "harness", "local"]}
                defaultValue="manual"
              />
              <EnumField
                name="scarcity"
                label={t("Scarcity")}
                values={["abundant", "normal", "scarce", "critical"]}
                defaultValue="normal"
              />
              <EnumField
                name="status"
                label={t("Status")}
                values={["available", "unavailable", "exhausted"]}
                defaultValue="available"
              />
            </details>
          </>
        )}
        {form.kind === "agent" && (
          <>
            <Field label={t("Name")}>
              {(id) => (
                <input
                  id={id}
                  name="name"
                  required
                  placeholder={t("Travel Researcher")}
                />
              )}
            </Field>
            <Field label={t("Role")}>
              {(id) => (
                <input
                  id={id}
                  name="role"
                  required
                  placeholder={t("Research specialist")}
                />
              )}
            </Field>
            <Field label={t("Mission")}>
              {(id) => <textarea id={id} name="mission" required rows={3} />}
            </Field>
            <Field
              label={t("Instructions")}
              hint={t("One instruction per line.")}
            >
              {(id) => <textarea id={id} name="instructions" rows={3} />}
            </Field>
            <Field label={t("Expertise")} hint={t("One area per line.")}>
              {(id) => <textarea id={id} name="expertise" rows={2} />}
            </Field>
            <details>
              <summary>{t("Advanced")}</summary>
              <label className="check">
                <input type="checkbox" name="allowed" />{" "}
                {t("Can delegate")}{" "}
              </label>
              <Field
                label={t("Maximum direct delegates")}
                hint={t(
                  "Leave empty for no direct-child limit; policy still controls whether delegation is allowed.",
                )}
              >
                {(id) => (
                  <input
                    id={id}
                    type="number"
                    name="max_children"
                    min="0"
                    max="4294967295"
                    step="1"
                  />
                )}
              </Field>
              <p className="muted">
                {" "}
                {t(
                  "This creates revision 1. Historical definitions are preserved.",
                )}{" "}
              </p>
            </details>
          </>
        )}
        {form.kind === "session" && (
          <>
            <Field label={t("Label")}>
              {(id) => (
                <input
                  id={id}
                  name="label"
                  required
                  placeholder={t("Research conversation")}
                />
              )}
            </Field>
            <Field label={t("Resource")}>
              {(id) => (
                <select id={id} name="resource" required defaultValue="">
                  <option value="" disabled>
                    {" "}
                    {t("Choose a resource")}{" "}
                  </option>
                  {snapshot.resources.map((resource) => (
                    <option key={resource.id} value={resource.id}>
                      {resource.name} · {resource.provider}
                    </option>
                  ))}
                </select>
              )}
            </Field>
            <Field label={t("Agent")}>
              {(id) => (
                <select id={id} name="agent" required defaultValue="">
                  <option value="" disabled>
                    {" "}
                    {t("Choose an exact agent revision")}{" "}
                  </option>
                  {snapshot.agents.map((agent, index) => (
                    <option key={`${agent.id}@${agent.revision}`} value={index}>
                      {agent.name} · {agent.role} {t("· revision")}{" "}
                      {agent.revision}
                    </option>
                  ))}
                </select>
              )}
            </Field>
            <Field
              label={t("External reference (optional)")}
              hint={t(
                "A chat URL or an opaque label. No provider login is required.",
              )}
            >
              {(id) => <input id={id} name="external_ref" />}
            </Field>
            {(!snapshot.resources.length || !snapshot.agents.length) && (
              <p className="muted">
                {" "}
                {t(
                  "Create a resource and an agent before creating a session.",
                )}{" "}
              </p>
            )}
          </>
        )}
        {form.kind === "task" && (
          <>
            <Field label={t("Title")}>
              {(id) => (
                <input
                  id={id}
                  name="title"
                  required
                  placeholder={t("Research transportation")}
                />
              )}
            </Field>
            <Field label={t("Objective")}>
              {(id) => <textarea id={id} name="objective" required rows={4} />}
            </Field>
            <details>
              <summary>{t("Advanced")}</summary>
              <Field label={t("Parent work")}>
                {(id) => (
                  <select id={id} name="parent">
                    <option value="">{t("No parent")}</option>
                    {snapshot.tasks.map((task) => (
                      <option key={task.id} value={task.id}>
                        {task.title}
                      </option>
                    ))}
                  </select>
                )}
              </Field>
              <fieldset>
                <legend>{t("Dependencies")}</legend>
                {snapshot.tasks.length ? (
                  snapshot.tasks.map((task) => (
                    <label className="check" key={task.id}>
                      <input
                        type="checkbox"
                        name="dependency"
                        value={task.id}
                      />
                      {task.title}
                    </label>
                  ))
                ) : (
                  <p className="muted">{t("No existing work to depend on.")}</p>
                )}
              </fieldset>
            </details>
          </>
        )}
        {form.kind === "assignment" && (
          <>
            <Field label={t("Work")}>
              {(id) => (
                <select
                  id={id}
                  name="task"
                  required
                  defaultValue={form.task_id ?? ""}
                >
                  <option value="" disabled>
                    {" "}
                    {t("Choose work")}{" "}
                  </option>
                  {snapshot.tasks.map((task) => (
                    <option key={task.id} value={task.id}>
                      {task.title}
                    </option>
                  ))}
                </select>
              )}
            </Field>
            <Field label={t("Session")}>
              {(id) => (
                <select id={id} name="session" required defaultValue="">
                  <option value="" disabled>
                    {" "}
                    {t("Choose a session")}{" "}
                  </option>
                  {snapshot.sessions.map((session) => (
                    <option key={session.id} value={session.id}>
                      {session.label} · {stateLabel(session.status, t)}
                    </option>
                  ))}
                </select>
              )}
            </Field>
            <p className="muted">
              {" "}
              {t(
                "This assignment preserves the session's exact agent revision. Resource selection remains explicit.",
              )}{" "}
            </p>
          </>
        )}
        {form.kind === "delegation" && (
          <>
            <Field label={t("From assigned work")}>
              {(id) => (
                <select id={id} name="from" required defaultValue="">
                  <option value="" disabled>
                    {" "}
                    {t("Choose a parent assignment")}{" "}
                  </option>
                  {snapshot.assignments.map((assignment) => (
                    <option key={assignment.id} value={assignment.id}>
                      {assignmentName(snapshot, assignment, t)}
                    </option>
                  ))}
                </select>
              )}
            </Field>
            <Field label={t("To assigned work")}>
              {(id) => (
                <select id={id} name="to" required defaultValue="">
                  <option value="" disabled>
                    {" "}
                    {t("Choose a child assignment")}{" "}
                  </option>
                  {snapshot.assignments.map((assignment) => (
                    <option key={assignment.id} value={assignment.id}>
                      {assignmentName(snapshot, assignment, t)}
                    </option>
                  ))}
                </select>
              )}
            </Field>
            <p className="muted">
              {" "}
              {t(
                "The parent agent's stored delegation policy is enforced. Work dependencies remain a separate graph.",
              )}{" "}
            </p>
          </>
        )}
        <footer>
          <button type="button" disabled={busy} onClick={() => showForm(null)}>
            {" "}
            {t("Cancel")}{" "}
          </button>
          <button className="primary" disabled={busy} type="submit">
            {busy
              ? t("Saving…")
              : form.kind === "assignment"
                ? t("Assign work")
                : form.kind === "delegation"
                  ? t("Record delegation")
                  : t("Create")}
          </button>
        </footer>
      </form>
    </dialog>
  );
}
function EnumField({
  name,
  label,
  values,
  defaultValue,
}: {
  name: string;
  label: string;
  values: string[];
  defaultValue: string;
}) {
  const { t } = useI18n();
  return (
    <Field label={label}>
      {(id) => (
        <select id={id} name={name} defaultValue={defaultValue}>
          {values.map((value) => (
            <option key={value} value={value}>
              {stateLabel(value, t)}
            </option>
          ))}
        </select>
      )}
    </Field>
  );
}
