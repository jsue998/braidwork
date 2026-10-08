import { en } from "./en";
import { es } from "./es";
import type { IpcError } from "../types/ipc";
export type Language = "en" | "es";
export type MessageKey = keyof typeof en;
export type Translator = (
  key: MessageKey,
  values?: Record<string, string | number>,
) => string;
export type Theme = "dark" | "light" | "system";
export const LANGUAGE_KEY = "braidwork.ui.language";
export const THEME_KEY = "braidwork.ui.theme";
export function resolveLanguage(
  saved: string | null,
  browser: string,
): Language {
  if (saved === "en" || saved === "es") return saved;
  return browser.toLowerCase().startsWith("es") ? "es" : "en";
}
export function resolveTheme(saved: string | null): Theme {
  return saved === "light" || saved === "system" ? saved : "dark";
}
export const effectiveTheme = (theme: Theme, systemDark: boolean) =>
  theme === "system" ? (systemDark ? "dark" : "light") : theme;
export function translator(language: Language): Translator {
  const messages = language === "es" ? es : en;
  return (key, values) =>
    messages[key].replace(/\{(\w+)\}/g, (placeholder, name: string) =>
      values?.[name] === undefined ? placeholder : String(values[name]),
    );
}
export const english = translator("en");
const errorMessages: Record<string, MessageKey> = {
  operation_failed: "The operation could not be completed.",
  no_project: "Open or create a project first.",
  destination_exists:
    "That folder already exists. Choose another folder name or open it instead.",
  invalid_parent: "Choose an existing parent folder.",
  invalid_folder:
    "Use a single portable folder name without reserved names, separators, or trailing spaces/dots.",
  create_directory:
    "The project folder could not be created. Check its location and permissions.",
  invalid_name: "The project name must contain non-whitespace text.",
  invalid_input:
    "The selected input is invalid. Check the fields and technical details.",
  not_found: "Could not find that record. Refresh the project and try again.",
  already_exists: "That record already exists.",
  missing_reference:
    "The selected reference does not exist. Refresh and check the selection.",
  delegation_denied:
    "That agent is not allowed to delegate more work. Check its delegation policy and direct-delegate limit.",
  workflow:
    "This action is not available in the recorded workflow state. See technical details.",
  invalid_project_data:
    "Stored project data could not be reconstructed. No data was repaired or replaced.",
  unsupported_schema:
    "This database version is not supported by this Braidwork version.",
  integrity:
    "These references are inconsistent with the project. Refresh and check the selected work, session, and agent revision.",
  store: "The project operation could not be saved or read.",
  already_initialized:
    "This folder already contains a Braidwork project. Open it instead.",
  project_not_found:
    "No Braidwork project was found in this folder. Select its root or create a project.",
  database_missing:
    "This project's database is missing. It has not been recreated.",
  unsupported_format:
    "This project format is not supported by this Braidwork version.",
  context_forbidden: "A zero context budget does not allow selected context.",
  content_missing:
    "The result's content file is missing. Its metadata has been preserved.",
  artifact_content: "The result content could not be read or written.",
  self_delegation: "An assignment cannot delegate work to itself.",
  project:
    "The project could not be opened or changed. Existing data has been preserved.",
  external_open: "Could not open the external reference in your browser.",
  operation_interrupted:
    "The operation was interrupted. Refresh or reopen the project.",
  state_unavailable: "The desktop service is unavailable. Restart Braidwork.",
};
export function errorMessage(error: IpcError, t: Translator): string {
  const key = errorMessages[error.code];
  return key ? t(key) : error.message;
}
// UI-only suggestion. Rust validates the actual folder and constructs the canonical path.
export function suggestFolder(name: string): string {
  return name
    .normalize("NFKD")
    .replace(/\p{M}/gu, "")
    .toLowerCase()
    .replace(/[^\p{L}\p{N}]+/gu, "-")
    .replace(/^-+|-+$/g, "");
}

type PreferenceKey = typeof LANGUAGE_KEY | typeof THEME_KEY;
type PreferenceStorage = Pick<Storage, "getItem" | "setItem">;
export function readUiPreference(
  key: PreferenceKey,
  storage?: PreferenceStorage,
): string | null {
  try {
    return (storage ?? localStorage).getItem(key);
  } catch {
    return null;
  }
}
export function persistUiPreference(
  key: PreferenceKey,
  value: string,
  storage?: PreferenceStorage,
): void {
  try {
    (storage ?? localStorage).setItem(key, value);
  } catch {
    /* UI preferences remain usable without storage. */
  }
}
