<script setup lang="ts">
import { useRouter } from "vue-router";
import { useI18n } from "vue-i18n";
import { CompassOutline } from "@vicons/ionicons5";

const router = useRouter();
const { t } = useI18n();
</script>

<template>
  <div class="nf">
    <n-result status="404" :title="t('auth.notFoundTitle')" :description="t('auth.notFoundDesc')">
      <template #icon>
        <n-icon :component="CompassOutline" />
      </template>
      <template #footer>
        <n-button type="primary" @click="router.push('/')">{{ t("auth.backHome") }}</n-button>
      </template>
    </n-result>
  </div>
</template>

<style scoped>
.nf {
  min-height: 100vh;
  display: flex;
  align-items: center;
  justify-content: center;
}
</style>
