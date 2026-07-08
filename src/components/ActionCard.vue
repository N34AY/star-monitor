<script setup lang="ts">
import type { ActionDef } from "../lib/types";
import { t } from "../lib/i18n";

defineProps<{
  action: ActionDef;
  description?: string;
  currentValue?: string | null;
  busy: boolean;
  disabled: boolean;
}>();

const emit = defineEmits<{ (e: "run", action: ActionDef): void }>();
</script>

<template>
  <article class="action-card">
    <div class="action-head">
      <h3>{{ action.name }}</h3>
      <span v-if="currentValue" class="chip current">{{ currentValue }}</span>
    </div>
    <p v-if="description" class="action-desc">{{ description }}</p>
    <button :disabled="disabled" @click="emit('run', action)">
      {{ busy ? t("common.sending") : action.name }}
    </button>
  </article>
</template>

<style scoped>
.action-card {
  border: 1px solid var(--line);
  border-radius: 12px;
  background: rgba(255, 255, 255, 0.03);
  padding: 0.85rem;
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}

.action-head {
  display: flex;
  flex-wrap: wrap;
  justify-content: space-between;
  align-items: flex-start;
  gap: 0.4rem 0.5rem;
}

.action-head h3 {
  font-size: 0.95rem;
  flex: 1 1 auto;
  min-width: 0;
}

.chip.current {
  background: rgba(255, 255, 255, 0.08);
  color: var(--ink-soft);
  white-space: normal;
  flex-shrink: 0;
}

.action-desc {
  margin: 0;
  color: var(--ink-soft);
  font-size: 0.82rem;
  flex: 1;
}

.action-card button {
  align-self: flex-start;
}
</style>
