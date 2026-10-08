import { english, type Translator, type MessageKey } from "../i18n";
import type {
  Assignment,
  AssignmentStatus,
  Snapshot,
  Usage,
} from "../types/ipc";
const statusKeys: Record<string, MessageKey> = {
  pending: "Pending",
  in_progress: "In progress",
  completed: "Completed",
  prepared: "Prepared",
  dispatched: "Dispatched",
  result_received: "Result received",
  ready: "Ready",
  busy: "Busy",
  dormant: "Dormant",
  manual: "Manual",
  api: "Api",
  harness: "Harness",
  local: "Local",
  abundant: "Abundant",
  normal: "Normal",
  scarce: "Scarce",
  critical: "Critical",
  available: "Available",
  unavailable: "Unavailable",
  exhausted: "Exhausted",
  text: "Text",
  analysis: "Analysis",
  finding: "Finding",
  documentation: "Documentation",
  question: "Question",
  data: "Data",
  patch: "Patch",
  test_result: "Test result",
  not_performed: "Not performed",
  accepted: "Accepted",
  rejected: "Rejected",
  failed: "Failed",
};
export function stateLabel(state: string, t: Translator = english): string {
  const key = statusKeys[state];
  return key ? t(key) : state;
}
export function assignmentLabel(
  state: AssignmentStatus,
  t: Translator = english,
): string {
  return t(
    {
      prepared: "Assigned",
      dispatched: "Dispatched",
      result_received: "Result received",
    }[state] as MessageKey,
  );
}
export function completion(snapshot: Pick<Snapshot, "tasks">): {
  completed: number;
  total: number;
  percent: number | null;
} {
  const total = snapshot.tasks.length;
  const completed = snapshot.tasks.filter(
    (task) => task.status === "completed",
  ).length;
  return {
    total,
    completed,
    percent: total === 0 ? null : Math.round((completed / total) * 100),
  };
}
export function sessionsByResource(
  snapshot: Pick<Snapshot, "resources" | "sessions">,
) {
  return snapshot.resources.map((resource) => ({
    resource,
    sessions: snapshot.sessions.filter(
      (session) => session.resource_id === resource.id,
    ),
  }));
}
export function agentFor(snapshot: Snapshot, assignment: Assignment) {
  return snapshot.agents.find(
    (agent) =>
      agent.id === assignment.agent_spec_id &&
      agent.revision === assignment.agent_spec_revision,
  );
}
export function assignmentName(
  snapshot: Snapshot,
  assignment: Assignment,
  t: Translator = english,
) {
  const task = snapshot.tasks.find((task) => task.id === assignment.task_id);
  const session = snapshot.sessions.find(
    (session) => session.id === assignment.session_id,
  );
  return `${task?.title ?? t("Missing task")} · ${session?.label ?? t("Missing session")}`;
}
export function attention(snapshot: Snapshot, t: Translator = english) {
  return snapshot.assignments.map((assignment) => {
    const workflow = snapshot.workflow.find(
      (link) => link.assignment_id === assignment.id,
    );
    const reason =
      assignment.status === "result_received"
        ? workflow?.result?.receipt.verification.decision === "accepted"
          ? t("Result received — verification accepted")
          : workflow?.result?.receipt.verification.decision === "rejected"
            ? t("Result received — verification rejected")
            : t("Result received — review not performed")
        : assignment.status === "dispatched"
          ? t("Waiting for a result to be imported")
          : workflow?.capsule_id
            ? t("Instructions ready — dispatch not recorded")
            : t("Prepare instructions");
    return { assignment, reason };
  });
}
export function delegationGraph(snapshot: Snapshot, t: Translator = english) {
  // Deterministic grid, deliberately independent of graph topology (cycles are valid data).
  const nodes = [...snapshot.assignments]
    .sort((a, b) => (a.id < b.id ? -1 : a.id > b.id ? 1 : 0))
    .map((assignment, index) => ({
      id: assignment.id,
      type: "assignment" as const,
      ariaLabel: assignmentName(snapshot, assignment, t),
      position: { x: (index % 3) * 310, y: Math.floor(index / 3) * 180 },
      data: {
        title:
          snapshot.tasks.find((task) => task.id === assignment.task_id)
            ?.title ?? t("Missing task"),
        role: agentFor(snapshot, assignment)?.role ?? t("Missing agent"),
        session:
          snapshot.sessions.find(
            (session) => session.id === assignment.session_id,
          )?.label ?? t("Missing session"),
        status: assignmentLabel(assignment.status, t),
      },
    }));
  const edges = snapshot.delegations.map((edge) => ({
    id: edge.id,
    source: edge.parent_assignment,
    target: edge.child_assignment,
    label: t("Delegated"),
    type: "smoothstep",
    markerEnd: { type: "arrowclosed" as const },
  }));
  return { nodes, edges };
}
export const modelLabel = (model: string | null, t: Translator = english) =>
  model ?? t("Unknown model");
export function usageLabel(usage: Usage, t: Translator = english) {
  const tokens = t("Input: {input} · Output: {output} tokens", {
    input: usage.input_tokens ?? t("unknown"),
    output: usage.output_tokens ?? t("unknown"),
  });
  const cost = usage.cost
    ? t("{amount} micros {currency}", {
        amount: usage.cost.amount_micros,
        currency: usage.cost.currency,
      })
    : t("Cost: unknown");
  return `${tokens} · ${cost}`;
}
export const lines = (text: string) =>
  text
    .split("\n")
    .map((line) => line.trim())
    .filter(Boolean);
