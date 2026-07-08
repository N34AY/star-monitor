export function formatNum(v: number | null | undefined, digits = 2) {
  if (v == null || Number.isNaN(v)) return "-";
  return v.toFixed(digits);
}

export function formatInt(v: number | null | undefined) {
  if (v == null || Number.isNaN(v)) return "-";
  return `${Math.round(v)}`;
}

export function formatDuration(v: number | null | undefined) {
  if (!v && v !== 0) return "-";
  const total = Math.max(0, Math.floor(v));
  const h = Math.floor(total / 3600);
  const m = Math.floor((total % 3600) / 60);
  const s = total % 60;
  return `${h}h ${m}m ${s}s`;
}

export function formatPointing(az: number | null | undefined, el: number | null | undefined) {
  if (az == null || el == null) return "-";
  return `AZ ${az.toFixed(1)}°, EL ${el.toFixed(1)}°`;
}

export function titleCaseFromConst(name: string) {
  return name
    .toLowerCase()
    .split("_")
    .map((w) => (w.length ? w[0].toUpperCase() + w.slice(1) : w))
    .join(" ");
}
