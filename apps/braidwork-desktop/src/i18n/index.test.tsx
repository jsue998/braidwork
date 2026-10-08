import { describe, expect, it } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import { en } from "./en";
import { es } from "./es";
import {
  LANGUAGE_KEY,
  THEME_KEY,
  effectiveTheme,
  errorMessage,
  readUiPreference,
  persistUiPreference,
  resolveLanguage,
  resolveTheme,
  suggestFolder,
  translator,
  type MessageKey,
} from "./index";
import { I18n } from "./context";
import { ContentPreview, ErrorNotice, PageHeading } from "../components/Common";
import {
  assignmentLabel,
  modelLabel,
  stateLabel,
  usageLabel,
} from "../lib/derive";

describe("typed EN/ES UI messages", () => {
  it("has exactly matching keys and a defined translation for every key", () => {
    expect(Object.keys(es).sort()).toEqual(Object.keys(en).sort());
    for (const language of ["en", "es"] as const) {
      const t = translator(language);
      for (const key of Object.keys(en) as MessageKey[]) {
        expect(typeof t(key)).toBe("string");
        expect(t(key).length).toBeGreaterThan(0);
      }
    }
  });
  it("resolves browser Spanish variants, English, and the English fallback", () => {
    expect(resolveLanguage(null, "es-MX")).toBe("es");
    expect(resolveLanguage(null, "es-ES")).toBe("es");
    expect(resolveLanguage(null, "en-US")).toBe("en");
    expect(resolveLanguage(null, "fr-FR")).toBe("en");
    expect(resolveLanguage(null, "")).toBe("en");
    expect(resolveLanguage("en", "es-MX")).toBe("en");
    expect(resolveLanguage("es", "en-US")).toBe("es");
    expect(resolveLanguage("unsupported", "en-US")).toBe("en");
  });
  it("interpolates literal values without translating user-provided data", () => {
    const t = translator("es");
    expect(t("Remove {path}", { path: "Team/{unknown}/Results" })).toBe(
      "Quitar Team/{unknown}/Results",
    );
    expect(modelLabel("Team", t)).toBe("Team");
    const html = renderToStaticMarkup(
      <I18n.Provider
        value={{
          language: "es",
          theme: "dark",
          t,
          setLanguage: () => {},
          setTheme: () => {},
        }}
      >
        <PageHeading title="Team" subtitle={t("Team")} />
        <ContentPreview text={"Work\nResults <script>"} />
      </I18n.Provider>,
    );
    expect(html).toContain("<h1>Team</h1>");
    expect(html).toContain("Equipo");
    expect(html).toContain("Work\nResults &lt;script&gt;");
  });
  it("localizes real states and unknown observations without changing their semantics", () => {
    const t = translator("es");
    expect(assignmentLabel("result_received", t)).toBe("Resultado recibido");
    expect(stateLabel("pending", t)).toBe("Pendiente");
    expect(stateLabel("not_performed", t)).toBe("No realizada");
    expect(modelLabel(null, t)).toBe("Modelo desconocido");
    expect(
      usageLabel({ input_tokens: null, output_tokens: "0", cost: null }, t),
    ).toBe("Entrada: desconocido · Salida: 0 tokens · Coste: desconocido");
  });
  it("uses error codes while preserving backend fallback and technical diagnostics", () => {
    const t = translator("es");
    const error = {
      code: "destination_exists",
      message: "Backend wording can change.",
      detail: "Original cause",
    };
    expect(errorMessage(error, t)).toBe(
      es[
        "That folder already exists. Choose another folder name or open it instead."
      ],
    );
    expect(errorMessage({ ...error, code: "future_code" }, t)).toBe(
      error.message,
    );
    const html = renderToStaticMarkup(
      <I18n.Provider
        value={{
          language: "es",
          theme: "dark",
          t,
          setLanguage: () => {},
          setTheme: () => {},
        }}
      >
        <ErrorNotice error={error} />
      </I18n.Provider>,
    );
    expect(html).toContain("Esa carpeta ya existe");
    expect(html).toContain("Detalles técnicos");
    expect(html).toContain("Original cause");
    expect(html).toContain(error.message);
  });
});

describe("local UI preferences and predictable folder suggestions", () => {
  it("defaults to dark and follows the OS only when system was selected", () => {
    expect(resolveTheme(null)).toBe("dark");
    expect(resolveTheme("invalid")).toBe("dark");
    expect(resolveTheme("light")).toBe("light");
    expect(resolveTheme("system")).toBe("system");
    expect(effectiveTheme("dark", false)).toBe("dark");
    expect(effectiveTheme("light", true)).toBe("light");
    expect(effectiveTheme("system", true)).toBe("dark");
    expect(effectiveTheme("system", false)).toBe("light");
  });
  it("persists only language/theme UI preferences and honors them on reopening", () => {
    const values = new Map<string, string>();
    const storage = {
      getItem: (key: string) => values.get(key) ?? null,
      setItem: (key: string, value: string) => {
        values.set(key, value);
      },
    };
    persistUiPreference(LANGUAGE_KEY, "es", storage);
    persistUiPreference(THEME_KEY, "light", storage);
    expect(
      resolveLanguage(readUiPreference(LANGUAGE_KEY, storage), "en-US"),
    ).toBe("es");
    expect(resolveTheme(readUiPreference(THEME_KEY, storage))).toBe("light");
    expect([...values.keys()]).toEqual([LANGUAGE_KEY, THEME_KEY]);
  });
  it("remains usable when local UI storage is unavailable", () => {
    const storage = {
      getItem: () => {
        throw new Error("blocked");
      },
      setItem: () => {
        throw new Error("blocked");
      },
    };
    expect(readUiPreference(LANGUAGE_KEY, storage)).toBeNull();
    expect(() => persistUiPreference(THEME_KEY, "dark", storage)).not.toThrow();
  });
  it("suggests deterministic folder names without rewriting the human display name", () => {
    const name = "Investigación de Semiconductores";
    expect(suggestFolder(name)).toBe("investigacion-de-semiconductores");
    expect(name).toBe("Investigación de Semiconductores");
    expect(suggestFolder(" Investigación VLSI! ")).toBe("investigacion-vlsi");
    expect(suggestFolder("日本 旅行")).toBe("日本-旅行");
    expect(suggestFolder(" / ")).toBe("");
  });
});
