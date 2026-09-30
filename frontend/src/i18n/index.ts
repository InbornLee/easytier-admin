import { createI18n } from "vue-i18n";
import zhCN from "./locales/zh-CN";
import en from "./locales/en";

export type AppLocale = "zh-CN" | "en";

export const SUPPORTED_LOCALES: { value: AppLocale; label: string }[] = [
  { value: "zh-CN", label: "简体中文" },
  { value: "en", label: "English" },
];

const STORAGE_KEY = "et-admin-locale";

function detectLocale(): AppLocale {
  try {
    const saved = localStorage.getItem(STORAGE_KEY);
    if (saved === "zh-CN" || saved === "en") return saved;
  } catch {
    /* ignore */
  }
  const lang = typeof navigator !== "undefined" ? navigator.language ?? "" : "";
  return lang.toLowerCase().startsWith("zh") ? "zh-CN" : "en";
}

export const i18n = createI18n({
  legacy: false,
  globalInjection: true,
  locale: detectLocale(),
  fallbackLocale: "zh-CN",
  messages: { "zh-CN": zhCN, en },
});

export function getLocale(): AppLocale {
  return (i18n.global.locale as unknown as { value: AppLocale }).value;
}

/** Translate a key outside of a component (e.g. in plain utility modules). */
export function translate(key: string, params?: Record<string, unknown>): string {
  return params ? i18n.global.t(key, params) : i18n.global.t(key);
}

export function setLocale(locale: AppLocale): void {
  (i18n.global.locale as unknown as { value: AppLocale }).value = locale;
  try {
    localStorage.setItem(STORAGE_KEY, locale);
  } catch {
    /* ignore */
  }
  if (typeof document !== "undefined") {
    document.documentElement.setAttribute("lang", locale);
  }
  if (typeof window !== "undefined") {
    window.dispatchEvent(new CustomEvent("et:locale", { detail: locale }));
  }
}
