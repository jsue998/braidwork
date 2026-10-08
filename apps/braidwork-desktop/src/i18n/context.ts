import { createContext, useContext } from "react";
import { english, type Language, type Theme, type Translator } from "./index";
interface Preferences {
  language: Language;
  theme: Theme;
  t: Translator;
  setLanguage: (language: Language) => void;
  setTheme: (theme: Theme) => void;
}
export const I18n = createContext<Preferences>({
  language: "en",
  theme: "dark",
  t: english,
  setLanguage: () => {},
  setTheme: () => {},
});
export const useI18n = () => useContext(I18n);
