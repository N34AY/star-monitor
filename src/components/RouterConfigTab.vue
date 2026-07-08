<script setup lang="ts">
import { ref, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import type { NetworkPatch, WifiConfigInfo, WifiConfigPatch } from "../lib/types";
import { t } from "../lib/i18n";

const props = defineProps<{
  routerAddress: string;
  config?: WifiConfigInfo | null;
}>();

const BANDS = [
  { value: "RF_2GHZ", label: "2.4GHz" },
  { value: "RF_5GHZ", label: "5GHz" },
  { value: "RF_5GHZ_HIGH", label: "5GHz (High)" },
] as const;
const editBand = ref<string>("RF_2GHZ");

type ScalarForm = {
  channel2ghz: number;
  channel5ghz: number;
  dfsEnabled: boolean;
  secureDns: boolean;
  bypassMode: boolean;
};

const scalarForm = ref<ScalarForm | null>(null);
const scalarOriginal = ref<ScalarForm | null>(null);
const loaded = ref(false);

const networkForm = ref({ ssid: "", password: "", hidden: false, disabled: false });
const originalNetwork = ref<{ ssid: string; hidden: boolean; disabled: boolean } | null>(null);

const saving = ref(false);
const message = ref<{ text: string; ok: boolean } | null>(null);

function loadNetworkForBand(band: string, cfg?: WifiConfigInfo | null) {
  const source = cfg ?? props.config;
  const net = source?.networks.find((n) => n.band === band);
  networkForm.value = { ssid: net?.ssid ?? "", password: "", hidden: net?.hidden ?? false, disabled: net?.disabled ?? false };
  originalNetwork.value = net ? { ssid: net.ssid, hidden: net.hidden, disabled: net.disabled } : null;
}

watch(
  () => props.config,
  (cfg) => {
    if (cfg && !loaded.value) {
      scalarForm.value = {
        channel2ghz: cfg.channel2ghz,
        channel5ghz: cfg.channel5ghz,
        dfsEnabled: cfg.dfsEnabled,
        secureDns: cfg.secureDns,
        bypassMode: cfg.bypassMode,
      };
      scalarOriginal.value = { ...scalarForm.value };
      loaded.value = true;
      loadNetworkForBand(editBand.value, cfg);
    }
  },
  { immediate: true }
);

watch(editBand, (band) => loadNetworkForBand(band));

async function save() {
  const patch: WifiConfigPatch = {};
  const s = scalarForm.value;
  const so = scalarOriginal.value;
  if (s && so) {
    if (s.channel2ghz !== so.channel2ghz) patch.channel2ghz = s.channel2ghz;
    if (s.channel5ghz !== so.channel5ghz) patch.channel5ghz = s.channel5ghz;
    if (s.dfsEnabled !== so.dfsEnabled) patch.dfsEnabled = s.dfsEnabled;
    if (s.secureDns !== so.secureDns) patch.secureDns = s.secureDns;
    if (s.bypassMode !== so.bypassMode) patch.bypassMode = s.bypassMode;
  }

  const netPatch: NetworkPatch = { band: editBand.value };
  let hasNetChange = false;
  const orig = originalNetwork.value;
  if (networkForm.value.ssid !== (orig?.ssid ?? "")) {
    netPatch.ssid = networkForm.value.ssid;
    hasNetChange = true;
  }
  if (networkForm.value.password) {
    netPatch.password = networkForm.value.password;
    hasNetChange = true;
  }
  if (networkForm.value.hidden !== (orig?.hidden ?? false)) {
    netPatch.hidden = networkForm.value.hidden;
    hasNetChange = true;
  }
  if (networkForm.value.disabled !== (orig?.disabled ?? false)) {
    netPatch.disabled = networkForm.value.disabled;
    hasNetChange = true;
  }
  if (hasNetChange) patch.network = netPatch;

  if (Object.keys(patch).length === 0) {
    message.value = { text: t("router.noChanges"), ok: true };
    return;
  }
  if (!window.confirm(t("router.saveConfirm"))) return;

  saving.value = true;
  message.value = null;
  try {
    await invoke("apply_wifi_config", { routerAddress: props.routerAddress, patch });
    message.value = { text: t("router.saveSuccess"), ok: true };
    if (s) scalarOriginal.value = { ...s };
    if (hasNetChange) {
      originalNetwork.value = {
        ssid: networkForm.value.ssid,
        hidden: networkForm.value.hidden,
        disabled: networkForm.value.disabled,
      };
      networkForm.value.password = "";
    }
  } catch (e) {
    message.value = { text: e instanceof Error ? e.message : String(e), ok: false };
  } finally {
    saving.value = false;
  }
}
</script>

<template>
  <section class="panel" v-if="scalarForm">
    <h2>{{ t("router.wifiConfiguration") }}</h2>

    <div class="config-grid">
      <div class="field">
        <label>{{ t("router.editNetwork") }}</label>
        <select v-model="editBand">
          <option v-for="b in BANDS" :key="b.value" :value="b.value">{{ b.label }}</option>
        </select>
      </div>
      <div class="field">
        <label>{{ t("router.ssid") }}</label>
        <input v-model="networkForm.ssid" />
      </div>
      <div class="field">
        <label>{{ t("router.password") }}</label>
        <input type="password" v-model="networkForm.password" autocomplete="new-password" />
      </div>
      <div class="field checkbox-field">
        <label><input type="checkbox" v-model="networkForm.hidden" /> {{ t("router.hiddenNetwork") }}</label>
      </div>
      <div class="field checkbox-field">
        <label><input type="checkbox" v-model="networkForm.disabled" /> {{ t("router.networkDisabled") }}</label>
      </div>
      <div class="field">
        <label>{{ t("router.channel2ghz") }}</label>
        <input type="number" min="0" max="255" v-model.number="scalarForm.channel2ghz" />
      </div>
      <div class="field">
        <label>{{ t("router.channel5ghz") }}</label>
        <input type="number" min="0" max="255" v-model.number="scalarForm.channel5ghz" />
      </div>
      <div class="field checkbox-field">
        <label><input type="checkbox" v-model="scalarForm.dfsEnabled" /> {{ t("router.dfsEnabled") }}</label>
      </div>
      <div class="field checkbox-field">
        <label><input type="checkbox" v-model="scalarForm.secureDns" /> {{ t("router.secureDns") }}</label>
      </div>
      <div class="field checkbox-field">
        <label><input type="checkbox" v-model="scalarForm.bypassMode" /> {{ t("router.bypassModeToggle") }}</label>
      </div>
    </div>

    <p v-if="message" :class="message.ok ? 'good' : 'bad'">{{ message.text }}</p>

    <button class="primary" :disabled="saving" @click="save">
      {{ saving ? t("common.sending") : t("router.saveChanges") }}
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
