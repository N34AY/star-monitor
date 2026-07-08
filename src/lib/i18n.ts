import { ref } from "vue";
import en from "../locales/en.json";
import ua from "../locales/ua.json";

export type Locale = "en" | "ua";

const dictionaries: Record<Locale, unknown> = { en, ua };

export const locale = ref<Locale>("en");

export function setLocale(l: Locale) {
  locale.value = l;
}

function lookup(dict: unknown, key: string): string | undefined {
  const value = key
    .split(".")
    .reduce<unknown>(
      (acc, part) => (acc && typeof acc === "object" ? (acc as Record<string, unknown>)[part] : undefined),
      dict
    );
  return typeof value === "string" ? value : undefined;
}

export function t(key: string, vars?: Record<string, string | number>): string {
  let template = lookup(dictionaries[locale.value], key) ?? lookup(dictionaries.en, key) ?? key;
  if (vars) {
    for (const [k, v] of Object.entries(vars)) {
      template = template.split(`{${k}}`).join(String(v));
    }
  }
  return template;
}
