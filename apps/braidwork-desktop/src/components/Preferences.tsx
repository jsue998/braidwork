import { useI18n } from "../i18n/context";
import type { Theme } from "../i18n";
export function Preferences() {
  const { t, language, setLanguage, theme, setTheme } = useI18n();
  return (
    <div className="preferences">
      <div
        className="language-controls"
        role="group"
        aria-label={t("Language")}
      >
        <button
          type="button"
          lang="en"
          aria-label={t("English")}
          aria-pressed={language === "en"}
          onClick={() => setLanguage("en")}
        >
          {t("EN")}
        </button>
        <button
          type="button"
          lang="es"
          aria-label={t("Español")}
          aria-pressed={language === "es"}
          onClick={() => setLanguage("es")}
        >
          {t("ES")}
        </button>
      </div>
      <label className="theme-control">
        <span className="sr-only">{t("Theme")}</span>
        <select
          aria-label={t("Theme")}
          value={theme}
          onChange={(event) => setTheme(event.target.value as Theme)}
        >
          <option value="dark">{t("Dark")}</option>
          <option value="light">{t("Light")}</option>
          <option value="system">{t("System")}</option>
        </select>
      </label>
    </div>
  );
}
