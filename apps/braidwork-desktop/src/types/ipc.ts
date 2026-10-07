// IPC is snake_case. All Rust u64 observations are lossless decimal strings.
export type AccessMode = "manual" | "api" | "harness" | "local";
export type Scarcity = "abundant" | "normal" | "scarce" | "critical";
export type ResourceStatus = "available" | "unavailable" | "exhausted";
export type TaskStatus = "pending" | "in_progress" | "completed";
export type AssignmentStatus = "prepared" | "dispatched" | "result_received";
export type ArtifactKind =
  | "patch"
  | "analysis"
  | "finding"
  | "test_result"
  | "documentation"
  | "question"
  | "text"
  | "data";
export interface ProjectInfo {
  name: string;
  root: string;
  format_version: number;
  schema_version: number;
}
export interface Resource {
  id: string;
  name: string;
  provider: string;
  access_mode: AccessMode;
  scarcity: Scarcity;
  status: ResourceStatus;
}
export interface Agent {
  id: string;
  revision: number;
  name: string;
  role: string;
  mission: string;
  instructions: string[];
  expertise: string[];
  delegation: { allowed: boolean; max_children: number | null };
}
export interface Session {
  id: string;
  label: string;
  resource_id: string;
  agent_spec_id: string;
  agent_spec_revision: number;
  external_ref: string | null;
  status: "ready" | "busy" | "dormant";
}
export interface Task {
  id: string;
  title: string;
  objective: string;
  parent: string | null;
  dependencies: string[];
  status: TaskStatus;
}
export interface Assignment {
  id: string;
  task_id: string;
  session_id: string;
  agent_spec_id: string;
  agent_spec_revision: number;
  status: AssignmentStatus;
}
export interface Delegation {
  id: string;
  parent_assignment: string;
  child_assignment: string;
}
export type Execution =
  | { outcome: "completed" }
  | { outcome: "failed"; reason: string };
export type Verification =
  | { decision: "not_performed" }
  | { decision: "accepted" | "rejected"; summary: string; evidence: string[] };
export interface Usage {
  input_tokens: string | null;
  output_tokens: string | null;
  cost: { amount_micros: string; currency: string } | null;
}
export interface Receipt {
  id: string;
  task_id: string;
  capsule_id: string;
  resource_id: string;
  model_id: string | null;
  access_mode: AccessMode;
  artifacts: string[];
  execution: Execution;
  verification: Verification;
  usage: Usage;
}
export interface Artifact {
  id: string;
  kind: ArtifactKind;
  media_type: string | null;
  size_bytes: string | null;
}
export interface ResultSummary {
  receipt: Receipt;
  artifacts: Artifact[];
}
export interface Workflow {
  assignment_id: string;
  capsule_id: string | null;
  result: ResultSummary | null;
}
export interface Snapshot {
  project: ProjectInfo;
  resources: Resource[];
  agents: Agent[];
  sessions: Session[];
  tasks: Task[];
  assignments: Assignment[];
  delegations: Delegation[];
  workflow: Workflow[];
}
export interface Instructions {
  capsule_id: string;
  rendered: string;
  max_estimated_tokens: string;
}
export interface AssignmentDetail {
  assignment: Assignment;
  task: Task;
  session: Session;
  agent: Agent;
  resource: Resource;
  external_url: string | null;
  instructions: Instructions | null;
  result: ResultSummary | null;
}
export interface ArtifactPreview {
  artifact: Artifact;
  status: "text" | "too_large" | "binary" | "invalid_utf8";
  text: string | null;
}
export interface ResultDetail {
  receipt: Receipt;
  artifacts: ArtifactPreview[];
}
export interface IpcError {
  code: string;
  message: string;
  detail: string | null;
}
export type ResourceInput = Omit<Resource, "id">;
export type AgentInput = Omit<Agent, "id" | "revision">;
export type SessionInput = Omit<Session, "id" | "status">;
export type TaskInput = Omit<Task, "id" | "status">;
export interface PreparationInput {
  inputs: { kind: "inline"; label: string; text: string }[];
  context_files: string[];
  constraints: string[];
  acceptance_criteria: string[];
  expected_outputs: { kind: ArtifactKind; description: string }[];
  max_context_tokens: string;
}
export interface IngestInput {
  model_id: string | null;
  kind: ArtifactKind | null;
  media_type: string | null;
  input_tokens: string | null;
  output_tokens: string | null;
  cost_micros: string | null;
  currency: string | null;
}
