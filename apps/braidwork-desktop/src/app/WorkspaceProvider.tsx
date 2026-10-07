import {
  useCallback,
  useEffect,
  useRef,
  useState,
  type ReactNode,
} from "react";
import { isTauri } from "@tauri-apps/api/core";
import { api, asError } from "../lib/api";
import type { IpcError, Snapshot } from "../types/ipc";
import {
  Workspace,
  type View,
  type Selection,
  type FormRequest,
} from "./context";
export function WorkspaceProvider({ children }: { children: ReactNode }) {
  const [snapshot, setSnapshot] = useState<Snapshot | null>(null);
  const [loading, setLoading] = useState(() => isTauri());
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<IpcError | null>(null);
  const [feedback, setFeedback] = useState<string | null>(null);
  const [view, setView] = useState<View>("Overview");
  const [selection, select] = useState<Selection | null>(null);
  const [form, showForm] = useState<FormRequest | null>(null);
  const [revision, setRevision] = useState(0);
  const request = useRef(0);
  const report = useCallback((error: unknown) => setError(asError(error)), []);
  // Recover the Rust-selected project after a webview reload; no fixture fallback.
  useEffect(() => {
    if (!isTauri()) return;
    let active = true;
    const sequence = ++request.current;
    void api
      .snapshot()
      .then((fresh) => {
        if (active && sequence === request.current) {
          setSnapshot(fresh);
          setRevision((r) => r + 1);
        }
      })
      .catch((error) => {
        if (
          active &&
          sequence === request.current &&
          asError(error).code !== "no_project"
        )
          report(error);
      })
      .finally(() => {
        if (active && sequence === request.current) setLoading(false);
      });
    return () => {
      active = false;
    };
  }, [report]);
  const refresh = useCallback(async () => {
    const sequence = ++request.current;
    setLoading(true);
    try {
      const fresh = await api.snapshot();
      if (sequence === request.current) {
        setSnapshot(fresh);
        setRevision((r) => r + 1);
      }
    } catch (error) {
      if (sequence === request.current) report(error);
    } finally {
      if (sequence === request.current) setLoading(false);
    }
  }, [report]);
  const mutate = useCallback(
    async <T,>(action: () => Promise<T>, message: string): Promise<T> => {
      setBusy(true);
      setError(null);
      try {
        const result = await action();
        setFeedback(message);
        await refresh();
        return result;
      } catch (error) {
        report(error);
        throw asError(error);
      } finally {
        setBusy(false);
      }
    },
    [refresh, report],
  );
  const resetView = () => {
    select(null);
    showForm(null);
    setView("Overview");
  };
  const open = async (path: string) => {
    await mutate(() => api.open(path), "Project opened");
    resetView();
  };
  const init = async (path: string, name: string) => {
    await mutate(() => api.init(path, name), "Project created");
    resetView();
  };
  const close = async () => {
    setBusy(true);
    try {
      await api.close();
      ++request.current;
      setSnapshot(null);
      setLoading(false);
      setError(null);
      setFeedback(null);
      resetView();
    } catch (error) {
      report(error);
    } finally {
      setBusy(false);
    }
  };
  return (
    <Workspace.Provider
      value={{
        snapshot,
        loading,
        busy,
        error,
        feedback,
        view,
        selection,
        form,
        revision,
        setView,
        select,
        showForm,
        refresh,
        open,
        init,
        close,
        mutate,
        report,
        clearError: () => setError(null),
        notify: setFeedback,
      }}
    >
      {children}
    </Workspace.Provider>
  );
}
