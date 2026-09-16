<script setup lang="ts">
import { locale, setLocale, t, type Locale } from "../lib/i18n";

const dishAddress = defineModel<string>("dishAddress", { required: true });
const dishEnabled = defineModel<boolean>("dishEnabled", { required: true });
const routerAddress = defineModel<string>("routerAddress", { required: true });
const routerEnabled = defineModel<boolean>("routerEnabled", { required: true });
const includeLocation = defineModel<boolean>("includeLocation", { required: true });
const includeGnss = defineModel<boolean>("includeGnss", { required: true });
const autoRefreshEnabled = defineModel<boolean>("autoRefreshEnabled", { required: true });
const refreshEverySeconds = defineModel<number>("refreshEverySeconds", { required: true });
const collectorPollIntervalS = defineModel<number>("collectorPollIntervalS", { required: true });
const collectorRetentionDays = defineModel<number>("collectorRetentionDays", { required: true });

defineProps<{ loading: boolean }>();
const emit = defineEmits<{
  (e: "refresh-now"): void;
  (e: "reset-defaults"): void;
  (e: "clear-history"): void;
}>();

function onLanguageChange(event: Event) {
  setLocale((event.target as HTMLSelectElement).value as Locale);
}
</script>

<template>
  <section class="panel">
    <h2>{{ t("settings.connection") }}</h2>

    <div class="field">
      <label for="dishAddress">{{ t("settings.dishAddress") }}</label>
      <input
        id="dishAddress"
        v-model="dishAddress"
        placeholder="http://192.168.100.1:9200"
        :disabled="loading"
      />
    </div>

    <div class="field">
      <label>
        <input type="checkbox" v-model="dishEnabled" />
        {{ t("settings.enableDishMonitoring") }}
      </label>
      <p class="hint">{{ t("settings.enableDishHint") }}</p>
    </div>

    <div class="field">
      <label>
        <input type="checkbox" v-model="routerEnabled" />
        {{ t("settings.enableRouterMonitoring") }}
      </label>
      <p class="hint">{{ t("settings.enableRouterHint") }}</p>
    </div>

    <div class="field" v-if="routerEnabled">
      <label for="routerAddress">{{ t("settings.routerAddress") }}</label>
      <input
        id="routerAddress"
        v-model="routerAddress"
        placeholder="http://192.168.1.1:9000"
      />
    </div>

    <h2>{{ t("settings.data") }}</h2>

    <div class="toggles">
      <label>
        <input type="checkbox" v-model="includeLocation" />
        {{ t("settings.includeLocation") }}
      </label>
      <label>
        <input type="checkbox" v-model="includeGnss" />
        {{ t("settings.includeGnss") }}
      </label>
    </div>

    <h2>{{ t("settings.refresh") }}</h2>

    <div class="live-controls">
      <label>
        <input type="checkbox" v-model="autoRefreshEnabled" />
        {{ t("settings.autoRefresh") }}
      </label>
      <label>
        {{ t("settings.every") }}
        <input type="number" v-model.number="refreshEverySeconds" min="2" max="60" step="1" />
        {{ t("settings.sec") }}
      </label>
    </div>

    <h2>{{ t("settings.historyTitle") }}</h2>
    <p class="hint">{{ t("settings.historyHint") }}</p>

    <div class="live-controls">
      <label>
        {{ t("settings.every") }}
        <input type="number" v-model.number="collectorPollIntervalS" min="2" max="300" step="1" />
        {{ t("settings.sec") }}
      </label>
      <label>
        {{ t("settings.keepFor") }}
        <input type="number" v-model.number="collectorRetentionDays" min="1" max="30" step="1" />
        {{ t("settings.days") }}
      </label>
    </div>

    <div class="settings-actions">
      <button @click="emit('clear-history')">{{ t("settings.clearHistory") }}</button>
    </div>

    <h2>{{ t("settings.language") }}</h2>

    <div class="field">
      <select :value="locale" @change="onLanguageChange">
        <option value="en">{{ t("settings.languageEnglish") }}</option>
        <option value="ua">{{ t("settings.languageUkrainian") }}</option>
      </select>
    </div>

    <div class="settings-actions">
      <button class="primary" @click="emit('refresh-now')" :disabled="loading || !dishAddress">
        {{ loading ? t("common.refreshing") : t("settings.refreshNow") }}
      </button>
      <button @click="emit('reset-defaults')">{{ t("settings.resetDefaults") }}</button>
    </div>
    <p class="hint">{{ t("settings.persistHint") }}</p>
  </section>
</template>

<style scoped>
h2 {
  margin: 1.25rem 0 0.6rem;
}

h2:first-child {
  margin-top: 0;
}

.field {
  margin-bottom: 0.9rem;
}

.field label {
  display: block;
  font-weight: 600;
  margin-bottom: 0.3rem;
}


.hint {
  margin: 0.3rem 0 0;
  font-size: 0.82rem;
  color: var(--ink-soft);
  font-weight: 400;
}

.toggles,
.live-controls {
  display: flex;
  flex-wrap: wrap;
  gap: 1rem;
  color: var(--ink-soft);
}

.live-controls input[type="number"] {
  width: 5rem;
  margin: 0 0.4rem;
  padding: 0.2rem 0.45rem;
}

.settings-actions {
  margin-top: 1rem;
  display: flex;
  flex-wrap: wrap;
  gap: 0.6rem;
}
</style>
