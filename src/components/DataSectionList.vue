<script setup lang="ts">
import { computed, ref, watch } from "vue";
import type { DataSection } from "../lib/types";
import { t } from "../lib/i18n";

const props = defineProps<{
  sections: DataSection[];
  titleNamespace: string;
  fetchSection: (key: string) => Promise<DataSection>;
}>();

const expandedSections = ref<Record<string, boolean>>({});
const loadingSection = ref<Record<string, boolean>>({});
const sectionOverrides = ref<Record<string, DataSection>>({});
const errorMessage = ref<string | null>(null);

const displayedSections = computed(() =>
  props.sections.map((base) => sectionOverrides.value[base.key] ?? base)
);

// A fresh snapshot brings fresh eager sections; only lazily-loaded (idle)
// sections need their fetched override preserved across refreshes.
watch(
  () => props.sections,
  () => {
    const next: Record<string, DataSection> = {};
    for (const base of props.sections) {
      if (base.status === "idle" && expandedSections.value[base.key] && sectionOverrides.value[base.key]) {
        next[base.key] = sectionOverrides.value[base.key];
      }
    }
    sectionOverrides.value = next;
  }
);

function isExpanded(key: string) {
  return !!expandedSections.value[key];
}

function isSectionLoading(key: string) {
  return !!loadingSection.value[key];
}

async function loadSection(key: string, opts: { silent?: boolean } = {}) {
  if (isSectionLoading(key)) return;
  loadingSection.value = { ...loadingSection.value, [key]: true };
  if (!opts.silent) errorMessage.value = null;

  try {
    const data = await props.fetchSection(key);
    sectionOverrides.value = { ...sectionOverrides.value, [key]: data };
  } catch (e) {
    errorMessage.value = e instanceof Error ? e.message : String(e);
  } finally {
    loadingSection.value = { ...loadingSection.value, [key]: false };
  }
}

async function toggleSection(key: string) {
  const next = !expandedSections.value[key];
  expandedSections.value = { ...expandedSections.value, [key]: next };
  if (!next) return;

  const section = displayedSections.value.find((s) => s.key === key);
  if (section && (section.status === "idle" || !section.raw)) {
    await loadSection(key, { silent: true });
  }
}

function badge(section: DataSection) {
  if (section.status === "ok") return { text: t("common.ok"), cls: "good" };
  if (section.status === "unsupported") return { text: t("common.unsupported"), cls: "warn" };
  if (section.status === "restricted") return { text: t("common.restricted"), cls: "warn" };
  if (section.status === "idle") return { text: t("common.lazy"), cls: "muted" };
  return { text: t("common.error"), cls: "bad" };
}

function sectionTitle(section: DataSection): string {
  const key = `${props.titleNamespace}.${section.key}`;
  const translated = t(key);
  return translated === key ? section.title : translated;
}
</script>

<template>
  <section class="sections">
    <p v-if="errorMessage" class="bad panel">{{ errorMessage }}</p>
    <article class="panel section-card" v-for="section in displayedSections" :key="section.key">
      <div class="section-header">
        <h3>{{ sectionTitle(section) }}</h3>
        <div class="section-tools">
          <span :class="`chip ${badge(section).cls}`">{{ badge(section).text }}</span>
          <button @click="toggleSection(section.key)">
            {{ isExpanded(section.key) ? t("common.collapse") : t("common.expand") }}
          </button>
          <button v-if="isExpanded(section.key)" @click="loadSection(section.key)">
            {{ isSectionLoading(section.key) ? t("common.refreshing") : t("common.reload") }}
          </button>
        </div>
      </div>

      <p class="meta">{{ t("common.responseField", { field: section.responseField }) }}</p>
      <p v-if="section.error" :class="section.status === 'error' ? 'bad' : 'warn'">{{ section.error }}</p>

      <pre v-if="isExpanded(section.key) && section.raw">{{ section.raw }}</pre>
    </article>
  </section>
</template>

<style scoped>
.sections {
  display: grid;
  gap: 0.75rem;
}

.section-card {
  animation: rise 0.32s ease both;
}

@keyframes rise {
  from {
    opacity: 0;
    transform: translateY(8px);
  }
  to {
    opacity: 1;
    transform: translateY(0);
  }
}

.section-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 0.5rem;
}

.section-header h3 {
  margin: 0;
  font-size: 1.03rem;
}

.section-tools {
  display: flex;
  gap: 0.4rem;
  align-items: center;
}

pre {
  margin: 0.4rem 0 0;
  white-space: pre-wrap;
  word-break: break-word;
  font-family: "JetBrains Mono", monospace;
  font-size: 0.78rem;
  color: var(--ink);
  background: rgba(255, 255, 255, 0.04);
  border: 1px solid var(--line);
  border-radius: 12px;
  padding: 0.75rem;
  max-height: 290px;
  overflow: auto;
}
</style>
