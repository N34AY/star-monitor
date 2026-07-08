<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import ObstructionMapCanvas from "./ObstructionMapCanvas.vue";
import ActionCard from "./ActionCard.vue";
import { formatDuration, formatNum } from "../lib/format";
import { t } from "../lib/i18n";
import type { ActionDef, ObstructionDetail, ObstructionMap } from "../lib/types";

const props = defineProps<{
  dishAddress: string;
  obstruction?: ObstructionDetail | null;
  clearAction?: ActionDef | null;
  clearing: boolean;
}>();

const emit = defineEmits<{ (e: "clear"): void }>();

const map = ref<ObstructionMap | null>(null);
const loading = ref(false);
const error = ref<string | null>(null);
let timer: number | null = null;

async function loadMap(opts: { silent?: boolean } = {}) {
  if (loading.value) return;
  loading.value = true;
  if (!opts.silent) error.value = null;

  try {
    map.value = await invoke<ObstructionMap>("fetch_dish_obstruction_map", {
      dishAddress: props.dishAddress,
    });
    error.value = null;
  } catch (e) {
    error.value = e instanceof Error ? e.message : String(e);
  } finally {
    loading.value = false;
  }
}

defineExpose({ loadMap });

onMounted(() => {
  void loadMap();
  timer = window.setInterval(() => void loadMap({ silent: true }), 10000);
});

onBeforeUnmount(() => {
  if (timer !== null) window.clearInterval(timer);
});
</script>

<template>
  <section class="panel">
    <div class="tab-head">
      <h2>{{ t("obstruction.title") }}</h2>
      <div class="tab-actions">
        <button @click="loadMap()" :disabled="loading">{{ loading ? t("common.refreshing") : t("obstruction.refreshMap") }}</button>
      </div>
    </div>

    <p v-if="error" class="bad">{{ error }}</p>

    <div class="layout">
      <ObstructionMapCanvas :map="map" />

      <div class="stats" v-if="obstruction">
        <div class="stat-row">
          <span>{{ t("obstruction.obstructed") }}</span>
          <strong :class="obstruction.currentlyObstructed ? 'bad' : 'good'">
            {{ obstruction.currentlyObstructed ? t("obstruction.obstructedYesNow") : t("common.no") }}
          </strong>
        </div>
        <div class="stat-row">
          <span>{{ t("obstruction.fractionObstructed") }}</span>
          <strong>{{ formatNum(obstruction.fractionObstructedPercent, 2) }}%</strong>
        </div>
        <div class="stat-row">
          <span>{{ t("obstruction.timeObstructed") }}</span>
          <strong>{{ formatDuration(obstruction.timeObstructedS) }}</strong>
        </div>
        <div class="stat-row" v-if="obstruction.avgProlongedObstructionDurationS != null">
          <span>{{ t("obstruction.avgProlonged") }}</span>
          <strong>{{ formatNum(obstruction.avgProlongedObstructionDurationS, 1) }}s</strong>
        </div>
        <div class="stat-row" v-if="obstruction.avgProlongedObstructionIntervalS != null">
          <span>{{ t("obstruction.avgInterval") }}</span>
          <strong>{{ formatDuration(obstruction.avgProlongedObstructionIntervalS) }}</strong>
        </div>
        <div class="stat-row">
          <span>{{ t("obstruction.patchesValid") }}</span>
          <strong>{{ obstruction.patchesValid }}</strong>
        </div>
      </div>
    </div>

    <p class="legend">
      <span class="dot clear"></span> {{ t("obstruction.legendClear") }}
      <span class="dot obstructed"></span> {{ t("obstruction.legendObstructed") }}
      <span class="dot unknown"></span> {{ t("obstruction.legendUnknown") }}
      <span class="muted"> {{ t("obstruction.legendNote") }}</span>
    </p>
  </section>

  <section class="panel" v-if="clearAction">
    <h2>{{ t("obstruction.controls") }}</h2>
    <div class="actions-grid">
      <ActionCard
        :action="{ ...clearAction, name: t(`actions.dish.${clearAction.id}.name`) }"
        :description="t(`actions.dish.${clearAction.id}.description`)"
        :current-value="obstruction ? t('obstruction.currentlyObstructedPercent', { percent: formatNum(obstruction.fractionObstructedPercent, 2) }) : null"
        :busy="clearing"
        :disabled="clearing"
        @run="emit('clear')"
      />
    </div>
  </section>
</template>

<style scoped>
.tab-head {
  display: flex;
  justify-content: space-between;
  align-items: center;
  flex-wrap: wrap;
  gap: 0.5rem;
  margin-bottom: 0.5rem;
}

.tab-actions {
  display: flex;
  gap: 0.5rem;
}

.layout {
  display: grid;
  grid-template-columns: minmax(0, 1.4fr) minmax(220px, 1fr);
  gap: 1rem;
  align-items: start;
}

.stats {
  display: grid;
  gap: 0.5rem;
  align-content: start;
  padding-top: 0.5rem;
}

.stat-row {
  display: flex;
  justify-content: space-between;
  gap: 0.75rem;
  border-bottom: 1px solid var(--line);
  padding-bottom: 0.4rem;
  font-size: 0.9rem;
}

.legend {
  display: flex;
  align-items: center;
  gap: 0.4rem;
  flex-wrap: wrap;
  font-size: 0.82rem;
  color: var(--ink-soft);
  margin: 0.75rem 0 0;
}

.dot {
  width: 10px;
  height: 10px;
  border-radius: 50%;
  display: inline-block;
}

.dot.clear {
  background: rgba(255, 255, 255, 0.75);
  border: 1px solid rgba(255, 255, 255, 0.3);
}

.dot.obstructed {
  background: #d03b3b;
}

.dot.unknown {
  background: rgba(255, 255, 255, 0.08);
  border: 1px solid rgba(255, 255, 255, 0.15);
}

@media (max-width: 720px) {
  .layout {
    grid-template-columns: 1fr;
  }
}
</style>
