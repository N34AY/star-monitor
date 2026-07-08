<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import OverviewTab from "./components/OverviewTab.vue";
import DishTab from "./components/DishTab.vue";
import RouterTab from "./components/RouterTab.vue";
import DiagnosticsTab from "./components/DiagnosticsTab.vue";
import SettingsTab from "./components/SettingsTab.vue";
import type {
  ActionDef,
  ActionExecutionResult,
  RouterSnapshot,
  StarlinkSnapshot,
  TabKey,
} from "./lib/types";
import { loadSettings, saveSettings } from "./lib/settings";
import { locale, setLocale, t } from "./lib/i18n";

const dishAddress = ref("");
const dishEnabled = ref(true);
const routerAddress = ref("");
const routerEnabled = ref(false);
const includeLocation = ref(true);
const includeGnss = ref(false);

const autoRefreshEnabled = ref(true);
const refreshEverySeconds = ref(3);
let refreshTimer: number | null = null;

const defaultDishAddress = ref("");
const defaultRouterAddress = ref("");
let settingsLoaded = false;

const loading = ref(false);
const dishActionLoading = ref<string | null>(null);
const routerActionLoading = ref<string | null>(null);
const clearingObstruction = ref(false);

const errorMessage = ref<string | null>(null);
const infoMessage = ref<string | null>(null);

const snapshot = ref<StarlinkSnapshot | null>(null);
const routerSnapshot = ref<RouterSnapshot | null>(null);
const routerError = ref<string | null>(null);

const activeTab = ref<TabKey>("overview");
const allTabs: { key: TabKey; visible: () => boolean }[] = [
  { key: "overview", visible: () => true },
  { key: "dish", visible: () => dishEnabled.value },
  { key: "router", visible: () => routerEnabled.value },
  { key: "diagnostics", visible: () => true },
  { key: "settings", visible: () => true },
];
const tabs = computed(() => allTabs.filter((tb) => tb.visible()));

watch(tabs, (visible) => {
  if (!visible.some((tb) => tb.key === activeTab.value)) {
    activeTab.value = "overview";
  }
});

const dishTabRef = ref<InstanceType<typeof DishTab> | null>(null);

async function loadDefaults() {
  const [dish, router] = await Promise.all([
    invoke<string>("default_dish_address"),
    invoke<string>("default_router_address"),
  ]);
  defaultDishAddress.value = dish;
  defaultRouterAddress.value = router;

  const persisted = loadSettings();
  dishAddress.value = persisted?.dishAddress || dish;
  dishEnabled.value = persisted?.dishEnabled ?? true;
  routerAddress.value = persisted?.routerAddress || router;
  routerEnabled.value = persisted?.routerEnabled ?? false;
  includeLocation.value = persisted?.includeLocation ?? true;
  includeGnss.value = persisted?.includeGnss ?? false;
  autoRefreshEnabled.value = persisted?.autoRefreshEnabled ?? true;
  refreshEverySeconds.value = persisted?.refreshEverySeconds ?? 3;
  setLocale(persisted?.language ?? "en");

  settingsLoaded = true;
}

function persistSettings() {
  if (!settingsLoaded) return; // avoid clobbering saved settings with initial ref defaults
  saveSettings({
    dishAddress: dishAddress.value,
    dishEnabled: dishEnabled.value,
    routerAddress: routerAddress.value,
    routerEnabled: routerEnabled.value,
    includeLocation: includeLocation.value,
    includeGnss: includeGnss.value,
    autoRefreshEnabled: autoRefreshEnabled.value,
    refreshEverySeconds: refreshEverySeconds.value,
    language: locale.value,
  });
}

async function resetToDefaults() {
  dishAddress.value = defaultDishAddress.value;
  dishEnabled.value = true;
  routerAddress.value = defaultRouterAddress.value;
  routerEnabled.value = false;
  includeLocation.value = true;
  includeGnss.value = false;
  autoRefreshEnabled.value = true;
  refreshEverySeconds.value = 3;
  setLocale("en");
  await tick();
}

async function refreshSnapshot(opts: { silent?: boolean } = {}) {
  if (loading.value) return;
  loading.value = true;
  if (!opts.silent) errorMessage.value = null;

  try {
    snapshot.value = await invoke<StarlinkSnapshot>("fetch_starlink_snapshot", {
      dishAddress: dishAddress.value,
      includeLocation: includeLocation.value,
      includeGnss: includeGnss.value,
      includeHistory: false,
    });
    errorMessage.value = null; // clear any stale error now that we're reconnected
  } catch (e) {
    // Background polls stay quiet on failure (e.g. the dish is mid-reboot
    // and will reconnect on its own) - only a deliberate refresh surfaces
    // the error banner.
    if (!opts.silent) errorMessage.value = e instanceof Error ? e.message : String(e);
  } finally {
    loading.value = false;
  }
}

async function refreshRouter(opts: { silent?: boolean } = {}) {
  try {
    routerSnapshot.value = await invoke<RouterSnapshot>("fetch_router_snapshot", {
      routerAddress: routerAddress.value,
    });
    routerError.value = null;
  } catch (e) {
    routerError.value = e instanceof Error ? e.message : String(e);
    if (!opts.silent) routerSnapshot.value = null;
  }
}

async function tick(opts: { silent?: boolean } = {}) {
  if (dishEnabled.value) {
    await refreshSnapshot(opts);
  }
  if (routerEnabled.value) {
    await refreshRouter(opts);
  }
}

function confirmText(domain: "dish" | "router", action: ActionDef): string | null {
  if (!action.confirm) return null;
  const key = `actions.${domain}.${action.id}.confirm`;
  const translated = t(key);
  return translated === key ? action.confirm : translated;
}

async function runDishAction(action: ActionDef) {
  const confirm = confirmText("dish", action);
  if (confirm && !window.confirm(confirm)) return;

  dishActionLoading.value = action.id;
  errorMessage.value = null;
  infoMessage.value = null;

  try {
    const result = await invoke<ActionExecutionResult>("trigger_dish_action", {
      dishAddress: dishAddress.value,
      actionId: action.id,
    });
    infoMessage.value = result.message;
    // A reboot takes the dish offline for a while - an immediate refresh
    // would just fail and stomp the success message with a scary error.
    // The regular auto-refresh timer will pick it back up once it's online.
    if (action.id !== "reboot") {
      await refreshSnapshot();
    }
  } catch (e) {
    errorMessage.value = e instanceof Error ? e.message : String(e);
  } finally {
    dishActionLoading.value = null;
  }
}

async function runRouterAction(action: ActionDef) {
  const confirm = confirmText("router", action);
  if (confirm && !window.confirm(confirm)) return;

  routerActionLoading.value = action.id;
  errorMessage.value = null;
  infoMessage.value = null;

  try {
    const result = await invoke<ActionExecutionResult>("trigger_router_action", {
      routerAddress: routerAddress.value,
      actionId: action.id,
    });
    infoMessage.value = result.message;
    // Same reasoning as the dish reboot case above.
    if (action.id !== "reboot") {
      await refreshRouter();
    }
  } catch (e) {
    errorMessage.value = e instanceof Error ? e.message : String(e);
  } finally {
    routerActionLoading.value = null;
  }
}

async function clearObstructionMap() {
  const action = snapshot.value?.actions.find((a) => a.id === "clearObstructionMap");
  if (!action) return;
  const confirm = confirmText("dish", action);
  if (confirm && !window.confirm(confirm)) return;

  clearingObstruction.value = true;
  try {
    const result = await invoke<ActionExecutionResult>("trigger_dish_action", {
      dishAddress: dishAddress.value,
      actionId: action.id,
    });
    infoMessage.value = result.message;
    await dishTabRef.value?.reloadObstructionMap();
  } catch (e) {
    errorMessage.value = e instanceof Error ? e.message : String(e);
  } finally {
    clearingObstruction.value = false;
  }
}

function stopAutoRefresh() {
  if (refreshTimer !== null) {
    window.clearInterval(refreshTimer);
    refreshTimer = null;
  }
}

function restartAutoRefresh() {
  stopAutoRefresh();
  if (!autoRefreshEnabled.value) return;

  const period = Math.max(2, Number(refreshEverySeconds.value) || 3) * 1000;
  refreshTimer = window.setInterval(() => void tick({ silent: true }), period);
}

watch([autoRefreshEnabled, refreshEverySeconds], restartAutoRefresh);
watch(dishEnabled, (enabled) => {
  if (enabled) void refreshSnapshot({ silent: true });
  else {
    snapshot.value = null;
    errorMessage.value = null;
  }
});
watch(routerEnabled, (enabled) => {
  if (enabled) void refreshRouter({ silent: true });
  else {
    routerSnapshot.value = null;
    routerError.value = null;
  }
});

watch(
  [
    dishAddress,
    dishEnabled,
    routerAddress,
    routerEnabled,
    includeLocation,
    includeGnss,
    autoRefreshEnabled,
    refreshEverySeconds,
    locale,
  ],
  persistSettings
);

onMounted(async () => {
  await loadDefaults();
  await tick();
  restartAutoRefresh();
});

onBeforeUnmount(stopAutoRefresh);
</script>

<template>
  <main class="page">
    <header class="hero">
      <h1>{{ t("app.title") }}</h1>
      <p>{{ routerEnabled ? t("app.subtitleWithRouter") : t("app.subtitle") }}</p>
    </header>

    <nav class="tab-bar">
      <button
        v-for="tab in tabs"
        :key="tab.key"
        :class="{ active: activeTab === tab.key }"
        @click="activeTab = tab.key"
      >
        {{ t(`tabs.${tab.key}`) }}
      </button>
    </nav>

    <section class="panel messages" v-if="errorMessage || infoMessage">
      <p v-if="errorMessage" class="bad">{{ errorMessage }}</p>
      <p v-if="infoMessage" class="good">{{ infoMessage }}</p>
    </section>

    <OverviewTab
      v-if="activeTab === 'overview'"
      :snapshot="snapshot"
      :router-enabled="routerEnabled"
      :router-snapshot="routerSnapshot"
    />

    <DishTab
      v-if="activeTab === 'dish'"
      ref="dishTabRef"
      :dish-address="dishAddress"
      :snapshot="snapshot"
      :action-loading="dishActionLoading"
      :obstruction="snapshot?.overview?.obstruction"
      :clear-action="snapshot?.actions.find((a) => a.id === 'clearObstructionMap')"
      :clearing="clearingObstruction"
      @run-action="runDishAction"
      @clear="clearObstructionMap"
    />

    <RouterTab
      v-if="activeTab === 'router'"
      :router-enabled="routerEnabled"
      :router-address="routerAddress"
      :router-snapshot="routerSnapshot"
      :router-error="routerError"
      :action-loading="routerActionLoading"
      @run-action="runRouterAction"
    />

    <DiagnosticsTab
      v-if="activeTab === 'diagnostics' && snapshot"
      :dish-address="dishAddress"
      :sections="snapshot.sections"
      :router-enabled="routerEnabled"
      :router-address="routerAddress"
      :router-sections="routerSnapshot?.sections ?? []"
    />

    <SettingsTab
      v-if="activeTab === 'settings'"
      v-model:dish-address="dishAddress"
      v-model:dish-enabled="dishEnabled"
      v-model:router-address="routerAddress"
      v-model:router-enabled="routerEnabled"
      v-model:include-location="includeLocation"
      v-model:include-gnss="includeGnss"
      v-model:auto-refresh-enabled="autoRefreshEnabled"
      v-model:refresh-every-seconds="refreshEverySeconds"
      :loading="loading"
      @refresh-now="tick()"
      @reset-defaults="resetToDefaults"
    />
  </main>
</template>

<style>
:root {
  /* The whole app is unconditionally dark themed. This hints the engine
     to render native, otherwise-unstylable UI chrome (select dropdown
     popups, scrollbars) in its dark variant too - without it, WebKitGTK
     on Linux renders <select> popups with the OS's light GTK theme
     regardless of any CSS applied to the <select>/<option> elements. */
  color-scheme: dark;
  --page: #0d0d0d;
  --surface: #17171a;
  --panel: rgba(255, 255, 255, 0.035);
  --ink: #ffffff;
  --ink-soft: #c3c2b7;
  --muted: #898781;
  --line: rgba(255, 255, 255, 0.1);
  --accent: #3987e5;
  --good: #0ca30c;
  --bad: #d03b3b;
  --warn: #fab219;
}

* {
  box-sizing: border-box;
}

body {
  margin: 0;
  font-family: system-ui, -apple-system, "Segoe UI", sans-serif;
  color: var(--ink);
  background: var(--page);
  min-height: 100vh;
}

.page {
  width: min(1200px, 96vw);
  margin: 0 auto;
  padding: 2rem 0 3rem;
}

.hero {
  margin-bottom: 1.25rem;
}

.hero h1 {
  margin: 0;
  font-size: clamp(1.7rem, 3.2vw, 2.4rem);
  letter-spacing: 0.01em;
}

.hero p {
  margin: 0.4rem 0 0;
  color: var(--ink-soft);
}

.tab-bar {
  display: flex;
  flex-wrap: wrap;
  gap: 0.4rem;
  background: var(--panel);
  border: 1px solid var(--line);
  border-radius: 14px;
  padding: 0.35rem;
  margin-bottom: 1rem;
}

.tab-bar button {
  background: transparent;
  border: 1px solid transparent;
  color: var(--ink-soft);
}

.tab-bar button.active {
  background: rgba(255, 255, 255, 0.08);
  color: var(--ink);
  border-color: var(--line);
}

.panel {
  background: var(--surface);
  border: 1px solid var(--line);
  border-radius: 16px;
  padding: 1rem;
  margin-bottom: 1rem;
}

h2 {
  margin: 0 0 0.5rem;
  font-size: 1.05rem;
}

h3 {
  margin: 0;
}

input,
select {
  width: 100%;
  border: 1px solid var(--line);
  border-radius: 10px;
  padding: 0.6rem 0.7rem;
  font: inherit;
  color: var(--ink);
  background: rgba(255, 255, 255, 0.04);
}

/* Belt-and-suspenders for engines that do honor option-level styling
   (color-scheme above is what fixes WebKitGTK's native popup). */
option {
  background-color: var(--surface);
  color: var(--ink);
}

input[type="checkbox"] {
  width: auto;
}

button {
  border: 1px solid var(--line);
  border-radius: 999px;
  padding: 0.45rem 0.85rem;
  font: inherit;
  font-weight: 600;
  cursor: pointer;
  background: rgba(255, 255, 255, 0.05);
  color: var(--ink);
  transition: transform 0.15s ease, background 0.15s ease;
}

button:hover:enabled {
  background: rgba(255, 255, 255, 0.1);
}

button:disabled {
  opacity: 0.55;
  cursor: not-allowed;
}

button.primary {
  background: var(--accent);
  border-color: var(--accent);
  color: #fff;
}

.chip {
  border-radius: 999px;
  padding: 0.2rem 0.6rem;
  font-size: 0.8rem;
  font-weight: 700;
  background: rgba(255, 255, 255, 0.06);
}

.good {
  color: var(--good);
}

.bad {
  color: var(--bad);
}

.warn {
  color: var(--warn);
}

.muted {
  color: var(--muted);
}

.meta {
  margin: 0.35rem 0;
  color: var(--ink-soft);
  font-size: 0.86rem;
}

.messages {
  margin-bottom: 1rem;
}

.metrics-grid {
  display: grid;
  grid-template-columns: repeat(4, minmax(0, 1fr));
  gap: 0.65rem;
  margin: 0.75rem 0 0;
}

.metric-card {
  border: 1px solid var(--line);
  border-radius: 12px;
  background: rgba(255, 255, 255, 0.03);
  padding: 0.75rem;
  display: grid;
  gap: 0.15rem;
}

.metric-card span {
  color: var(--ink-soft);
  font-size: 0.82rem;
}

.metric-card strong {
  font-size: 1.15rem;
}

.metric-card small {
  color: var(--ink-soft);
}

.actions-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(230px, 1fr));
  gap: 0.65rem;
}

@media (max-width: 980px) {
  .metrics-grid {
    grid-template-columns: repeat(2, minmax(0, 1fr));
  }
}

@media (max-width: 720px) {
  .page {
    width: min(96vw, 720px);
    padding-top: 1.3rem;
  }
}
</style>
