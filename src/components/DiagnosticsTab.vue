<script setup lang="ts">
import { invoke } from "@tauri-apps/api/core";
import DataSectionList from "./DataSectionList.vue";
import type { DataSection } from "../lib/types";
import { t } from "../lib/i18n";

const props = defineProps<{
  dishAddress: string;
  sections: DataSection[];
  routerEnabled: boolean;
  routerAddress: string;
  routerSections: DataSection[];
}>();

function fetchDishSection(key: string) {
  return invoke<DataSection>("fetch_starlink_section", { dishAddress: props.dishAddress, sectionKey: key });
}

function fetchRouterSection(key: string) {
  return invoke<DataSection>("fetch_router_section", { routerAddress: props.routerAddress, sectionKey: key });
}
</script>

<template>
  <h2 v-if="routerEnabled && routerSections.length">{{ t("tabs.dish") }}</h2>
  <DataSectionList :sections="sections" title-namespace="diagnostics.sections" :fetch-section="fetchDishSection" />

  <template v-if="routerEnabled && routerSections.length">
    <h2 class="group-heading">{{ t("tabs.router") }}</h2>
    <DataSectionList
      :sections="routerSections"
      title-namespace="router.sections"
      :fetch-section="fetchRouterSection"
    />
  </template>
</template>

<style scoped>
.group-heading {
  margin-top: 1.25rem;
}
</style>
