<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from "vue";
import { useRouter } from "vue-router";
import { enUS, dateEnUS, dateZhCN, zhCN } from "naive-ui";
import { useAuthStore } from "@/stores/auth";
import { naiveTheme, themeOverrides } from "@/utils/theme";
import { getLocale, type AppLocale } from "@/i18n";

const router = useRouter();
const auth = useAuthStore();

const locale = ref<AppLocale>(getLocale());
function syncLocale() {
  locale.value = getLocale();
}

const naiveLocale = computed(() => (locale.value === "zh-CN" ? zhCN : enUS));
const naiveDateLocale = computed(() => (locale.value === "zh-CN" ? dateZhCN : dateEnUS));

function onUnauthorized() {
  auth.clear();
  const current = router.currentRoute.value.name;
  if (current !== "login" && current !== "setup") {
    router.push({ name: "login" });
  }
}

onMounted(() => {
  window.addEventListener("et:unauthorized", onUnauthorized);
  window.addEventListener("et:locale", syncLocale);
});
onUnmounted(() => {
  window.removeEventListener("et:unauthorized", onUnauthorized);
  window.removeEventListener("et:locale", syncLocale);
});
</script>

<template>
  <n-config-provider
    :theme="naiveTheme"
    :theme-overrides="themeOverrides"
    :locale="naiveLocale"
    :date-locale="naiveDateLocale"
  >
    <n-loading-bar-provider>
      <n-message-provider>
        <n-dialog-provider>
          <n-notification-provider>
            <n-global-style />
            <router-view />
          </n-notification-provider>
        </n-dialog-provider>
      </n-message-provider>
    </n-loading-bar-provider>
  </n-config-provider>
</template>
