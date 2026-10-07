import { useState, type FormEvent } from "react";
import { FolderOpen, Plus, ArrowLeft } from "lucide-react";
import { useWorkspace } from "../app/context";
import { chooseDirectory } from "../lib/native";
import { Field, ErrorNotice } from "../components/Common";
export function StartScreen() {
  const { open, init, busy, loading, error, report, clearError } =
    useWorkspace();
  const [folder, setFolder] = useState<string | null>(null);
  const [name, setName] = useState("");
  const choose = async (create: boolean) => {
    try {
      clearError();
      const path = await chooseDirectory();
      if (!path) return;
      if (create) {
        setFolder(path);
        setName(path.split(/[\\/]/).filter(Boolean).at(-1) ?? "");
      } else await open(path);
    } catch (error) {
      report(error);
    }
  };
  const submit = async (event: FormEvent) => {
    event.preventDefault();
    if (!folder) return;
    try {
      await init(folder, name);
    } catch (error) {
      report(error);
    }
  };
  return (
    <main className="start-screen">
      <div className="start-content">
        <p className="eyebrow">LOCAL WORKSPACE · YOUR AI TEAM</p>
        <h1>Braidwork</h1>
        <p className="start-tagline">Coordinate the AI you already have.</p>
        <p className="start-description">
          Bring your specialists together around a shared project.
          <br />
          The work stays here. Your AI conversations stay wherever you prefer.
        </p>
        {error && <ErrorNotice error={error} dismiss={clearError} />}
        {folder ? (
          <form className="start-form" onSubmit={(event) => void submit(event)}>
            <h2>Create project</h2>
            <Field label="Project name">
              {(id) => (
                <input
                  id={id}
                  required
                  value={name}
                  onChange={(event) => setName(event.target.value)}
                  autoFocus
                />
              )}
            </Field>
            <p className="reference">{folder}</p>
            <div className="actions">
              <button
                type="button"
                disabled={busy || loading}
                onClick={() => setFolder(null)}
              >
                <ArrowLeft size={16} /> Back
              </button>
              <button
                className="primary"
                type="submit"
                disabled={busy || loading}
              >
                {busy ? "Creating…" : "Create project"}
              </button>
            </div>
          </form>
        ) : (
          <div className="start-actions">
            <button
              className="primary"
              disabled={busy || loading}
              onClick={() => void choose(false)}
            >
              <FolderOpen size={18} /> Open Project
            </button>
            <button
              disabled={busy || loading}
              onClick={() => void choose(true)}
            >
              <Plus size={18} /> Create Project
            </button>
          </div>
        )}
        {(busy || loading) && (
          <p role="status">
            {loading
              ? "Loading project state…"
              : folder
                ? "Creating project…"
                : "Opening project…"}
          </p>
        )}
        <p className="start-note">
          Local-first. Manual by design. No account required.
        </p>
      </div>
      <div className="start-mark" aria-hidden="true">
        <span />
        <span />
        <span />
        <span />
      </div>
    </main>
  );
}
