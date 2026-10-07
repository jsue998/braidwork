import {
  LayoutDashboard,
  UsersRound,
  ListTodo,
  Layers,
  FileCheck2,
  RefreshCw,
  PanelLeftClose,
  X,
} from "lucide-react";
import type { ReactNode } from "react";
import { useWorkspace, type View } from "../app/context";
import { ErrorNotice } from "./Common";
import { Inspector } from "../features/Inspector";
const navigation = [
  { view: "Overview", icon: LayoutDashboard },
  { view: "Team", icon: UsersRound },
  { view: "Work", icon: ListTodo },
  { view: "Resources", icon: Layers },
  { view: "Results", icon: FileCheck2 },
] satisfies { view: View; icon: typeof LayoutDashboard }[];
export function AppShell({ children }: { children: ReactNode }) {
  const {
    snapshot,
    view,
    setView,
    loading,
    busy,
    error,
    selection,
    form,
    clearError,
    feedback,
    notify,
    close,
    refresh,
  } = useWorkspace();
  if (!snapshot) return null;
  return (
    <div className="app-shell">
      <header className="topbar">
        <strong className="wordmark">Braidwork</strong>
        <div className="project-heading">
          <strong>{snapshot.project.name}</strong>
          <span title={snapshot.project.root}>{snapshot.project.root}</span>
        </div>
        <div className="actions">
          <button
            className="icon-button"
            disabled={busy || loading}
            aria-label="Refresh project"
            title="Refresh project"
            onClick={() => void refresh()}
          >
            <RefreshCw size={17} />
          </button>
          <button disabled={busy} onClick={() => void close()}>
            <PanelLeftClose size={16} /> Close project
          </button>
        </div>
      </header>
      <div className="workspace-layout">
        <nav className="sidebar" aria-label="Main navigation">
          <p className="eyebrow">WORKSPACE</p>
          {navigation.map(({ view: item, icon: Icon }) => (
            <button
              key={item}
              aria-current={item === view ? "page" : undefined}
              onClick={() => setView(item)}
            >
              <Icon size={18} />
              {item}
            </button>
          ))}
          <div className="sidebar-note">
            Your project owns the state.
            <br />
            AI receives the instructions.
          </div>
        </nav>
        <main className="workspace-main">
          {error && !selection && !form && (
            <ErrorNotice error={error} dismiss={clearError} />
          )}
          {loading && (
            <p className="loading-line" role="status">
              Refreshing canonical project state…
            </p>
          )}
          {children}
        </main>
        <Inspector />
      </div>
      {feedback && (
        <div className="feedback" role="status">
          <span>{feedback}</span>
          <button
            className="icon-button"
            aria-label="Dismiss notification"
            onClick={() => notify("")}
          >
            <X size={15} />
          </button>
        </div>
      )}
    </div>
  );
}
