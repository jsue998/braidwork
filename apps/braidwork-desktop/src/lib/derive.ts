import type {
  Assignment,
  AssignmentStatus,
  Snapshot,
  Usage,
} from "../types/ipc";
export const stateLabel = (state: string) =>
  state.replaceAll("_", " ").replace(/^./, (char) => char.toUpperCase());
export function assignmentLabel(state: AssignmentStatus): string {
  return {
    prepared: "Assigned",
    dispatched: "Dispatched",
    result_received: "Result received",
  }[state];
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
export function assignmentName(snapshot: Snapshot, assignment: Assignment) {
  const task = snapshot.tasks.find((task) => task.id === assignment.task_id);
  const session = snapshot.sessions.find(
    (session) => session.id === assignment.session_id,
  );
  return `${task?.title ?? "Missing task"} · ${session?.label ?? "Missing session"}`;
}
export function attention(snapshot: Snapshot) {
  return snapshot.assignments.map((assignment) => {
    const workflow = snapshot.workflow.find(
      (link) => link.assignment_id === assignment.id,
    );
    const reason =
      assignment.status === "result_received"
        ? workflow?.result?.receipt.verification.decision === "accepted"
          ? "Result received — verification accepted"
          : workflow?.result?.receipt.verification.decision === "rejected"
            ? "Result received — verification rejected"
            : "Result received — review not performed"
        : assignment.status === "dispatched"
          ? "Waiting for a result to be imported"
          : workflow?.capsule_id
            ? "Instructions ready — dispatch not recorded"
            : "Prepare instructions";
    return { assignment, reason };
  });
}
export function delegationGraph(snapshot: Snapshot) {
  // Deterministic grid, deliberately independent of graph topology (cycles are valid data).
  const nodes = [...snapshot.assignments]
    .sort((a, b) => (a.id < b.id ? -1 : a.id > b.id ? 1 : 0))
    .map((assignment, index) => ({
      id: assignment.id,
      type: "assignment" as const,
      ariaLabel: assignmentName(snapshot, assignment),
      position: { x: (index % 3) * 310, y: Math.floor(index / 3) * 180 },
      data: {
        title:
          snapshot.tasks.find((task) => task.id === assignment.task_id)
            ?.title ?? "Missing task",
        role: agentFor(snapshot, assignment)?.role ?? "Missing agent",
        session:
          snapshot.sessions.find(
            (session) => session.id === assignment.session_id,
          )?.label ?? "Missing session",
        status: assignmentLabel(assignment.status),
      },
    }));
  const edges = snapshot.delegations.map((edge) => ({
    id: edge.id,
    source: edge.parent_assignment,
    target: edge.child_assignment,
    label: "Delegated",
    type: "smoothstep",
    markerEnd: { type: "arrowclosed" as const },
  }));
  return { nodes, edges };
}
export const modelLabel = (model: string | null) => model ?? "Unknown model";
export function usageLabel(usage: Usage) {
  const tokens = `Input: ${usage.input_tokens ?? "unknown"} · Output: ${usage.output_tokens ?? "unknown"} tokens`;
  const cost = usage.cost
    ? `${usage.cost.amount_micros} micros ${usage.cost.currency}`
    : "Cost: unknown";
  return `${tokens} · ${cost}`;
}
export const lines = (text: string) =>
  text
    .split("\n")
    .map((line) => line.trim())
    .filter(Boolean);
