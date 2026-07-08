<script setup lang="ts">
import { ref } from "vue";
import RouterInfoTab from "./RouterInfoTab.vue";
import RouterConfigTab from "./RouterConfigTab.vue";
import RouterHistoryTab from "./RouterHistoryTab.vue";
import type { ActionDef, RouterSnapshot } from "../lib/types";
import { t } from "../lib/i18n";

defineProps<{
  routerEnabled: boolean;
  routerAddress: string;
  routerSnapshot: RouterSnapshot | null;
  routerError: string | null;
  actionLoading: string | null;
}>();

const emit = defineEmits<{ (e: "run-action", action: ActionDef): void }>();

type SubTab = "info" | "config" | "history";
const activeSub = ref<SubTab>("info");
const subTabs: SubTab[] = ["info", "config", "history"];
</script>

<template>
  <section class="panel" v-if="!routerEnabled">
    <h2>{{ t("router.title") }}</h2>
    <p class="meta">{{ t("router.disabledMessage") }}</p>
  </section>

  <section class="panel" v-else-if="!routerSnapshot || !routerSnapshot.overview">
    <h2>{{ t("router.title") }}</h2>
    <p class="warn">{{ t("router.notDetected", { address: routerAddress }) }}</p>
    <p v-if="routerError" class="meta">{{ routerError }}</p>
  </section>

  <template v-else>
    <p v-if="routerError" class="warn meta panel">
      {{ t("router.becameUnreachable", { error: routerError }) }}
    </p>

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

    <RouterInfoTab
      v-if="activeSub === 'info'"
      :router-snapshot="routerSnapshot"
      :action-loading="actionLoading"
      @run-action="emit('run-action', $event)"
    />

    <RouterConfigTab
      v-if="activeSub === 'config'"
      :router-address="routerAddress"
      :config="routerSnapshot.overview.config"
    />

    <RouterHistoryTab v-if="activeSub === 'history'" :router-address="routerAddress" />
  </template>
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
