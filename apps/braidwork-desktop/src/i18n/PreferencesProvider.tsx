import {
  useCallback,
  useEffect,
  useMemo,
  useState,
  type ReactNode,
} from "react";
import {
  LANGUAGE_KEY,
  THEME_KEY,
  effectiveTheme,
  resolveLanguage,
  resolveTheme,
  translator,
  readUiPreference,
  persistUiPreference,
  type Language,
  type Theme,
} from "./index";
import { I18n } from "./context";
export function PreferencesProvider({ children }: { children: ReactNode }) {
  const [language, updateLanguage] = useState(() =>
    resolveLanguage(
      readUiPreference(LANGUAGE_KEY),
      typeof navigator === "undefined" ? "en" : navigator.language,
    ),
  );
  const [theme, updateTheme] = useState(() =>
    resolveTheme(readUiPreference(THEME_KEY)),
  );
  const t = useMemo(() => translator(language), [language]);
  const setLanguage = useCallback((value: Language) => {
    updateLanguage(value);
    persistUiPreference(LANGUAGE_KEY, value);
  }, []);
  const setTheme = useCallback((value: Theme) => {
    updateTheme(value);
    persistUiPreference(THEME_KEY, value);
  }, []);
  useEffect(() => {
    document.documentElement.lang = language;
  }, [language]);
  useEffect(() => {
    const media = window.matchMedia("(prefers-color-scheme: dark)");
    const apply = () => {
      document.documentElement.dataset.theme = effectiveTheme(
        theme,
        media.matches,
      );
    };
    apply();
    if (theme !== "system") return;
    media.addEventListener("change", apply);
    return () => media.removeEventListener("change", apply);
  }, [theme]);
  return (
    <I18n.Provider value={{ language, theme, t, setLanguage, setTheme }}>
      {children}
    </I18n.Provider>
  );
}
