import { invoke } from "@tauri-apps/api/core";
import type {
  Agent,
  AgentInput,
  Assignment,
  AssignmentDetail,
  Delegation,
  IngestInput,
  Instructions,
  PreparationInput,
  ProjectInfo,
  Resource,
  ResourceInput,
  ResultDetail,
  ResultSummary,
  Session,
  SessionInput,
  Snapshot,
  Task,
  TaskInput,
  TaskStatus,
  IpcError,
} from "../types/ipc";
export function asError(error: unknown): IpcError {
  if (
    typeof error === "object" &&
    error !== null &&
    "message" in error &&
    typeof error.message === "string"
  ) {
    return {
      code:
        "code" in error && typeof error.code === "string"
          ? error.code
          : "operation_failed",
      message: error.message,
      detail:
        "detail" in error && typeof error.detail === "string"
          ? error.detail
          : null,
    };
  }
  return {
    code: "operation_failed",
    message: "The operation could not be completed.",
    detail: typeof error === "string" ? error : null,
  };
}
async function call<T>(
  command: string,
  args?: Record<string, unknown>,
): Promise<T> {
  try {
    return await invoke<T>(command, args);
  } catch (error) {
    throw asError(error);
  }
}
export const api = {
  open: (root: string) => call<ProjectInfo>("open_project", { root }),
  init: (root: string, name: string) =>
    call<ProjectInfo>("init_project", { root, name }),
  close: () => call<void>("close_project"),
  snapshot: () => call<Snapshot>("workspace_snapshot"),
  resource: (input: ResourceInput) =>
    call<Resource>("create_resource", { input }),
  agent: (input: AgentInput) => call<Agent>("create_agent", { input }),
  session: (input: SessionInput) => call<Session>("create_session", { input }),
  task: (input: TaskInput) => call<Task>("create_task", { input }),
  status: (id: string, status: TaskStatus) =>
    call<Task>("set_task_status", { id, status }),
  assignment: (task_id: string, session_id: string) =>
    call<Assignment>("create_assignment", { task_id, session_id }),
  delegation: (from: string, to: string) =>
    call<Delegation>("create_delegation", { from, to }),
  prepare: (assignment_id: string, input: PreparationInput) =>
    call<Instructions>("prepare_capsule", { assignment_id, input }),
  render: (capsule_id: string) =>
    call<Instructions>("render_capsule", { capsule_id }),
  dispatch: (assignment_id: string) =>
    call<Assignment>("mark_dispatched", { assignment_id }),
  ingestText: (assignment_id: string, text: string, input: IngestInput) =>
    call<ResultSummary>("ingest_text", { assignment_id, text, input }),
  ingestFile: (assignment_id: string, path: string, input: IngestInput) =>
    call<ResultSummary>("ingest_file", { assignment_id, path, input }),
  assignmentDetail: (assignment_id: string) =>
    call<AssignmentDetail>("assignment_detail", { assignment_id }),
  resultDetail: (assignment_id: string) =>
    call<ResultDetail>("result_detail", { assignment_id }),
  openExternal: (session_id: string) =>
    call<void>("open_external_reference", { session_id }),
};
