import { describe, expect, it } from "vitest";
import {
  agentFor,
  assignmentLabel,
  attention,
  completion,
  delegationGraph,
  modelLabel,
  sessionsByResource,
  usageLabel,
} from "./derive";
import { asError } from "./api";
import type { Snapshot } from "../types/ipc";
function snapshot(): Snapshot {
  return {
    project: {
      name: "Japan Trip",
      root: "project-root",
      format_version: 1,
      schema_version: 2,
    },
    resources: [
      {
        id: "ai-account",
        name: "Primary AI account",
        provider: "Example",
        access_mode: "manual",
        scarcity: "normal",
        status: "available",
      },
    ],
    agents: [
      {
        id: "researcher",
        revision: 1,
        name: "Travel Researcher",
        role: "Research specialist",
        mission: "Investigate travel options",
        instructions: [],
        expertise: [],
        delegation: { allowed: true, max_children: null },
      },
      {
        id: "researcher",
        revision: 2,
        name: "Travel Researcher v2",
        role: "New role",
        mission: "New mission",
        instructions: [],
        expertise: [],
        delegation: { allowed: true, max_children: null },
      },
    ],
    sessions: [
      {
        id: "coordinator-chat",
        label: "Coordinator chat",
        resource_id: "ai-account",
        agent_spec_id: "researcher",
        agent_spec_revision: 1,
        external_ref: null,
        status: "ready",
      },
      {
        id: "research-chat",
        label: "Research chat",
        resource_id: "ai-account",
        agent_spec_id: "researcher",
        agent_spec_revision: 1,
        external_ref: "Travel research chat",
        status: "ready",
      },
    ],
    tasks: [
      {
        id: "rail",
        title: "Research transportation",
        objective: "Compare train options",
        parent: null,
        dependencies: [],
        status: "pending",
      },
      {
        id: "itinerary",
        title: "Build itinerary",
        objective: "Plan the trip",
        parent: null,
        dependencies: ["rail"],
        status: "completed",
      },
    ],
    assignments: [
      {
        id: "a",
        task_id: "rail",
        session_id: "coordinator-chat",
        agent_spec_id: "researcher",
        agent_spec_revision: 1,
        status: "prepared",
      },
      {
        id: "b",
        task_id: "itinerary",
        session_id: "research-chat",
        agent_spec_id: "researcher",
        agent_spec_revision: 1,
        status: "dispatched",
      },
    ],
    delegations: [
      { id: "edge", parent_assignment: "a", child_assignment: "b" },
    ],
    workflow: [
      { assignment_id: "a", capsule_id: null, result: null },
      { assignment_id: "b", capsule_id: "capsule-b", result: null },
    ],
  };
}
describe("canonical workspace presentation", () => {
  it("groups multiple sessions under the same resource without collapsing them", () => {
    const groups = sessionsByResource(snapshot());
    expect(groups).toHaveLength(1);
    expect(groups[0]?.sessions.map((s) => s.id)).toEqual([
      "coordinator-chat",
      "research-chat",
    ]);
  });
  it("computes completion from declared task status only", () => {
    expect(completion(snapshot())).toEqual({
      completed: 1,
      total: 2,
      percent: 50,
    });
  });
  it("does not fabricate a percentage when no tasks exist", () => {
    expect(completion({ tasks: [] })).toEqual({
      completed: 0,
      total: 0,
      percent: null,
    });
  });
  it("does not count result-received assignments as task completion", () => {
    const s = snapshot();
    s.tasks.forEach((t) => {
      t.status = "pending";
    });
    s.assignments.forEach((a) => {
      a.status = "result_received";
    });
    expect(completion(s).completed).toBe(0);
  });
  it("uses explicit assignment labels with no implied acceptance", () => {
    expect(assignmentLabel("prepared")).toBe("Assigned");
    expect(assignmentLabel("dispatched")).toBe("Dispatched");
    expect(assignmentLabel("result_received")).toBe("Result received");
  });
  it("resolves exact historical revision instead of the latest agent", () => {
    const s = snapshot();
    expect(agentFor(s, s.assignments[0]!)).toMatchObject({
      revision: 1,
      role: "Research specialist",
    });
  });
  it("derives needs attention from persistent states and capsule links", () => {
    const s = snapshot();
    expect(attention(s).map((a) => a.reason)).toEqual([
      "Prepare instructions",
      "Waiting for a result to be imported",
    ]);
    s.workflow[0]!.capsule_id = "capsule-a";
    expect(attention(s)[0]?.reason).toBe(
      "Instructions ready — dispatch not recorded",
    );
    s.assignments[0]!.status = "result_received";
    expect(attention(s)[0]?.reason).toBe(
      "Result received — review not performed",
    );
  });
  it("builds graph nodes from assignments and edges only from delegations", () => {
    const s = snapshot();
    const graph = delegationGraph(s);
    expect(graph.nodes.map((n) => n.id)).toEqual(["a", "b"]);
    expect(graph.nodes[0]?.data).toMatchObject({
      title: "Research transportation",
      session: "Coordinator chat",
      role: "Research specialist",
    });
    expect(graph.edges).toMatchObject([
      { id: "edge", source: "a", target: "b" },
    ]);
    s.delegations = [];
    expect(delegationGraph(s).edges).toEqual([]);
  });
  it("represents cycles and uses deterministic grid positions without topological sorting", () => {
    const s = snapshot();
    s.delegations.push({
      id: "cycle",
      parent_assignment: "b",
      child_assignment: "a",
    });
    const first = delegationGraph(s);
    s.assignments.reverse();
    expect(delegationGraph(s)).toEqual(first);
    expect(first.edges).toHaveLength(2);
  });
  it("distinguishes unknown model and usage from observed zero", () => {
    expect(modelLabel(null)).toBe("Unknown model");
    expect(modelLabel("known-model")).toBe("known-model");
    expect(
      usageLabel({ input_tokens: null, output_tokens: null, cost: null }),
    ).toContain("unknown");
    const known = usageLabel({
      input_tokens: "0",
      output_tokens: "18446744073709551615",
      cost: { amount_micros: "0", currency: "JPY" },
    });
    expect(known).toContain("Input: 0");
    expect(known).toContain("18446744073709551615");
    expect(known).toContain("0 micros JPY");
    expect(known).not.toContain("unknown");
  });
  it("retains structured IPC error categories and diagnostics", () => {
    expect(
      asError({
        code: "delegation_denied",
        message: "Not allowed",
        detail: "Policy limit",
      }),
    ).toEqual({
      code: "delegation_denied",
      message: "Not allowed",
      detail: "Policy limit",
    });
    expect(asError("native error").detail).toBe("native error");
  });
});
