<script setup lang="ts">
import { computed, ref } from "vue";

type Series = { name: string; color: string; points: number[] };

const props = defineProps<{
  series: Series[];
  unit: string;
  height?: number;
}>();

const H = props.height ?? 180;
const PAD_L = 42;
const PAD_R = 12;
const PAD_T = 10;
const PAD_B = 22;
const W = 600;

const maxLen = computed(() => Math.max(1, ...props.series.map((s) => s.points.length)));

const yBounds = computed(() => {
  const all = props.series.flatMap((s) => s.points).filter((v) => Number.isFinite(v));
  if (all.length === 0) return { min: 0, max: 1 };
  let min = Math.min(0, ...all);
  let max = Math.max(...all);
  if (min === max) {
    min -= 1;
    max += 1;
  }
  const pad = (max - min) * 0.08;
  return { min: min - pad, max: max + pad };
});

function xFor(i: number) {
  const n = maxLen.value;
  if (n <= 1) return PAD_L;
  return PAD_L + (i / (n - 1)) * (W - PAD_L - PAD_R);
}

function yFor(v: number) {
  const { min, max } = yBounds.value;
  const t = (v - min) / (max - min || 1);
  return H - PAD_B - t * (H - PAD_T - PAD_B);
}

function pathFor(points: number[]) {
  return points
    .map((v, i) => `${i === 0 ? "M" : "L"}${xFor(i).toFixed(1)},${yFor(v).toFixed(1)}`)
    .join(" ");
}

const yTicks = computed(() => {
  const { min, max } = yBounds.value;
  const steps = 4;
  return Array.from({ length: steps + 1 }, (_, i) => min + ((max - min) * i) / steps);
});

const hoverIndex = ref<number | null>(null);
const hoverX = ref(0);

function onMove(e: MouseEvent, svgEl: SVGSVGElement) {
  const rect = svgEl.getBoundingClientRect();
  const relX = ((e.clientX - rect.left) / rect.width) * W;
  const n = maxLen.value;
  if (n <= 1) {
    hoverIndex.value = 0;
    hoverX.value = xFor(0);
    return;
  }
  const idx = Math.round(((relX - PAD_L) / (W - PAD_L - PAD_R)) * (n - 1));
  hoverIndex.value = Math.min(n - 1, Math.max(0, idx));
  hoverX.value = xFor(hoverIndex.value);
}

function onLeave() {
  hoverIndex.value = null;
}

const hasData = computed(() => props.series.some((s) => s.points.length > 0));
</script>

<template>
  <div class="chart-wrap">
    <div class="legend" v-if="series.length > 1">
      <span class="legend-item" v-for="s in series" :key="s.name">
        <span class="swatch" :style="{ background: s.color }"></span>{{ s.name }}
      </span>
    </div>
    <svg
      v-if="hasData"
      :viewBox="`0 0 ${W} ${H}`"
      preserveAspectRatio="none"
      class="chart-svg"
      @mousemove="(e) => onMove(e, e.currentTarget as SVGSVGElement)"
      @mouseleave="onLeave"
    >
      <line
        v-for="(t, i) in yTicks"
        :key="i"
        :x1="PAD_L"
        :x2="W - PAD_R"
        :y1="yFor(t)"
        :y2="yFor(t)"
        class="gridline"
      />
      <text v-for="(t, i) in yTicks" :key="'t' + i" :x="PAD_L - 6" :y="yFor(t) + 3" class="tick-label">
        {{ t.toFixed(1) }}
      </text>

      <path v-for="s in series" :key="s.name" :d="pathFor(s.points)" fill="none" :stroke="s.color" stroke-width="2" />

      <line v-if="hoverIndex !== null" :x1="hoverX" :x2="hoverX" :y1="PAD_T" :y2="H - PAD_B" class="crosshair" />
      <circle
        v-for="s in series"
        :key="'d' + s.name"
        v-show="hoverIndex !== null && s.points[hoverIndex!] != null"
        :cx="hoverX"
        :cy="hoverIndex !== null ? yFor(s.points[hoverIndex] ?? 0) : 0"
        r="3"
        :fill="s.color"
      />
    </svg>
    <p v-else class="no-data">No data</p>

    <div
      v-if="hoverIndex !== null"
      class="tooltip"
      :style="{ left: `${(hoverX / W) * 100}%` }"
    >
      <div v-for="s in series" :key="s.name" class="tooltip-row">
        <span class="swatch" :style="{ background: s.color }"></span>
        {{ s.name }}: <strong>{{ s.points[hoverIndex]?.toFixed(2) ?? "-" }}</strong> {{ unit }}
      </div>
    </div>
  </div>
</template>

<style scoped>
.chart-wrap {
  position: relative;
}

.legend {
  display: flex;
  gap: 1rem;
  margin-bottom: 0.4rem;
  font-size: 0.8rem;
  color: var(--ink-soft);
}

.legend-item {
  display: inline-flex;
  align-items: center;
  gap: 0.35rem;
}

.swatch {
  width: 9px;
  height: 9px;
  border-radius: 2px;
  display: inline-block;
}

.chart-svg {
  width: 100%;
  height: auto;
  display: block;
}

.gridline {
  stroke: rgba(255, 255, 255, 0.08);
  stroke-width: 1;
}

.tick-label {
  fill: var(--muted);
  font-size: 9px;
  text-anchor: end;
}

.crosshair {
  stroke: rgba(255, 255, 255, 0.25);
  stroke-width: 1;
}

.no-data {
  color: var(--muted);
  font-size: 0.85rem;
  padding: 2rem 0;
  text-align: center;
}

.tooltip {
  position: absolute;
  top: 0;
  transform: translateX(-50%);
  background: #17171a;
  border: 1px solid var(--line);
  border-radius: 8px;
  padding: 0.4rem 0.55rem;
  font-size: 0.78rem;
  white-space: nowrap;
  pointer-events: none;
  z-index: 5;
}

.tooltip-row {
  display: flex;
  align-items: center;
  gap: 0.35rem;
}
</style>
