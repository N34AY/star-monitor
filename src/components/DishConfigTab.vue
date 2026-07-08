<script setup lang="ts">
import { ref, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import type { DishConfigInfo, DishConfigPatch } from "../lib/types";
import { t } from "../lib/i18n";

const props = defineProps<{
  dishAddress: string;
  config?: DishConfigInfo | null;
}>();

type DishConfigForm = {
  snowMeltMode: string;
  powerSaveMode: boolean;
  powerSaveStartMinutes: number;
  powerSaveDurationMinutes: number;
  levelDishMode: string;
  swupdateThreeDayDeferralEnabled: boolean;
  swupdateRebootHour: number;
};

const form = ref<DishConfigForm | null>(null);
const original = ref<DishConfigForm | null>(null);
const loaded = ref(false);
const saving = ref(false);
const message = ref<{ text: string; ok: boolean } | null>(null);

watch(
  () => props.config,
  (cfg) => {
    if (cfg && !loaded.value) {
      form.value = { ...cfg };
      original.value = { ...cfg };
      loaded.value = true;
    }
  },
  { immediate: true }
);

function buildPatch(): DishConfigPatch | null {
  const f = form.value;
  const o = original.value;
  if (!f || !o) return null;

  const patch: DishConfigPatch = {};
  if (f.snowMeltMode !== o.snowMeltMode) patch.snowMeltMode = f.snowMeltMode;
  if (f.powerSaveMode !== o.powerSaveMode) patch.powerSaveMode = f.powerSaveMode;
  if (f.powerSaveStartMinutes !== o.powerSaveStartMinutes) {
    patch.powerSaveStartMinutes = f.powerSaveStartMinutes;
  }
  if (f.powerSaveDurationMinutes !== o.powerSaveDurationMinutes) {
    patch.powerSaveDurationMinutes = f.powerSaveDurationMinutes;
  }
  if (f.levelDishMode !== o.levelDishMode) patch.levelDishMode = f.levelDishMode;
  if (f.swupdateThreeDayDeferralEnabled !== o.swupdateThreeDayDeferralEnabled) {
    patch.swupdateThreeDayDeferralEnabled = f.swupdateThreeDayDeferralEnabled;
  }
  if (f.swupdateRebootHour !== o.swupdateRebootHour) {
    patch.swupdateRebootHour = f.swupdateRebootHour;
  }
  return Object.keys(patch).length > 0 ? patch : null;
}

async function save() {
  const patch = buildPatch();
  if (!patch) {
    message.value = { text: t("overview.noChanges"), ok: true };
    return;
  }
  if (!window.confirm(t("overview.saveConfirm"))) return;

  saving.value = true;
  message.value = null;
  try {
    await invoke("apply_dish_config", { dishAddress: props.dishAddress, patch });
    message.value = { text: t("overview.saveSuccess"), ok: true };
    original.value = form.value ? { ...form.value } : null;
  } catch (e) {
    message.value = { text: e instanceof Error ? e.message : String(e), ok: false };
  } finally {
    saving.value = false;
  }
}
</script>

<template>
  <section class="panel" v-if="form">
    <h2>{{ t("overview.dishConfiguration") }}</h2>

    <div class="config-grid">
      <div class="field">
        <label>{{ t("overview.snowMeltMode") }}</label>
        <select v-model="form.snowMeltMode">
          <option value="AUTO">{{ t("overview.snowMeltAuto") }}</option>
          <option value="ALWAYS_ON">{{ t("overview.snowMeltAlwaysOn") }}</option>
          <option value="ALWAYS_OFF">{{ t("overview.snowMeltAlwaysOff") }}</option>
        </select>
      </div>

      <div class="field">
        <label>{{ t("overview.levelDishMode") }}</label>
        <select v-model="form.levelDishMode">
          <option value="TILT_LIKE_NORMAL">{{ t("overview.levelNormal") }}</option>
          <option value="FORCE_LEVEL">{{ t("overview.levelForceLevel") }}</option>
        </select>
      </div>

      <div class="field checkbox-field">
        <label>
          <input type="checkbox" v-model="form.powerSaveMode" />
          {{ t("overview.powerSaveMode") }}
        </label>
      </div>

      <div class="field">
        <label>{{ t("overview.powerSaveStart") }}</label>
        <input type="number" min="0" max="1439" v-model.number="form.powerSaveStartMinutes" />
      </div>

      <div class="field">
        <label>{{ t("overview.powerSaveDuration") }}</label>
        <input type="number" min="0" max="1439" v-model.number="form.powerSaveDurationMinutes" />
      </div>

      <div class="field checkbox-field">
        <label>
          <input type="checkbox" v-model="form.swupdateThreeDayDeferralEnabled" />
          {{ t("overview.swupdateDeferral") }}
        </label>
      </div>

      <div class="field">
        <label>{{ t("overview.swupdateRebootHour") }}</label>
        <input type="number" min="0" max="23" v-model.number="form.swupdateRebootHour" />
      </div>
    </div>

    <p v-if="message" :class="message.ok ? 'good' : 'bad'">{{ message.text }}</p>

    <button class="primary" :disabled="saving" @click="save">
      {{ saving ? t("common.sending") : t("overview.saveChanges") }}
    </button>
  </section>
  <section class="panel" v-else>
    <p class="meta">{{ t("history.noData") }}</p>
  </section>
</template>

<style scoped>
.config-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
  gap: 0.8rem;
  margin-bottom: 1rem;
}

.field label {
  display: block;
  font-weight: 600;
  margin-bottom: 0.3rem;
  font-size: 0.88rem;
}

.field select {
  width: 100%;
  border: 1px solid var(--line);
  border-radius: 10px;
  padding: 0.6rem 0.7rem;
  font: inherit;
  color: var(--ink);
  background: rgba(255, 255, 255, 0.04);
}

.checkbox-field {
  display: flex;
  align-items: center;
}

.checkbox-field label {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  font-weight: 400;
  margin-bottom: 0;
}
</style>
