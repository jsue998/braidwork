import type { MessageKey } from "../i18n";
import { createContext, useContext } from "react";
import type { IpcError, Snapshot } from "../types/ipc";
export type View = "Overview" | "Team" | "Work" | "Resources" | "Results";
export type Selection =
  | {
      kind: "resource" | "session" | "task" | "assignment" | "result";
      id: string;
    }
  | { kind: "agent"; id: string; revision: number };
export type FormKind =
  | "resource"
  | "agent"
  | "session"
  | "task"
  | "assignment"
  | "delegation";
export interface FormRequest {
  kind: FormKind;
  task_id?: string;
}
export interface WorkspaceContext {
  snapshot: Snapshot | null;
  loading: boolean;
  busy: boolean;
  error: IpcError | null;
  feedback: MessageKey | null;
  view: View;
  selection: Selection | null;
  form: FormRequest | null;
  revision: number;
  setView: (view: View) => void;
  select: (selection: Selection | null) => void;
  showForm: (form: FormRequest | null) => void;
  refresh: () => Promise<void>;
  open: (path: string) => Promise<void>;
  init: (parent: string, folder: string, name: string) => Promise<void>;
  close: () => Promise<void>;
  mutate: <T>(action: () => Promise<T>, message: MessageKey) => Promise<T>;
  report: (error: unknown) => void;
  clearError: () => void;
  notify: (message: MessageKey | null) => void;
}
export const Workspace = createContext<WorkspaceContext | null>(null);
export function useWorkspace() {
  const context = useContext(Workspace);
  if (!context) throw new Error("Workspace provider is missing");
  return context;
}
