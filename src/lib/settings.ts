import type { Locale } from "./i18n";

const STORAGE_KEY = "starlink-flight-deck:settings";

export type PersistedSettings = {
  dishAddress: string;
  dishEnabled: boolean;
  routerAddress: string;
  routerEnabled: boolean;
  includeLocation: boolean;
  includeGnss: boolean;
  autoRefreshEnabled: boolean;
  refreshEverySeconds: number;
  language: Locale;
};

export function loadSettings(): Partial<PersistedSettings> | null {
  try {
    const raw = window.localStorage.getItem(STORAGE_KEY);
    if (!raw) return null;
    return JSON.parse(raw) as Partial<PersistedSettings>;
  } catch {
    return null;
  }
}

export function saveSettings(settings: PersistedSettings) {
  try {
    window.localStorage.setItem(STORAGE_KEY, JSON.stringify(settings));
  } catch {
    // localStorage unavailable (e.g. storage disabled) - settings just won't persist
  }
}

export function clearSettings() {
  try {
    window.localStorage.removeItem(STORAGE_KEY);
  } catch {
    // ignore
  }
}
