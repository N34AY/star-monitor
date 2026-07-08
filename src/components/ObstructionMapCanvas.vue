<script setup lang="ts">
import { onMounted, ref, watch } from "vue";
import type { ObstructionMap } from "../lib/types";
import { locale, t } from "../lib/i18n";

const props = defineProps<{ map: ObstructionMap | null }>();

const canvasRef = ref<HTMLCanvasElement | null>(null);
const SIZE = 560;

function draw() {
  const canvas = canvasRef.value;
  if (!canvas) return;
  const ctx = canvas.getContext("2d");
  if (!ctx) return;

  const dpr = window.devicePixelRatio || 1;
  canvas.width = SIZE * dpr;
  canvas.height = SIZE * dpr;
  ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
  ctx.clearRect(0, 0, SIZE, SIZE);

  const cx = SIZE / 2;
  const cy = SIZE / 2;
  const maxRadius = SIZE / 2 - 28;

  const map = props.map;
  if (!map || map.numRows === 0 || map.numCols === 0 || map.snr.length === 0) {
    ctx.fillStyle = "rgba(255,255,255,0.25)";
    ctx.font = "13px system-ui, sans-serif";
    ctx.textAlign = "center";
    ctx.fillText(t("obstruction.noData"), cx, cy);
    return;
  }

  // Ring guides
  ctx.strokeStyle = "rgba(255,255,255,0.10)";
  ctx.lineWidth = 1;
  for (let ring = 1; ring <= 3; ring++) {
    ctx.beginPath();
    ctx.arc(cx, cy, (maxRadius * ring) / 3, 0, Math.PI * 2);
    ctx.stroke();
  }

  // Axis ticks + labels
  const showCompass = map.referenceFrame === "FRAME_EARTH";
  const labels = showCompass ? ["N", "E", "S", "W"] : ["0°", "90°", "180°", "270°"];
  ctx.fillStyle = "rgba(255,255,255,0.45)";
  ctx.font = "12px system-ui, sans-serif";
  ctx.textAlign = "center";
  ctx.textBaseline = "middle";
  ctx.fillText(labels[0], cx, cy - maxRadius - 12);
  ctx.fillText(labels[1], cx + maxRadius + 14, cy);
  ctx.fillText(labels[2], cx, cy + maxRadius + 12);
  ctx.fillText(labels[3], cx - maxRadius - 14, cy);

  // The response is already a flat fisheye raster (row = y pixel, col = x
  // pixel), not a polar (row=radius, col=angle) grid - the circular look
  // comes entirely from cells beyond max_theta_deg being "no data" (negative)
  // in the square image's corners. Plot each sample directly at its pixel.
  const numRows = map.numRows;
  const numCols = map.numCols;
  const cellW = (maxRadius * 2) / numCols;
  const cellH = (maxRadius * 2) / numRows;
  const cellR = Math.max(1.2, Math.min(cellW, cellH) * 0.42);
  const left = cx - maxRadius;
  const top = cy - maxRadius;

  for (let row = 0; row < numRows; row++) {
    const y = top + row * cellH + cellH / 2;
    for (let col = 0; col < numCols; col++) {
      const value = map.snr[row * numCols + col];
      const x = left + col * cellW + cellW / 2;

      if (value < 0) {
        // Negative sentinel = no data collected yet for this direction
        // (dish hasn't tracked a satellite through it) - not an obstruction.
        ctx.fillStyle = "rgba(255,255,255,0.05)";
      } else {
        // 0 = obstructed, 1 = clear line of sight; interpolate red -> white.
        const clarity = Math.min(1, value);
        const r = Math.round(208 + (255 - 208) * clarity);
        const g = Math.round(59 + (255 - 59) * clarity);
        const b = Math.round(59 + (255 - 59) * clarity);
        ctx.fillStyle = `rgb(${r},${g},${b})`;
      }
      ctx.beginPath();
      ctx.arc(x, y, cellR, 0, Math.PI * 2);
      ctx.fill();
    }
  }

  // Boresight center marker
  ctx.fillStyle = "rgba(255,255,255,0.6)";
  ctx.beginPath();
  ctx.arc(cx, cy, 2.5, 0, Math.PI * 2);
  ctx.fill();
}

onMounted(draw);
watch(() => props.map, draw, { deep: false });
watch(locale, draw);
</script>

<template>
  <div class="map-wrap">
    <canvas ref="canvasRef" :style="{ width: '100%', maxWidth: SIZE + 'px', aspectRatio: '1 / 1' }"></canvas>
  </div>
</template>

<style scoped>
.map-wrap {
  display: flex;
  justify-content: center;
  padding: 0.5rem;
}

canvas {
  display: block;
}
</style>
