<script setup lang="ts">
import { computed, onMounted, ref, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import LineChart from "./LineChart.vue";
import { t } from "../lib/i18n";
import type { CollectorStatus, HistoryEvent, HistorySamples } from "../lib/types";

type RangeKey = "24h" | "3d" | "7d" | "all";
const RANGE_MS: Record<RangeKey, number> = {
  "24h": 24 * 60 * 60 * 1000,
  "3d": 3 * 24 * 60 * 60 * 1000,
  "7d": 7 * 24 * 60 * 60 * 1000,
  all: 0,
};

const range = ref<RangeKey>("3d");
const sourceFilter = ref<"all" | "dish" | "router">("all");

const events = ref<HistoryEvent[]>([]);
const samples = ref<HistorySamples | null>(null);
const status = ref<CollectorStatus | null>(null);
const loading = ref(false);
const error = ref<string | null>(null);

const ACCENT = "#3987e5";
const ACCENT_2 = "#199e70";

const badKinds = new Set(["unreachable", "reboot_detected", "dish_outage", "alert_raised"]);

function sinceUnixMs(): number {
  const span = RANGE_MS[range.value];
  return span === 0 ? 0 : Date.now() - span;
}

async function load() {
  loading.value = true;
  error.value = null;
  try {
    const [evs, st] = await Promise.all([
      invoke<HistoryEvent[]>("query_events", {
        source: sourceFilter.value === "all" ? null : sourceFilter.value,
        sinceUnixMs: sinceUnixMs(),
        limit: 500,
      }),
      invoke<CollectorStatus>("get_collector_status"),
    ]);
    events.value = evs;
    status.value = st;

    if (sourceFilter.value !== "all") {
      samples.value = await invoke<HistorySamples>("query_samples", {
        source: sourceFilter.value,
        sinceUnixMs: sinceUnixMs(),
        maxPoints: 300,
      });
    } else {
      samples.value = null;
    }
  } catch (e) {
    error.value = e instanceof Error ? e.message : String(e);
  } finally {
    loading.value = false;
  }
}

function formatWhen(tsMs: number) {
  return new Date(tsMs).toLocaleString();
}

function kindLabel(kind: string) {
  const key = `incidents.kind.${kind}`;
  const translated = t(key);
  return translated === key ? kind : translated;
}

const latencyPoints = computed(() => samples.value?.points.map((p) => p.latencyMs ?? 0) ?? []);
const dropPoints = computed(() => samples.value?.points.map((p) => p.pingDropPercent ?? 0) ?? []);

watch([range, sourceFilter], load);
onMounted(load);
</script>

<template>
  <section class="panel">
    <div class="tab-head">
      <h2>{{ t("incidents.title") }}</h2>
      <button @click="load" :disabled="loading">
        {{ loading ? t("history.refreshing") : t("history.refresh") }}
      </button>
    </div>
    <p class="meta" v-if="status">
      {{ t("incidents.collectorStatus", { samples: status.sampleCount, events: status.eventCount }) }}
      <span v-if="!status.running" class="warn">{{ t("incidents.collectorPaused") }}</span>
    </p>
    <p v-if="error" class="bad">{{ error }}</p>

    <div class="controls">
      <div class="control-group">
        <button
          v-for="r in (['24h', '3d', '7d', 'all'] as RangeKey[])"
          :key="r"
          :class="{ active: range === r }"
          @click="range = r"
        >
          {{ t(`incidents.range.${r}`) }}
        </button>
      </div>
      <div class="control-group">
        <button
          v-for="s in (['all', 'dish', 'router'] as const)"
          :key="s"
          :class="{ active: sourceFilter === s }"
          @click="sourceFilter = s"
        >
          {{ t(`incidents.source.${s}`) }}
        </button>
      </div>
    </div>

    <div class="charts-grid" v-if="samples">
      <div class="chart-block">
        <p class="meta">{{ t("history.latencyChart") }}</p>
        <LineChart :series="[{ name: t('history.latencyChart'), color: ACCENT, points: latencyPoints }]" unit="ms" />
      </div>
      <div class="chart-block">
        <p class="meta">{{ t("history.pingDropChart") }}</p>
        <LineChart :series="[{ name: t('history.pingDropChart'), color: ACCENT_2, points: dropPoints }]" unit="%" />
      </div>
    </div>

    <h3>{{ t("incidents.timeline") }}</h3>
    <p v-if="!events.length" class="meta">{{ t("incidents.noEvents") }}</p>
    <div class="table-wrap" v-else>
      <table>
        <thead>
          <tr>
            <th>{{ t("incidents.colWhen") }}</th>
            <th>{{ t("incidents.colSource") }}</th>
            <th>{{ t("incidents.colKind") }}</th>
            <th>{{ t("incidents.colDetail") }}</th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="(e, i) in events" :key="i">
            <td>{{ formatWhen(e.tsUnixMs) }}</td>
            <td>{{ e.source }}</td>
            <td :class="badKinds.has(e.kind) ? 'bad' : 'good'">{{ kindLabel(e.kind) }}</td>
            <td>{{ e.detail }}</td>
          </tr>
        </tbody>
      </table>
    </div>
  </section>
</template>

<style scoped>
.tab-head {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.controls {
  display: flex;
  flex-wrap: wrap;
  justify-content: space-between;
  gap: 0.6rem;
  margin: 0.75rem 0 1rem;
}

.control-group {
  display: flex;
  flex-wrap: wrap;
  gap: 0.35rem;
}

.control-group button.active {
  background: var(--accent);
  border-color: var(--accent);
  color: #fff;
}

.charts-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(260px, 1fr));
  gap: 1rem;
  margin-bottom: 1rem;
}

.chart-block {
  min-width: 0;
}

h3 {
  margin: 1rem 0 0.5rem;
  font-size: 0.95rem;
}

.table-wrap {
  overflow-x: auto;
}

table {
  width: 100%;
  border-collapse: collapse;
  font-size: 0.86rem;
}

th,
td {
  text-align: left;
  padding: 0.45rem 0.6rem;
  border-bottom: 1px solid var(--line);
  white-space: nowrap;
}

th {
  color: var(--ink-soft);
  font-weight: 600;
}
</style>
