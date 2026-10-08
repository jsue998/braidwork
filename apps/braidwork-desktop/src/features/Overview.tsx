import { useI18n } from "../i18n/context";
import { ArrowRight, Plus } from "lucide-react";
import { useWorkspace } from "../app/context";
import { assignmentName, attention, completion } from "../lib/derive";
import { Empty, PageHeading, Technical } from "../components/Common";
export function Overview() {
  const { t } = useI18n();
  const { snapshot, select, showForm } = useWorkspace();
  if (!snapshot) return null;
  const progress = completion(snapshot);
  const needs = attention(snapshot, t);
  return (
    <>
      <PageHeading
        title={snapshot.project.name}
        subtitle={t("Organize your team. Direct the work. Keep the results.")}
        action={
          <button
            className="primary"
            onClick={() => showForm({ kind: "task" })}
          >
            <Plus size={16} /> {t("Create work")}{" "}
          </button>
        }
      />
      <section className="metrics" aria-label={t("Real project counts")}>
        {[
          [t("Tasks"), snapshot.tasks.length],
          [t("Sessions"), snapshot.sessions.length],
          [t("Assigned work"), snapshot.assignments.length],
          [
            t("Results received"),
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
          {" "}
          {t("Tasks:")}{" "}
          {snapshot.tasks.filter((t) => t.status === "pending").length}{" "}
          {t("pending ·")}{" "}
          {snapshot.tasks.filter((t) => t.status === "in_progress").length}{" "}
          {t("in progress ·")} {progress.completed} {t("completed")}{" "}
        </p>
        <p>
          {" "}
          {t("Assigned work:")}{" "}
          {snapshot.assignments.filter((a) => a.status === "prepared").length}{" "}
          {t("assigned ·")}{" "}
          {snapshot.assignments.filter((a) => a.status === "dispatched").length}{" "}
          {t("dispatched ·")}{" "}
          {
            snapshot.assignments.filter((a) => a.status === "result_received")
              .length
          }{" "}
          {t("results received")}{" "}
        </p>
      </div>
      <section className="section">
        <div className="section-heading">
          <h2>{t("Task completion")}</h2>
          <span>
            {progress.percent === null
              ? t("No tasks yet")
              : t("{completed} / {total} tasks declared completed", {
                  completed: progress.completed,
                  total: progress.total,
                })}
          </span>
        </div>
        {progress.percent !== null ? (
          <>
            <progress
              value={progress.completed}
              max={progress.total}
              aria-label={t("User-declared task completion")}
            />
            <p className="muted">
              {progress.percent}
              {t(
                "% of tasks declared completed by you. Result receipt does not complete a task.",
              )}{" "}
            </p>
          </>
        ) : (
          <p className="muted">
            {" "}
            {t("Create your first task to begin organizing the project.")}{" "}
          </p>
        )}
      </section>
      <section className="section">
        <div className="section-heading">
          <h2>{t("Needs attention")}</h2>
          <span>{t("Derived from recorded workflow state")}</span>
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
                  <strong>{assignmentName(snapshot, assignment, t)}</strong>
                  <small>{reason}</small>
                </span>
                <ArrowRight size={17} />
              </button>
            ))}
          </div>
        ) : (
          <Empty title={t("Your workspace is ready")}>
            {" "}
            {t(
              "Create agents and sessions, then assign work to your team.",
            )}{" "}
          </Empty>
        )}
      </section>
      <Technical>
        <p className="footnote">
          {" "}
          {t("Project format")} {snapshot.project.format_version}{" "}
          {t("· Store schema")} {snapshot.project.schema_version}{" "}
          {t("· Local canonical state")}{" "}
        </p>
      </Technical>
    </>
  );
}
