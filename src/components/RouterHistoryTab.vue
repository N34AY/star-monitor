<script setup lang="ts">
import { onMounted, ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import LineChart from "./LineChart.vue";
import { t } from "../lib/i18n";
import type { RouterHistory } from "../lib/types";

const props = defineProps<{
  routerAddress: string;
}>();

const history = ref<RouterHistory | null>(null);
const loading = ref(false);
const error = ref<string | null>(null);

const ACCENT = "#3987e5";

async function load() {
  loading.value = true;
  error.value = null;
  try {
    history.value = await invoke<RouterHistory>("fetch_router_history", { routerAddress: props.routerAddress });
  } catch (e) {
    error.value = e instanceof Error ? e.message : String(e);
  } finally {
    loading.value = false;
  }
}

onMounted(load);
</script>

<template>
  <section class="panel">
    <div class="tab-head">
      <h2>{{ t("history.routerTitle") }}</h2>
      <button @click="load" :disabled="loading">
        {{ loading ? t("history.refreshing") : t("history.refresh") }}
      </button>
    </div>
    <p v-if="error" class="bad">{{ error }}</p>

    <div class="charts-grid">
      <div class="chart-block">
        <p class="meta">{{ t("history.latencyChart") }}</p>
        <LineChart
          :series="[{ name: t('history.latencyChart'), color: ACCENT, points: history?.pingLatencyMs ?? [] }]"
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
}

.chart-block {
  min-width: 0;
}
</style>
