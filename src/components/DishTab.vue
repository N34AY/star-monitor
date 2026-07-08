<script setup lang="ts">
import { ref } from "vue";
import DishInfoTab from "./DishInfoTab.vue";
import ObstructionMapTab from "./ObstructionMapTab.vue";
import DishConfigTab from "./DishConfigTab.vue";
import DishHistoryTab from "./DishHistoryTab.vue";
import type { ActionDef, ObstructionDetail, StarlinkSnapshot } from "../lib/types";
import { t } from "../lib/i18n";

defineProps<{
  dishAddress: string;
  snapshot: StarlinkSnapshot | null;
  actionLoading: string | null;
  obstruction?: ObstructionDetail | null;
  clearAction?: ActionDef | null;
  clearing: boolean;
}>();

const emit = defineEmits<{ (e: "run-action", action: ActionDef): void; (e: "clear"): void }>();

type SubTab = "info" | "obstruction" | "config" | "history";
const activeSub = ref<SubTab>("info");
const subTabs: SubTab[] = ["info", "obstruction", "config", "history"];

const obstructionRef = ref<InstanceType<typeof ObstructionMapTab> | null>(null);
defineExpose({
  reloadObstructionMap: () => obstructionRef.value?.loadMap(),
});
</script>

<template>
  <nav class="sub-tab-bar">
    <button
      v-for="tab in subTabs"
      :key="tab"
      :class="{ active: activeSub === tab }"
      @click="activeSub = tab"
    >
      {{ t(`subtabs.${tab}`) }}
    </button>
  </nav>

  <DishInfoTab
    v-if="activeSub === 'info'"
    :snapshot="snapshot"
    :action-loading="actionLoading"
    @run-action="emit('run-action', $event)"
  />

  <ObstructionMapTab
    v-if="activeSub === 'obstruction'"
    ref="obstructionRef"
    :dish-address="dishAddress"
    :obstruction="obstruction"
    :clear-action="clearAction"
    :clearing="clearing"
    @clear="emit('clear')"
  />

  <DishConfigTab v-if="activeSub === 'config'" :dish-address="dishAddress" :config="snapshot?.overview?.config" />

  <DishHistoryTab v-if="activeSub === 'history'" :dish-address="dishAddress" />
</template>

<style scoped>
.sub-tab-bar {
  display: flex;
  flex-wrap: wrap;
  gap: 0.35rem;
  background: var(--panel);
  border: 1px solid var(--line);
  border-radius: 12px;
  padding: 0.3rem;
  margin-bottom: 1rem;
}

.sub-tab-bar button {
  background: transparent;
  border: 1px solid transparent;
  color: var(--ink-soft);
  padding: 0.35rem 0.7rem;
  font-size: 0.88rem;
}

.sub-tab-bar button.active {
  background: rgba(255, 255, 255, 0.08);
  color: var(--ink);
  border-color: var(--line);
}
</style>
