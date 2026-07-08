<script setup lang="ts">
import { computed } from "vue";
import ActionCard from "./ActionCard.vue";
import type { ActionDef, StarlinkSnapshot } from "../lib/types";
import { formatDuration, formatNum, titleCaseFromConst } from "../lib/format";
import { t } from "../lib/i18n";

const props = defineProps<{
  snapshot: StarlinkSnapshot | null;
  actionLoading: string | null;
}>();

const emit = defineEmits<{ (e: "run-action", action: ActionDef): void }>();

const overview = computed(() => props.snapshot?.overview ?? null);

// Toggle-pair actions (stow/unstow, gps inhibit on/off) collapse to whichever
// single action is relevant given the dish's current known state, instead of
// always showing both mutually-exclusive buttons at once.
const controlActions = computed(() => {
  const actions = props.snapshot?.actions ?? [];
  const o = overview.value;
  const byId = (id: string) => actions.find((a) => a.id === id);

  const result: ActionDef[] = [];
  for (const action of actions) {
    if (action.id === "clearObstructionMap") continue;

    if (action.id === "stow" || action.id === "unstow") {
      if (result.some((a) => a.id === "stow" || a.id === "unstow")) continue;
      const wantStow = !(o?.stowRequested ?? false);
      const chosen = (wantStow ? byId("stow") : byId("unstow")) ?? action;
      result.push(chosen);
      continue;
    }

    if (action.id === "gpsInhibitOn" || action.id === "gpsInhibitOff") {
      if (result.some((a) => a.id === "gpsInhibitOn" || a.id === "gpsInhibitOff")) continue;
      const wantInhibit = !(o?.gps.inhibited ?? false);
      const chosen = (wantInhibit ? byId("gpsInhibitOn") : byId("gpsInhibitOff")) ?? action;
      result.push(chosen);
      continue;
    }

    result.push(action);
  }
  return result;
});

function actionName(action: ActionDef): string {
  return t(`actions.dish.${action.id}.name`);
}

function actionDescription(action: ActionDef): string {
  return t(`actions.dish.${action.id}.description`);
}

function currentValueFor(actionId: string): string | null {
  const o = overview.value;
  if (!o) return null;
  if (actionId === "gpsInhibitOn" || actionId === "gpsInhibitOff") {
    return t("overview.gpsInhibitCurrent", { state: o.gps.inhibited ? t("overview.on") : t("overview.off") });
  }
  if (actionId === "stow" || actionId === "unstow") {
    return o.stowRequested ? t("overview.stowed") : t("overview.notStowed");
  }
  return null;
}

function formatLastBoot(unixS: number) {
  if (!unixS) return "-";
  return new Date(unixS * 1000).toLocaleString();
}

function formatTimezone(offsetS: number) {
  const sign = offsetS < 0 ? "-" : "+";
  const abs = Math.abs(offsetS);
  const h = Math.floor(abs / 3600);
  const m = Math.floor((abs % 3600) / 60);
  return `UTC${sign}${String(h).padStart(2, "0")}:${String(m).padStart(2, "0")}`;
}

function formatCoord(v: number | null | undefined, digits = 5) {
  if (v == null) return "-";
  return v.toFixed(digits);
}
</script>

<template>
  <section class="panel" v-if="overview">
    <h2>{{ t("overview.status") }}</h2>
    <div class="metrics-grid">
      <article class="metric-card">
        <span>{{ t("overview.disablementCode") }}</span>
        <strong :class="overview.disablementCode === 'OKAY' ? 'good' : 'bad'">{{ overview.disablementCode }}</strong>
      </article>
      <article class="metric-card" v-if="overview.outage">
        <span>{{ t("overview.outageCause") }}</span>
        <strong class="bad">{{ overview.outage.cause }}</strong>
        <small>{{ formatDuration(overview.outage.durationS) }}</small>
      </article>
      <article class="metric-card">
        <span>{{ t("overview.stowRequested") }}</span>
        <strong>{{ overview.stowRequested ? t("common.yes") : t("common.no") }}</strong>
      </article>
      <article class="metric-card">
        <span>{{ t("overview.mobilityClass") }}</span>
        <strong>{{ titleCaseFromConst(overview.mobilityClass) }}</strong>
      </article>
      <article class="metric-card">
        <span>{{ t("overview.softwareUpdate") }}</span>
        <strong>{{ titleCaseFromConst(overview.softwareUpdateState) }}</strong>
      </article>
      <article class="metric-card">
        <span>{{ t("overview.ethernetSpeed") }}</span>
        <strong>{{ overview.ethSpeedMbps > 0 ? overview.ethSpeedMbps : "-" }}</strong>
        <small>{{ overview.ethSpeedMbps > 0 ? "Mbps" : "" }}</small>
      </article>
      <article class="metric-card">
        <span>{{ t("overview.connectedRouters") }}</span>
        <strong>{{ overview.connectedRoutersCount }}</strong>
      </article>
      <article class="metric-card">
        <span>{{ t("overview.rebootReason") }}</span>
        <strong>{{ titleCaseFromConst(overview.rebootReason.replace(/^REBOOT_REASON_/, "")) }}</strong>
      </article>
      <article class="metric-card">
        <span>{{ t("overview.lastBoot") }}</span>
        <strong>{{ formatLastBoot(overview.lastBootUnixS) }}</strong>
      </article>
    </div>
  </section>

  <section class="panel" v-if="overview">
    <h2>{{ t("overview.gpsLocation") }}</h2>
    <div class="metrics-grid">
      <article class="metric-card">
        <span>{{ t("overview.valid") }}</span>
        <strong :class="overview.gps.valid ? 'good' : 'bad'">{{ overview.gps.valid ? t("common.yes") : t("common.no") }}</strong>
      </article>
      <article class="metric-card">
        <span>{{ t("overview.satellites") }}</span>
        <strong>{{ overview.gps.sats }}</strong>
      </article>
      <article class="metric-card">
        <span>{{ t("overview.inhibited") }}</span>
        <strong>{{ overview.gps.inhibited ? t("common.yes") : t("common.no") }}</strong>
      </article>
      <article class="metric-card">
        <span>{{ t("overview.noSatsAfterTtff") }}</span>
        <strong>{{ overview.gps.noSatsAfterTtff ? t("common.yes") : t("common.no") }}</strong>
      </article>
      <article class="metric-card">
        <span>{{ t("overview.timezone") }}</span>
        <strong>{{ formatTimezone(overview.utcOffsetS) }}</strong>
      </article>
      <article class="metric-card" v-if="overview.countryCode">
        <span>{{ t("overview.countryCode") }}</span>
        <strong>{{ overview.countryCode }}</strong>
      </article>
      <template v-if="overview.location">
        <article class="metric-card">
          <span>{{ t("overview.latitude") }}</span>
          <strong>{{ formatCoord(overview.location.lat) }}</strong>
        </article>
        <article class="metric-card">
          <span>{{ t("overview.longitude") }}</span>
          <strong>{{ formatCoord(overview.location.lon) }}</strong>
        </article>
        <article class="metric-card">
          <span>{{ t("overview.altitude") }}</span>
          <strong>{{ formatNum(overview.location.altM, 1) }}</strong>
          <small>m</small>
        </article>
        <article class="metric-card">
          <span>{{ t("overview.positionSource") }}</span>
          <strong>{{ titleCaseFromConst(overview.location.source) }}</strong>
        </article>
        <article class="metric-card">
          <span>{{ t("overview.accuracy") }}</span>
          <strong>{{ formatNum(overview.location.accuracyM, 1) }}</strong>
          <small>m</small>
        </article>
      </template>
    </div>
  </section>

  <section class="panel" v-if="snapshot">
    <h2>{{ t("overview.controls") }}</h2>
    <div class="actions-grid">
      <ActionCard
        v-for="action in controlActions"
        :key="action.id"
        :action="{ ...action, name: actionName(action) }"
        :description="actionDescription(action)"
        :current-value="currentValueFor(action.id)"
        :busy="actionLoading === action.id"
        :disabled="actionLoading !== null"
        @run="emit('run-action', action)"
      />
    </div>
  </section>
</template>
