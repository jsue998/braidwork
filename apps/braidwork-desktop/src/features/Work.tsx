import { useState } from "react";
import { Plus, GitBranch } from "lucide-react";
import { useWorkspace } from "../app/context";
import { agentFor, assignmentLabel } from "../lib/derive";
import { Badge, Empty, PageHeading } from "../components/Common";
import { DelegationGraph } from "./DelegationGraph";
export function Work() {
  const [tab, setTab] = useState<"tasks" | "assignments" | "delegations">(
    "tasks",
  );
  const { snapshot, select, showForm } = useWorkspace();
  if (!snapshot) return null;
  return (
    <>
      <PageHeading
        title="Work"
        subtitle="Define the goal, assign it to a session, and track the handoff."
        action={
          <button
            className="primary"
            onClick={() => showForm({ kind: "task" })}
          >
            <Plus size={16} /> Create work
          </button>
        }
      />
      <nav className="tabs" aria-label="Work views">
        {(["tasks", "assignments", "delegations"] as const).map((value) => (
          <button
            key={value}
            aria-current={tab === value ? "page" : undefined}
            onClick={() => setTab(value)}
          >
            {value === "tasks"
              ? "Tasks"
              : value === "assignments"
                ? "Assigned work"
                : "Delegations"}
          </button>
        ))}
      </nav>
      {tab === "tasks" &&
        (snapshot.tasks.length ? (
          <div className="row-list">
            {snapshot.tasks.map((task) => (
              <button
                className="entity-row"
                key={task.id}
                onClick={() => select({ kind: "task", id: task.id })}
              >
                <span className="grow">
                  <strong>{task.title}</strong>
                  <small className="ellipsis">{task.objective}</small>
                  <small>
                    {
                      snapshot.assignments.filter((a) => a.task_id === task.id)
                        .length
                    }{" "}
                    assignments{task.parent ? " · Has parent" : ""}
                    {task.dependencies.length
                      ? ` · ${task.dependencies.length} prerequisites`
                      : ""}
                  </small>
                </span>
                <Badge state={task.status} />
              </button>
            ))}
          </div>
        ) : (
          <Empty
            title="No work yet."
            action={
              <button onClick={() => showForm({ kind: "task" })}>
                Create work
              </button>
            }
          >
            Begin with a clear title and objective.
          </Empty>
        ))}
      {tab === "assignments" && (
        <>
          <div className="toolbar">
            <span className="muted">
              Each assignment captures one session and agent revision.
            </span>
            <button onClick={() => showForm({ kind: "assignment" })}>
              Assign to AI
            </button>
          </div>
          {snapshot.assignments.length ? (
            <div className="table-wrap">
              <table>
                <thead>
                  <tr>
                    <th>Work</th>
                    <th>Session</th>
                    <th>Agent</th>
                    <th>State</th>
                  </tr>
                </thead>
                <tbody>
                  {snapshot.assignments.map((assignment) => (
                    <tr key={assignment.id}>
                      <td>
                        <button
                          className="text-button"
                          onClick={() =>
                            select({ kind: "assignment", id: assignment.id })
                          }
                        >
                          {snapshot.tasks.find(
                            (t) => t.id === assignment.task_id,
                          )?.title ?? "Missing task"}
                        </button>
                      </td>
                      <td>
                        {snapshot.sessions.find(
                          (s) => s.id === assignment.session_id,
                        )?.label ?? "Missing session"}
                      </td>
                      <td>
                        {agentFor(snapshot, assignment)?.name ??
                          "Missing agent"}
                      </td>
                      <td>{assignmentLabel(assignment.status)}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          ) : (
            <Empty title="No assigned work yet.">
              Choose a task and assign it to an existing session.
            </Empty>
          )}
        </>
      )}
      {tab === "delegations" && (
        <>
          <div className="toolbar">
            <span className="muted">
              {snapshot.delegations.length} recorded delegations
            </span>
            <button onClick={() => showForm({ kind: "delegation" })}>
              <GitBranch size={16} /> Delegate work
            </button>
          </div>
          <DelegationGraph />
        </>
      )}
    </>
  );
}
