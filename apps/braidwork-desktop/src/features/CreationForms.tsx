import { useEffect, useRef, type FormEvent } from "react";
import { X } from "lucide-react";
import { useWorkspace } from "../app/context";
import { api } from "../lib/api";
import { lines, assignmentName, stateLabel } from "../lib/derive";
import type { AccessMode, ResourceStatus, Scarcity } from "../types/ipc";
import { ErrorNotice, Field } from "../components/Common";
const text = (data: FormData, key: string) => String(data.get(key) ?? "");
export function CreationForms() {
  const { snapshot, form, busy, showForm, mutate, select, error, clearError } =
    useWorkspace();
  const dialog = useRef<HTMLDialogElement>(null);
  useEffect(() => {
    const node = dialog.current;
    node?.showModal();
    return () => node?.close();
  }, [form?.kind]);
  if (!snapshot || !form) return null;
  const titles = {
    resource: "Create AI resource",
    agent: "Create agent",
    session: "Create session",
    task: "Create work",
    assignment: "Assign to AI",
    delegation: "Delegate work",
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
          if (!agent) throw new Error("Choose an existing agent revision.");
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
    } catch {
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
          aria-label="Close form"
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
            <Field label="Name">
              {(id) => (
                <input
                  id={id}
                  name="name"
                  required
                  placeholder="Primary AI account"
                />
              )}
            </Field>
            <Field label="Provider">
              {(id) => (
                <input
                  id={id}
                  name="provider"
                  required
                  placeholder="Provider name"
                />
              )}
            </Field>
            <details>
              <summary>Advanced</summary>
              <EnumField
                name="access_mode"
                label="Access mode"
                values={["manual", "api", "harness", "local"]}
                defaultValue="manual"
              />
              <EnumField
                name="scarcity"
                label="Scarcity"
                values={["abundant", "normal", "scarce", "critical"]}
                defaultValue="normal"
              />
              <EnumField
                name="status"
                label="Status"
                values={["available", "unavailable", "exhausted"]}
                defaultValue="available"
              />
            </details>
          </>
        )}
        {form.kind === "agent" && (
          <>
            <Field label="Name">
              {(id) => (
                <input
                  id={id}
                  name="name"
                  required
                  placeholder="Travel Researcher"
                />
              )}
            </Field>
            <Field label="Role">
              {(id) => (
                <input
                  id={id}
                  name="role"
                  required
                  placeholder="Research specialist"
                />
              )}
            </Field>
            <Field label="Mission">
              {(id) => <textarea id={id} name="mission" required rows={3} />}
            </Field>
            <Field label="Instructions" hint="One instruction per line.">
              {(id) => <textarea id={id} name="instructions" rows={3} />}
            </Field>
            <Field label="Expertise" hint="One area per line.">
              {(id) => <textarea id={id} name="expertise" rows={2} />}
            </Field>
            <details>
              <summary>Advanced</summary>
              <label className="check">
                <input type="checkbox" name="allowed" /> Can delegate
              </label>
              <Field
                label="Maximum direct delegates"
                hint="Leave empty for no direct-child limit; policy still controls whether delegation is allowed."
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
                This creates revision 1. Historical definitions are preserved.
              </p>
            </details>
          </>
        )}
        {form.kind === "session" && (
          <>
            <Field label="Label">
              {(id) => (
                <input
                  id={id}
                  name="label"
                  required
                  placeholder="Research conversation"
                />
              )}
            </Field>
            <Field label="Resource">
              {(id) => (
                <select id={id} name="resource" required defaultValue="">
                  <option value="" disabled>
                    Choose a resource
                  </option>
                  {snapshot.resources.map((resource) => (
                    <option key={resource.id} value={resource.id}>
                      {resource.name} · {resource.provider}
                    </option>
                  ))}
                </select>
              )}
            </Field>
            <Field label="Agent">
              {(id) => (
                <select id={id} name="agent" required defaultValue="">
                  <option value="" disabled>
                    Choose an exact agent revision
                  </option>
                  {snapshot.agents.map((agent, index) => (
                    <option key={`${agent.id}@${agent.revision}`} value={index}>
                      {agent.name} · {agent.role} · revision {agent.revision}
                    </option>
                  ))}
                </select>
              )}
            </Field>
            <Field
              label="External reference (optional)"
              hint="A chat URL or an opaque label. No provider login is required."
            >
              {(id) => <input id={id} name="external_ref" />}
            </Field>
            {(!snapshot.resources.length || !snapshot.agents.length) && (
              <p className="muted">
                Create a resource and an agent before creating a session.
              </p>
            )}
          </>
        )}
        {form.kind === "task" && (
          <>
            <Field label="Title">
              {(id) => (
                <input
                  id={id}
                  name="title"
                  required
                  placeholder="Research transportation"
                />
              )}
            </Field>
            <Field label="Objective">
              {(id) => <textarea id={id} name="objective" required rows={4} />}
            </Field>
            <details>
              <summary>Advanced</summary>
              <Field label="Parent work">
                {(id) => (
                  <select id={id} name="parent">
                    <option value="">No parent</option>
                    {snapshot.tasks.map((task) => (
                      <option key={task.id} value={task.id}>
                        {task.title}
                      </option>
                    ))}
                  </select>
                )}
              </Field>
              <fieldset>
                <legend>Dependencies</legend>
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
                  <p className="muted">No existing work to depend on.</p>
                )}
              </fieldset>
            </details>
          </>
        )}
        {form.kind === "assignment" && (
          <>
            <Field label="Work">
              {(id) => (
                <select
                  id={id}
                  name="task"
                  required
                  defaultValue={form.task_id ?? ""}
                >
                  <option value="" disabled>
                    Choose work
                  </option>
                  {snapshot.tasks.map((task) => (
                    <option key={task.id} value={task.id}>
                      {task.title}
                    </option>
                  ))}
                </select>
              )}
            </Field>
            <Field label="Session">
              {(id) => (
                <select id={id} name="session" required defaultValue="">
                  <option value="" disabled>
                    Choose a session
                  </option>
                  {snapshot.sessions.map((session) => (
                    <option key={session.id} value={session.id}>
                      {session.label} · {stateLabel(session.status)}
                    </option>
                  ))}
                </select>
              )}
            </Field>
            <p className="muted">
              This assignment preserves the session's exact agent revision.
              Resource selection remains explicit.
            </p>
          </>
        )}
        {form.kind === "delegation" && (
          <>
            <Field label="From assigned work">
              {(id) => (
                <select id={id} name="from" required defaultValue="">
                  <option value="" disabled>
                    Choose a parent assignment
                  </option>
                  {snapshot.assignments.map((assignment) => (
                    <option key={assignment.id} value={assignment.id}>
                      {assignmentName(snapshot, assignment)}
                    </option>
                  ))}
                </select>
              )}
            </Field>
            <Field label="To assigned work">
              {(id) => (
                <select id={id} name="to" required defaultValue="">
                  <option value="" disabled>
                    Choose a child assignment
                  </option>
                  {snapshot.assignments.map((assignment) => (
                    <option key={assignment.id} value={assignment.id}>
                      {assignmentName(snapshot, assignment)}
                    </option>
                  ))}
                </select>
              )}
            </Field>
            <p className="muted">
              The parent agent's stored delegation policy is enforced. Work
              dependencies remain a separate graph.
            </p>
          </>
        )}
        <footer>
          <button type="button" disabled={busy} onClick={() => showForm(null)}>
            Cancel
          </button>
          <button className="primary" disabled={busy} type="submit">
            {busy
              ? "Saving…"
              : form.kind === "assignment"
                ? "Assign work"
                : form.kind === "delegation"
                  ? "Record delegation"
                  : "Create"}
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
  return (
    <Field label={label}>
      {(id) => (
        <select id={id} name={name} defaultValue={defaultValue}>
          {values.map((value) => (
            <option key={value} value={value}>
              {stateLabel(value)}
            </option>
          ))}
        </select>
      )}
    </Field>
  );
}
