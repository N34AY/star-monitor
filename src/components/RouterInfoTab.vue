<script setup lang="ts">
import { computed } from "vue";
import ActionCard from "./ActionCard.vue";
import type { ActionDef, RouterSnapshot } from "../lib/types";
import { formatDuration, formatNum } from "../lib/format";
import { t } from "../lib/i18n";

const props = defineProps<{
  routerSnapshot: RouterSnapshot;
  actionLoading: string | null;
}>();

const emit = defineEmits<{ (e: "run-action", action: ActionDef): void }>();

const overview = computed(() => props.routerSnapshot.overview!);
const softwareVersion = computed(() => overview.value.softwareVersion ?? "-");

function actionName(action: ActionDef): string {
  return t(`actions.router.${action.id}.name`);
}

function actionDescription(action: ActionDef): string {
  return t(`actions.router.${action.id}.description`);
}

function rebootFromUpdateText() {
  const info = overview.value.rebootFromUpdate;
  if (!info) return null;
  const when = info.lastOccurredUnixS ? new Date(info.lastOccurredUnixS * 1000).toLocaleString() : "-";
  return t("router.rebootFromUpdateValue", { count: info.count, when });
}
</script>

<template>
  <section class="panel">
    <h2>{{ t("router.title") }}</h2>
    <p class="meta">{{ t("router.softwareUptime", { version: softwareVersion, uptime: formatDuration(overview.uptimeS) }) }}</p>

    <div class="metrics-grid">
      <article class="metric-card">
        <span>{{ t("router.wifiPing") }}</span>
        <strong>{{ formatNum(overview.pingLatencyMs, 1) }}</strong>
        <small>ms, {{ formatNum(overview.pingDropPercent, 2) }}% drop</small>
      </article>
      <article class="metric-card">
        <span>{{ t("router.dishPing") }}</span>
        <strong>{{ formatNum(overview.dishPingLatencyMs, 1) }}</strong>
        <small>ms, {{ formatNum(overview.dishPingDropPercent, 2) }}% drop</small>
      </article>
      <article class="metric-card">
        <span>{{ t("router.connectedClients") }}</span>
        <strong>{{ overview.clients.length }}</strong>
        <small></small>
      </article>
      <article class="metric-card">
        <span>{{ t("router.setup") }}</span>
        <strong :class="overview.setupComplete ? 'good' : 'warn'">
          {{ overview.setupComplete ? t("router.complete") : t("router.incomplete") }}
        </strong>
        <small>{{ overview.bypassMode ? t("router.bypassMode") : "" }}</small>
      </article>
    </div>

    <div class="alerts" v-if="overview.alerts.length">
      <span class="chip bad" v-for="a in overview.alerts" :key="a">{{ a }}</span>
    </div>
    <p v-else class="good meta">{{ t("router.noAlerts") }}</p>

    <p class="meta" v-if="overview.ssids.length">{{ t("router.ssids", { ssids: overview.ssids.join(", ") }) }}</p>
    <p class="meta" v-if="rebootFromUpdateText()">{{ t("router.rebootFromUpdate") }}: {{ rebootFromUpdateText() }}</p>

    <div class="actions-grid">
      <ActionCard
        v-for="action in routerSnapshot.actions"
        :key="action.id"
        :action="{ ...action, name: actionName(action) }"
        :description="actionDescription(action)"
        :busy="actionLoading === action.id"
        :disabled="actionLoading !== null"
        @run="emit('run-action', action)"
      />
    </div>
  </section>

  <section class="panel" v-if="overview.clients.length">
    <h2>{{ t("router.connectedClientsTitle") }}</h2>
    <div class="table-wrap">
      <table>
        <thead>
          <tr>
            <th>{{ t("router.colName") }}</th>
            <th>{{ t("router.colIp") }}</th>
            <th>{{ t("router.colBand") }}</th>
            <th>{{ t("router.colSignal") }}</th>
            <th>{{ t("router.colSnr") }}</th>
            <th>{{ t("router.colConnected") }}</th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="c in overview.clients" :key="c.macAddress">
            <td>{{ c.name }}</td>
            <td>{{ c.ipAddress || "-" }}</td>
            <td>{{ c.iface }}</td>
            <td>{{ formatNum(c.signalStrength, 0) }}</td>
            <td>{{ formatNum(c.snr, 0) }}</td>
            <td>{{ formatDuration(c.associatedTimeS) }}</td>
          </tr>
        </tbody>
      </table>
    </div>
  </section>
</template>

<style scoped>
.alerts {
  display: flex;
  flex-wrap: wrap;
  gap: 0.4rem;
  margin: 0.5rem 0;
}

.actions-grid {
  margin-top: 0.75rem;
}

.table-wrap {
  overflow-x: auto;
}

table {
  width: 100%;
  border-collapse: collapse;
  font-size: 0.86rem;
}

th,
td {
  text-align: left;
  padding: 0.45rem 0.6rem;
  border-bottom: 1px solid var(--line);
  white-space: nowrap;
}

th {
  color: var(--ink-soft);
  font-weight: 600;
}
</style>
