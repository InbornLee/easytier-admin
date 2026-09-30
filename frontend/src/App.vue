<script setup lang="ts">
import { onMounted, onUnmounted } from "vue";
import { useRouter } from "vue-router";
import { useAuthStore } from "@/stores/auth";
import { naiveTheme, themeOverrides } from "@/utils/theme";

const router = useRouter();
const auth = useAuthStore();

function onUnauthorized() {
  auth.clear();
  const current = router.currentRoute.value.name;
  if (current !== "login" && current !== "setup") {
    router.push({ name: "login" });
  }
}

onMounted(() => window.addEventListener("et:unauthorized", onUnauthorized));
onUnmounted(() => window.removeEventListener("et:unauthorized", onUnauthorized));
</script>

<template>
  <n-config-provider :theme="naiveTheme" :theme-overrides="themeOverrides">
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
