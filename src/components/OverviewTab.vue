<script setup lang="ts">
import { computed } from "vue";
import type { RouterSnapshot, StarlinkSnapshot } from "../lib/types";
import { formatDuration, formatInt, formatNum, formatPointing } from "../lib/format";
import { t } from "../lib/i18n";

const props = defineProps<{
  snapshot: StarlinkSnapshot | null;
  routerEnabled: boolean;
  routerSnapshot: RouterSnapshot | null;
}>();

const overview = computed(() => props.snapshot?.overview ?? null);
const keyMetrics = computed(() => overview.value?.keyMetrics ?? null);

const fetchedAtText = computed(() => {
  if (!props.snapshot) return "-";
  return new Date(props.snapshot.fetchedAtUnixS * 1000).toLocaleTimeString();
});

const okCount = computed(() => props.snapshot?.sections.filter((s) => s.ok).length ?? 0);
const sectionCount = computed(() => props.snapshot?.sections.length ?? 0);

const keyMetricCards = computed(() => {
  const m = keyMetrics.value;
  if (!m) return [];
  return [
    { label: t("metrics.downlink"), value: formatNum(m.downlinkMbps, 2), unit: "Mbps" },
    { label: t("metrics.uplink"), value: formatNum(m.uplinkMbps, 2), unit: "Mbps" },
    { label: t("metrics.latency"), value: formatNum(m.latencyMs, 1), unit: "ms" },
    { label: t("metrics.packetLoss"), value: formatNum(m.pingDropPercent, 2), unit: "%" },
    { label: t("metrics.obstruction"), value: formatNum(m.obstructionPercent, 2), unit: "%" },
    { label: t("metrics.gpsSats"), value: formatInt(m.gpsSats), unit: "" },
    { label: t("metrics.uptime"), value: formatDuration(m.uptimeS), unit: "" },
    { label: t("metrics.pointing"), value: formatPointing(m.boresightAzimuthDeg, m.boresightElevationDeg), unit: "" },
  ];
});
</script>

<template>
  <section class="status-grid">
    <article class="panel stat">
      <h2>{{ t("overview.dishLink") }}</h2>
      <p :class="snapshot?.dishUp ? 'good' : 'bad'">
        {{ snapshot?.dishUp ? t("common.connected") : t("common.unavailable") }}
      </p>
    </article>

    <article class="panel stat">
      <h2>{{ t("overview.sections") }}</h2>
      <p>{{ t("overview.sectionsHealthy", { ok: okCount, total: sectionCount }) }}</p>
    </article>

    <article class="panel stat">
      <h2>{{ t("overview.lastFetch") }}</h2>
      <p>{{ fetchedAtText }}</p>
    </article>

    <article class="panel stat" v-if="routerEnabled">
      <h2>{{ t("overview.routerLink") }}</h2>
      <p :class="routerSnapshot ? 'good' : 'bad'">
        {{ routerSnapshot ? t("common.connected") : t("common.unavailable") }}
      </p>
    </article>
  </section>

  <section class="panel" v-if="snapshot">
    <h2>{{ t("overview.keyMetrics") }}</h2>
    <p class="meta">{{ t("overview.software", { version: keyMetrics?.softwareVersion ?? "-" }) }}</p>
    <div class="metrics-grid">
      <article class="metric-card" v-for="metric in keyMetricCards" :key="metric.label">
        <span>{{ metric.label }}</span>
        <strong>{{ metric.value }}</strong>
        <small>{{ metric.unit }}</small>
      </article>
    </div>
  </section>

  <section class="panel" v-if="overview">
    <h2>{{ t("overview.alerts") }}</h2>
    <div class="alerts" v-if="overview.alerts.length">
      <span class="chip bad" v-for="a in overview.alerts" :key="a">{{ a }}</span>
    </div>
    <p v-else class="good meta">{{ t("overview.noActiveAlerts") }}</p>
  </section>

  <section class="panel" v-if="routerEnabled && routerSnapshot?.overview">
    <h2>{{ t("overview.routerSummary") }}</h2>
    <div class="metrics-grid">
      <article class="metric-card">
        <span>{{ t("overview.wifiPing") }}</span>
        <strong>{{ formatNum(routerSnapshot.overview.pingLatencyMs, 1) }}</strong>
        <small>ms</small>
      </article>
      <article class="metric-card">
        <span>{{ t("overview.clients") }}</span>
        <strong>{{ routerSnapshot.overview.clients.length }}</strong>
      </article>
      <article class="metric-card">
        <span>{{ t("overview.alerts") }}</span>
        <strong :class="routerSnapshot.overview.alerts.length ? 'bad' : 'good'">
          {{ routerSnapshot.overview.alerts.length || t("common.none") }}
        </strong>
      </article>
    </div>
  </section>
</template>

<style scoped>
.status-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(160px, 1fr));
  gap: 0.8rem;
  margin-bottom: 1rem;
}

.stat h2 {
  margin: 0 0 0.4rem;
  font-size: 0.95rem;
  color: var(--ink-soft);
}

.stat p {
  margin: 0;
  font-size: 1.15rem;
  font-weight: 700;
}

.alerts {
  display: flex;
  flex-wrap: wrap;
  gap: 0.4rem;
}
</style>
