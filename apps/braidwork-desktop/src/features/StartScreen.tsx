import { useState, type FormEvent } from "react";
import { FolderOpen, Plus } from "lucide-react";
import { useWorkspace } from "../app/context";
import { chooseDirectory } from "../lib/native";
import { Field, ErrorNotice } from "../components/Common";
import { Preferences } from "../components/Preferences";
import { useI18n } from "../i18n/context";
import { suggestFolder } from "../i18n";
export function StartScreen() {
  const { t } = useI18n();
  const { open, init, busy, loading, error, report, clearError } =
    useWorkspace();
  const [creating, setCreating] = useState(false);
  const [parent, setParent] = useState<string | null>(null);
  const [name, setName] = useState("");
  const [folder, setFolder] = useState("");
  const [folderEdited, setFolderEdited] = useState(false);
  const choose = async (create: boolean) => {
    try {
      clearError();
      const path = await chooseDirectory(
        t(create ? "Choose a parent folder" : "Choose a project folder"),
      );
      if (!path) return;
      if (create) setParent(path);
      else await open(path);
    } catch (error) {
      report(error);
    }
  };
  const submit = async (event: FormEvent) => {
    event.preventDefault();
    if (!parent) return;
    try {
      await init(parent, folder, name);
    } catch (error) {
      report(error);
    }
  };
  // Display-only preview. Rust joins and validates the actual destination.
  const separator = parent?.startsWith("/") || !parent?.includes("\\") ? "/" : "\\";
  const preview = parent
    ? `${parent.replace(/[\\/]+$/, "")}${separator}${folder}`
    : null;
  return (
    <main className="start-screen">
      <div className="start-preferences">
        <Preferences />
      </div>
      <div className="start-content">
        <p className="eyebrow">{t("LOCAL AI ORCHESTRATION")}</p>
        <h1>{t("Braidwork")}</h1>
        <p className="start-tagline">
          {t("Coordinate the AI you already have.")}
        </p>
        {!creating && (
          <p className="start-description">
            {t("Bring your specialists together around a shared project.")}
            <br />
            {t(
              "The work stays here. Your AI conversations stay wherever you prefer.",
            )}
          </p>
        )}
        {error && <ErrorNotice error={error} dismiss={clearError} />}
        {creating ? (
          <form className="start-form" onSubmit={(event) => void submit(event)}>
            <h2>{t("New project")}</h2>
            <Field label={t("Project name")}>
              {(id) => (
                <input
                  id={id}
                  required
                  value={name}
                  autoFocus
                  onChange={(event) => {
                    const value = event.target.value;
                    setName(value);
                    if (!folderEdited) setFolder(suggestFolder(value));
                  }}
                />
              )}
            </Field>
            <Field
              label={t("Folder")}
              hint={t("Folder name is independent from the display name.")}
            >
              {(id) => (
                <input
                  id={id}
                  required
                  value={folder}
                  onChange={(event) => {
                    setFolderEdited(true);
                    setFolder(event.target.value);
                  }}
                />
              )}
            </Field>
            <Field
              label={t("Location")}
              hint={t(
                "Choose where to create the project. A new child folder will be created.",
              )}
            >
              {(id) => (
                <div className="location-picker">
                  <input
                    id={id}
                    readOnly
                    value={parent ?? ""}
                    placeholder={t("Choose a parent folder")}
                  />
                  <button
                    type="button"
                    disabled={busy || loading}
                    onClick={() => void choose(true)}
                  >
                    {t("Browse")}
                  </button>
                </div>
              )}
            </Field>
            {preview && (
              <div className="destination-preview">
                <small>{t("Project will be created at:")}</small>
                <p className="reference">{preview}</p>
              </div>
            )}
            <div className="actions">
              <button
                type="button"
                disabled={busy || loading}
                onClick={() => {
                  setCreating(false);
                  clearError();
                }}
              >
                {t("Cancel")}
              </button>
              <button
                className="primary"
                type="submit"
                disabled={busy || loading || !parent || !folder || !name.trim()}
              >
                {t(busy ? "Creating…" : "Create project")}
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
              <FolderOpen size={18} />
              {t("Open Project")}
            </button>
            <button
              disabled={busy || loading}
              onClick={() => {
                setCreating(true);
                clearError();
              }}
            >
              <Plus size={18} />
              {t("New project")}
            </button>
          </div>
        )}
        {(busy || loading) && (
          <p role="status">
            {t(
              loading
                ? "Loading project state…"
                : creating
                  ? "Creating project…"
                  : "Opening project…",
            )}
          </p>
        )}
        <p className="start-note">
          {t("LOCAL-FIRST · MANUAL-FIRST · NO ACCOUNT REQUIRED")}
        </p>
      </div>
    </main>
  );
}
