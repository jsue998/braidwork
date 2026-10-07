import { useWorkspace } from "../app/context";
import { agentFor, modelLabel, stateLabel, usageLabel } from "../lib/derive";
import { Empty, PageHeading } from "../components/Common";
export function Results() {
  const { snapshot, select } = useWorkspace();
  if (!snapshot) return null;
  const results = snapshot.workflow.filter((link) => link.result !== null);
  return (
    <>
      <PageHeading
        title="Results"
        subtitle="Received work and its provenance. Receipt does not imply acceptance."
      />
      {results.length ? (
        <div className="row-list">
          {results.map((link) => {
            const assignment = snapshot.assignments.find(
              (a) => a.id === link.assignment_id,
            );
            const result = link.result;
            if (!result || !assignment) return null;
            return (
              <button
                className="entity-row result-row"
                key={link.assignment_id}
                onClick={() =>
                  select({ kind: "result", id: link.assignment_id })
                }
              >
                <span>
                  <strong>
                    {snapshot.tasks.find((t) => t.id === assignment.task_id)
                      ?.title ?? "Missing task"}
                  </strong>
                  <small>
                    {agentFor(snapshot, assignment)?.name} ·{" "}
                    {
                      snapshot.sessions.find(
                        (s) => s.id === assignment.session_id,
                      )?.label
                    }
                  </small>
                  <small>
                    {modelLabel(result.receipt.model_id)} ·{" "}
                    {result.artifacts.map((a) => stateLabel(a.kind)).join(", ")}
                  </small>
                  <small>{usageLabel(result.receipt.usage)}</small>
                </span>
                <span>
                  Verification:
                  <br />
                  {stateLabel(result.receipt.verification.decision)}
                </span>
              </button>
            );
          })}
        </div>
      ) : (
        <Empty title="No results received yet.">
          Prepare instructions for assigned work, copy them to your external AI,
          then paste or import its result.
        </Empty>
      )}
    </>
  );
}
