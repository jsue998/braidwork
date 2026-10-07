import { ArrowRight, Plus } from "lucide-react";
import { useWorkspace } from "../app/context";
import { assignmentName, attention, completion } from "../lib/derive";
import { Empty, PageHeading, Technical } from "../components/Common";
export function Overview() {
  const { snapshot, select, showForm } = useWorkspace();
  if (!snapshot) return null;
  const progress = completion(snapshot);
  const needs = attention(snapshot);
  return (
    <>
      <PageHeading
        title={snapshot.project.name}
        subtitle="Organize your team. Direct the work. Keep the results."
        action={
          <button
            className="primary"
            onClick={() => showForm({ kind: "task" })}
          >
            <Plus size={16} /> Create work
          </button>
        }
      />
      <section className="metrics" aria-label="Real project counts">
        {[
          ["Tasks", snapshot.tasks.length],
          ["Sessions", snapshot.sessions.length],
          ["Assigned work", snapshot.assignments.length],
          [
            "Results received",
            snapshot.workflow.filter((w) => w.result).length,
          ],
        ].map(([label, value]) => (
          <div key={label}>
            <strong>{value}</strong>
            <span>{label}</span>
          </div>
        ))}
      </section>
      <div className="status-counts">
        <p>
          Tasks: {snapshot.tasks.filter((t) => t.status === "pending").length}{" "}
          pending ·{" "}
          {snapshot.tasks.filter((t) => t.status === "in_progress").length} in
          progress · {progress.completed} completed
        </p>
        <p>
          Assigned work:{" "}
          {snapshot.assignments.filter((a) => a.status === "prepared").length}{" "}
          assigned ·{" "}
          {snapshot.assignments.filter((a) => a.status === "dispatched").length}{" "}
          dispatched ·{" "}
          {
            snapshot.assignments.filter((a) => a.status === "result_received")
              .length
          }{" "}
          results received
        </p>
      </div>
      <section className="section">
        <div className="section-heading">
          <h2>Task completion</h2>
          <span>
            {progress.percent === null
              ? "No tasks yet"
              : `${progress.completed} / ${progress.total} tasks declared completed`}
          </span>
        </div>
        {progress.percent !== null ? (
          <>
            <progress
              value={progress.completed}
              max={progress.total}
              aria-label="User-declared task completion"
            />
            <p className="muted">
              {progress.percent}% of tasks declared completed by you. Result
              receipt does not complete a task.
            </p>
          </>
        ) : (
          <p className="muted">
            Create your first task to begin organizing the project.
          </p>
        )}
      </section>
      <section className="section">
        <div className="section-heading">
          <h2>Needs attention</h2>
          <span>Derived from recorded workflow state</span>
        </div>
        {needs.length ? (
          <div className="attention-list">
            {needs.map(({ assignment, reason }) => (
              <button
                className="attention-row"
                key={assignment.id}
                onClick={() =>
                  select({ kind: "assignment", id: assignment.id })
                }
              >
                <span>
                  <strong>{assignmentName(snapshot, assignment)}</strong>
                  <small>{reason}</small>
                </span>
                <ArrowRight size={17} />
              </button>
            ))}
          </div>
        ) : (
          <Empty title="Your workspace is ready">
            Create agents and sessions, then assign work to your team.
          </Empty>
        )}
      </section>
      <Technical>
        <p className="footnote">
          Project format {snapshot.project.format_version} · Store schema{" "}
          {snapshot.project.schema_version} · Local canonical state
        </p>
      </Technical>
    </>
  );
}
