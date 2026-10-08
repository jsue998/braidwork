import { useI18n } from "../i18n/context";
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
import { Preferences } from "./Preferences";
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
  const { t } = useI18n();
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
        <strong className="wordmark">{t("Braidwork")}</strong>
        <div className="project-heading">
          <strong>{snapshot.project.name}</strong>
          <span title={snapshot.project.root}>{snapshot.project.root}</span>
        </div>
        <div className="actions">
          <Preferences />
          <button
            className="icon-button"
            disabled={busy || loading}
            aria-label={t("Refresh project")}
            title={t("Refresh project")}
            onClick={() => void refresh()}
          >
            <RefreshCw size={17} />
          </button>
          <button disabled={busy} onClick={() => void close()}>
            <PanelLeftClose size={16} /> {t("Close project")}{" "}
          </button>
        </div>
      </header>
      <div className="workspace-layout">
        <nav className="sidebar" aria-label={t("Main navigation")}>
          <p className="eyebrow">{t("WORKSPACE")}</p>
          {navigation.map(({ view: item, icon: Icon }) => (
            <button
              key={t(item)}
              aria-current={item === view ? "page" : undefined}
              onClick={() => setView(item)}
            >
              <Icon size={18} />
              {t(item)}
            </button>
          ))}
          <div className="sidebar-note">
            {" "}
            {t("Your project owns the state.")} <br />{" "}
            {t("AI receives the instructions.")}{" "}
          </div>
        </nav>
        <main className="workspace-main">
          {error && !selection && !form && (
            <ErrorNotice error={error} dismiss={clearError} />
          )}
          {loading && (
            <p className="loading-line" role="status">
              {" "}
              {t("Refreshing canonical project state…")}{" "}
            </p>
          )}
          {children}
        </main>
        <Inspector />
      </div>
      {feedback && (
        <div className="feedback" role="status">
          <span>{t(feedback)}</span>
          <button
            className="icon-button"
            aria-label={t("Dismiss notification")}
            onClick={() => notify(null)}
          >
            <X size={15} />
          </button>
        </div>
      )}
    </div>
  );
}
