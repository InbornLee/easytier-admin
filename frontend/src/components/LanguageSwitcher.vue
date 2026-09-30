<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from "vue";
import { NIcon } from "naive-ui";
import { LanguageOutline } from "@vicons/ionicons5";
import { SUPPORTED_LOCALES, getLocale, setLocale, type AppLocale } from "@/i18n";

const locale = ref<AppLocale>(getLocale());

function sync() {
  locale.value = getLocale();
}

const options = SUPPORTED_LOCALES.map((l) => ({ label: l.label, key: l.value }));
const label = computed(
  () => SUPPORTED_LOCALES.find((l) => l.value === locale.value)?.label ?? "",
);

function select(key: string) {
  setLocale(key as AppLocale);
}

onMounted(() => window.addEventListener("et:locale", sync));
onUnmounted(() => window.removeEventListener("et:locale", sync));
</script>

<template>
  <n-dropdown :options="options" trigger="click" @select="select">
    <n-button quaternary size="small">
      <template #icon><n-icon :component="LanguageOutline" /></template>
      {{ label }}
    </n-button>
  </n-dropdown>
</template>
