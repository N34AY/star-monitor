<script setup lang="ts">
import { onMounted, ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import LineChart from "./LineChart.vue";
import { formatDuration } from "../lib/format";
import { t } from "../lib/i18n";
import type { DishHistory } from "../lib/types";

const props = defineProps<{
  dishAddress: string;
}>();

const history = ref<DishHistory | null>(null);
const loading = ref(false);
const error = ref<string | null>(null);

const ACCENT = "#3987e5";
const ACCENT_2 = "#199e70";

async function load() {
  loading.value = true;
  error.value = null;
  try {
    history.value = await invoke<DishHistory>("fetch_dish_history", { dishAddress: props.dishAddress });
  } catch (e) {
    error.value = e instanceof Error ? e.message : String(e);
  } finally {
    loading.value = false;
  }
}

function outageWhen(unixS: number) {
  if (!unixS) return "-";
  return new Date(unixS * 1000).toLocaleString();
}

onMounted(load);
</script>

<template>
  <section class="panel">
    <div class="tab-head">
      <h2>{{ t("history.dishTitle") }}</h2>
      <button @click="load" :disabled="loading">
        {{ loading ? t("history.refreshing") : t("history.refresh") }}
      </button>
    </div>
    <p v-if="error" class="bad">{{ error }}</p>

    <div class="charts-grid">
      <div class="chart-block">
        <p class="meta">{{ t("history.latencyChart") }}</p>
        <LineChart
          :series="[{ name: t('history.latencyChart'), color: ACCENT, points: history?.latencyMs ?? [] }]"
          unit="ms"
        />
      </div>
      <div class="chart-block">
        <p class="meta">{{ t("history.pingDropChart") }}</p>
        <LineChart
          :series="[{ name: t('history.pingDropChart'), color: ACCENT, points: history?.pingDropPercent ?? [] }]"
          unit="%"
        />
      </div>
      <div class="chart-block">
        <p class="meta">{{ t("history.throughputChart") }}</p>
        <LineChart
          :series="[
            { name: t('metrics.downlink'), color: ACCENT, points: history?.downlinkMbps ?? [] },
            { name: t('metrics.uplink'), color: ACCENT_2, points: history?.uplinkMbps ?? [] },
          ]"
          unit="Mbps"
        />
      </div>
      <div class="chart-block" v-if="history?.powerInW.length">
        <p class="meta">{{ t("history.powerChart") }}</p>
        <LineChart
          :series="[{ name: t('history.powerChart'), color: ACCENT, points: history?.powerInW ?? [] }]"
          unit="W"
        />
      </div>
    </div>

    <h3>{{ t("history.outages") }}</h3>
    <p v-if="!history?.outages.length" class="meta">{{ t("history.noOutages") }}</p>
    <div class="table-wrap" v-else>
      <table>
        <thead>
          <tr>
            <th>{{ t("history.outageCause") }}</th>
            <th>{{ t("history.outageWhen") }}</th>
            <th>{{ t("history.outageDuration") }}</th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="(o, i) in history.outages" :key="i">
            <td>{{ o.cause }}</td>
            <td>{{ outageWhen(o.startUnixS) }}</td>
            <td>{{ formatDuration(o.durationS) }}</td>
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
