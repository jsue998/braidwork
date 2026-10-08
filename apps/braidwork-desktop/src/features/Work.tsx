import { useI18n } from "../i18n/context";
import { useState, lazy, Suspense } from "react";
import { Plus, GitBranch } from "lucide-react";
import { useWorkspace } from "../app/context";
import { agentFor, assignmentLabel } from "../lib/derive";
import { Badge, Empty, PageHeading } from "../components/Common";
const DelegationGraph = lazy(() =>
  import("./DelegationGraph").then((module) => ({
    default: module.DelegationGraph,
  })),
);
export function Work() {
  const { t } = useI18n();
  const [tab, setTab] = useState<"tasks" | "assignments" | "delegations">(
    "tasks",
  );
  const { snapshot, select, showForm } = useWorkspace();
  if (!snapshot) return null;
  return (
    <>
      <PageHeading
        title={t("Work")}
        subtitle={t(
          "Define the goal, assign it to a session, and track the handoff.",
        )}
        action={
          <button
            className="primary"
            onClick={() => showForm({ kind: "task" })}
          >
            <Plus size={16} /> {t("Create work")}{" "}
          </button>
        }
      />
      <nav className="tabs" aria-label={t("Work views")}>
        {(["tasks", "assignments", "delegations"] as const).map((value) => (
          <button
            key={value}
            aria-current={tab === value ? "page" : undefined}
            onClick={() => setTab(value)}
          >
            {value === "tasks"
              ? t("Tasks")
              : value === "assignments"
                ? t("Assigned work")
                : t("Delegations")}
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
                    {t("assignments")}
                    {task.parent ? t(" · Has parent") : ""}
                    {task.dependencies.length
                      ? t(" · {count} prerequisites", {
                          count: task.dependencies.length,
                        })
                      : ""}
                  </small>
                </span>
                <Badge state={task.status} />
              </button>
            ))}
          </div>
        ) : (
          <Empty
            title={t("No work yet.")}
            action={
              <button onClick={() => showForm({ kind: "task" })}>
                {" "}
                {t("Create work")}{" "}
              </button>
            }
          >
            {" "}
            {t("Begin with a clear title and objective.")}{" "}
          </Empty>
        ))}
      {tab === "assignments" && (
        <>
          <div className="toolbar">
            <span className="muted">
              {" "}
              {t(
                "Each assignment captures one session and agent revision.",
              )}{" "}
            </span>
            <button onClick={() => showForm({ kind: "assignment" })}>
              {" "}
              {t("Assign to AI")}{" "}
            </button>
          </div>
          {snapshot.assignments.length ? (
            <div className="table-wrap">
              <table>
                <thead>
                  <tr>
                    <th>{t("Work")}</th>
                    <th>{t("Session")}</th>
                    <th>{t("Agent")}</th>
                    <th>{t("State")}</th>
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
                          )?.title ?? t("Missing task")}
                        </button>
                      </td>
                      <td>
                        {snapshot.sessions.find(
                          (s) => s.id === assignment.session_id,
                        )?.label ?? t("Missing session")}
                      </td>
                      <td>
                        {agentFor(snapshot, assignment)?.name ??
                          t("Missing agent")}
                      </td>
                      <td>{assignmentLabel(assignment.status, t)}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          ) : (
            <Empty title={t("No assigned work yet.")}>
              {" "}
              {t("Choose a task and assign it to an existing session.")}{" "}
            </Empty>
          )}
        </>
      )}
      {tab === "delegations" && (
        <>
          <div className="toolbar">
            <span className="muted">
              {snapshot.delegations.length} {t("recorded delegations")}{" "}
            </span>
            <button onClick={() => showForm({ kind: "delegation" })}>
              <GitBranch size={16} /> {t("Delegate work")}{" "}
            </button>
          </div>
          <Suspense
            fallback={<p role="status">{t("Loading delegation graph…")}</p>}
          >
            <DelegationGraph />
          </Suspense>
        </>
      )}
    </>
  );
}
